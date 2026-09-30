import { describe, expect, it } from "vitest"

import { DEFAULT_SETTINGS } from "./engine/session"
import type { Block, Metrics, NBackSummary } from "./types"
import {
  daysSince,
  doseFraction,
  levelChange,
  nonComparableReasons,
  pooled,
  TRANSFER_EVERY_DAYS,
  trainingCurve,
  transferStatus,
} from "./view"

const m = (hits: number, misses: number, false_alarms: number, correct_rejections: number, d_prime: number, c: number): Metrics => ({
  hits,
  misses,
  false_alarms,
  correct_rejections,
  d_prime,
  c,
})

const summary = (id: string, started_at: number, mean_n: number, comparable = true, mode = "dual"): NBackSummary => ({
  id,
  started_at,
  mode,
  blocks: 15,
  void_blocks: 0,
  start_n: 1,
  max_n: Math.ceil(mean_n),
  mean_n,
  d_prime: { position: 1.5, letter: null },
  comparable,
})

const block = (over: Partial<Block>): Block => ({
  index: 0,
  n: 2,
  void: false,
  void_reason: null,
  trials: [],
  metrics: { position: null, letter: null },
  next_n: 2,
  ...over,
})

describe("nonComparableReasons", () => {
  it("is empty for the defaults", () => {
    expect(nonComparableReasons(DEFAULT_SETTINGS)).toEqual([])
  })
  it("names each difference", () => {
    const r = nonComparableReasons({ ...DEFAULT_SETTINGS, lures: true, in_trial_feedback: true, up_max_errors: 3 })
    expect(r).toEqual(["in-trial feedback is on", "lures are on", "the adaptation rule differs from the default"])
  })
  it("flags timing changes", () => {
    expect(nonComparableReasons({ ...DEFAULT_SETTINGS, trial_ms: 2500 })).toEqual(["timing differs from the default"])
  })
  it("ignores keys and block count, which stay comparable", () => {
    expect(nonComparableReasons({ ...DEFAULT_SETTINGS, blocks: 20, keys: { position: "KeyF", letter: "KeyJ" } })).toEqual([])
  })
})

describe("doseFraction", () => {
  it("clamps to the cap", () => {
    expect(doseFraction(30, 60)).toBe(0.5)
    expect(doseFraction(90, 60)).toBe(1)
    expect(doseFraction(-5, 60)).toBe(0)
  })
  it("survives a bad cap or value", () => {
    expect(doseFraction(10, 0)).toBe(0)
    expect(doseFraction(Number.NaN, 60)).toBe(0)
  })
})

describe("transferStatus", () => {
  const day = 86_400
  it("has three states", () => {
    expect(transferStatus(null).state).toBe("No check yet")
    expect(transferStatus(TRANSFER_EVERY_DAYS - 1)).toMatchObject({ tone: "ok", state: "Not due" })
    expect(transferStatus(TRANSFER_EVERY_DAYS)).toMatchObject({ tone: "warn", state: "Due" })
  })
  it("counts whole days and never goes negative", () => {
    expect(daysSince(null, 1000)).toBeNull()
    expect(daysSince(0, 3.5 * day)).toBe(3)
    expect(daysSince(1000, 10)).toBe(0)
  })
})

describe("pooled", () => {
  it("sums counts and averages d' and c", () => {
    const p = pooled([m(4, 2, 1, 13, 2, 0.2), m(6, 0, 3, 11, 1, -0.2)])
    expect(p).toEqual({ hits: 10, misses: 2, false_alarms: 4, correct_rejections: 24, d_prime: 1.5, c: 0 })
  })
  it("is null with nothing scored", () => {
    expect(pooled([])).toBeNull()
  })
})

describe("levelChange", () => {
  it("reads the direction from the block", () => {
    expect(levelChange(block({ n: 2, next_n: 3 }), false).dir).toBe("up")
    expect(levelChange(block({ n: 3, next_n: 2 }), false).dir).toBe("down")
    expect(levelChange(block({ n: 2, next_n: 2 }), false).dir).toBe("same")
  })
  it("void and transfer blocks never read as a change", () => {
    expect(levelChange(block({ void: true, void_reason: "focus lost" }), false).dir).toBe("void")
    expect(levelChange(block({ n: 2, next_n: 2 }), true).dir).toBe("fixed")
  })
})

describe("trainingCurve", () => {
  it("plots comparable sessions of one mode, oldest first", () => {
    const { points, notComparable } = trainingCurve(
      [summary("c", 300, 3), summary("a", 100, 1), summary("x", 150, 9, false), summary("p", 200, 4, true, "position"), summary("b", 200, 2)],
      "dual",
    )
    expect(points.map((p) => p.level)).toEqual([1, 2, 3])
    expect(points.map((p) => p.session)).toEqual([1, 2, 3])
    expect(notComparable).toBe(1)
  })
  it("carries the rolling median and its range", () => {
    const { points } = trainingCurve([1, 3, 2].map((n, i) => summary(String(i), i, n)), "dual", 3)
    expect(points[2]).toMatchObject({ median: 2, lo: 1, hi: 3 })
  })
  it("is empty without sessions", () => {
    expect(trainingCurve([], "letter")).toEqual({ points: [], notComparable: 0 })
  })
})
