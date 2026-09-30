import { describe, expect, it } from "vitest"

import { rollingMedian, weekMinutes } from "./history"

describe("rollingMedian", () => {
  it("takes the median of each trailing window, with the window's range", () => {
    expect(rollingMedian([1, 3, 2, 5, 4], 3)).toEqual([
      { median: 1, lo: 1, hi: 1 },
      { median: 2, lo: 1, hi: 3 },
      { median: 2, lo: 1, hi: 3 },
      { median: 3, lo: 2, hi: 5 },
      { median: 4, lo: 2, hi: 5 },
    ])
  })
  it("averages the middle pair for even windows", () => {
    expect(rollingMedian([1, 2, 4, 8], 4)[3].median).toBe(3)
  })
})

describe("weekMinutes", () => {
  it("sums session minutes started in the last 7 days", () => {
    const now = 1_000_000
    const day = 86_400
    const sessions = [
      { started_at: now - 600, ended_at: now },
      { started_at: now - 3 * day, ended_at: now - 3 * day + 1200 },
      { started_at: now - 8 * day, ended_at: now - 8 * day + 1800 },
    ]
    expect(weekMinutes(sessions, now)).toEqual({ minutes: 30, sessions: 2 })
  })
})
