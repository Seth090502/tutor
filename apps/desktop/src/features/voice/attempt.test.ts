import { describe, expect, it } from "vitest"

import type { StudyAssistance, StudyAttempt } from "@/lib/study"
import { attemptKey, correctOptionId, offerWords, optionList, ratingWord, reviewWords } from "./attempt"

describe("optionList", () => {
  it("puts options in A-D order whatever order the server sent", () => {
    expect(optionList({ B: "Both sides agree", A: "f(a) is defined", D: "f is smooth", C: "Nothing" }).map((o) => o.id)).toEqual([
      "A",
      "B",
      "C",
      "D",
    ])
  })

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

describe("reviewWords", () => {
  const hint: StudyAssistance = {
    kind: "nudge",
    rung: 1,
    source: "ui",
    timestamp: 1,
    content_ref: "hint:1",
    text: "t",
    before_commit: true,
  }
  const closed = (
    disposition: NonNullable<StudyAttempt["closure"]>["scheduler_disposition"],
    extra: Partial<StudyAttempt> = {},
    reason: NonNullable<StudyAttempt["closure"]>["reason"] = "correct",
  ) =>
    ({
      status: "closed",
      assistance: [],
      first_response: { response_id: "r", response: "B", input_mode: "typed", committed_at: 1, transcript_confirmation: null },
      closure: { reason, closed_at: 2, scheduler_disposition: disposition },
      ...extra,
    }) as StudyAttempt

  it("says nothing while the attempt is live", () => {
    expect(reviewWords({ closure: null } as StudyAttempt)).toBeNull()
  })

  it("names why each rating was given (review-policy/2)", () => {
    expect(reviewWords(closed({ eligible_for_fsrs: true, rating: "good", provenance: "verified", first_outcome: "correct" }))).toBe(
      "Good, correct without help",
    )
    expect(
      reviewWords(closed({ eligible_for_fsrs: true, rating: "again", provenance: "assisted", first_outcome: "correct" }, { assistance: [hint] })),
    ).toBe("Again, because a hint was used")
    expect(
      reviewWords(
        closed({ eligible_for_fsrs: true, rating: "again", provenance: "assisted", first_outcome: "correct" }, {
          assistance: [{ ...hint, kind: "source_open", rung: null }],
        }),
      ),
    ).toBe("Again, because you had help before answering")
    expect(reviewWords(closed({ eligible_for_fsrs: true, rating: "again", provenance: "verified", first_outcome: "incorrect" }))).toBe(
      "Again, because the first answer was not correct",
    )
    expect(
      reviewWords(
        closed(
          { eligible_for_fsrs: true, rating: "again", provenance: "verified", first_outcome: "incorrect" },
          { first_response: { response_id: "r", response: "", input_mode: "dont_know", committed_at: 1, transcript_confirmation: null } },
          "revealed",
        ),
      ),
    ).toBe("Again, because you did not know it")
    expect(reviewWords(closed({ eligible_for_fsrs: true, rating: "good", provenance: "self_report", first_outcome: "correct" }))).toBe(
      "Good, by your own judgement",
    )
  })

  it("says when nothing was scheduled", () => {
    expect(reviewWords(closed({ eligible_for_fsrs: false, rating: null }, {}, "invalidated"))).toBe(
      "Not scheduled: the question was set aside",
    )
    expect(reviewWords(closed({ eligible_for_fsrs: false, rating: null }, {}, "abandoned"))).toBe(
      "Not scheduled: the question was left before a check",
    )
  })
})

describe("offerWords", () => {
  it("words the known offer and shows others readably", () => {
    expect(offerWords("worked_example_then_fresh_problem")).toMatch(/worked example.*fresh problem/)
    expect(offerWords("something_new")).toBe("something new")
  })
})
