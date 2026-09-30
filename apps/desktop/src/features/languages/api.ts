// The backend routes the Languages screen calls: the C7 practice attempts (used directly, not through
// the voice tools) and the C10 language routes. Shapes are the contracts in docs/tutor/CONTRACTS.md.
import { BACKEND_URL, HttpError, fetchJson } from "@/lib/backend"
import type { TargetLanguage } from "./_pending/language"
import type { Band } from "./agent"
import { cardMeta, orderQueue, type Card, type CardKind, type CheckResult, type Confidence, type HintRung, type ReviewResult } from "./drill"

const PRACTICE = "/api/question-notebook/practice"
const QUEUE_LIMIT = 100

export type Entry = {
  id: number
  question: string
  question_type?: string
  categories?: { name?: string }[] | null
  tags?: (string | { name?: string })[] | null
}
export type QueueItem = { entry: Entry; state: { version?: number } }

const timezone = () => Intl.DateTimeFormat().resolvedOptions().timeZone
const questionPath = (id: number) => `${PRACTICE}/questions/${id}`
const attemptPath = (id: string) => `${PRACTICE}/attempts/${encodeURIComponent(id)}`

export const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))
/** A rejection that is not an HTTP status means nothing answered on the port. */
export const isOffline = (e: unknown) => !(e instanceof HttpError)

/** This language's due cards, production first. */
export async function loadQueue(lang: TargetLanguage): Promise<QueueItem[]> {
  const queue = await fetchJson<QueueItem[]>(
    BACKEND_URL,
    `${PRACTICE}/queue?limit=${QUEUE_LIMIT}&timezone=${encodeURIComponent(timezone())}`,
  )
  return orderQueue(queue, lang)
}

/**
 * Opens an attempt for a queue item. The queue entry also carries the answer key; only the prompt
 * and the kind are read from it, and the answer is shown from the check result after the attempt.
 */
export async function openCard(item: QueueItem): Promise<Card> {
  const attempt = await fetchJson<{ attempt_id: string; rungs_available: number }>(
    BACKEND_URL,
    `${questionPath(item.entry.id)}/attempts`,
    { method: "POST" },
  )
  const kind: CardKind = cardMeta(item.entry).kind
  return {
    questionId: item.entry.id,
    attemptId: attempt.attempt_id,
    prompt: item.entry.question,
    kind,
    rungsAvailable: attempt.rungs_available,
  }
}

export async function sendConfidence(attemptId: string, level: Confidence): Promise<void> {
  await fetchJson<{ ok: boolean }>(BACKEND_URL, `${attemptPath(attemptId)}/confidence`, {
    method: "POST",
    body: JSON.stringify({ level }),
  })
}

export async function requestHint(attemptId: string): Promise<HintRung> {
  const h = await fetchJson<{ rung: number; text: string | null; is_answer: boolean }>(
    BACKEND_URL,
    `${attemptPath(attemptId)}/hint`,
    { method: "POST" },
  )
  return { rung: h.rung, text: h.text, isAnswer: h.is_answer }
}

export async function checkAnswer(attemptId: string, answer: string): Promise<CheckResult> {
  const c = await fetchJson<{
    correct: boolean | null
    correct_answer: string
    explanation: string
    misconception?: string | null
    diagnostic?: string | null
  }>(BACKEND_URL, `${attemptPath(attemptId)}/check`, { method: "POST", body: JSON.stringify({ answer }) })
  return {
    correct: c.correct,
    correctAnswer: c.correct_answer,
    explanation: c.explanation,
    misconception: c.misconception || null,
    diagnostic: c.diagnostic || null,
  }
}

/** 32 random characters from [A-Za-z0-9], the review endpoint's idempotency key. */
function requestId(): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
  return Array.from(crypto.getRandomValues(new Uint8Array(32)), (b) => alphabet[b % alphabet.length]).join("")
}

function localTime(epochSeconds: number | null | undefined): string | null {
  if (epochSeconds == null) return null
  return new Date(epochSeconds * 1000).toLocaleString("en-US", {
    timeZone: timezone(),
    dateStyle: "medium",
    timeStyle: "short",
  })
}

/**
 * Records the review. With an attempt id the server computes the rating (a client rating is a 422), and
 * `judged_correct` goes only with a check that returned null. One retry on 409 with a fresh version.
 */
export async function submitReview(card: Card, answer: string, judged: boolean | null): Promise<ReviewResult> {
  const id = requestId()
  const version = async () => (await fetchJson<QueueItem>(BACKEND_URL, questionPath(card.questionId))).state.version ?? 0
  const review = async (v: number) =>
    fetchJson<{ rating: string; due_at: number | null }>(BACKEND_URL, `${questionPath(card.questionId)}/review`, {
      method: "POST",
      body: JSON.stringify({
        attempt_id: card.attemptId,
        request_id: id,
        version: v,
        answer,
        ...(judged === null ? {} : { judged_correct: judged }),
      }),
    })
  let out: { rating: string; due_at: number | null }
  try {
    out = await review(await version())
  } catch (e) {
    if (!(e instanceof HttpError && e.status === 409)) throw e
    out = await review(await version())
  }
  return { rating: out.rating, nextDue: localTime(out.due_at) }
}

// ---- C10 language routes -----------------------------------------------------------------------

export type GradedText = {
  text: string
  sentences: string[]
  coverage: number
  unknown: string[]
  questions: { q: string; a: string }[]
}

export const gradedText = (lang: TargetLanguage, band: Band, topic?: string) =>
  fetchJson<GradedText>(BACKEND_URL, "/api/languages/graded-text", {
    method: "POST",
    body: JSON.stringify({ lang, band, ...(topic?.trim() ? { topic: topic.trim() } : {}) }),
  })

export const addCards = (lang: TargetLanguage, items: { text: string; meaning: string; example?: string }[]) =>
  fetchJson<{ created: number; entry_ids: number[] }>(BACKEND_URL, "/api/languages/cards", {
    method: "POST",
    body: JSON.stringify({ lang, items }),
  })

/** The command that builds a starter deck (C10); the screen shows it, it does not run it. */
export const starterDeckCommand = (lang: TargetLanguage, size = 200) =>
  `python -m deeptutor.services.languages.build_deck --lang ${lang} --size ${size}`
