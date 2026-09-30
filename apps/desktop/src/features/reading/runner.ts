// The requests behind each step of the flow. Each one reports into the reducer, so the screens stay
// free of fetch logic and the whole sequence runs in a node test.
import { ServiceFault, toFault, type ReadingApi } from "./api"
import type { FlowAction } from "./flow"
import type { Confidence } from "./types"

export type Ctx = {
  api: Pick<ReadingApi, "openSession" | "prepare" | "answer" | "recall" | "summary" | "acceptAtoms">
  dispatch: (a: FlowAction) => void
}

/** One sentence for a fault: which service is down, or what the service that did answer said. */
export function faultText(f: ServiceFault): string {
  if (!f.offline) return f.message
  return `${f.service === "backend" ? "The backend" : f.service} is not running.`
}

/**
 * Opens the session (unless one exists) and prepares it. A retry after a failed preparation passes the
 * session id, so the section is not opened twice.
 */
export async function openAndPrepare(ctx: Ctx, docId: string, fromChunk: string, sessionId: string | null): Promise<void> {
  let id = sessionId
  const fail = (stage: "session" | "prepare", e: unknown) => {
    const f = toFault("backend", e)
    ctx.dispatch({ type: "failed", failure: { stage, service: f.service, message: f.offline ? faultText(f) : f.message, offline: f.offline } })
  }
  if (!id) {
    try {
      const opened = await ctx.api.openSession(docId, fromChunk)
      id = opened.session_id
      ctx.dispatch({ type: "sessionOpened", sessionId: id, section: opened.section })
    } catch (e) {
      return fail("session", e)
    }
  }
  try {
    ctx.dispatch({ type: "prepared", prep: await ctx.api.prepare(id) })
  } catch (e) {
    fail("prepare", e)
  }
}

async function step<T>(ctx: Ctx, request: () => Promise<T>, done: (result: T) => FlowAction): Promise<void> {
  ctx.dispatch({ type: "busy" })
  try {
    ctx.dispatch(done(await request()))
  } catch (e) {
    ctx.dispatch({ type: "stepFailed", message: faultText(toFault("backend", e)) })
  }
}

export const sendAnswer = (ctx: Ctx, sessionId: string, id: string | number, answer: string, confidence: Confidence) =>
  step(ctx, () => ctx.api.answer(sessionId, id, answer, confidence), (result) => ({ type: "answered", id, answer, confidence, result }))

export const sendRecall = (ctx: Ctx, sessionId: string, text: string) =>
  step(ctx, () => ctx.api.recall(sessionId, text), () => ({ type: "recalled", text }))

export const sendSummary = (ctx: Ctx, sessionId: string, text: string, judgement: Confidence) =>
  step(ctx, () => ctx.api.summary(sessionId, text, judgement), (result) => ({ type: "summarized", text, judgement, result }))

/** Accepting nothing is a decision but not a request. */
export async function sendAtoms(ctx: Ctx, sessionId: string, ids: (string | number)[]): Promise<void> {
  if (ids.length === 0) return ctx.dispatch({ type: "atomsDecided", accepted: [], entryIds: [] })
  return step(ctx, () => ctx.api.acceptAtoms(sessionId, ids), (r) => ({ type: "atomsDecided", accepted: ids, entryIds: r.entry_ids }))
}
