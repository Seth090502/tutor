// Pure helpers for the attempt on the Tutor screen. The attempt itself is the C12 public state the
// StudySession accepted (src/lib/study.ts); nothing here holds answer text that did not come from
// `attempt.released`.
import type { ConfidenceLevel, StudyAttempt } from "@/lib/study"

export type AttemptOption = { id: string; text: string }

/** The C12 hint ladder, rungs 1-4. There is no answer rung: giving up reveals the answer. */
export const HINT_RUNGS = ["nudge", "hint", "key step", "worked example"] as const
export const CONFIDENCE_WORDS = ["guess", "unsure", "fairly sure", "certain"] as const
/** The reasons the learner can give for skipping confidence. */
export const SKIP_REASONS = ["can't tell", "rather not say"] as const

export const confidenceWord = (level: ConfidenceLevel) => CONFIDENCE_WORDS[level - 1]

/** Letters sort first, so "A".."D" stay in order whatever order the server sent them in. */
export function optionList(options: Record<string, string>): AttemptOption[] {
  return Object.entries(options)
    .sort(([a], [b]) => a.localeCompare(b, "en", { numeric: true }))
    .map(([id, text]) => ({ id, text }))
}

/** Live until the server closes it. */
export const isLive = (attempt: StudyAttempt | null): attempt is StudyAttempt =>
  Boolean(attempt && attempt.status !== "closed")

/** The hint rungs given so far, in order. */
export const hintsGiven = (attempt: StudyAttempt) => attempt.assistance.filter((a) => a.rung != null)

/** The newest check: the first answer's, or the latest retry's. */
export const lastCheck = (attempt: StudyAttempt) => attempt.checks[attempt.checks.length - 1] ?? null

/** Which option the released answer names, by its key or its full text; null when it is neither. */
export function correctOptionId(options: AttemptOption[], correctAnswer: string): string | null {
  const want = correctAnswer.trim().toLowerCase()
  if (!want) return null
  const match =
    options.find((o) => o.id.toLowerCase() === want) ?? options.find((o) => o.text.trim().toLowerCase() === want)
  return match?.id ?? null
}

export type AttemptKey = { kind: "answer"; index: number } | { kind: "confidence"; level: ConfidenceLevel }

/** A-D choose an answer (only when there are options to choose from); 1-4 rate confidence. */
export function attemptKey(key: string, optionCount: number): AttemptKey | null {
  if (key.length !== 1) return null
  const letter = "abcd".indexOf(key.toLowerCase())
  if (letter >= 0) return letter < optionCount ? { kind: "answer", index: letter } : null
  const digit = "1234".indexOf(key)
  return digit >= 0 ? { kind: "confidence", level: (digit + 1) as ConfidenceLevel } : null
}

/** "Good" and "Again" come from the server; anything else is shown as sent. */
export function ratingWord(rating: string): string {
  return rating ? rating.charAt(0).toUpperCase() + rating.slice(1).toLowerCase() : rating
}

/** The review the server applied at closure, with why, in words (review-policy/2); null while live. */
export function reviewWords(attempt: StudyAttempt): string | null {
  const { closure } = attempt
  if (!closure) return null
  const { rating, provenance, first_outcome } = closure.scheduler_disposition
  if (!rating) {
    if (closure.reason === "invalidated") return "Not scheduled: the question was set aside"
    if (closure.reason === "abandoned") return "Not scheduled: the question was left before a check"
    return "Not scheduled"
  }
  const word = ratingWord(rating)
  if (provenance === "self_report") return `${word}, by your own judgement`
  if (rating === "good") return `${word}, correct without help`
  if (attempt.first_response?.input_mode === "dont_know") return `${word}, because you did not know it`
  if (first_outcome !== "correct") return `${word}, because the first answer was not correct`
  return attempt.assistance.some((a) => a.before_commit && a.rung != null)
    ? `${word}, because a hint was used`
    : `${word}, because you had help before answering`
}

const OFFERS: Record<string, string> = {
  worked_example_then_fresh_problem: "Two tries missed: a worked example on a parallel problem, then a fresh problem of the same kind.",
}

export const offerWords = (offer: string) => OFFERS[offer] ?? offer.replace(/_/g, " ")
