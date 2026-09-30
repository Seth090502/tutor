// History helpers for the training curve and the weekly dose reminder.

export type Band = { median: number; lo: number; hi: number }

/** Median of each trailing window of up to `size` values, with the window's min and max. */
export function rollingMedian(values: number[], size: number): Band[] {
  return values.map((_, i) => {
    const w = values.slice(Math.max(0, i - size + 1), i + 1).sort((a, b) => a - b)
    const mid = w.length >> 1
    const median = w.length % 2 ? w[mid] : (w[mid - 1] + w[mid]) / 2
    return { median, lo: w[0], hi: w[w.length - 1] }
  })
}

export const WEEK_SOFT_CAP_MIN = 60
export const WEEK_SESSIONS = 3

/** Minutes and session count for sessions started in the 7 days before `now` (epoch seconds). */
export function weekMinutes(sessions: { started_at: number; ended_at: number }[], now: number) {
  const recent = sessions.filter((s) => s.started_at > now - 7 * 86_400)
  const seconds = recent.reduce((sum, s) => sum + Math.max(0, s.ended_at - s.started_at), 0)
  return { minutes: Math.round(seconds / 60), sessions: recent.length }
}
