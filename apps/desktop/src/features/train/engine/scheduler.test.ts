import { describe, expect, it } from "vitest"

import { LookaheadScheduler, blockOnsets, contextTimeAt, isNearestFrame, trialAt } from "./scheduler"

/** A fake audio clock the test advances by hand. */
const fakeClock = (t = 0) => ({ currentTime: t })

describe("LookaheadScheduler", () => {
  it("schedules each onset once, ahead of time, within the lookahead window", () => {
    const clock = fakeClock()
    const onsets = blockOnsets(0.5, 22, 3000)
    const calls: { i: number; when: number; at: number; late: boolean }[] = []
    const s = new LookaheadScheduler(clock, 0.12, (i, when, late) => calls.push({ i, when, at: clock.currentTime, late }))
    s.setOnsets(onsets)
    // A 25 ms timer, as setInterval would drive it, for the whole block.
    for (let t = 0; t <= 70; t += 0.025) {
      clock.currentTime = t
      s.tick()
    }
    expect(calls.map((c) => c.i)).toEqual(onsets.map((_, i) => i))
    for (const c of calls) {
      expect(c.when).toBe(onsets[c.i])
      expect(c.late).toBe(false)
      expect(c.at).toBeLessThanOrEqual(c.when)
      expect(c.when - c.at).toBeLessThanOrEqual(0.12 + 1e-9)
    }
    expect(s.done).toBe(true)
  })

  it("flags onsets the timer reached too late instead of dropping or repeating them", () => {
    const clock = fakeClock()
    const calls: [number, boolean][] = []
    const s = new LookaheadScheduler(clock, 0.1, (i, _when, late) => calls.push([i, late]))
    s.setOnsets([1, 4, 7])
    clock.currentTime = 0.95
    s.tick()
    clock.currentTime = 4.3 // the timer stalled (throttled window) past onset 1
    s.tick()
    s.tick()
    clock.currentTime = 6.95
    s.tick()
    expect(calls).toEqual([
      [0, false],
      [1, true],
      [2, false],
    ])
  })

  it("stops scheduling after cancel", () => {
    const clock = fakeClock()
    const calls: number[] = []
    const s = new LookaheadScheduler(clock, 0.1, (i) => calls.push(i))
    s.setOnsets([0.05, 3])
    s.tick()
    s.cancel()
    clock.currentTime = 3
    s.tick()
    expect(calls).toEqual([0])
    expect(s.done).toBe(true)
  })
})

describe("frame alignment", () => {
  it("shows the stimulus on the frame nearest the scheduled time", () => {
    const frame = 1000 / 60
    // Frames at 0, 16.7, 33.3 ms; onset at 20 ms: the 16.7 ms frame is nearest (3.3 ms early).
    expect(isNearestFrame(0, 20, frame)).toBe(false)
    expect(isNearestFrame(frame, 20, frame)).toBe(true)
    // Onset at 28 ms: 33.3 is nearer than 16.7.
    expect(isNearestFrame(frame, 28, frame)).toBe(false)
    expect(isNearestFrame(2 * frame, 28, frame)).toBe(true)
  })

  it("maps performance time to the audio clock through an output timestamp", () => {
    const stamp = { contextTime: 10, performanceTime: 5000 }
    expect(contextTimeAt(5250, stamp)).toBeCloseTo(10.25, 9)
    expect(contextTimeAt(4000, stamp)).toBeCloseTo(9, 9)
  })

  it("lays out trial onsets on the audio clock", () => {
    expect(blockOnsets(2, 3, 3000)).toEqual([2, 5, 8])
  })
})

describe("trialAt", () => {
  it("attributes a response to the trial whose window contains it", () => {
    const onsets = [1, 4, 7]
    expect(trialAt(onsets, 0.9, 3000)).toBe(-1)
    expect(trialAt(onsets, 1, 3000)).toBe(0)
    expect(trialAt(onsets, 3.99, 3000)).toBe(0)
    expect(trialAt(onsets, 4, 3000)).toBe(1)
    expect(trialAt(onsets, 9.99, 3000)).toBe(2)
    expect(trialAt(onsets, 10, 3000)).toBe(-1)
  })
})
