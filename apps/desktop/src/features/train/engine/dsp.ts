// Sample-level processing for stimulus clips: silence trimming, loudness (RMS) normalisation and
// tone synthesis. Pure functions on mono Float32 samples.

export const MAX_CLIP_MS = 440 // clips stay under 450 ms
const FADE_MS = 10

export function rms(x: Float32Array): number {
  let sum = 0
  for (const v of x) sum += v * v
  return x.length ? Math.sqrt(sum / x.length) : 0
}

/** Drop leading and trailing samples quieter than `thresholdDb` below the peak, cap the clip at
 * MAX_CLIP_MS and fade the last 10 ms so a cut never clicks. */
export function trimSilence(x: Float32Array, sampleRate: number, thresholdDb = -35): Float32Array {
  let peak = 0
  for (const v of x) peak = Math.max(peak, Math.abs(v))
  if (peak === 0) return new Float32Array(0)
  const floor = peak * 10 ** (thresholdDb / 20)
  let start = 0
  while (start < x.length && Math.abs(x[start]) < floor) start++
  let end = x.length
  while (end > start && Math.abs(x[end - 1]) < floor) end--
  const max = Math.floor((sampleRate * MAX_CLIP_MS) / 1000)
  const y = x.slice(start, Math.min(end, start + max))
  const fade = Math.min(y.length, Math.floor((sampleRate * FADE_MS) / 1000))
  for (let k = 0; k < fade; k++) y[y.length - 1 - k] *= k / fade
  return y
}

/** Scale to `targetRms`, limited so the peak stays at or under `ceiling`. */
export function normalizeRms(x: Float32Array, targetRms: number, ceiling = 0.95): Float32Array {
  const level = rms(x)
  if (level === 0) return x.slice()
  let peak = 0
  for (const v of x) peak = Math.max(peak, Math.abs(v))
  const gain = Math.min(targetRms / level, ceiling / peak)
  return x.map((v) => v * gain)
}

/** A sine tone with 10 ms raised-cosine ramps, peak 0.5. */
export function synthTone(hz: number, sampleRate: number, ms: number): Float32Array {
  const len = Math.round((sampleRate * ms) / 1000)
  const ramp = Math.floor((sampleRate * FADE_MS) / 1000)
  return Float32Array.from({ length: len }, (_, i) => {
    const edge = Math.min(i, len - 1 - i)
    const env = edge >= ramp ? 1 : 0.5 - 0.5 * Math.cos((Math.PI * edge) / ramp)
    return 0.5 * env * Math.sin((2 * Math.PI * hz * i) / sampleRate)
  })
}
