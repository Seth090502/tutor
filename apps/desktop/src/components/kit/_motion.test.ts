import { describe, expect, it } from "vitest"

import { clamp01, easeOutCubic, smoothing, tween, type FrameEnv } from "./_motion"

/** Manual frame scheduler: flush(ts) runs every callback queued so far. */
function fakeEnv() {
  let next = 1
  const queue = new Map<number, (ts: number) => void>()
  const env: FrameEnv = {
    raf: (cb) => {
      const id = next++
      queue.set(id, cb)
      return id
    },
    caf: (id) => void queue.delete(id),
  }
  const flush = (ts: number) => {
    const cbs = [...queue.values()]
    queue.clear()
    cbs.forEach((cb) => cb(ts))
  }
  return { env, flush, pending: () => queue.size }
}

describe("easing helpers", () => {
  it("easeOutCubic hits its endpoints and decelerates", () => {
    expect(easeOutCubic(0)).toBe(0)
    expect(easeOutCubic(1)).toBe(1)
    expect(easeOutCubic(0.5)).toBeCloseTo(0.875, 6)
  })

  it("smoothing grows with dt and stays below 1", () => {
    expect(smoothing(0)).toBe(0)
    expect(smoothing(1 / 60)).toBeLessThan(smoothing(1 / 30))
    expect(smoothing(10)).toBeLessThanOrEqual(1)
  })

  it("clamp01 bounds values", () => {
    expect([clamp01(-1), clamp01(0.4), clamp01(3)]).toEqual([0, 0.4, 1])
  })
})

describe("tween", () => {
  it("runs from -> to over the duration and finishes exactly on target", () => {
    const { env, flush } = fakeEnv()
    const seen: number[] = []
    let done = 0
    tween({ from: 0, to: 100, duration: 900, onFrame: (v) => seen.push(v), onDone: () => done++ }, env)
    flush(1000) // first frame defines the start
    flush(1450)
    flush(1900)
    expect(seen[0]).toBe(0)
    expect(seen[1]).toBeCloseTo(100 * easeOutCubic(0.5), 6)
    expect(seen[2]).toBe(100)
    expect(done).toBe(1)
  })

  it("cancel stops delivery and leaves nothing scheduled", () => {
    const { env, flush, pending } = fakeEnv()
    const seen: number[] = []
    const cancel = tween({ from: 0, to: 10, duration: 500, onFrame: (v) => seen.push(v) }, env)
    flush(0)
    cancel()
    expect(pending()).toBe(0)
    flush(250)
    expect(seen).toHaveLength(1)
  })

  it("a zero duration jumps straight to the target", () => {
    const { env, flush } = fakeEnv()
    const seen: number[] = []
    tween({ from: 5, to: 9, duration: 0, onFrame: (v) => seen.push(v) }, env)
    flush(0)
    expect(seen).toEqual([9])
  })
})
