import { describe, expect, it } from "vitest"

import type { StudyAttempt, StudyQueueItem } from "@/lib/study"
import {
  assistanceNote,
  canHint,
  cardMeta,
  diffChars,
  feedbackLine,
  formatDue,
  hintBlocker,
  nextEntry,
  orderQueue,
  outcomeOf,
  ratingLine,
  targetSentence,
  verdictWord,
} from "./drill"

const item = (entry_id: number, ...tags: string[]): StudyQueueItem => ({
  entry_id,
  prompt: `p${entry_id}`,
  question_type: "short_answer",
  options: {},
  lang: "es",
  tags,
  due_at: 1,
})

const attempt = (over: Partial<StudyAttempt> = {}, rungs = 4): StudyAttempt => ({
  attempt_id: "a1",
  session_id: "s1",
  entry_id: 7,
  item_version: "v",
  mode: "practice",
  status: "remediating",
  opened_at: 1,
  item: { prompt: "the river", question_type: "short_answer", options: {}, lang: "es", rungs_available: rungs },
  first_response: null,
  repairs: [],
  confidence: null,
  checks: [],
  assistance: [],
  pending_proposal: null,
  closure: null,
  offer: null,
  released: null,
  ...over,
})
const hint = (rung: number) => ({ kind: `hint_${rung}`, rung, source: "ui" as const, timestamp: 1, content_ref: null, text: `h${rung}`, before_commit: false })

describe("queue and card tags", () => {
  it("reads the card kind from the tags", () => {
    expect(cardMeta(item(1, "lang:es", "Card:Production"))).toEqual({ kind: "production" })
    expect(cardMeta(item(1, "lang:ja", "card:listening"))).toEqual({ kind: "listening" })
    expect(cardMeta(item(1, "lang:es"))).toEqual({ kind: "unknown" })
    expect(cardMeta(item(1, "lang:es", "card:cloze"))).toEqual({ kind: "unknown" })
  })

  it("puts production cards first and keeps the server's order otherwise", () => {
    const items = [
      item(1, "lang:es", "card:listening"),
      item(2, "lang:es"),
      item(4, "lang:es", "card:recognition"),
      item(5, "lang:es", "card:production"),
      item(6, "lang:es", "card:production"),
    ]
    expect(orderQueue(items).map((i) => i.entry_id)).toEqual([5, 6, 4, 1, 2])
    expect(orderQueue([])).toEqual([])
  })

  it("does not filter by language: the server did", () => {
    expect(orderQueue([item(1, "lang:fr", "card:production")])).toHaveLength(1)
  })

  it("picks the first card not finished and not on screen", () => {
    const queue = [item(1), item(2), item(3)]
    expect(nextEntry(queue, [], null)?.entry_id).toBe(1)
    expect(nextEntry(queue, [1], null)?.entry_id).toBe(2)
    expect(nextEntry(queue, [1], attempt({ entry_id: 2 }))?.entry_id).toBe(3)
    expect(nextEntry(queue, [1, 2], attempt({ entry_id: 3 }))).toBeUndefined()
  })
})

describe("hints", () => {
  it("open only after a miss, up to the server's rung count", () => {
    expect(canHint(attempt({ status: "open" }))).toBe(false)
    expect(canHint(attempt({ status: "ready" }))).toBe(false)
    expect(canHint(attempt())).toBe(true)
    expect(canHint(attempt({ assistance: [hint(1), hint(2)] }, 2))).toBe(false)
    expect(hintBlocker(attempt({ assistance: [hint(1), hint(2)] }, 2))).toBe("No more hints for this card.")
  })

  it("never pass rung 4, and say when a card has none", () => {
    expect(canHint(attempt({ assistance: [hint(1), hint(2), hint(3), hint(4)] }, 9))).toBe(false)
    expect(canHint(attempt({}, 0))).toBe(false)
    expect(hintBlocker(attempt({}, 0))).toBe("This card has no hints.")
    expect(hintBlocker(attempt())).toBeNull()
    expect(hintBlocker(attempt({ status: "open" }))).toBeNull()
  })

  it("does not count a gloss as a hint", () => {
    const gloss = { ...hint(1), kind: "gloss", rung: null, text: null }
    expect(canHint(attempt({ assistance: [gloss] }, 1))).toBe(true)
  })
})

describe("feedback", () => {
  it("speaks the target: the answer for production cards, the prompt otherwise", () => {
    expect(targetSentence("production", "the river", "el río")).toBe("el río")
    expect(targetSentence("recognition", "el río", "the river")).toBe("el río")
  })

  it("names the accent slip without a reference answer", () => {
    expect(feedbackLine({ verdict: "incorrect", diagnostic: "accent" })).toMatch(/Check your accents/)
    expect(feedbackLine({ verdict: "incorrect", diagnostic: null })).toMatch(/does not match/)
    expect(feedbackLine({ verdict: "correct", diagnostic: null })).toBe("Your answer matches the reference.")
    expect(feedbackLine({ verdict: "unverifiable", diagnostic: null })).toMatch(/cannot be checked automatically/)
    expect(verdictWord({ verdict: "incorrect", diagnostic: "accent" })).toBe("Accent slip")
    expect(verdictWord({ verdict: "incorrect", diagnostic: null })).toBe("Not quite")
    expect(verdictWord({ verdict: "unverifiable", diagnostic: null })).toBe("Compare")
  })

  it("marks the characters that differ in an accent slip", () => {
    expect(diffChars("el rio", "el río")).toEqual([
      { text: "el r", differs: false },
      { text: "i", differs: true },
      { text: "o", differs: false },
    ])
    expect(diffChars("rio", "el río")).toEqual([{ text: "rio", differs: false }])
    expect(diffChars("café", "café")).toEqual([{ text: "café", differs: false }])
  })

  it("describes assistance", () => {
    expect(assistanceNote(0)).toBe("No hints used.")
    expect(assistanceNote(1)).toMatch(/^1 hint used/)
    expect(assistanceNote(3)).toMatch(/^3 hints used/)
  })
})

describe("closure", () => {
  const closed = (reason: NonNullable<StudyAttempt["closure"]>["reason"], disposition: NonNullable<StudyAttempt["closure"]>["scheduler_disposition"]) =>
    attempt({ status: "closed", closure: { reason, closed_at: 2, scheduler_disposition: disposition } })

  it("words the rating with its provenance", () => {
    expect(ratingLine(closed("correct", { eligible_for_fsrs: true, rating: "good", provenance: "verified" }))).toEqual({
      rating: "Good",
      why: "The app checked your first answer and you had no help before it.",
    })
    expect(ratingLine(closed("correct", { eligible_for_fsrs: true, rating: "again", provenance: "assisted" })).why).toMatch(/assisted/)
    expect(ratingLine(closed("self_assessed", { eligible_for_fsrs: true, rating: "good", provenance: "self_report" })).why).toMatch(/your own judgement/)
  })

  it("says so when no review was recorded", () => {
    expect(ratingLine(closed("abandoned", { eligible_for_fsrs: false, rating: null }))).toEqual({ rating: null, why: "No review was recorded for this card." })
    expect(ratingLine(attempt())).toEqual({ rating: null, why: "No review was recorded for this card." })
  })

  it("names how the attempt ended", () => {
    const d = { eligible_for_fsrs: true, rating: "again" as const }
    expect(outcomeOf(closed("revealed", d))).toEqual({ word: "Answer shown", ok: false })
    expect(outcomeOf(closed("abandoned", d)).ok).toBeNull()
    expect(outcomeOf({ ...closed("self_assessed", d), self_assessment: { judged: "correct", recorded_at: 3 } })).toEqual({ word: "You had it", ok: true })
    expect(outcomeOf({ ...closed("self_assessed", d), self_assessment: { judged: "incorrect", recorded_at: 3 } }).word).toBe("You missed it")
    const miss = { response_id: "r", verdict: "incorrect" as const, checker_id: "c", checker_version: "1", assumptions: [], limitations: [], evidence_refs: [], checked_at: 1, diagnostic: null }
    expect(outcomeOf({ ...closed("correct", d), checks: [miss] }).word).toBe("Correct on a repair")
  })

  it("formats the next review time, or nothing", () => {
    expect(formatDue(null)).toBeNull()
    expect(formatDue(1790000000)).toMatch(/2026/)
  })
})
