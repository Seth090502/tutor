import { describe, expect, it } from "vitest"

import {
  assistanceSegments,
  calibrationRows,
  countDomain,
  dailySeries,
  gaugeMax,
  masteryTree,
  nextDue,
  phaseDays,
  sortTopics,
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
