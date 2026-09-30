// Pure helpers behind the Train screen's presentation: what to show, not how it looks.
import { rollingMedian } from "./engine/history"
import { DEFAULT_SETTINGS } from "./engine/session"
import type { Block, Metrics, Mode, NBackSummary, Settings } from "./types"

export const TRANSFER_EVERY_DAYS = 28
export const CURVE_WINDOW = 5

/** Why a session would not go on the training curve; empty when it is comparable. */
export function nonComparableReasons(s: Settings): string[] {
  return [
    s.in_trial_feedback && "in-trial feedback is on",
    s.lures && "lures are on",
    (s.trial_ms !== DEFAULT_SETTINGS.trial_ms || s.stimulus_ms !== DEFAULT_SETTINGS.stimulus_ms) &&
      "timing differs from the default",
    (s.up_max_errors !== DEFAULT_SETTINGS.up_max_errors || s.down_min_errors !== DEFAULT_SETTINGS.down_min_errors) &&
      "the adaptation rule differs from the default",
  ].filter((r): r is string => typeof r === "string")
}

/** Share of the weekly soft cap used, clamped to 0..1 for the meter. */
export const doseFraction = (minutes: number, cap: number) =>
  cap > 0 && Number.isFinite(minutes) ? Math.min(1, Math.max(0, minutes / cap)) : 0

/** Whole days since `lastAt` (epoch seconds), or null when there is no earlier check. */
export const daysSince = (lastAt: number | null, now: number) =>
  lastAt === null ? null : Math.max(0, Math.floor((now - lastAt) / 86_400))

export type TransferStatus = { tone: "off" | "ok" | "warn"; state: string; detail: string }

export function transferStatus(days: number | null): TransferStatus {
  if (days === null) return { tone: "off", state: "No check yet", detail: "Take the first one when you are ready." }
  if (days >= TRANSFER_EVERY_DAYS) return { tone: "warn", state: "Due", detail: `Last check ${days} days ago.` }
  return { tone: "ok", state: "Not due", detail: `Last check ${days} days ago; next in ${TRANSFER_EVERY_DAYS - days} days.` }
}

/** Parse a typed integer and clamp it into range; null while the text is not a whole number. */
export function commitInt(draft: string, min: number, max: number): number | null {
  if (draft.trim() === "") return null
  const v = Number(draft)
  return Number.isFinite(v) ? Math.min(max, Math.max(min, Math.round(v))) : null
}

/** Sum counts over blocks; d' and c are the mean of the per-block values. */
export function pooled(ms: Metrics[]): Metrics | null {
  if (ms.length === 0) return null
  const sum = (k: keyof Metrics) => ms.reduce((a, m) => a + m[k], 0)
  return {
    hits: sum("hits"),
    misses: sum("misses"),
    false_alarms: sum("false_alarms"),
    correct_rejections: sum("correct_rejections"),
    d_prime: sum("d_prime") / ms.length,
    c: sum("c") / ms.length,
  }
}

export type LevelChange = { dir: "up" | "down" | "same" | "void" | "fixed"; from: number; to: number }

/** The level change after a block. Transfer blocks never adapt ("fixed"). */
export function levelChange(block: Block, transfer: boolean): LevelChange {
  const { n, next_n: to } = block
  if (block.void) return { dir: "void", from: n, to }
  if (transfer) return { dir: "fixed", from: n, to }
  return { dir: to > n ? "up" : to < n ? "down" : "same", from: n, to }
}

export type CurvePoint = {
  session: number // 1-based position on the curve
  startedAt: number
  level: number
  median: number
  lo: number
  hi: number
  dPosition: number | null
  dLetter: number | null
}

/** The training curve for one mode: comparable sessions only, oldest first, with a rolling median
 * band. Sessions that are not comparable are counted, not plotted. */
export function trainingCurve(
  sessions: NBackSummary[],
  mode: Exclude<Mode, "transfer">,
  window = CURVE_WINDOW,
): { points: CurvePoint[]; notComparable: number } {
  const ofMode = sessions.filter((s) => s.mode === mode)
  const comparable = ofMode.filter((s) => s.comparable).sort((a, b) => a.started_at - b.started_at)
  const bands = rollingMedian(
    comparable.map((s) => s.mean_n),
    window,
  )
  return {
    notComparable: ofMode.length - comparable.length,
    points: comparable.map((s, i) => ({
      session: i + 1,
      startedAt: s.started_at,
      level: s.mean_n,
      ...bands[i],
      dPosition: s.d_prime.position,
      dLetter: s.d_prime.letter,
    })),
  }
}
