import { describe, expect, it } from "vitest"

import type { EvalRun } from "@/lib/backend"
import { sampleRuns } from "./fixtures"
import { annotate, deltaKind, fmtDelta, normDecision, orderMetrics, primaryMetric, tally, trajectorySeries } from "./model"

const run = (over: Partial<EvalRun> & { test?: number }): EvalRun => ({
  run_id: over.run_id ?? "r",
  created_at: over.created_at ?? 1,
  eval: over.eval ?? "retrieval",
  dataset: "d",
  change: over.change ?? "c",
  config: {},
  splits: { train: { n: 1, metrics: {} }, test: { n: 1, metrics: over.test === undefined ? {} : { "recall@10": over.test } } },
  per_domain: {},
  decision: over.decision ?? "keep",
  cost_usd: over.cost_usd ?? 0,
})

describe("annotate", () => {
  it("compares each run with the latest earlier baseline or keep run, not the previous run", () => {
    const items = annotate(
      [
        run({ created_at: 1, decision: "baseline", test: 0.6 }),
        run({ created_at: 2, decision: "keep", test: 0.7 }),
        run({ created_at: 3, decision: "revert", test: 0.65 }),
        run({ created_at: 4, decision: "pending", test: 0.72 }),
      ],
      "recall@10",
    )
    expect(items.map((a) => a.delta?.vs)).toEqual([undefined, 1, 2, 2])
    expect(items[0].delta).toBeNull()
    expect(items[1].delta?.value).toBeCloseTo(0.1)
    expect(items[2].delta?.value).toBeCloseTo(-0.05)
    expect(items[3].delta?.value).toBeCloseTo(0.02)
  })

  it("numbers runs from the oldest, whatever order they arrive in", () => {
    const items = annotate([run({ run_id: "b", created_at: 20 }), run({ run_id: "a", created_at: 10 })], "recall@10")
    expect(items.map((a) => [a.run.run_id, a.n])).toEqual([["a", 1], ["b", 2]])
  })

  it("does not compare across different evals", () => {
    const items = annotate([run({ created_at: 1, eval: "retrieval", test: 0.6 }), run({ created_at: 2, eval: "tutoring", test: 0.9 })], "recall@10")
    expect(items[1].delta).toBeNull()
  })

  it("treats an unknown decision as pending", () => {
    expect(normDecision("maybe")).toBe("pending")
    expect(annotate([run({ decision: "maybe" as never })], "recall@10")[0].decision).toBe("pending")
  })
})

describe("primaryMetric", () => {
  it("prefers recall@10 and follows the newest run that recorded metrics", () => {
    expect(primaryMetric(sampleRuns())).toBe("recall@10")
    expect(primaryMetric([run({ created_at: 1, test: 0.5 }), run({ created_at: 2 })])).toBe("recall@10")
    expect(primaryMetric([run({ created_at: 1 })])).toBeNull()
  })

  it("orders the known metrics first", () => {
    expect(orderMetrics(["zeta", "ndcg@10", "recall@10", "alpha"])).toEqual(["recall@10", "ndcg@10", "alpha", "zeta"])
  })
})

describe("trajectorySeries", () => {
  it("puts the decision shape and a note on every test point and bands on both splits", () => {
    const runs = sampleRuns()
    const series = trajectorySeries(annotate(runs, "recall@10"), "recall@10")
    const test = series.find((s) => s.id === "test")!
    expect(test.points).toHaveLength(runs.length)
    expect(test.points.map((p) => p.marker)).toEqual(["diamond", "circle", "cross", "circle", "circle", "cross", "cross", "circle", "cross", "circle", "triangle"])
    expect(test.points[2].note).toContain("Revert")
    expect(test.points.every((p) => p.lo !== undefined && p.hi !== undefined && p.lo <= p.y && p.y <= p.hi)).toBe(true)
    expect(series.find((s) => s.id === "train")!.points.every((p) => p.marker === undefined)).toBe(true)
  })
})

describe("deltas and tallies", () => {
  it("signs with a real minus and calls tiny changes flat", () => {
    expect(fmtDelta(0.0214)).toBe("+0.021")
    expect(fmtDelta(-0.0134)).toBe("−0.013")
    expect(deltaKind(0.0004)).toBe("flat")
    expect(deltaKind(-0.02)).toBe("down")
  })

  it("counts decisions and sums cost", () => {
    const t = tally(annotate(sampleRuns(), "recall@10"))
    expect(t).toMatchObject({ runs: 11, kept: 5, reverted: 4, pending: 1 })
    expect(t.cost).toBeGreaterThan(8)
  })
})
