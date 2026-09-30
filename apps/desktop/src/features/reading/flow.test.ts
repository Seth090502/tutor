import { describe, expect, it } from "vitest"

import { PREPARED, SECTION } from "./fixtures"
import {
  STEPS,
  STEP_LABEL,
  canAdvance,
  flowReducer,
  highConfidenceMiss,
  initFlow,
  nextQuestion,
  visibleSteps,
  type FlowAction,
  type FlowState,
} from "./flow"
import type { Prepared } from "./types"

const run = (s: FlowState, ...actions: FlowAction[]) => actions.reduce(flowReducer, s)
const ready = (prep: Prepared = PREPARED, mode: "section" | "paper" = "section") =>
  run(initFlow(mode), { type: "sessionOpened", sessionId: "s1", section: SECTION }, { type: "prepared", prep })
const answer = (id: string, correct: boolean | null = true, confidence: 1 | 2 | 3 | 4 = 2): FlowAction => ({
  type: "answered",
  id,
  answer: "my guess",
  confidence,
  result: { correct },
})
const SUMMARY: FlowAction = { type: "summarized", text: "s", judgement: 3, result: { covered: 4, missed: [{ unit: "u", chunk_id: "d-analysis:45" }] } }

describe("opening and preparing", () => {
  it("starts opening, then preparing once the session exists, then on the first prequestion", () => {
    let s = initFlow("section")
    expect(s).toMatchObject({ phase: "opening", sessionId: null })
    s = run(s, { type: "sessionOpened", sessionId: "s1", section: SECTION })
    expect(s).toMatchObject({ phase: "preparing", sessionId: "s1", section: SECTION })
    s = run(s, { type: "prepared", prep: PREPARED })
    expect(s).toMatchObject({ phase: "step", step: "prequestions" })
  })
  it("goes straight to reading when preparation returned no prequestions", () => {
    expect(ready({ ...PREPARED, prequestions: [] })).toMatchObject({ phase: "step", step: "reading" })
  })
  it("a failed preparation keeps the session, and retry prepares again", () => {
    let s = run(initFlow("section"), { type: "sessionOpened", sessionId: "s1", section: SECTION })
    s = run(s, { type: "failed", failure: { stage: "prepare", service: "backend", message: "no API key", offline: false } })
    expect(s).toMatchObject({ phase: "failed", failure: { stage: "prepare", message: "no API key" } })
    s = run(s, { type: "retry" })
    expect(s).toMatchObject({ phase: "preparing", sessionId: "s1", failure: null })
  })
  it("a failed session start retries from the start", () => {
    let s = run(initFlow("section"), { type: "failed", failure: { stage: "session", service: "backend", message: "offline", offline: true } })
    s = run(s, { type: "retry" })
    expect(s).toMatchObject({ phase: "opening", sessionId: null, failure: null })
  })
  it("ignores retry when nothing failed", () => {
    const s = ready()
    expect(flowReducer(s, { type: "retry" })).toBe(s)
  })
})

describe("prequestions", () => {
  it("asks the unanswered ones in order and holds the reading until all are answered", () => {
    let s = ready()
    expect(nextQuestion(s)?.id).toBe("q1")
    expect(canAdvance(s)).toBe(false)
    expect(flowReducer(s, { type: "advance" })).toBe(s)
    s = run(s, answer("q1"))
    expect(nextQuestion(s)?.id).toBe("q2")
    expect(canAdvance(s)).toBe(false)
    s = run(s, answer("q2", false, 4))
    expect(nextQuestion(s)).toBeNull()
    expect(canAdvance(s)).toBe(true)
    expect(run(s, { type: "advance" })).toMatchObject({ step: "reading" })
  })
  it("keeps the learner's answer, confidence and the result, and leaves the explanation for later", () => {
    const s = run(ready(), { type: "answered", id: "q1", answer: "x", confidence: 4, result: { correct: false, explanation: "because" } })
    expect(s.answers.q1).toEqual({ answer: "x", confidence: 4, correct: false, explanation: "because" })
    expect(s.step).toBe("prequestions")
  })
  it("keys numeric question ids by their string", () => {
    const prep = { ...PREPARED, prequestions: [{ id: 7, q: "?" }] }
    const s = run(ready(prep), { type: "answered", id: 7, answer: "a", confidence: 1, result: { correct: null } })
    expect(s.answers["7"]).toMatchObject({ correct: null, explanation: null })
    expect(nextQuestion(s)).toBeNull()
  })
  it("does not let an answer change once recorded", () => {
    const s = run(ready(), answer("q1", true, 1))
    expect(run(s, { type: "answered", id: "q1", answer: "later", confidence: 4, result: { correct: false } })).toBe(s)
  })
  it("flags a confident miss, and only that", () => {
    const a = (correct: boolean | null, confidence: 1 | 2 | 3 | 4) => ({ answer: "", confidence, correct, explanation: null })
    expect(highConfidenceMiss(a(false, 3))).toBe(true)
    expect(highConfidenceMiss(a(false, 4))).toBe(true)
    expect(highConfidenceMiss(a(false, 2))).toBe(false)
    expect(highConfidenceMiss(a(true, 4))).toBe(false)
    expect(highConfidenceMiss(a(null, 4))).toBe(false)
  })
})

describe("the walk to the end", () => {
  const toReading = () => run(ready(), answer("q1"), answer("q2"), { type: "advance" })

  it("goes prequestions, reading, recall, summary, missed, atoms, explanations, done", () => {
    let s = toReading()
    const seen: string[] = [s.step]
    const go = (...a: FlowAction[]) => {
      s = run(s, ...a)
      seen.push(s.phase === "done" ? "done" : s.step)
    }
    go({ type: "advance" }) // reading -> recall
    go({ type: "recalled", text: "from memory" }, { type: "advance" })
    go(SUMMARY, { type: "advance" })
    go({ type: "advance" }) // missed -> atoms
    go({ type: "atomsDecided", accepted: ["a1"], entryIds: [11] }, { type: "advance" })
    go({ type: "advance" }) // explanations -> done
    expect(seen).toEqual(["reading", "recall", "summary", "missed", "atoms", "explanations", "done"])
  })
  it("holds recall, summary and atoms until they are done; recall can be skipped", () => {
    let s = run(toReading(), { type: "advance" })
    expect(s.step).toBe("recall")
    expect(canAdvance(s)).toBe(false)
    s = run(s, { type: "recallSkipped" })
    expect(s.recall).toBe("skipped")
    s = run(s, { type: "advance" })
    expect(s.step).toBe("summary")
    expect(canAdvance(s)).toBe(false)
    s = run(s, SUMMARY, { type: "advance" }, { type: "advance" })
    expect(s.step).toBe("atoms")
    expect(canAdvance(s)).toBe(false)
  })
  it("accepting no atoms is a decision too", () => {
    const s = run(toReading(), { type: "advance" }, { type: "recalled", text: "r" }, { type: "advance" }, SUMMARY, { type: "advance" }, { type: "advance" }, { type: "atomsDecided", accepted: [], entryIds: [] })
    expect(canAdvance(s)).toBe(true)
    expect(s.atoms).toEqual({ accepted: [], entryIds: [] })
  })
  it("ignores results that arrive in the wrong step", () => {
    const s = toReading()
    expect(flowReducer(s, SUMMARY)).toBe(s)
    expect(flowReducer(s, { type: "recalled", text: "x" })).toBe(s)
    expect(flowReducer(s, { type: "atomsDecided", accepted: [], entryIds: [] })).toBe(s)
    expect(flowReducer(s, answer("q1"))).toBe(s)
  })
  it("skips the atom step when there are no candidate atoms, and the answers step when nothing was asked", () => {
    const prep = { ...PREPARED, prequestions: [], atoms: [] }
    let s = run(ready(prep), { type: "advance" }, { type: "recalled", text: "r" }, { type: "advance" }, SUMMARY, { type: "advance" })
    expect(s.step).toBe("missed")
    s = run(s, { type: "advance" })
    expect(s.phase).toBe("done")
  })
  it("goes from missed to the answers step when there are no atoms but there were prequestions", () => {
    const prep = { ...PREPARED, atoms: [] }
    const s = run(ready(prep), answer("q1"), answer("q2"), { type: "advance" }, { type: "advance" }, { type: "recalled", text: "r" }, { type: "advance" }, SUMMARY, { type: "advance" }, { type: "advance" })
    expect(s.step).toBe("explanations")
  })
})

describe("requests in flight", () => {
  it("marks busy, records a failure without leaving the step, and clears both on the next result", () => {
    let s = run(ready(), { type: "busy" })
    expect(s.busy).toBe(true)
    s = run(s, { type: "stepFailed", message: "backend is offline" })
    expect(s).toMatchObject({ busy: false, error: "backend is offline", step: "prequestions" })
    s = run(s, { type: "busy" })
    expect(s.error).toBeNull()
    s = run(s, answer("q1"))
    expect(s).toMatchObject({ busy: false, error: null })
  })
})

describe("steps by mode", () => {
  it("has the seven steps in the C11 order", () => {
    expect(STEPS).toEqual(["prequestions", "reading", "recall", "summary", "missed", "atoms", "explanations"])
  })
  it("words the paper variant: predict, evidence table, critique", () => {
    expect(STEP_LABEL.paper.prequestions).toBe("Predict")
    expect(STEP_LABEL.paper.recall).toBe("Evidence")
    expect(STEP_LABEL.paper.summary).toBe("Critique")
    expect(STEP_LABEL.section.recall).toBe("Recall")
  })
  it("lists only the steps this section will have", () => {
    expect(visibleSteps(ready())).toEqual(STEPS)
    expect(visibleSteps(ready({ ...PREPARED, prequestions: [], atoms: [] }))).toEqual(["reading", "recall", "summary", "missed"])
    expect(visibleSteps(initFlow("section"))).toEqual(STEPS)
  })
})
