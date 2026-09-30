// Client tools for the ElevenLabs voice tutor. The agent config declares the same names and
// parameter schemas (apps/desktop/agent/tool_configs/). Every tool returns a string for the
// LLM and never throws into the SDK: failures come back as a short "Error: ..." sentence.
// Practice runs as attempts (contract C7): the server holds the answer key and the hint ladder,
// and the model sees the answer only in check_answer's result, after the attempt is closed.
// What the tools did is published as TutorEvents (contract C8) for the Tutor screen.
import { BACKEND_URL, HttpError, RAG_ENGINE_URL, fetchJson } from "@/lib/backend"

const PRACTICE = "/api/question-notebook/practice"
const EXCERPT_CHARS = 600
const CALIBRATION_DAYS = 30

type Entry = {
  id: number
  question: string
  question_type?: string
  options?: Record<string, string>
  categories?: { name: string }[] | null
}
type PracticeQuestion = { entry: Entry; state: { version?: number } }
type Attempt = { attempt_id: string; rungs_available: number }
type Hint = { rung: number; text: string | null; is_answer: boolean }
type Check = {
  correct: boolean | null
  correct_answer: string
  explanation: string
  misconception: string | null
}
type ReviewOutcome = { rating: string; due_at: number | null }
type PracticeSummary = {
  total: number
  mistakes: number
  due: number
  overdue: number
  reviewed_today: number
  next_due_at: number | null
}
type Calibration = { level: number; attempts: number; correct: number }
type SearchHit = {
  title: string
  domain: string
  source_type: string
  page_start: number | null
  page_end: number | null
  text: string
}

export type TutorEvent =
  | {
      type: "question"
      attempt_id: string
      question_id: number
      question: string
      question_type: string
      options: Record<string, string>
    }
  | { type: "confidence"; attempt_id: string; level: 1 | 2 | 3 | 4 }
  | { type: "hint"; attempt_id: string; rung: number; text: string | null; is_answer: boolean }
  | {
      type: "checked"
      attempt_id: string
      correct: boolean | null
      correct_answer: string
      explanation: string
      misconception: string | null
    }
  | { type: "reviewed"; attempt_id: string; rating: string; next_due: string | null }

const listeners = new Set<(e: TutorEvent) => void>()

export function onTutorEvent(listener: (e: TutorEvent) => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** A listener that throws must not turn a completed tool call into an error for the agent. */
function emit(event: TutorEvent) {
  for (const listener of [...listeners]) {
    try {
      listener(event)
    } catch (e) {
      console.warn("tutor event listener failed", e)
    }
  }
}

/** A bad argument from the LLM; its message is safe to say aloud. */
class ToolInputError extends Error {}

type Params = Record<string, unknown>
export type TutorTool = (params: Params) => Promise<string>

const timezone = () => Intl.DateTimeFormat().resolvedOptions().timeZone

function localTime(epochSeconds: number | null | undefined): string | null {
  if (epochSeconds == null) return null
  return new Date(epochSeconds * 1000).toLocaleString("en-US", {
    timeZone: timezone(),
    dateStyle: "medium",
    timeStyle: "short",
  })
}

function questionId(value: unknown): number {
  const id = typeof value === "string" ? Number(value.trim()) : value
  if (typeof id !== "number" || !Number.isInteger(id) || id <= 0) {
    throw new ToolInputError("question_id must be the number returned by get_next_question.")
  }
  return id
}

function attemptId(value: unknown): string {
  if (typeof value !== "string" || !value.trim()) {
    throw new ToolInputError("attempt_id must be the attempt_id returned by get_next_question.")
  }
  return value.trim()
}

function confidence(value: unknown): 1 | 2 | 3 | 4 {
  const level = typeof value === "string" ? Number(value.trim()) : value
  if (level !== 1 && level !== 2 && level !== 3 && level !== 4) {
    throw new ToolInputError("level must be a whole number from 1 to 4.")
  }
  return level
}

function text(value: unknown, name: string): string {
  if (typeof value === "number") return String(value)
  if (typeof value !== "string" || !value.trim()) throw new ToolInputError(`${name} is required.`)
  return value.trim()
}

function optionalText(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined
}

function flag(value: unknown, name: string): boolean {
  if (typeof value === "boolean") return value
  if (value === "true" || value === "false") return value === "true"
  throw new ToolInputError(`${name} must be true or false.`)
}

/** 32 random characters from [A-Za-z0-9], the review endpoint's idempotency key. */
function requestId(): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
  const bytes = crypto.getRandomValues(new Uint8Array(32))
  // 256 % 62 skews a few characters slightly; irrelevant for a unique id.
  return Array.from(bytes, (b) => alphabet[b % alphabet.length]).join("")
}

function pages(hit: SearchHit): string {
  if (hit.page_start == null) return ""
  return hit.page_end == null || hit.page_end === hit.page_start
    ? String(hit.page_start)
    : `${hit.page_start}-${hit.page_end}`
}

function excerpt(s: string): string {
  return s.length <= EXCERPT_CHARS ? s : `${s.slice(0, EXCERPT_CHARS - 1)}…`
}

function questionPath(id: number) {
  return `${PRACTICE}/questions/${id}`
}

function attemptPath(id: string) {
  return `${PRACTICE}/attempts/${encodeURIComponent(id)}`
}

/** What the tutor does when the server has no written text for a rung. */
function missingRungInstruction(rung: number): string {
  return rung >= 4
    ? "No written worked example for this question. Work a parallel problem of the same kind, not this one, step by step, then give the learner a fresh problem of that kind to try."
    : "No written hint for this rung. Give one short conceptual nudge in your own words, without stating the answer or anything that gives it away."
}

/** Wraps a tool so any failure becomes a short sentence the agent can say. */
function safe(name: string, service: string, run: TutorTool): TutorTool {
  return async (params) => {
    try {
      return await run(params ?? {})
    } catch (e) {
      console.warn(`voice tool ${name} failed`, e)
      if (e instanceof ToolInputError) return `Error: ${e.message}`
      if (e instanceof HttpError) {
        return e.status === 404
          ? `Error: the ${service} could not find that.`
          : `Error: the ${service} returned HTTP ${e.status}.`
      }
      return `Error: the ${service} is not reachable.`
    }
  }
}

const PRACTICE_SERVICE = "practice service"
const LIBRARY_SERVICE = "library search"

export const tutorTools = {
  get_next_question: safe("get_next_question", PRACTICE_SERVICE, async ({ domain }) => {
    const queue = await fetchJson<PracticeQuestion[]>(
      BACKEND_URL,
      `${PRACTICE}/queue?limit=20&timezone=${encodeURIComponent(timezone())}`,
    )
    if (queue.length === 0) return "No practice questions are due right now."
    const wanted = optionalText(domain)?.toLowerCase()
    const match =
      wanted && queue.find((q) => q.entry.categories?.some((c) => c.name.toLowerCase() === wanted))
    const { entry } = match || queue[0]
    const attempt = await fetchJson<Attempt>(BACKEND_URL, `${questionPath(entry.id)}/attempts`, {
      method: "POST",
    })
    // Built field by field: the queue entry carries the answer key and hints, which must not
    // reach the tutor (or the screen) before check_answer has closed the attempt.
    const question = {
      attempt_id: attempt.attempt_id,
      question_id: entry.id,
      question: entry.question,
      question_type: entry.question_type ?? "",
      options: entry.options ?? {},
    }
    emit({ type: "question", ...question })
    return JSON.stringify({ ...question, rungs_available: attempt.rungs_available })
  }),

  set_confidence: safe("set_confidence", PRACTICE_SERVICE, async ({ attempt_id, level }) => {
    const attempt = attemptId(attempt_id)
    const value = confidence(level)
    await fetchJson<{ ok: boolean }>(BACKEND_URL, `${attemptPath(attempt)}/confidence`, {
      method: "POST",
      body: JSON.stringify({ level: value }),
    })
    emit({ type: "confidence", attempt_id: attempt, level: value })
    return JSON.stringify({ recorded: true, level: value })
  }),

  get_hint: safe("get_hint", PRACTICE_SERVICE, async ({ attempt_id }) => {
    const attempt = attemptId(attempt_id)
    const hint = await fetchJson<Hint>(BACKEND_URL, `${attemptPath(attempt)}/hint`, { method: "POST" })
    const { rung, text: hintText, is_answer } = hint
    emit({ type: "hint", attempt_id: attempt, rung, text: hintText, is_answer })
    return JSON.stringify({
      rung,
      text: hintText,
      is_answer,
      ...(hintText == null ? { instruction: missingRungInstruction(rung) } : {}),
    })
  }),

  check_answer: safe("check_answer", PRACTICE_SERVICE, async ({ attempt_id, answer }) => {
    const attempt = attemptId(attempt_id)
    const given = text(answer, "answer")
    const check = await fetchJson<Check>(BACKEND_URL, `${attemptPath(attempt)}/check`, {
      method: "POST",
      body: JSON.stringify({ answer: given }),
    })
    // The check closes the attempt, so the answer key may now reach the tutor.
    const misconception = check.misconception || null
    emit({
      type: "checked",
      attempt_id: attempt,
      correct: check.correct,
      correct_answer: check.correct_answer,
      explanation: check.explanation,
      misconception,
    })
    return JSON.stringify({
      correct: check.correct,
      correct_answer: check.correct_answer,
      explanation: check.explanation,
      ...(misconception ? { misconception } : {}),
    })
  }),

  record_review: safe(
    "record_review",
    PRACTICE_SERVICE,
    async ({ attempt_id, question_id, answer, judged_correct }) => {
      const attempt = attemptId(attempt_id)
      const id = questionId(question_id)
      const given = text(answer, "answer")
      const judged = judged_correct == null ? undefined : flag(judged_correct, "judged_correct")
      const request_id = requestId()
      const version = async () =>
        (await fetchJson<PracticeQuestion>(BACKEND_URL, questionPath(id))).state.version ?? 0
      // No rating: with attempt_id the server computes it from the attempt (C7).
      const review = async (v: number) =>
        fetchJson<ReviewOutcome>(BACKEND_URL, `${questionPath(id)}/review`, {
          method: "POST",
          body: JSON.stringify({
            attempt_id: attempt,
            request_id,
            version: v,
            answer: given,
            ...(judged === undefined ? {} : { judged_correct: judged }),
          }),
        })
      let outcome: ReviewOutcome
      try {
        outcome = await review(await version())
      } catch (e) {
        // 409: the question changed since we read its version. Re-read once and retry.
        if (!(e instanceof HttpError && e.status === 409)) throw e
        outcome = await review(await version())
      }
      const next_due = localTime(outcome.due_at)
      emit({ type: "reviewed", attempt_id: attempt, rating: outcome.rating, next_due })
      return JSON.stringify({ rating: outcome.rating, next_due })
    },
  ),

  search_library: safe("search_library", LIBRARY_SERVICE, async ({ query, domain }) => {
    const q = text(query, "query")
    const d = optionalText(domain)
    const { hits } = await fetchJson<{ hits: SearchHit[] }>(RAG_ENGINE_URL, "/search", {
      method: "POST",
      body: JSON.stringify({ query: q, k: 5, ...(d ? { domains: [d] } : {}) }),
    })
    return JSON.stringify({
      hits: hits.map((h) => ({
        title: h.title,
        domain: h.domain,
        source_type: h.source_type,
        pages: pages(h),
        excerpt: excerpt(h.text),
      })),
    })
  }),

  get_progress: safe("get_progress", PRACTICE_SERVICE, async () => {
    const tz = encodeURIComponent(timezone())
    const s = await fetchJson<PracticeSummary>(BACKEND_URL, `${PRACTICE}/summary?timezone=${tz}`)
    const { calibration } = await fetchJson<{ calibration?: Calibration[] }>(
      BACKEND_URL,
      `${PRACTICE}/analytics?timezone=${tz}&days=${CALIBRATION_DAYS}`,
    )
    return JSON.stringify({
      total: s.total,
      due: s.due,
      overdue: s.overdue,
      mistakes: s.mistakes,
      reviewed_today: s.reviewed_today,
      next_due: localTime(s.next_due_at),
      ...(Array.isArray(calibration)
        ? {
            calibration_30_days: calibration.map((c) => ({
              level: c.level,
              attempts: c.attempts,
              correct: c.correct,
            })),
          }
        : {}),
    })
  }),
} satisfies Record<string, TutorTool>

export type TutorToolName = keyof typeof tutorTools
