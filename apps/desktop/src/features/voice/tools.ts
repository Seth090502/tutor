// Client tools for the ElevenLabs voice tutor. The agent config declares the same names and
// parameter schemas (apps/desktop/agent/tool_configs/). Every tool returns a string for the
// LLM and never throws into the SDK: failures come back as a short "Error: ..." sentence.
import { BACKEND_URL, HttpError, RAG_ENGINE_URL, fetchJson } from "@/lib/backend"

const PRACTICE = "/api/question-notebook/practice"
const EXCERPT_CHARS = 600

type Entry = {
  id: number
  question: string
  question_type?: string
  options?: Record<string, string>
  correct_answer?: string
  explanation?: string
  misconception?: string
  categories?: { name: string }[] | null
}
type PracticeQuestion = { entry: Entry; state: { version?: number } }
type ReviewOutcome = { due_at: number | null }
type PracticeSummary = {
  total: number
  mistakes: number
  due: number
  overdue: number
  reviewed_today: number
  next_due_at: number | null
}
type SearchHit = {
  title: string
  domain: string
  source_type: string
  page_start: number | null
  page_end: number | null
  text: string
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
    // Built field by field: the queue entry carries the answer key, which must not reach the
    // tutor before check_answer has graded the learner.
    return JSON.stringify({
      question_id: entry.id,
      question: entry.question,
      question_type: entry.question_type ?? "",
      options: entry.options ?? {},
    })
  }),

  check_answer: safe("check_answer", PRACTICE_SERVICE, async ({ question_id, answer }) => {
    const id = questionId(question_id)
    const given = text(answer, "answer")
    const { correct } = await fetchJson<{ correct: boolean | null }>(BACKEND_URL, `${questionPath(id)}/check`, {
      method: "POST",
      body: JSON.stringify({ answer: given }),
    })
    // Only after grading does the tutor see the answer key.
    const { entry } = await fetchJson<PracticeQuestion>(BACKEND_URL, questionPath(id))
    return JSON.stringify({
      correct,
      correct_answer: entry.correct_answer ?? "",
      explanation: entry.explanation ?? "",
      ...(typeof entry.misconception === "string" && entry.misconception
        ? { misconception: entry.misconception }
        : {}),
    })
  }),

  record_review: safe("record_review", PRACTICE_SERVICE, async ({ question_id, answer, correct }) => {
    const id = questionId(question_id)
    const given = text(answer, "answer")
    const rating = flag(correct, "correct") ? "good" : "again"
    const request_id = requestId()
    const version = async () =>
      (await fetchJson<PracticeQuestion>(BACKEND_URL, questionPath(id))).state.version ?? 0
    const review = async (v: number) =>
      fetchJson<ReviewOutcome>(BACKEND_URL, `${questionPath(id)}/review`, {
        method: "POST",
        body: JSON.stringify({ answer: given, self_report: false, request_id, version: v, rating }),
      })
    let outcome: ReviewOutcome
    try {
      outcome = await review(await version())
    } catch (e) {
      // 409: the question changed since we read its version. Re-read once and retry.
      if (!(e instanceof HttpError && e.status === 409)) throw e
      outcome = await review(await version())
    }
    return JSON.stringify({ rating, next_due: localTime(outcome.due_at) })
  }),

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
    const s = await fetchJson<PracticeSummary>(
      BACKEND_URL,
      `${PRACTICE}/summary?timezone=${encodeURIComponent(timezone())}`,
    )
    return JSON.stringify({
      total: s.total,
      due: s.due,
      overdue: s.overdue,
      mistakes: s.mistakes,
      reviewed_today: s.reviewed_today,
      next_due: localTime(s.next_due_at),
    })
  }),
} satisfies Record<string, TutorTool>

export type TutorToolName = keyof typeof tutorTools
