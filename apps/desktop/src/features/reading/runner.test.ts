import { describe, expect, it, vi } from "vitest"

import { ServiceFault, type ReadingApi } from "./api"
import { PREPARED, SECTION } from "./fixtures"
import { flowReducer, initFlow, type FlowAction, type FlowState } from "./flow"
import { faultText, openAndPrepare, sendAnswer, sendAtoms, sendRecall, sendSummary } from "./runner"

type Api = Pick<ReadingApi, "openSession" | "prepare" | "answer" | "recall" | "summary" | "acceptAtoms">

function harness(api: Partial<Api>) {
  let state = initFlow("section")
  const dispatch = (a: FlowAction) => {
    state = flowReducer(state, a)
  }
  const full: Api = {
    openSession: vi.fn(async () => ({ session_id: "s1", section: SECTION })),
    prepare: vi.fn(async () => PREPARED),
    answer: vi.fn(async () => ({ correct: true })),
    recall: vi.fn(async () => ({ ok: true })),
    summary: vi.fn(async () => ({ covered: 5, missed: [{ unit: "u", chunk_id: "d-analysis:45" }] })),
    acceptAtoms: vi.fn(async () => ({ entry_ids: [11, 12] })),
    ...api,
  }
  return { ctx: { api: full, dispatch }, full, get state(): FlowState { return state } }
}
const offline = (service: "backend" | "rag-engine" = "backend") => new ServiceFault(service, true, null, "Failed to fetch")
const http = (status: number, message: string) => new ServiceFault("backend", false, status, message)

describe("openAndPrepare", () => {
  it("opens the session, prepares it and lands on the first prequestion", async () => {
    const h = harness({})
    await openAndPrepare(h.ctx, "d-analysis", "d-analysis:41", null)
    expect(h.full.openSession).toHaveBeenCalledWith("d-analysis", "d-analysis:41")
    expect(h.full.prepare).toHaveBeenCalledWith("s1")
    expect(h.state).toMatchObject({ phase: "step", step: "prequestions", sessionId: "s1" })
  })
  it("reports a backend that is not running as a failure of the session stage", async () => {
    const h = harness({ openSession: vi.fn(async () => { throw offline() }) })
    await openAndPrepare(h.ctx, "d", "c", null)
    expect(h.state).toMatchObject({ phase: "failed", failure: { stage: "session", service: "backend", offline: true } })
    expect(h.full.prepare).not.toHaveBeenCalled()
  })
  it("reports a failed preparation with the backend's reason, and a retry reuses the session", async () => {
    const prepare = vi.fn().mockRejectedValueOnce(http(503, "no API key configured")).mockResolvedValueOnce(PREPARED)
    const h = harness({ prepare })
    await openAndPrepare(h.ctx, "d", "c", null)
    expect(h.state).toMatchObject({ phase: "failed", failure: { stage: "prepare", offline: false, message: "no API key configured" } })
    h.ctx.dispatch({ type: "retry" })
    await openAndPrepare(h.ctx, "d", "c", h.state.sessionId)
    expect(h.full.openSession).toHaveBeenCalledTimes(1)
    expect(prepare).toHaveBeenCalledTimes(2)
    expect(h.state).toMatchObject({ phase: "step", step: "prequestions" })
  })
  it("wraps a non-fault error as a failure too", async () => {
    const h = harness({ prepare: vi.fn(async () => { throw new TypeError("Failed to fetch") }) })
    await openAndPrepare(h.ctx, "d", "c", null)
    expect(h.state.failure).toMatchObject({ stage: "prepare", offline: true })
  })
})

describe("step requests", () => {
  const started = async (api: Partial<Api> = {}) => {
    const h = harness(api)
    await openAndPrepare(h.ctx, "d", "c", null)
    return h
  }

  it("records an answer with its confidence", async () => {
    const h = await started({ answer: vi.fn(async () => ({ correct: false })) })
    await sendAnswer(h.ctx, "s1", "q1", "my guess", 4)
    expect(h.full.answer).toHaveBeenCalledWith("s1", "q1", "my guess", 4)
    expect(h.state.answers.q1).toMatchObject({ answer: "my guess", confidence: 4, correct: false })
    expect(h.state.busy).toBe(false)
  })
  it("keeps the step and says what failed when the backend drops out mid-step", async () => {
    const answer = vi.fn().mockRejectedValueOnce(offline()).mockResolvedValueOnce({ correct: true })
    const h = await started({ answer })
    await sendAnswer(h.ctx, "s1", "q1", "g", 2)
    expect(h.state).toMatchObject({ step: "prequestions", busy: false, error: "The backend is not running." })
    expect(h.state.answers).toEqual({})
    await sendAnswer(h.ctx, "s1", "q1", "g", 2)
    expect(h.state.error).toBeNull()
    expect(h.state.answers.q1).toBeDefined()
  })
  it("sends recall, summary and accepted atoms to their routes", async () => {
    const h = await started()
    h.ctx.dispatch({ type: "answered", id: "q1", answer: "a", confidence: 1, result: { correct: null } })
    h.ctx.dispatch({ type: "answered", id: "q2", answer: "a", confidence: 1, result: { correct: null } })
    h.ctx.dispatch({ type: "advance" })
    h.ctx.dispatch({ type: "advance" })
    await sendRecall(h.ctx, "s1", "what I remember")
    expect(h.full.recall).toHaveBeenCalledWith("s1", "what I remember")
    expect(h.state.recall).toEqual({ text: "what I remember" })
    h.ctx.dispatch({ type: "advance" })
    await sendSummary(h.ctx, "s1", "my summary", 3)
    expect(h.full.summary).toHaveBeenCalledWith("s1", "my summary", 3)
    expect(h.state.summary?.result.missed).toHaveLength(1)
    h.ctx.dispatch({ type: "advance" })
    h.ctx.dispatch({ type: "advance" })
    await sendAtoms(h.ctx, "s1", ["a1", "a3"])
    expect(h.full.acceptAtoms).toHaveBeenCalledWith("s1", ["a1", "a3"])
    expect(h.state.atoms).toEqual({ accepted: ["a1", "a3"], entryIds: [11, 12] })
  })
  it("makes no request when no atom is accepted", async () => {
    const h = await started()
    for (const a of [
      { type: "answered", id: "q1", answer: "a", confidence: 1, result: { correct: null } },
      { type: "answered", id: "q2", answer: "a", confidence: 1, result: { correct: null } },
      { type: "advance" },
      { type: "advance" },
      { type: "recallSkipped" },
      { type: "advance" },
      { type: "summarized", text: "s", judgement: 1, result: { covered: 0, missed: [] } },
      { type: "advance" },
      { type: "advance" },
    ] as FlowAction[]) h.ctx.dispatch(a)
    expect(h.state.step).toBe("atoms")
    await sendAtoms(h.ctx, "s1", [])
    expect(h.full.acceptAtoms).not.toHaveBeenCalled()
    expect(h.state.atoms).toEqual({ accepted: [], entryIds: [] })
  })
})

describe("faultText", () => {
  it("says which service is not running", () => {
    expect(faultText(offline("backend"))).toBe("The backend is not running.")
    expect(faultText(offline("rag-engine"))).toBe("rag-engine is not running.")
  })
  it("passes on what an answering service said", () => {
    expect(faultText(http(409, "attempt closed"))).toBe("attempt closed")
  })
})
