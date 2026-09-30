import { describe, expect, it } from "vitest"

import {
  arcPath,
  areaPath,
  bandPath,
  cubicBezier,
  easeInOut,
  focusTarget,
  gaugeTicks,
  indexNodes,
  layoutSunburst,
  lineageOf,
  linearScale,
  linePath,
  loneEntries,
  markerPath,
  navigateArcs,
  nearestIndex,
  needleAngle,
  niceTicks,
  niceTimeTicks,
  percentages,
  phaseGeometry,
  phasePath,
  placeTooltip,
  placeTooltipAbove,
  polar,
  repelLabels,
  ROOT_ID,
  runsOf,
  tickStride,
  whiskerPath,
  sparklinePoints,
  stackLayout,
  truncateLabel,
  type SunburstDatum,
} from "./geometry"

describe("linearScale", () => {
  it("maps the domain onto the range and inverts", () => {
    const s = linearScale(0, 10, 100, 300)
    expect(s(0)).toBe(100)
    expect(s(10)).toBe(300)
    expect(s(5)).toBe(200)
    expect(s.invert(200)).toBe(5)
  })
  it("collapses a zero-width domain to the middle of the range", () => {
    expect(linearScale(3, 3, 0, 100)(3)).toBe(50)
  })
  it("supports inverted ranges (y axes)", () => {
    const s = linearScale(0, 1, 200, 0)
    expect(s(0)).toBe(200)
    expect(s(1)).toBe(0)
  })
})

describe("niceTicks", () => {
  it("gives round ticks covering the data", () => {
    const t = niceTicks(0, 1)
    expect(t.ticks).toEqual([0, 0.2, 0.4, 0.6, 0.8, 1])
  })
  it("expands the domain to the tick grid", () => {
    const t = niceTicks(3, 97)
    expect(t.min).toBe(0)
    expect(t.max).toBe(100)
    expect(t.ticks).toEqual([0, 20, 40, 60, 80, 100])
  })
  it("handles fractional data without float noise", () => {
    const t = niceTicks(0.61, 0.93)
    expect(t.ticks[0]).toBeLessThanOrEqual(0.61)
    expect(t.ticks[t.ticks.length - 1]).toBeGreaterThanOrEqual(0.93)
    for (const v of t.ticks) expect(String(v).length).toBeLessThan(7)
  })
  it("pads a flat series instead of returning one tick", () => {
    const t = niceTicks(5, 5)
    expect(t.ticks.length).toBeGreaterThan(2)
    expect(t.min).toBeLessThan(5)
    expect(t.max).toBeGreaterThan(5)
  })
  it("works for negative ranges and swapped arguments", () => {
    const t = niceTicks(10, -10)
    expect(t.min).toBe(-10)
    expect(t.ticks).toContain(0)
  })
})

describe("niceTimeTicks", () => {
  it("lands on local midnights for a multi-day span", () => {
    const t0 = new Date(2026, 8, 1, 13, 5).getTime()
    const t1 = new Date(2026, 8, 30, 9, 0).getTime()
    const { ticks, unit } = niceTimeTicks(t0, t1, 6)
    expect(unit).toBe("day")
    expect(ticks.length).toBeGreaterThan(1)
    expect(ticks.length).toBeLessThanOrEqual(7)
    for (const t of ticks) {
      const d = new Date(t)
      expect([d.getHours(), d.getMinutes()]).toEqual([0, 0])
      expect(t).toBeGreaterThanOrEqual(t0)
      expect(t).toBeLessThanOrEqual(t1)
    }
  })
  it("uses hours for spans under two days", () => {
    const t0 = new Date(2026, 8, 1, 8, 0).getTime()
    const { unit, ticks } = niceTimeTicks(t0, t0 + 10 * 3_600_000, 6)
    expect(unit).toBe("hour")
    expect(ticks.length).toBeGreaterThan(1)
  })
})

describe("nearestIndex", () => {
  it("finds the closest entry", () => {
    expect(nearestIndex([0, 10, 20, 30], 12)).toBe(1)
    expect(nearestIndex([0, 10, 20, 30], 26)).toBe(3)
    expect(nearestIndex([0, 10, 20, 30], -50)).toBe(0)
    expect(nearestIndex([7], 100)).toBe(0)
    expect(nearestIndex([], 1)).toBe(-1)
  })
})

describe("cubicBezier", () => {
  it("pins the endpoints and stays monotonic for the ease-inout token", () => {
    expect(easeInOut(0)).toBe(0)
    expect(easeInOut(1)).toBe(1)
    let prev = 0
    for (let i = 1; i <= 50; i++) {
      const v = easeInOut(i / 50)
      expect(v).toBeGreaterThanOrEqual(prev)
      prev = v
    }
    // The token is front-loaded: hand-solving x(s) = 0.5 gives s of about 0.595 and y of about 0.64.
    expect(easeInOut(0.5)).toBeCloseTo(0.63, 1)
  })
  it("is the identity for a linear curve", () => {
    const lin = cubicBezier(0, 0, 1, 1)
    expect(lin(0.3)).toBeCloseTo(0.3, 4)
  })
})

describe("paths", () => {
  it("builds line, area and band paths", () => {
    const pts = [
      { x: 0, y: 10 },
      { x: 10, y: 5.555 },
    ]
    expect(linePath(pts)).toBe("M0 10L10 5.56")
    expect(areaPath(pts, 20)).toBe("M0 10L10 5.56L10 20L0 20Z")
    expect(bandPath(pts, [{ x: 0, y: 12 }, { x: 10, y: 9 }])).toBe("M0 10L10 5.56L10 9L0 12Z")
    expect(bandPath(pts, [])).toBe("")
  })
  it("splits runs around gaps", () => {
    expect(runsOf([1, 2, 0, 3, 0, 0, 4, 5], (n) => n > 0)).toEqual([[1, 2], [3], [4, 5]])
  })
})

describe("repelLabels", () => {
  const gap = 14
  const noOverlap = (ys: number[]) => {
    const s = [...ys].sort((a, b) => a - b)
    for (let i = 1; i < s.length; i++) expect(s[i] - s[i - 1]).toBeGreaterThanOrEqual(gap - 1e-6)
  }
  it("leaves well-spaced labels alone", () => {
    expect(repelLabels([20, 80, 140], gap, 0, 200)).toEqual([20, 80, 140])
  })
  it("separates a tight cluster around its centre and keeps order", () => {
    const out = repelLabels([100, 102, 104], gap, 0, 300)
    noOverlap(out)
    expect(out[0]).toBeLessThan(out[1])
    expect(out[1]).toBeLessThan(out[2])
    expect(out[1]).toBeCloseTo(102, 5)
  })
  it("preserves the caller's order when targets are unsorted", () => {
    const out = repelLabels([104, 100, 102], gap, 0, 300)
    expect(out[1]).toBeLessThan(out[2])
    expect(out[2]).toBeLessThan(out[0])
  })
  it("stays inside the bounds", () => {
    const out = repelLabels([2, 3, 4], gap, 0, 200)
    noOverlap(out)
    expect(Math.min(...out)).toBeGreaterThanOrEqual(0)
    const low = repelLabels([198, 199, 200], gap, 0, 200)
    noOverlap(low)
    expect(Math.max(...low)).toBeLessThanOrEqual(200)
  })
  it("cascades merges across several clusters", () => {
    const out = repelLabels([50, 60, 70, 80, 90, 100], gap, 0, 400)
    noOverlap(out)
    expect(out).toHaveLength(6)
  })
  it("handles empty and single input", () => {
    expect(repelLabels([], gap, 0, 100)).toEqual([])
    expect(repelLabels([42], gap, 0, 100)).toEqual([42])
  })
})

describe("placeTooltip", () => {
  it("sits right of the anchor and flips near the right edge", () => {
    expect(placeTooltip(100, 10, 120, 60, 600, 300).x).toBe(114)
    const flipped = placeTooltip(560, 10, 120, 60, 600, 300)
    expect(flipped.x).toBe(560 - 14 - 120)
  })
  it("clamps vertically", () => {
    expect(placeTooltip(0, 999, 50, 60, 600, 300).y).toBe(236)
  })
})

describe("placeTooltipAbove", () => {
  it("centres above the anchor and clamps to the container", () => {
    expect(placeTooltipAbove(200, 150, 100, 40, 400, 300)).toEqual({ x: 150, y: 96 })
    expect(placeTooltipAbove(10, 150, 100, 40, 400, 300).x).toBe(4)
    expect(placeTooltipAbove(395, 150, 100, 40, 400, 300).x).toBe(296)
  })
  it("drops below the anchor when it would leave the top", () => {
    expect(placeTooltipAbove(200, 20, 100, 40, 400, 300).y).toBe(34)
  })
})

describe("markers", () => {
  it("returns a path for every shape, closed except the open cross", () => {
    for (const s of ["circle", "diamond", "square", "triangle"] as const) expect(markerPath(s, 5).endsWith("Z")).toBe(true)
    expect(markerPath("cross", 5)).toBe("M-4.5 -4.5L4.5 4.5M4.5 -4.5L-4.5 4.5")
  })
})

describe("arcPath", () => {
  it("draws a quarter sector between two radii", () => {
    expect(arcPath(50, 100, 0, Math.PI / 2)).toBe("M0 -100A100 100 0 0 1 100 0L50 0A50 50 0 0 0 0 -50Z")
  })
  it("uses the large-arc flag past a half turn", () => {
    expect(arcPath(50, 100, 0, Math.PI * 1.5)).toContain("0 1 1")
  })
  it("returns a full ring with a hole as two subpaths", () => {
    const d = arcPath(50, 100, 0, Math.PI * 2)
    expect(d.match(/M/g)).toHaveLength(2)
  })
  it("returns nothing for a collapsed sector", () => {
    expect(arcPath(50, 100, 1, 1)).toBe("")
    expect(arcPath(100, 100, 0, 1)).toBe("")
  })
  it("starts at twelve o'clock and runs clockwise", () => {
    const p = polar(10, Math.PI / 2)
    expect(p.x).toBeCloseTo(10)
    expect(p.y).toBeCloseTo(0)
  })
})

describe("gauge", () => {
  it("lays 21 ticks over 124 degrees with a major tick every fifth", () => {
    const t = gaugeTicks(100, 100, 70, 80)
    expect(t).toHaveLength(21)
    expect(t[0].angle).toBeCloseTo(-62)
    expect(t[20].angle).toBeCloseTo(62)
    expect(t[10].angle).toBeCloseTo(0)
    expect(t.filter((x) => x.major)).toHaveLength(5)
    expect(t[10].x1).toBeCloseTo(100)
    expect(t[10].y2).toBeCloseTo(20)
  })
  it("maps values to needle angles and clamps", () => {
    expect(needleAngle(0, 0, 40)).toBe(-62)
    expect(needleAngle(40, 0, 40)).toBe(62)
    expect(needleAngle(20, 0, 40)).toBe(0)
    expect(needleAngle(99, 0, 40)).toBe(62)
    expect(needleAngle(-5, 0, 40)).toBe(-62)
    expect(needleAngle(1, 5, 5)).toBe(-62)
  })
})

describe("moon phases", () => {
  it("has a straight terminator at half and a full-width ellipse at the extremes", () => {
    expect(phaseGeometry(0.5, 10).rx).toBe(0)
    expect(phaseGeometry(0.25, 10).rx).toBe(5)
    expect(phaseGeometry(0.75, 10).rx).toBe(5)
  })
  it("bulges toward the lit side for crescents and the dark side for gibbous", () => {
    expect(phaseGeometry(0.2, 10).sweep).toBe(0)
    expect(phaseGeometry(0.8, 10).sweep).toBe(1)
  })
  it("renders nothing when new and a full disc when full", () => {
    expect(phasePath(0, 10)).toBe("")
    expect(phasePath(1, 10)).toContain("A10 10 0 1 1")
    expect(phasePath(3, 10)).toBe(phasePath(1, 10))
  })
  it("encloses the requested share of the disc", () => {
    // Area of right semicircle +/- half ellipse, from the same arc parameters the path uses.
    for (const lit of [0.1, 0.3, 0.5, 0.7, 0.9]) {
      const r = 10
      const { rx, sweep } = phaseGeometry(lit, r)
      const area = (Math.PI * r * r) / 2 + (sweep === 1 ? 1 : -1) * (Math.PI * rx * r) / 2
      expect(area / (Math.PI * r * r)).toBeCloseTo(lit, 6)
    }
  })
  it("writes the half-moon path", () => {
    expect(phasePath(0.5, 10)).toBe("M0 -10A10 10 0 0 1 0 10A0 10 0 0 1 0 -10Z")
  })
})

describe("stack bar", () => {
  it("spreads segments proportionally with 2px gaps", () => {
    const seg = stackLayout([50, 25, 25], 202, 2)
    expect(seg[0]).toEqual({ x: 0, w: 99 })
    expect(seg[1].x).toBe(101)
    expect(seg[2].x + seg[2].w).toBeCloseTo(202)
  })
  it("gives empty segments no width and no gap", () => {
    const seg = stackLayout([10, 0, 10], 102, 2)
    expect(seg[1].w).toBe(0)
    expect(seg[2].x).toBe(seg[0].w + 2)
  })
  it("returns zeros for an empty total", () => {
    expect(stackLayout([0, 0], 100)).toEqual([{ x: 0, w: 0 }, { x: 0, w: 0 }])
  })
  it("makes percentages that sum to exactly 100", () => {
    const p = percentages([1, 1, 1])
    expect(p.reduce((a, b) => a + b, 0)).toBe(100)
    expect(percentages([412, 268, 97, 153]).reduce((a, b) => a + b, 0)).toBe(100)
    expect(percentages([1, 2, 4], 1).reduce((a, b) => a + b, 0)).toBeCloseTo(100, 5)
    expect(percentages([0, 0])).toEqual([0, 0])
  })
})

const TREE: SunburstDatum[] = [
  {
    id: "math",
    label: "Math",
    children: [
      { id: "alg", label: "Algebra", children: [{ id: "lin", label: "Linear", value: 6, mastery: 1 }, { id: "quad", label: "Quadratics", value: 2, mastery: 0 }] },
      { id: "calc", label: "Calculus", children: [{ id: "lim", label: "Limits", value: 4, mastery: 0.5 }] },
    ],
  },
  { id: "eng", label: "English", children: [{ id: "gram", label: "Grammar", children: [{ id: "comma", label: "Commas", value: 4, mastery: 0.25 }] }] },
]

describe("sunburst layout", () => {
  const nodes = layoutSunburst(TREE)
  const byId = indexNodes(nodes)

  it("sums values up and weights mastery by value", () => {
    expect(byId.get(ROOT_ID)!.value).toBe(16)
    expect(byId.get("alg")!.value).toBe(8)
    expect(byId.get("alg")!.mastery).toBeCloseTo(0.75)
    expect(byId.get("math")!.value).toBe(12)
  })
  it("gives angle proportional to item count and tiles the circle", () => {
    const math = byId.get("math")!
    const eng = byId.get("eng")!
    expect(math.a0).toBe(0)
    expect(math.a1).toBeCloseTo(0.75)
    expect(eng.a0).toBeCloseTo(0.75)
    expect(eng.a1).toBeCloseTo(1)
    const alg = byId.get("alg")!
    expect(alg.a0).toBe(0)
    expect(alg.a1).toBeCloseTo(0.5)
  })
  it("tracks the top-level index for hue and trims beyond the ring limit", () => {
    expect(byId.get("lin")!.top).toBe(0)
    expect(byId.get("comma")!.top).toBe(1)
    const deep = layoutSunburst([{ id: "a", label: "A", children: [{ id: "b", label: "B", children: [{ id: "c", label: "C", children: [{ id: "d", label: "D" }] }] }] }], 3)
    expect(deep.find((n) => n.id === "d")).toBeUndefined()
    expect(deep.find((n) => n.id === "c")!.childIds).toEqual([])
  })
})

describe("sunburst zoom", () => {
  const nodes = layoutSunburst(TREE)
  const cfg = { hole: 60, radius: 180 }

  it("lays three rings out contiguously from the hole when fully zoomed out", () => {
    const t = focusTarget(nodes, ROOT_ID, cfg)
    expect(t.get("math")!.r0).toBe(60)
    expect(t.get("math")!.r1).toBe(100)
    expect(t.get("alg")!.r0).toBe(100)
    expect(t.get("lin")!.r1).toBe(180)
    expect(t.get("math")!.visible).toBe(true)
    expect(t.get(ROOT_ID)!.visible).toBe(false)
  })
  it("expands a zoomed node's children to the full circle and hides the rest", () => {
    const t = focusTarget(nodes, "math", cfg)
    expect(t.get("alg")!.a0).toBe(0)
    expect(t.get("alg")!.a1).toBeCloseTo(2 / 3)
    expect(t.get("calc")!.a1).toBeCloseTo(1)
    expect(t.get("lin")!.a1).toBeCloseTo(0.5)
    // two rings remain below Math, so each gets half the band
    expect(t.get("alg")!.r1).toBe(120)
    expect(t.get("lin")!.r1).toBe(180)
    const eng = t.get("eng")!
    expect(eng.visible).toBe(false)
    expect(eng.opacity).toBe(0)
    expect(eng.a1 - eng.a0).toBe(0)
    expect(t.get("math")!.visible).toBe(false)
  })
  it("collapses every ancestor of the focus into the hole", () => {
    const t = focusTarget(nodes, "alg", cfg)
    for (const id of [ROOT_ID, "math", "alg"]) expect(t.get(id)!.r1).toBe(60)
    expect(t.get("lin")!.visible).toBe(true)
    expect(t.get("lin")!.r0).toBe(60)
    expect(t.get("lin")!.r1).toBe(180)
  })
  it("returns the node, its ancestors and descendants as the lineage", () => {
    expect([...lineageOf(nodes, "alg")].sort()).toEqual(["alg", "lin", "math", "quad"])
    expect([...lineageOf(nodes, "lim")].sort()).toEqual(["calc", "lim", "math"])
    expect(lineageOf(nodes, "nope").size).toBe(0)
  })
})

describe("arc keyboard navigation", () => {
  const nodes = layoutSunburst(TREE)
  const all = () => true
  it("walks siblings with wrap-around", () => {
    expect(navigateArcs(nodes, all, "math", "ArrowRight")).toBe("eng")
    expect(navigateArcs(nodes, all, "eng", "ArrowRight")).toBe("math")
    expect(navigateArcs(nodes, all, "math", "ArrowLeft")).toBe("eng")
  })
  it("moves between rings", () => {
    expect(navigateArcs(nodes, all, "math", "ArrowDown")).toBe("alg")
    expect(navigateArcs(nodes, all, "alg", "ArrowUp")).toBe("math")
    expect(navigateArcs(nodes, all, "lin", "ArrowDown")).toBe("lin")
  })
  it("never lands on a hidden arc", () => {
    const visible = (id: string) => id !== ROOT_ID && id !== "math"
    expect(navigateArcs(nodes, visible, "alg", "ArrowUp")).toBe("alg")
  })
})

describe("sparklinePoints / truncateLabel", () => {
  it("spans the box with the high value at the top", () => {
    const p = sparklinePoints([1, 3, 2], 100, 20, 2)
    expect(p[0].x).toBe(2)
    expect(p[2].x).toBe(98)
    expect(p[1].y).toBe(2)
    expect(p[0].y).toBe(18)
  })
  it("centres a flat series", () => {
    const p = sparklinePoints([4, 4], 100, 20, 2)
    expect(p[0].y).toBe(p[1].y)
  })
  it("truncates with an ellipsis", () => {
    expect(truncateLabel("abcdef", 4)).toBe("abc…")
    expect(truncateLabel("abc", 4)).toBe("abc")
  })
})

describe("single-point whiskers", () => {
  it("picks out only the entries that have no neighbour in their run", () => {
    const runs = runsOf([1, 2, 0, 3, 0, 4, 5, 6], (n) => n > 0)
    expect(runs).toEqual([[1, 2], [3], [4, 5, 6]])
    expect(loneEntries(runs)).toEqual([3])
    expect(loneEntries([])).toEqual([])
  })

  it("draws a stem with a cap at each end", () => {
    expect(whiskerPath(10, 50, 20, 4)).toBe("M10 50V20M6 50H14M6 20H14")
  })
})

describe("tickStride", () => {
  it("labels every tick when they fit and thins them when they do not", () => {
    expect(tickStride(4, 300, 60)).toBe(1)
    expect(tickStride(10, 300, 60)).toBe(3)
    expect(tickStride(20, 200, 40)).toBe(5)
  })
  it("handles degenerate input", () => {
    expect(tickStride(1, 300, 60)).toBe(1)
    expect(tickStride(5, 0, 60)).toBe(1)
  })
})
