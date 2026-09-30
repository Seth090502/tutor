// The attempt the tutor is running with the learner right now, built from the C8 events that
// tools.ts publishes. Pure: a reducer plus the small vocabularies the panel shows.
import type { TutorEvent } from "./tools"

export type Confidence = 1 | 2 | 3 | 4

export type AttemptOption = { id: string; text: string }

export type Attempt = {
  attemptId: string
  questionId: number
  question: string
  questionType: string
  options: AttemptOption[]
  /** The confidence level, from the agent's event or the learner's own key press. */
  confidence: Confidence | null
  /** The option the learner sent from the keys, if any. */
  answer: string | null
  /** The highest hint rung given so far (0 = none) and its text, which may be null. */
  rung: number
  hintText: string | null
  hintIsAnswer: boolean
  checked: {
    correct: boolean | null
    correctAnswer: string
    explanation: string
    misconception: string | null
  } | null
  reviewed: { rating: string; nextDue: string | null } | null
}

/** What happens on the learner's side of the panel, before the agent's tools confirm it. */
export type LocalAction =
  | { type: "reset" }
  | { type: "answered"; attempt_id: string; choice: string }
  | { type: "rated"; attempt_id: string; level: Confidence }

export const HINT_RUNGS = ["nudge", "hint", "key step", "worked example", "answer"] as const
export const CONFIDENCE_WORDS = ["guess", "unsure", "fairly sure", "certain"] as const

export const confidenceWord = (level: Confidence) => CONFIDENCE_WORDS[level - 1]

/** Letters sort first, so "A".."D" stay in order whatever order the server sent them in. */
export function optionList(options: Record<string, string>): AttemptOption[] {
  return Object.entries(options)
    .sort(([a], [b]) => a.localeCompare(b, "en", { numeric: true }))
    .map(([id, text]) => ({ id, text }))
}

export function attemptReducer(state: Attempt | null, event: TutorEvent | LocalAction): Attempt | null {
  if (event.type === "question") {
    return {
      attemptId: event.attempt_id,
      questionId: event.question_id,
      question: event.question,
      questionType: event.question_type,
      options: optionList(event.options),
      confidence: null,
      answer: null,
      rung: 0,
      hintText: null,
      hintIsAnswer: false,
      checked: null,
      reviewed: null,
    }
  }
  if (event.type === "reset") return null
  // Anything for another attempt (a late event from the one before) is ignored.
  if (!state || event.attempt_id !== state.attemptId) return state

  switch (event.type) {
    case "confidence":
      return { ...state, confidence: event.level }
    case "rated":
      return { ...state, confidence: event.level }
    case "answered":
      return { ...state, answer: event.choice }
    case "hint":
      // Rungs only climb; a repeated or lower rung must not empty the ladder.
      return event.rung < state.rung
        ? state
        : { ...state, rung: event.rung, hintText: event.text, hintIsAnswer: event.is_answer }
    case "checked":
      return {
        ...state,
        checked: {
          correct: event.correct,
          correctAnswer: event.correct_answer,
          explanation: event.explanation,
          misconception: event.misconception,
        },
      }
    case "reviewed":
      return { ...state, reviewed: { rating: event.rating, nextDue: event.next_due } }
  }
}

/** Which option the check named, by its key or its full text; null when it is neither. */
export function correctOptionId(options: AttemptOption[], correctAnswer: string): string | null {
  const want = correctAnswer.trim().toLowerCase()
  if (!want) return null
  const match =
    options.find((o) => o.id.toLowerCase() === want) ?? options.find((o) => o.text.trim().toLowerCase() === want)
  return match?.id ?? null
}

export const answerMessage =(choice: string) => `My answer: ${choice}`
export const confidenceMessage = (level: Confidence) => `Confidence: ${level}`

export type AttemptKey = { kind: "answer"; index: number } | { kind: "confidence"; level: Confidence }

/** A-D choose an answer (only when there are options to choose from); 1-4 rate confidence. */
export function attemptKey(key: string, optionCount: number): AttemptKey | null {
  if (key.length !== 1) return null
  const letter = "abcd".indexOf(key.toLowerCase())
  if (letter >= 0) return letter < optionCount ? { kind: "answer", index: letter } : null
  const digit = "1234".indexOf(key)
  return digit >= 0 ? { kind: "confidence", level: (digit + 1) as Confidence } : null
}

/** "Good", "Hard" and "Again" come from the server; anything else is shown as sent. */
export function ratingWord(rating: string): string {
  return rating ? rating.charAt(0).toUpperCase() + rating.slice(1).toLowerCase() : rating
}
