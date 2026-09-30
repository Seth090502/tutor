import { describe, expect, it } from "vitest"

import { describePhaseDay, lineTable, rangeTable, stackTable, sunburstTable, phaseTable, sparklineTable, gaugeTable } from "./index"

describe("table alternatives", () => {
  it("lists one row per x with bands and notes in the series cells", () => {
    const t = lineTable(
      [
        { id: "a", label: "Train", points: [{ x: 1, y: 0.5, lo: 0.4, hi: 0.6 }, { x: 2, y: 0.6 }] },
        { id: "b", label: "Test", points: [{ x: 2, y: 0.4, note: "reverted" }] },
      ],
      { xTitle: "Run", xFormat: (x) => `#${x}`, yFormat: (y) => y.toFixed(2) },
    )
    expect(t.columns).toEqual(["Run", "Train", "Test"])
    expect(t.rows).toEqual([
      ["#1", "0.50 (0.40–0.60)", "—"],
      ["#2", "0.60", "0.40 reverted"],
    ])
  })

  it("makes one column per group with the interval in each cell", () => {
    const t = rangeTable([{ id: "r", label: "Recall", groups: [{ id: "tr", label: "Train", value: 0.8, lo: 0.7, hi: 0.9 }, { id: "te", label: "Test", value: 0.6 }] }])
    expect(t.columns).toEqual(["Metric", "Train", "Test"])
    expect(t.rows[0]).toEqual(["Recall", "0.8 (0.7–0.9)", "0.6"])
  })

  it("adds shares that sum to 100", () => {
    const t = stackTable([{ id: "a", label: "A", value: 1 }, { id: "b", label: "B", value: 1 }, { id: "c", label: "C", value: 1 }], "pages")
    expect(t.columns).toEqual(["Segment", "Pages", "Share"])
    const total = t.rows.reduce((s, r) => s + parseFloat(String(r[2])), 0)
    expect(total).toBeCloseTo(100, 5)
  })

  it("flattens the sunburst tree with levels and rolled-up values", () => {
    const t = sunburstTable([{ id: "m", label: "Math", children: [{ id: "a", label: "Algebra", children: [{ id: "l", label: "Linear", value: 4, mastery: 0.5 }] }] }])
    expect(t.rows.map((r) => [String(r[0]).trim(), r[1], r[2], r[3]])).toEqual([
      ["Math", "Subject", 4, "50%"],
      ["Algebra", "Topic", 4, "50%"],
      ["Linear", "Skill", 4, "50%"],
    ])
  })

  it("covers phases, sparklines and gauges", () => {
    expect(phaseTable([{ id: "1", label: "1", title: "Mon", value: 0.5 }, { id: "2", label: "2", title: "Tue", value: null, isToday: true }]).rows).toEqual([["Mon", "50%"], ["Tue (today)", "—"]])
    expect(sparklineTable([1, 2.345], "Reviews").rows).toEqual([["1", "1"], ["2", "2.35"]])
    expect(gaugeTable({ value: 12, max: 40, label: "Due" }).rows).toEqual([["Due", 12, 0, 40]])
  })
})

describe("phase discs wording", () => {
  const day = { id: "1", label: "14", title: "Mon, Sep 14", value: 0.724 }
  it("describes a day with the default unit, today, and a day not yet reached", () => {
    expect(describePhaseDay(day)).toBe("Mon, Sep 14: 72% of the daily goal")
    expect(describePhaseDay({ ...day, isToday: true })).toBe("Mon, Sep 14 (today): 72% of the daily goal")
    expect(describePhaseDay({ ...day, value: null })).toBe("Mon, Sep 14: not yet")
  })
  it("takes the unit from the caller and caps above 100%", () => {
    expect(describePhaseDay({ ...day, value: 1.6 }, "of your busiest day")).toBe("Mon, Sep 14: 100% of your busiest day")
  })
  it("names the share column in the table", () => {
    expect(phaseTable([day]).columns).toEqual(["Day", "Share of daily goal"])
    expect(phaseTable([day], "Reviews", "Share of your busiest day").columns).toEqual(["Day", "Share of your busiest day"])
  })
})
