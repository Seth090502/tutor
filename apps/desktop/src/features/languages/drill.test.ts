import { describe, expect, it } from "vitest"

import {
  assistanceNote,
  canCheck,
  canHint,
  canReview,
  cardMeta,
  diffChars,
  drillReducer,
  feedbackLine,
  hintBlocker,
  needsJudgement,
  newDrill,
  orderQueue,
  targetSentence,
  type Card,
  type CheckResult,
  type DrillAction,
  type DrillState,
} from "./drill"

const card = (over: Partial<Card> = {}): Card => ({
  questionId: 7,
  attemptId: "att-1",
  prompt: "the river",
  kind: "production",
  rungsAvailable: 4,
  ...over,
})
const check = (over: Partial<CheckResult> = {}): CheckResult => ({
  correct: true,
  correctAnswer: "el río",
  explanation: "Tatoeba #1",
  misconception: null,
  diagnostic: null,
  ...over,
})
const run = (s: DrillState, ...actions: DrillAction[]) => actions.reduce(drillReducer, s)
const typed = (text: string, s = newDrill(card())) => run(s, { type: "draft", text })
const hint = (rung: number): DrillAction => ({ type: "hint:done", hint: { rung, text: `h${rung}`, isAnswer: false } })

describe("hint ladder", () => {
  it("needs an attempt before the first hint", () => {
    const s = newDrill(card())
    expect(canHint(s)).toBe(false)
    expect(hintBlocker(s)).toBe("Type an attempt first.")
    expect(drillReducer(s, { type: "hint:start" })).toBe(s)
    expect(canHint(typed("  "))).toBe(false)
  })

  it("needs a new attempt before each further rung", () => {
    let s = run(typed("el rio"), { type: "hint:start" }, hint(1))
    expect(s.hints).toHaveLength(1)
    expect(canHint(s)).toBe(false)
    expect(hintBlocker(s)).toBe("Change your attempt before the next hint.")
    s = run(s, { type: "draft", text: "el rio " })
    expect(canHint(s)).toBe(false) // whitespace is not a new attempt
    s = run(s, { type: "draft", text: "la rio" })
    expect(canHint(s)).toBe(true)
  })

  it("stops at the server's rung count and never passes rung 4", () => {
    const limited = run(
      typed("a", newDrill(card({ rungsAvailable: 2 }))),
      { type: "hint:start" },
      hint(1),
      { type: "draft", text: "b" },
      { type: "hint:start" },
      hint(2),
      { type: "draft", text: "c" },
    )
    expect(canHint(limited)).toBe(false)
    expect(hintBlocker(limited)).toBe("No more hints for this card.")

    let s = typed("x")
    for (let i = 1; i <= 4; i++) s = run(s, { type: "draft", text: `try ${i}` }, { type: "hint:start" }, hint(i))
    expect(s.hints).toHaveLength(4)
    expect(canHint(run(s, { type: "draft", text: "again" }))).toBe(false)
  })

  it("is unavailable on a card with no hints, and while another request is pending", () => {
    const none = typed("a", newDrill(card({ rungsAvailable: 0 })))
    expect(canHint(none)).toBe(false)
    expect(hintBlocker(none)).toBe("This card has no hints.")
    const busy = run(typed("a"), { type: "hint:start" })
    expect(busy.pending).toBe("hint")
    expect(drillReducer(busy, { type: "hint:start" })).toBe(busy)
  })

  it("stops if the server ever returns the answer rung", () => {
    const s = run(
      typed("a"),
      { type: "hint:start" },
      { type: "hint:done", hint: { rung: 5, text: "el río", isAnswer: true } },
      { type: "draft", text: "b" },
    )
    expect(canHint(s)).toBe(false)
  })
})

describe("check", () => {
  it("needs an attempt and a confidence", () => {
    expect(canCheck(newDrill(card()))).toBe(false)
    expect(canCheck(typed("el río"))).toBe(false)
    const noText = run(newDrill(card()), { type: "confidence", level: 3 })
    expect(canCheck(noText)).toBe(false)
    const ready = run(typed("el río"), { type: "confidence", level: 3 })
    expect(canCheck(ready)).toBe(true)
  })

  it("locks the attempt and the confidence after the check", () => {
    let s = run(typed("el rio"), { type: "confidence", level: 2 }, { type: "check:start" })
    expect(s.pending).toBe("check")
    expect(drillReducer(s, { type: "confidence", level: 4 })).toBe(s)
    s = run(s, { type: "check:done", result: check({ correct: false, diagnostic: "accent" }) })
    expect(s.phase).toBe("checked")
    expect(drillReducer(s, { type: "draft", text: "changed" })).toBe(s)
    expect(drillReducer(s, { type: "confidence", level: 4 })).toBe(s)
    expect(canHint(s)).toBe(false)
    expect(drillReducer(s, { type: "check:start" })).toBe(s)
  })

  it("ignores a check result nobody asked for", () => {
    const s = run(typed("a"), { type: "confidence", level: 1 })
    expect(drillReducer(s, { type: "check:done", result: check() })).toBe(s)
  })

  it("lets a failed check be retried with the same attempt", () => {
    let s = run(typed("a"), { type: "confidence", level: 1 }, { type: "check:start" }, { type: "failed", message: "offline" })
    expect(s.error).toBe("offline")
    expect(s.pending).toBeNull()
    expect(canCheck(s)).toBe(true)
    s = run(s, { type: "draft", text: "ab" })
    expect(s.error).toBeNull()
  })
})

describe("review and self-judgement", () => {
  const checkedWith = (result: CheckResult) =>
    run(typed("a"), { type: "confidence", level: 3 }, { type: "check:start" }, { type: "check:done", result })

  it("can be reviewed straight away when the check decided", () => {
    const s = checkedWith(check({ correct: false }))
    expect(needsJudgement(s)).toBe(false)
    expect(canReview(s)).toBe(true)
    const done = run(s, { type: "review:start" }, { type: "review:done", result: { rating: "again", nextDue: "Oct 2" } })
    expect(done.phase).toBe("done")
    expect(done.review).toEqual({ rating: "again", nextDue: "Oct 2" })
  })

  it("waits for the learner's verdict when the check returned null", () => {
    let s = checkedWith(check({ correct: null }))
    expect(needsJudgement(s)).toBe(true)
    expect(canReview(s)).toBe(false)
    expect(drillReducer(s, { type: "review:start" })).toBe(s)
    s = run(s, { type: "judge", correct: true })
    expect(needsJudgement(s)).toBe(false)
    expect(s.judged).toBe(true)
    expect(canReview(s)).toBe(true)
    expect(drillReducer(s, { type: "judge", correct: false })).toBe(s) // the verdict is not editable
  })

  it("does not take a verdict when the check already decided", () => {
    const s = checkedWith(check({ correct: true }))
    expect(drillReducer(s, { type: "judge", correct: false })).toBe(s)
  })

  it("keeps the checked state when the review request fails", () => {
    const s = run(checkedWith(check()), { type: "review:start" }, { type: "failed", message: "409" })
    expect(s.phase).toBe("checked")
    expect(canReview(s)).toBe(true)
  })
})

describe("queue and card tags", () => {
  const entry = (id: number, names: string[]) => ({ entry: { id, categories: names.map((name) => ({ name })) } })

  it("reads the language and card kind from categories or tags", () => {
    expect(cardMeta({ categories: [{ name: "lang:es" }, { name: "Card:Production" }] })).toEqual({ lang: "es", kind: "production" })
    expect(cardMeta({ tags: ["lang:ja", "card:listening"] })).toEqual({ lang: "ja", kind: "listening" })
    expect(cardMeta({ categories: [{ name: "math" }] })).toEqual({ lang: null, kind: "unknown" })
    expect(cardMeta({ categories: null })).toEqual({ lang: null, kind: "unknown" })
  })

  it("keeps one language and puts production cards first", () => {
    const items = [
      entry(1, ["lang:es", "card:listening"]),
      entry(2, ["math"]),
      entry(3, ["lang:fr", "card:production"]),
      entry(4, ["lang:es", "card:recognition"]),
      entry(5, ["lang:es", "card:production"]),
      entry(6, ["lang:es", "card:production"]),
    ]
    expect(orderQueue(items, "es").map((i) => i.entry.id)).toEqual([5, 6, 4, 1])
    expect(orderQueue(items, "de")).toEqual([])
  })
})

describe("feedback", () => {
  it("speaks the target: the answer for production cards, the prompt otherwise", () => {
    expect(targetSentence(card(), check())).toBe("el río")
    expect(targetSentence(card({ kind: "recognition", prompt: "el río" }), check({ correctAnswer: "the river" }))).toBe("el río")
  })

  it("names the accent slip", () => {
    expect(feedbackLine(check({ correct: false, diagnostic: "accent" }))).toMatch(/accent or diacritic/)
    expect(feedbackLine(check({ correct: false }))).toBe("Yours does not match the reference.")
    expect(feedbackLine(check({ correct: true }))).toBe("Your answer matches the reference.")
    expect(feedbackLine(check({ correct: null }))).toMatch(/cannot be checked automatically/)
  })

  it("marks the characters that differ in an accent slip", () => {
    expect(diffChars("el rio", "el río")).toEqual([
      { text: "el r", differs: false },
      { text: "i", differs: true },
      { text: "o", differs: false },
    ])
    expect(diffChars("cafe", "café")).toEqual([
      { text: "caf", differs: false },
      { text: "e", differs: true },
    ])
    expect(diffChars("rio", "el río")).toEqual([{ text: "rio", differs: false }])
  })

  it("does not treat a decomposed accent as a difference", () => {
    expect(diffChars("café", "café")).toEqual([{ text: "café", differs: false }])
  })

  it("describes assistance", () => {
    expect(assistanceNote(0)).toBe("No hints used.")
    expect(assistanceNote(1)).toMatch(/^1 hint used/)
    expect(assistanceNote(3)).toMatch(/^3 hints used/)
  })
})
