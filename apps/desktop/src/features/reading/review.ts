// Small pure helpers for the review steps (missed units, answers).
import type { Confidence } from "./types"

/** The 1-4 scale as probabilities (docs/tutor/PEDAGOGY.md). */
export const CONFIDENCE_P = [0.25, 0.5, 0.75, 0.95] as const

export type Calibration = { verdict: "close" | "over" | "under" | "none"; expected: number; actual: number }

/** The learner's 1-4 judgement of their summary against the share of idea units it actually covered. */
export function calibration(judgement: Confidence, covered: number, total: number): Calibration {
  const expected = CONFIDENCE_P[judgement - 1]
  if (total <= 0) return { verdict: "none", expected, actual: 0 }
  const actual = covered / total
  const gap = expected - actual
  return { verdict: gap > 0.2 ? "over" : gap < -0.2 ? "under" : "close", expected, actual }
}

/** `text` cut to about `max` characters at a word boundary. */
export function shortText(text: string, max: number): { text: string; cut: boolean } {
  if (text.length <= max) return { text, cut: false }
  const at = text.lastIndexOf(" ", max)
  return { text: `${text.slice(0, at > 0 ? at : max).trimEnd()}…`, cut: true }
}
