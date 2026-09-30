// Jaeggi/Thompson per-block adaptation: up when every active stream has at most `up_max_errors`
// errors, down when any stream has at least `down_min_errors` (default 6, i.e. more than 5).
import type { Metrics } from "../types"
import { errors } from "./scoring"

export type Rule = { up_max_errors: number; down_min_errors: number }

export function nextLevel(n: number, metrics: { position: Metrics | null; letter: Metrics | null }, rule: Rule): number {
  const errs = [metrics.position, metrics.letter].filter((m): m is Metrics => m !== null).map(errors)
  if (errs.some((e) => e >= rule.down_min_errors)) return Math.max(1, n - 1)
  if (errs.every((e) => e <= rule.up_max_errors)) return n + 1
  return n
}

/** The rule as shown on screen and logged with the session settings. */
export const ruleText = (rule: Rule) =>
  `Up a level when each stream has ${rule.up_max_errors} or fewer errors; down when either has ${rule.down_min_errors} or more. Errors are misses plus false alarms.`
