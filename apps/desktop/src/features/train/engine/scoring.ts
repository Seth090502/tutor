// Signal-detection scoring per stream. Only trials i >= n are scored: earlier trials have no
// n-back item, so a response there is logged but not counted.
import type { Metrics, Trial } from "../types"
import type { Streams } from "./generate"

/** Inverse standard normal CDF (Acklam's rational approximation, relative error < 1.2e-9). */
export function probit(p: number): number {
  if (!(p > 0 && p < 1)) throw new RangeError(`probit needs 0 < p < 1, got ${p}`)
  const a = [-39.69683028665376, 220.9460984245205, -275.9285104469687, 138.357751867269, -30.66479806614716, 2.506628277459239]
  const b = [-54.47609879822406, 161.5858368580409, -155.6989798598866, 66.80131188771972, -13.28068155288572]
  const c = [-0.007784894002430293, -0.3223964580411365, -2.400758277161838, -2.549732539343734, 4.374664141464968, 2.938163982698783]
  const d = [0.007784695709041462, 0.3224671290700398, 2.445134137142996, 3.754408661907416]
  const lo = 0.02425
  let x: number
  if (p < lo) {
    const q = Math.sqrt(-2 * Math.log(p))
    x = (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1)
  } else if (p > 1 - lo) {
    const q = Math.sqrt(-2 * Math.log(1 - p))
    x = -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1)
  } else {
    const q = p - 0.5
    const r = q * q
    x = ((((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q) / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1)
  }
  return x
}

/** Hits, misses, false alarms, correct rejections, d' and c with the log-linear correction
 * (add 0.5 to each count and 1 to each total), so rates of 0 or 1 stay finite. */
export function scoreStream(trials: Trial[], stream: keyof Streams, n: number): Metrics {
  const [match, response] = stream === "position" ? (["pos_match", "pos_response"] as const) : (["let_match", "let_response"] as const)
  let hits = 0
  let misses = 0
  let fas = 0
  let crs = 0
  for (const t of trials) {
    if (t.i < n) continue
    if (t[match]) {
      if (t[response]) hits++
      else misses++
    } else if (t[response]) fas++
    else crs++
  }
  const zH = probit((hits + 0.5) / (hits + misses + 1))
  const zF = probit((fas + 0.5) / (fas + crs + 1))
  return { hits, misses, false_alarms: fas, correct_rejections: crs, d_prime: zH - zF, c: -(zH + zF) / 2 }
}

export function scoreBlock(trials: Trial[], n: number, streams: Streams) {
  return {
    position: streams.position ? scoreStream(trials, "position", n) : null,
    letter: streams.letter ? scoreStream(trials, "letter", n) : null,
  }
}

/** Errors for adaptation: misses plus false alarms. */
export const errors = (m: Metrics) => m.misses + m.false_alarms
