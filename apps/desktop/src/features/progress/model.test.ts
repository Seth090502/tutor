import { describe, expect, it } from "vitest"

import {
  assistanceSegments,
  calibrationRows,
  calibrationSeries,
  countDomain,
  dailySeries,
  gaugeMax,
  masteryTree,
  nextDue,
  phaseDays,
  sortTopics,
  wilsonPct,
  type Topic,
} from "./model"

const day = (date: string, reviews: number) => ({ date, questions: 0, mistakes: 0, reviews })

describe("phaseDays", () => {
  it("lights each day relative to the busiest day and rings today", () => {
    const days = phaseDays([day("2026-09-28", 5), day("2026-09-29", 20), day("2026-09-30", 10)], "2026-09-30")
    expect(days.map((d) => d.value)).toEqual([0.25, 1, 0.5])
    expect(days.map((d) => d.isToday)).toEqual([false, false, true])
    expect(days[0].label).toBe("28")
  })
  it("leaves every disc dark when nothing was reviewed", () => {
    expect(phaseDays([day("2026-09-30", 0)], "2026-09-30")[0].value).toBe(0)
  })
})

describe("nextDue", () => {
  const now = Date.UTC(2026, 8, 30, 12, 0, 0)
  it("reports none, now, minutes, hours and days", () => {
    expect(nextDue(null, now).value).toBe("None")
    expect(nextDue(now / 1000 - 60, now).value).toBe("Now")
    expect(nextDue(now / 1000 + 20 * 60, now)).toMatchObject({ value: 20, label: "Next due in" })
    expect(nextDue(now / 1000 + 3 * 3600, now).value).toBe(3)
    expect(nextDue(now / 1000 + 4 * 86400, now).value).toBe(4)
  })
})

describe("gaugeMax", () => {
  it("is the smallest round scale at or above the count", () => {
    expect([0, 10, 11, 50, 51, 7000].map(gaugeMax)).toEqual([10, 10, 20, 50, 100, 7000])
  })
})

describe("assistanceSegments", () => {
  it("separates revealed attempts from hinted ones, since hinted includes them", () => {
    const [u, h, r] = assistanceSegments({ attempts: 107, hinted: 38, revealed: 9 })
    expect([u.value, h.value, r.value]).toEqual([69, 29, 9])
  })
  it("never goes negative on inconsistent input", () => {
    const v = assistanceSegments({ attempts: 5, hinted: 2, revealed: 4 }).map((s) => s.value)
    expect(v.every((n) => n >= 0)).toBe(true)
    expect(v.reduce((a, b) => a + b, 0)).toBe(5)
  })
})

describe("calibrationRows", () => {
  it("fills all four levels and leaves unattempted levels without a percentage", () => {
    const rows = calibrationRows([{ level: 3, attempts: 40, correct: 29 }])
    expect(rows.map((r) => r.pct)).toEqual([null, null, 73, null])
  })
})

describe("countDomain", () => {
  it("keeps whole-number ticks", () => {
    expect(countDomain([{ id: "a", label: "a", points: [{ x: 0, y: 1 }] }])).toEqual([0, 4])
    expect(countDomain([{ id: "a", label: "a", points: [{ x: 0, y: 13 }] }])).toEqual([0, 16])
  })
})

describe("dailySeries", () => {
  it("makes one series per measure with local-midnight x values", () => {
    const s = dailySeries([{ date: "2026-09-30", questions: 1, mistakes: 2, reviews: 3 }])
    expect(s.map((x) => x.points[0].y)).toEqual([1, 2, 3])
    expect(new Date(s[0].points[0].x).getDate()).toBe(30)
  })
})

describe("masteryTree", () => {
  const counts = { mastered: 1, learning: 1, new: 2, total: 4 }
  it("nests path, module and skill, and falls back to counts without modules", () => {
    const detailed: Topic = {
      path_id: "p",
      name: "P",
      map: { counts, modules: [{ id: "m", name: "M", knowledge_points: [{ id: "k", name: "K", mastery: 0.4 }] }] },
    }
    const bare: Topic = { path_id: "q", name: "Q", map: { counts } }
    const none: Topic = { path_id: "r", name: "R", map: { counts: { mastered: 0, learning: 0, new: 0, total: 0 } } }
    const tree = masteryTree([detailed, bare, none])
    expect(tree).toHaveLength(2)
    expect(tree[0].children?.[0].children?.[0]).toMatchObject({ label: "K", mastery: 0.4 })
    expect(tree[1]).toMatchObject({ value: 4, mastery: 0.25 })
  })
})

describe("sortTopics", () => {
  it("puts topics with the most due reviews first", () => {
    const t = (n: string, due: number): Topic => ({ path_id: n, name: n, map: { due_reviews: due } })
    expect(sortTopics([t("b", 0), t("a", 0), t("c", 3)]).map((x) => x.name)).toEqual(["c", "a", "b"])
  })
})

describe("wilsonPct", () => {
  it("is null without attempts", () => {
    expect(wilsonPct(0, 0)).toBeNull()
  })
  it("brackets the observed share and is wider for fewer attempts", () => {
    const [lo, hi] = wilsonPct(29, 40)!
    expect(lo).toBeCloseTo(57.2, 0)
    expect(hi).toBeCloseTo(83.9, 0)
    const [lo4, hi4] = wilsonPct(3, 4)!
    expect(hi4 - lo4).toBeGreaterThan(hi - lo)
  })
  it("stays inside 0 to 100 at the extremes", () => {
    const [lo0, hi0] = wilsonPct(0, 10)!
    expect(lo0).toBe(0)
    expect(hi0).toBeGreaterThan(0)
    const [lo1, hi1] = wilsonPct(10, 10)!
    expect(hi1).toBe(100)
    expect(lo1).toBeLessThan(100)
  })
})

describe("calibrationSeries", () => {
  it("puts each level on its own category and draws the reference dashed", () => {
    const { series, labels } = calibrationSeries(calibrationRows([{ level: 2, attempts: 10, correct: 5 }, { level: 4, attempts: 3, correct: 3 }]))
    expect(labels).toEqual(["Guess", "Unsure", "Fairly sure", "Certain"])
    const [actual, reference] = series
    expect(actual.points.map((p) => p.x)).toEqual([1, 3])
    expect(actual.points.map((p) => p.y)).toEqual([50, 100])
    expect(reference.dashed).toBe(true)
    expect(reference.points.map((p) => p.y)).toEqual([25, 50, 75, 95])
  })
  it("gives every plotted point an interval and flags small samples in its note", () => {
    const { series } = calibrationSeries(calibrationRows([{ level: 1, attempts: 3, correct: 1 }, { level: 3, attempts: 40, correct: 29 }]))
    const [a, b] = series[0].points
    expect(a.lo).toBeLessThan(a.y)
    expect(a.hi).toBeGreaterThan(a.y)
    expect(a.note).toBe("1 of 3 right, small sample")
    expect(b.note).toBe("29 of 40 right")
  })
  it("leaves out a level with no attempts", () => {
    const { series } = calibrationSeries(calibrationRows([]))
    expect(series[0].points).toEqual([])
    expect(series[1].points).toHaveLength(4)
  })
})
