import { describe, expect, it } from "vitest"

import {
  answerMessage,
  attemptKey,
  attemptReducer,
  confidenceMessage,
  correctOptionId,
  optionList,
  ratingWord,
  type Attempt,
} from "./attempt"
import type { TutorEvent } from "./tools"

const question: TutorEvent = {
  type: "question",
  attempt_id: "a1",
  question_id: 7,
  question: "Which limit exists?",
  question_type: "multiple_choice",
  options: { B: "Both sides agree", A: "f(a) is defined", D: "f is smooth", C: "Nothing" },
}

const run = (...events: Parameters<typeof attemptReducer>[1][]) =>
  events.reduce<Attempt | null>((s, e) => attemptReducer(s, e), null)

describe("attemptReducer", () => {
  it("starts an attempt from a question event, with options in A-D order", () => {
    const a = run(question)!
    expect(a.attemptId).toBe("a1")
    expect(a.options.map((o) => o.id)).toEqual(["A", "B", "C", "D"])
    expect(a.rung).toBe(0)
    expect(a.checked).toBeNull()
  })

  it("ignores events before any question and events for another attempt", () => {
    expect(run({ type: "confidence", attempt_id: "a1", level: 2 })).toBeNull()
    const a = run(question, { type: "confidence", attempt_id: "zzz", level: 4 })!
    expect(a.confidence).toBeNull()
  })

  it("takes confidence from the agent and from the learner's own key", () => {
    expect(run(question, { type: "confidence", attempt_id: "a1", level: 3 })!.confidence).toBe(3)
    expect(run(question, { type: "rated", attempt_id: "a1", level: 1 })!.confidence).toBe(1)
  })

  it("records the chosen answer", () => {
    expect(run(question, { type: "answered", attempt_id: "a1", choice: "B" })!.answer).toBe("B")
  })

  it("fills the hint ladder upward and keeps the latest text, which may be null", () => {
    const a = run(
      question,
      { type: "hint", attempt_id: "a1", rung: 1, text: "Think about both sides", is_answer: false },
      { type: "hint", attempt_id: "a1", rung: 2, text: null, is_answer: false },
    )!
    expect(a.rung).toBe(2)
    expect(a.hintText).toBeNull()
    // A late, lower rung never empties the ladder.
    const b = attemptReducer(a, { type: "hint", attempt_id: "a1", rung: 1, text: "old", is_answer: false })!
    expect(b.rung).toBe(2)
    expect(b.hintText).toBeNull()
  })

  it("flags the answer rung", () => {
    const a = run(question, { type: "hint", attempt_id: "a1", rung: 5, text: "B", is_answer: true })!
    expect(a.rung).toBe(5)
    expect(a.hintIsAnswer).toBe(true)
  })

  it("holds the verdict, explanation and misconception after the check, then the review", () => {
    const a = run(
      question,
      {
        type: "checked",
        attempt_id: "a1",
        correct: false,
        correct_answer: "B",
        explanation: "One-sided limits must agree.",
        misconception: "The limit must equal f(a).",
      },
      { type: "reviewed", attempt_id: "a1", rating: "again", next_due: "Oct 2, 9:30 AM" },
    )!
    expect(a.checked).toEqual({
      correct: false,
      correctAnswer: "B",
      explanation: "One-sided limits must agree.",
      misconception: "The limit must equal f(a).",
    })
    expect(a.reviewed).toEqual({ rating: "again", nextDue: "Oct 2, 9:30 AM" })
  })

  it("clears the attempt on reset, such as when a new call connects", () => {
    expect(run(question, { type: "reset" })).toBeNull()
  })

  it("replaces the attempt when the next question arrives", () => {
    const a = run(question, { type: "confidence", attempt_id: "a1", level: 3 }, { ...question, attempt_id: "a2" } as TutorEvent)!
    expect(a.attemptId).toBe("a2")
    expect(a.confidence).toBeNull()
  })
})

describe("optionList", () => {
  it("sorts numbered options numerically", () => {
    expect(optionList({ "10": "x", "2": "y", "1": "z" }).map((o) => o.id)).toEqual(["1", "2", "10"])
  })
})

describe("correctOptionId", () => {
  const options = optionList({ A: "Seven", B: "Both sides agree" })

  it("finds the option by key or by text, ignoring case and spacing", () => {
    expect(correctOptionId(options, " b ")).toBe("B")
    expect(correctOptionId(options, "both sides agree")).toBe("B")
  })

  it("returns null when the answer names no option", () => {
    expect(correctOptionId(options, "7")).toBeNull()
    expect(correctOptionId(options, "")).toBeNull()
  })
})

describe("messages to the agent", () => {
  it("match the C8 wording", () => {
    expect(answerMessage("B")).toBe("My answer: B")
    expect(confidenceMessage(3)).toBe("Confidence: 3")
  })
})

describe("attemptKey", () => {
  it("maps A-D to answers only while those options exist", () => {
    expect(attemptKey("b", 4)).toEqual({ kind: "answer", index: 1 })
    expect(attemptKey("D", 4)).toEqual({ kind: "answer", index: 3 })
    expect(attemptKey("d", 3)).toBeNull()
    expect(attemptKey("a", 0)).toBeNull()
  })

  it("maps 1-4 to confidence, even with no options", () => {
    expect(attemptKey("1", 0)).toEqual({ kind: "confidence", level: 1 })
    expect(attemptKey("4", 4)).toEqual({ kind: "confidence", level: 4 })
  })

  it("ignores everything else", () => {
    for (const k of ["5", "0", "e", "Enter", "ArrowDown", " "]) expect(attemptKey(k, 4)).toBeNull()
  })
})

describe("ratingWord", () => {
  it("capitalises the server's rating", () => {
    expect(ratingWord("good")).toBe("Good")
    expect(ratingWord("AGAIN")).toBe("Again")
    expect(ratingWord("")).toBe("")
  })
})
