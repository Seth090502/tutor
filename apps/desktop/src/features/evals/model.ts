// Pure helpers for the Evals screen: metric lookup, the incumbent-delta rule and the trajectory series.
import type { EvalRun, SplitResult } from "@/lib/backend"
import type { MarkerShape, Series, SeriesPoint } from "@/components/charts"

export type Decision = EvalRun["decision"]

const DECISIONS: readonly Decision[] = ["baseline", "keep", "revert", "pending"]

/** An unknown decision string in a hand-edited run file reads as pending rather than crashing the screen. */
export const normDecision = (d: unknown): Decision => (DECISIONS.includes(d as Decision) ? (d as Decision) : "pending")

/** The shape is the signal: the same glyph marks the chart point and the badge. Tone colours the badge only. */
export const DECISION_META: Record<Decision, { label: string; shape: MarkerShape; tone: string }> = {
  baseline: { label: "Baseline", shape: "diamond", tone: "var(--ink-2)" },
  keep: { label: "Keep", shape: "circle", tone: "var(--ok)" },
  revert: { label: "Revert", shape: "cross", tone: "var(--bad)" },
  pending: { label: "Pending", shape: "triangle", tone: "var(--warn)" },
}

/** Props for the kit DecisionBadge. */
export const decisionBadge = (d: Decision) => ({ shape: DECISION_META[d].shape, label: DECISION_META[d].label, color: DECISION_META[d].tone })

/** Held-out test is the one to trust: solid, in the world accent. Train is muted and hatched or dashed. */
export const TEST_COLOR = "var(--chart-1)"
export const TRAIN_COLOR = "color-mix(in oklab, var(--ink) 46%, var(--card))"

const PREFERRED_METRICS = ["recall@10", "recall@5", "mrr@10", "ndcg@10"]

export const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v)
export const fmt3 = (v: number) => v.toFixed(3)

/** Signed to three places with a real minus sign. */
export const fmtDelta = (v: number) => `${v < 0 ? "−" : "+"}${Math.abs(v).toFixed(3)}`

export type DeltaKind = "up" | "down" | "flat"
export const deltaKind = (v: number): DeltaKind => (v >= 0.0005 ? "up" : v <= -0.0005 ? "down" : "flat")

export function metricOf(split: SplitResult | undefined, metric: string): number | null {
  const v = split?.metrics?.[metric]
  return isNum(v) ? v : null
}

export function ciOf(split: SplitResult | undefined, metric: string): [number, number] | null {
  const ci = split?.ci95?.[metric]
  return Array.isArray(ci) && isNum(ci[0]) && isNum(ci[1]) ? [ci[0], ci[1]] : null
}

/** recall@10 and friends first, anything else after, alphabetically. */
export function orderMetrics(names: Iterable<string>): string[] {
  const set = new Set(names)
  const rest = [...set].filter((m) => !PREFERRED_METRICS.includes(m)).sort()
  return [...PREFERRED_METRICS.filter((m) => set.has(m)), ...rest]
}

export function metricNames(split: SplitResult | undefined): string[] {
  return Object.keys(split?.metrics ?? {}).filter((k) => isNum(split?.metrics?.[k]))
}

const timeOf = (run: EvalRun) => (isNum(run.created_at) ? run.created_at : 0)

/** The metric the trajectory and the run list follow: the newest run that recorded any, recall@10 first. */
export function primaryMetric(runs: EvalRun[]): string | null {
  const newestFirst = [...runs].sort((a, b) => timeOf(b) - timeOf(a))
  for (const run of newestFirst) {
    const names = orderMetrics([...metricNames(run.splits?.test), ...metricNames(run.splits?.train)])
    if (names.length > 0) return names[0]
  }
  return null
}

export type Annotated = {
  run: EvalRun
  /** 1 for the oldest run, so a run keeps its number as new ones arrive. */
  n: number
  decision: Decision
  test: number | null
  train: number | null
  /** Test metric minus the incumbent's: the latest earlier baseline or keep run of the same eval. */
  delta: { value: number; vs: number } | null
}

/** Oldest first, each run with its numbers and its change against the incumbent it was compared with. */
export function annotate(runs: EvalRun[], metric: string): Annotated[] {
  const sorted = [...runs].sort((a, b) => timeOf(a) - timeOf(b))
  const incumbent = new Map<string, Annotated>()
  return sorted.map((run, i) => {
    const decision = normDecision(run.decision)
    const test = metricOf(run.splits?.test, metric)
    const inc = incumbent.get(run.eval)
    const item: Annotated = {
      run,
      n: i + 1,
      decision,
      test,
      train: metricOf(run.splits?.train, metric),
      delta: test !== null && inc?.test != null ? { value: test - inc.test, vs: inc.n } : null,
    }
    if ((decision === "baseline" || decision === "keep") && test !== null) incumbent.set(run.eval, item)
    return item
  })
}

const short = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1)}…` : s)

/** Train (dashed, muted) and held-out test (solid) for one eval, with 95% bands and decision markers on test. */
export function trajectorySeries(items: Annotated[], metric: string): Series[] {
  const point = (a: Annotated, y: number, split: SplitResult | undefined): SeriesPoint => {
    const ci = ciOf(split, metric)
    return { x: a.n, y, ...(ci ? { lo: ci[0], hi: ci[1] } : {}) }
  }
  const train: SeriesPoint[] = []
  const test: SeriesPoint[] = []
  for (const a of items) {
    if (a.train !== null) train.push(point(a, a.train, a.run.splits?.train))
    if (a.test !== null) {
      test.push({
        ...point(a, a.test, a.run.splits?.test),
        marker: DECISION_META[a.decision].shape,
        note: `${DECISION_META[a.decision].label} · ${short(a.run.change ?? "", 48)}`,
      })
    }
  }
  return [
    { id: "train", label: "Train", color: TRAIN_COLOR, dashed: true, points: train },
    { id: "test", label: "Held-out test", color: TEST_COLOR, points: test },
  ]
}

export type Tally = { runs: number; kept: number; reverted: number; pending: number; cost: number }

export function tally(items: Annotated[]): Tally {
  return {
    runs: items.length,
    kept: items.filter((a) => a.decision === "keep").length,
    reverted: items.filter((a) => a.decision === "revert").length,
    pending: items.filter((a) => a.decision === "pending").length,
    cost: items.reduce((sum, a) => sum + (isNum(a.run.cost_usd) ? a.run.cost_usd : 0), 0),
  }
}

// ---------- dates ----------

const dateOf = (epochSeconds: number) => new Date(epochSeconds * 1000)

export function fmtShortDate(epochSeconds: number): string {
  return isNum(epochSeconds) ? dateOf(epochSeconds).toLocaleDateString("en-US", { month: "short", day: "numeric" }) : "—"
}

export function fmtDateTime(epochSeconds: number): string {
  if (!isNum(epochSeconds)) return "—"
  const d = dateOf(epochSeconds)
  return `${d.toLocaleDateString("en-US", { month: "short", day: "numeric" })}, ${d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })}`
}

export function fmtUsd(n: number): string {
  if (!isNum(n)) return "—"
  return n.toLocaleString("en-US", { style: "currency", currency: "USD", minimumFractionDigits: 2, maximumFractionDigits: n > 0 && n < 1 ? 3 : 2 })
}
