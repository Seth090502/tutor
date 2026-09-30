import { describe, expect, it } from "vitest"

import { normalizeRms, rms, synthTone, trimSilence } from "./dsp"

const RATE = 16000
const tone = (ms: number, amp = 0.5, hz = 440) =>
  Float32Array.from({ length: Math.round((RATE * ms) / 1000) }, (_, i) => amp * Math.sin((2 * Math.PI * hz * i) / RATE))
const silence = (ms: number, level = 0) =>
  Float32Array.from({ length: Math.round((RATE * ms) / 1000) }, (_, i) => (i % 2 ? level : -level))
const concat = (...parts: Float32Array[]) => {
  const out = new Float32Array(parts.reduce((n, p) => n + p.length, 0))
  let o = 0
  for (const p of parts) {
    out.set(p, o)
    o += p.length
  }
  return out
}

describe("trimSilence", () => {
  it("cuts leading and trailing silence, keeping a little noise floor out", () => {
    const x = concat(silence(120, 0.001), tone(300), silence(250, 0.001))
    const y = trimSilence(x, RATE)
    const ms = (y.length / RATE) * 1000
    expect(ms).toBeGreaterThan(295)
    expect(ms).toBeLessThan(320)
  })

  it("caps clips under 450 ms and fades the cut to zero", () => {
    const y = trimSilence(concat(silence(50), tone(800)), RATE)
    expect((y.length / RATE) * 1000).toBeLessThan(450)
    expect(Math.abs(y[y.length - 1])).toBeLessThan(1e-3)
  })

  it("returns an empty clip for pure silence", () => {
    expect(trimSilence(silence(300), RATE)).toHaveLength(0)
  })
})

describe("normalizeRms", () => {
  it("brings quiet and loud clips to the same RMS", () => {
    const quiet = normalizeRms(tone(300, 0.05), 0.1)
    const loud = normalizeRms(tone(300, 0.6), 0.1)
    expect(rms(quiet)).toBeCloseTo(0.1, 3)
    expect(rms(loud)).toBeCloseTo(0.1, 3)
  })

  it("never pushes the peak over the ceiling", () => {
    const spiky = concat(silence(300), Float32Array.of(0.9), silence(300))
    const y = normalizeRms(spiky, 0.2, 0.95)
    expect(Math.max(...y.map(Math.abs))).toBeLessThanOrEqual(0.95 + 1e-6)
  })
})

describe("synthTone", () => {
  it("has the requested length and ramps in and out", () => {
    const y = synthTone(523.25, RATE, 400)
    expect(y).toHaveLength(Math.round(RATE * 0.4))
    expect(Math.abs(y[0])).toBeLessThan(1e-3)
    expect(Math.abs(y[y.length - 1])).toBeLessThan(0.01)
    expect(Math.max(...y.map(Math.abs))).toBeGreaterThan(0.4)
  })
})
