// Evals: one card per eval run file (contract C2), newest first, plus a recall@10 trajectory.
import { useCallback, useEffect, useState, type ReactNode } from "react"
import { CircleCheck, CircleX, Clock, Flag, FlaskConical, Monitor, RefreshCw, type LucideIcon } from "lucide-react"

import { Button } from "@/components/ui/button"
import { inTauri, shell, type EvalRun, type SplitResult } from "@/lib/backend"
import { Notice, Page, Panel, ScreenBoundary, StateMessage, errorText, fill, fmtDateTime, fmtUsd, niceStep } from "./ui"

type Decision = EvalRun["decision"]

const DECISIONS: Record<Decision, { label: string; icon: LucideIcon; color: string }> = {
  baseline: { label: "Baseline", icon: Flag, color: "var(--viz-neutral)" },
  keep: { label: "Keep", icon: CircleCheck, color: "var(--viz-good)" },
  revert: { label: "Revert", icon: CircleX, color: "var(--viz-bad)" },
  pending: { label: "Pending", icon: Clock, color: "var(--viz-warn)" },
}

const TRAIN = "var(--viz-1)"
const TEST = "var(--viz-2)"
const TRAJECTORY_METRIC = "recall@10"
const PREFERRED_METRICS = ["recall@5", "recall@10", "mrr@10", "ndcg@10"]

const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v)
const fmt3 = (v: number | undefined) => (isNum(v) ? v.toFixed(3) : "—")

function metricsOf(split: SplitResult | undefined) {
  return split?.metrics ?? {}
}

function ciOf(split: SplitResult | undefined, metric: string): [number, number] | null {
  const ci = split?.ci95?.[metric]
  return Array.isArray(ci) && isNum(ci[0]) && isNum(ci[1]) ? [ci[0], ci[1]] : null
}

function orderMetrics(names: Iterable<string>) {
  const set = new Set(names)
  const rest = [...set].filter((m) => !PREFERRED_METRICS.includes(m)).sort()
  return [...PREFERRED_METRICS.filter((m) => set.has(m)), ...rest]
}

export function EvalsScreen() {
  return (
    <ScreenBoundary>
      {inTauri ? <EvalsLoader /> : <StateMessage icon={Monitor} title="Open in the desktop app" detail="Eval runs are read from disk by the desktop shell." />}
    </ScreenBoundary>
  )
}

function EvalsLoader() {
  const [data, setData] = useState<{ runs: EvalRun[]; warnings: string[] } | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setData(await shell.evalsRuns())
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  if (!data) {
    return error ? <StateMessage icon={FlaskConical} title="Could not read eval runs" detail={error} /> : <StateMessage title="Loading…" />
  }
  return (
    <EvalsView
      runs={data.runs}
      warnings={data.warnings}
      error={error}
      action={
        <Button variant="outline" size="sm" onClick={() => void load()} disabled={loading}>
          <RefreshCw />
          Refresh
        </Button>
      }
    />
  )
}

export function EvalsView({
  runs,
  warnings,
  error,
  action,
}: {
  runs: EvalRun[]
  warnings: string[]
  error?: string | null
  action?: ReactNode
}) {
  // Run numbers count from the oldest run, so a run keeps its number as new ones arrive.
  const chronological = [...runs].sort((a, b) => a.created_at - b.created_at)
  const numbered = chronological.map((run, i) => ({ run, n: i + 1 }))
  const newestFirst = [...numbered].reverse()

  return (
    <Page>
      {action && <div className="flex justify-end">{action}</div>}
      {error && <Notice>Could not refresh: {error}</Notice>}
      {warnings.length > 0 && (
        <Notice>
          <ul className="space-y-1">
            {warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </Notice>
      )}
      {runs.length === 0 ? (
        <StateMessage icon={FlaskConical} title="No eval runs yet" detail="Runs appear here from evals/runs/*.json." />
      ) : (
        <>
          <Panel title={`Test ${TRAJECTORY_METRIC} across runs`}>
            <Trajectory points={numbered} />
          </Panel>
          {newestFirst.map(({ run, n }) => (
            <RunCard key={run.run_id} run={run} n={n} defaultOpen={n === runs.length} />
          ))}
        </>
      )}
    </Page>
  )
}

function DecisionBadge({ decision }: { decision: Decision }) {
  const d = DECISIONS[decision] ?? DECISIONS.pending
  return (
    <span className="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium">
      <d.icon className="size-3.5" style={{ color: d.color }} />
      {d.label}
    </span>
  )
}

// ---------- trajectory ----------

function Marker({ decision, x, y }: { decision: Decision; x: number; y: number }) {
  const d = DECISIONS[decision] ?? DECISIONS.pending
  const ring = { stroke: "var(--card)", strokeWidth: 2 }
  if (decision === "keep") return <circle cx={x} cy={y} r={5} style={{ fill: d.color, ...ring }} />
  if (decision === "revert") return <path d={`M${x},${y - 6}L${x + 6},${y}L${x},${y + 6}L${x - 6},${y}Z`} style={{ fill: d.color, ...ring }} />
  if (decision === "baseline") return <rect x={x - 4.5} y={y - 4.5} width={9} height={9} style={{ fill: d.color, ...ring }} />
  return <circle cx={x} cy={y} r={4.5} style={{ fill: "var(--card)", stroke: d.color, strokeWidth: 2 }} />
}

function Trajectory({ points }: { points: { run: EvalRun; n: number }[] }) {
  const data = points.flatMap(({ run, n }) => {
    const v = metricsOf(run.splits?.test)[TRAJECTORY_METRIC]
    return isNum(v) ? [{ run, n, v, ci: ciOf(run.splits?.test, TRAJECTORY_METRIC) }] : []
  })
  if (data.length === 0) return <p className="text-muted-foreground text-sm">No run has a test {TRAJECTORY_METRIC} yet.</p>

  const W = 760
  const H = 230
  const m = { l: 48, r: 20, t: 14, b: 30 }
  const iw = W - m.l - m.r
  const ih = H - m.t - m.b

  const lo = Math.min(...data.map((d) => d.ci?.[0] ?? d.v), ...data.map((d) => d.v))
  const hi = Math.max(...data.map((d) => d.ci?.[1] ?? d.v), ...data.map((d) => d.v))
  const step = niceStep(Math.max(hi - lo, 0.05) / 4)
  const yMin = Math.max(0, Math.floor(lo / step) * step)
  const yMax = Math.min(1, Math.ceil(hi / step) * step)
  const span = yMax > yMin ? yMax - yMin : 1
  const ticks: number[] = []
  for (let t = yMin; t <= yMax + step / 1e6; t += step) ticks.push(t)

  const x = (i: number) => m.l + (data.length === 1 ? iw / 2 : (i * iw) / (data.length - 1))
  const y = (v: number) => m.t + ih - ((Math.min(Math.max(v, yMin), yMax) - yMin) / span) * ih
  const labelEvery = Math.ceil(data.length / 10)

  return (
    <div>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label={`Test ${TRAJECTORY_METRIC} for each run, oldest to newest`}>
        {ticks.map((t) => (
          <g key={t}>
            <line x1={m.l} x2={W - m.r} y1={y(t)} y2={y(t)} style={{ stroke: "var(--border)" }} />
            <text x={m.l - 8} y={y(t)} textAnchor="end" dominantBaseline="middle" className="fill-muted-foreground text-[11px]">
              {t.toFixed(2)}
            </text>
          </g>
        ))}
        <polyline
          points={data.map((d, i) => `${x(i)},${y(d.v)}`).join(" ")}
          fill="none"
          strokeWidth={2}
          strokeLinejoin="round"
          style={{ stroke: "var(--viz-1)" }}
        />
        {data.map((d, i) =>
          d.ci ? (
            <g key={`ci${d.run.run_id}`} strokeWidth={1.5} style={{ stroke: "var(--viz-1)", opacity: 0.6 }}>
              <line x1={x(i)} x2={x(i)} y1={y(d.ci[0])} y2={y(d.ci[1])} />
              <line x1={x(i) - 4} x2={x(i) + 4} y1={y(d.ci[0])} y2={y(d.ci[0])} />
              <line x1={x(i) - 4} x2={x(i) + 4} y1={y(d.ci[1])} y2={y(d.ci[1])} />
            </g>
          ) : null,
        )}
        {data.map((d, i) => (
          <g key={d.run.run_id}>
            <Marker decision={d.run.decision} x={x(i)} y={y(d.v)} />
            <title>{`#${d.n} ${d.run.change}: ${d.v.toFixed(3)} (${DECISIONS[d.run.decision]?.label ?? d.run.decision})`}</title>
            {i % labelEvery === 0 && (
              <text x={x(i)} y={H - 10} textAnchor="middle" className="fill-muted-foreground text-[11px]">
                #{d.n}
              </text>
            )}
          </g>
        ))}
      </svg>
      <ul className="text-muted-foreground mt-2 flex flex-wrap gap-x-5 gap-y-1 text-xs">
        {(Object.keys(DECISIONS) as Decision[]).map((k) => (
          <li key={k} className="flex items-center gap-1.5">
            <svg width={14} height={14} viewBox="-7 -7 14 14" aria-hidden>
              <Marker decision={k} x={0} y={0} />
            </svg>
            {DECISIONS[k].label}
          </li>
        ))}
        <li>Whiskers: 95% CI</li>
        <li>Oldest to newest</li>
      </ul>
    </div>
  )
}

// ---------- run card ----------

function RunCard({ run, n, defaultOpen }: { run: EvalRun; n: number; defaultOpen: boolean }) {
  const domains = Object.entries(run.per_domain ?? {}).sort(([a], [b]) => a.localeCompare(b))
  return (
    <Panel>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="text-sm font-medium break-words">
            <span className="text-muted-foreground font-normal">#{n}</span> {run.change}
          </h3>
          <p className="text-muted-foreground mt-1 text-xs break-words">
            {run.eval} · {fmtDateTime(run.created_at)} · cost {fmtUsd(run.cost_usd)}
          </p>
          <p className="text-muted-foreground mt-0.5 font-mono text-[11px] break-all">{run.run_id}</p>
        </div>
        <DecisionBadge decision={run.decision} />
      </div>
      <div className="mt-4">
        <SplitChart run={run} />
      </div>
      {domains.length > 0 && (
        <details className="mt-4 border-t pt-3" open={defaultOpen}>
          <summary className="cursor-pointer text-sm font-medium select-none">Per domain</summary>
          <DomainTable domains={domains} />
        </details>
      )}
    </Panel>
  )
}

function SplitChart({ run }: { run: EvalRun }) {
  const train = run.splits?.train
  const test = run.splits?.test
  const names = orderMetrics([...Object.keys(metricsOf(train)), ...Object.keys(metricsOf(test))])
  if (names.length === 0) return <p className="text-muted-foreground text-sm">No metrics recorded.</p>

  const W = 720
  const H = 210
  const m = { l: 40, r: 10, t: 14, b: 26 }
  const iw = W - m.l - m.r
  const ih = H - m.t - m.b
  const top = Math.max(
    1,
    ...names.flatMap((k) => [metricsOf(train)[k], metricsOf(test)[k], ciOf(train, k)?.[1], ciOf(test, k)?.[1]].filter(isNum)),
  )
  const y = (v: number) => m.t + ih - (Math.min(Math.max(v, 0), top) / top) * ih
  const group = iw / names.length
  const bar = Math.min(30, group * 0.28)
  const gap = 4

  const Bar = ({ cx, split, metric, color }: { cx: number; split: SplitResult | undefined; metric: string; color: string }) => {
    const v = metricsOf(split)[metric]
    if (!isNum(v)) return null
    const ci = ciOf(split, metric)
    const top0 = y(v)
    const r = Math.min(3, bar / 2, y(0) - top0)
    const x0 = cx - bar / 2
    return (
      <g>
        <path
          d={`M${x0},${y(0)}V${top0 + r}Q${x0},${top0} ${x0 + r},${top0}H${x0 + bar - r}Q${x0 + bar},${top0} ${x0 + bar},${top0 + r}V${y(0)}Z`}
          style={fill(color)}
        >
          <title>{`${metric} (${color === TRAIN ? "train" : "test"}): ${v.toFixed(3)}${ci ? ` [${ci[0].toFixed(3)}, ${ci[1].toFixed(3)}]` : ""}`}</title>
        </path>
        {ci && (
          <g strokeWidth={1.5} className="stroke-foreground">
            <line x1={cx} x2={cx} y1={y(ci[0])} y2={y(ci[1])} />
            <line x1={cx - 4} x2={cx + 4} y1={y(ci[0])} y2={y(ci[0])} />
            <line x1={cx - 4} x2={cx + 4} y1={y(ci[1])} y2={y(ci[1])} />
          </g>
        )}
      </g>
    )
  }

  return (
    <div>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label="Train and test metrics with 95% confidence intervals">
        {[0, 0.25, 0.5, 0.75, 1].map((t) => t * top).map((t) => (
          <g key={t}>
            <line x1={m.l} x2={W - m.r} y1={y(t)} y2={y(t)} style={{ stroke: "var(--border)" }} />
            <text x={m.l - 6} y={y(t)} textAnchor="end" dominantBaseline="middle" className="fill-muted-foreground text-[11px]">
              {t.toFixed(2)}
            </text>
          </g>
        ))}
        {names.map((k, i) => {
          const cx = m.l + group * (i + 0.5)
          return (
            <g key={k}>
              <Bar cx={cx - bar / 2 - gap / 2} split={train} metric={k} color={TRAIN} />
              <Bar cx={cx + bar / 2 + gap / 2} split={test} metric={k} color={TEST} />
              <text x={cx} y={H - 8} textAnchor="middle" className="fill-foreground text-[11px]">
                {k}
              </text>
            </g>
          )
        })}
      </svg>
      <ul className="mt-2 flex flex-wrap gap-x-5 gap-y-1 text-xs">
        <li className="flex items-center gap-1.5">
          <span className="size-2.5 rounded-sm" style={{ background: TRAIN }} />
          Train (n={train?.n ?? 0})
        </li>
        <li className="flex items-center gap-1.5">
          <span className="size-2.5 rounded-sm" style={{ background: TEST }} />
          Test (n={test?.n ?? 0})
        </li>
        <li className="text-muted-foreground">Whiskers: 95% CI</li>
      </ul>
      <table className="mt-3 w-full text-xs">
        <thead>
          <tr className="text-muted-foreground border-b text-left">
            <th className="py-1.5 font-medium">Metric</th>
            <th className="py-1.5 font-medium">Train (95% CI)</th>
            <th className="py-1.5 font-medium">Test (95% CI)</th>
          </tr>
        </thead>
        <tbody>
          {names.map((k) => (
            <tr key={k} className="border-b last:border-0">
              <td className="py-1.5">{k}</td>
              <td className="py-1.5 tabular-nums">{withCi(train, k)}</td>
              <td className="py-1.5 tabular-nums">{withCi(test, k)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function withCi(split: SplitResult | undefined, metric: string) {
  const ci = ciOf(split, metric)
  return `${fmt3(metricsOf(split)[metric])}${ci ? ` [${ci[0].toFixed(3)}, ${ci[1].toFixed(3)}]` : ""}`
}

function DomainTable({ domains }: { domains: [string, EvalRun["per_domain"][string]][] }) {
  const names = orderMetrics(domains.flatMap(([, d]) => [...Object.keys(metricsOf(d?.train)), ...Object.keys(metricsOf(d?.test))]))
  if (names.length === 0) return <p className="text-muted-foreground mt-2 text-sm">No per-domain metrics.</p>
  return (
    <div className="mt-2 overflow-x-auto">
      <table className="w-full text-xs">
        <thead>
          <tr className="text-muted-foreground border-b text-left">
            <th rowSpan={2} className="py-1.5 pr-3 font-medium">
              Domain
            </th>
            <th rowSpan={2} className="px-3 py-1.5 text-right font-medium">
              n (train / test)
            </th>
            {names.map((k) => (
              <th key={k} colSpan={2} className="px-3 pt-1.5 text-center font-medium">
                {k}
              </th>
            ))}
          </tr>
          <tr className="text-muted-foreground border-b">
            {names.flatMap((k) => [
              <th key={`${k}a`} className="px-3 py-1 text-right font-medium">
                Train
              </th>,
              <th key={`${k}b`} className="px-3 py-1 text-right font-medium">
                Test
              </th>,
            ])}
          </tr>
        </thead>
        <tbody>
          {domains.map(([name, d]) => (
            <tr key={name} className="border-b last:border-0">
              <td className="py-1.5 pr-3">{name}</td>
              <td className="px-3 py-1.5 text-right tabular-nums">
                {d?.train?.n ?? 0} / {d?.test?.n ?? 0}
              </td>
              {names.flatMap((k) => [
                <td key={`${k}a`} className="px-3 py-1.5 text-right tabular-nums">
                  {fmt3(metricsOf(d?.train)[k])}
                </td>,
                <td key={`${k}b`} className="px-3 py-1.5 text-right tabular-nums">
                  {fmt3(metricsOf(d?.test)[k])}
                </td>,
              ])}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
