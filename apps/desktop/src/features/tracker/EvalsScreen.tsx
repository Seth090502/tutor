// Evals (contract C2 run files, read through shell.evalsRuns()): the latest run as the hero, the trajectory of the
// held-out test metric, and every run as a compact, expandable row. Design: docs/tutor/DESIGN.md.
import { Component, useCallback, useEffect, useMemo, useRef, useState, type ErrorInfo, type ReactNode } from "react"
import { FlaskConical, Monitor, RefreshCw, TriangleAlert } from "lucide-react"

import { Button } from "@/components/ui/button"
import { LedStatus, QuietNote, Rise } from "@/components/kit"
import { inTauri, shell, type EvalRun } from "@/lib/backend"
import { LatestRun } from "@/features/evals/LatestRun"
import { annotate, fmtUsd, primaryMetric, tally } from "@/features/evals/model"
import { Code, StateBlock } from "@/features/evals/parts"
import { RunList } from "@/features/evals/RunList"
import { Trajectory } from "@/features/evals/Trajectory"

// The entrance and the count-up play the first time the screen shows, not on every visit to the section.
let introPlayed = false

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Keeps a render bug from a hand-edited run file in this screen from blanking the whole app. */
class Boundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }
  static getDerivedStateFromError(error: Error) {
    return { error }
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Evals screen failed to render", error, info.componentStack)
  }
  render() {
    if (this.state.error) return <StateBlock icon={TriangleAlert} title="This screen failed to render">{this.state.error.message}</StateBlock>
    return this.props.children
  }
}

export function EvalsScreen() {
  return (
    <Boundary>
      {inTauri ? (
        <EvalsLoader />
      ) : (
        <StateBlock icon={Monitor} title="Open in the desktop app">
          Eval runs are read from disk by the desktop shell, so they cannot be shown in a plain browser.
        </StateBlock>
      )}
    </Boundary>
  )
}

function RefreshButton({ onClick, busy }: { onClick: () => void; busy: boolean }) {
  return (
    <Button variant="outline" size="sm" onClick={onClick} disabled={busy}>
      <RefreshCw />
      Refresh
    </Button>
  )
}

function EvalsLoader() {
  const [data, setData] = useState<{ runs: EvalRun[]; warnings: string[] } | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const res = await shell.evalsRuns()
      setData({ runs: Array.isArray(res?.runs) ? res.runs : [], warnings: Array.isArray(res?.warnings) ? res.warnings : [] })
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
    if (!error) return <Skeleton />
    return (
      <StateBlock icon={FlaskConical} title="Could not read eval runs" action={<RefreshButton onClick={() => void load()} busy={loading} />}>
        <span className="font-mono text-[12px] break-words">{error}</span>
      </StateBlock>
    )
  }
  return <EvalsView runs={data.runs} warnings={data.warnings} error={error} onRefresh={() => void load()} refreshing={loading} />
}

/** Quiet placeholders in the shape of the page. Static: nothing animates while it waits. */
function Skeleton() {
  const block = "border-hairline bg-card rounded-2xl border"
  const bar = "bg-muted rounded-md"
  return (
    <div role="status" aria-busy="true" className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 pt-7 pb-12">
      <span className="sr-only">Loading eval runs</span>
      <div className={`${block} h-64 p-6`}>
        <div className={`${bar} h-3 w-40`} />
        <div className={`${bar} mt-4 h-6 w-2/3`} />
        <div className={`${bar} mt-10 h-16 w-44`} />
      </div>
      <div className={`${block} h-72 p-6`}>
        <div className={`${bar} h-3 w-32`} />
      </div>
      <div className={`${block} h-48 p-6`}>
        <div className={`${bar} h-3 w-28`} />
      </div>
    </div>
  )
}

export function EvalsView({
  runs,
  warnings,
  error,
  onRefresh,
  refreshing = false,
}: {
  runs: EvalRun[]
  warnings: string[]
  error?: string | null
  onRefresh?: () => void
  refreshing?: boolean
}) {
  const enter = useRef(!introPlayed).current
  useEffect(() => {
    introPlayed = true
  }, [])

  const metric = useMemo(() => primaryMetric(runs), [runs])
  const items = useMemo(() => annotate(runs, metric ?? ""), [runs, metric])
  const latest = items[items.length - 1]
  const t = useMemo(() => tally(items), [items])

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 pt-7 pb-12">
      <Rise i={0} enter={enter} className="flex flex-wrap items-center justify-between gap-x-6 gap-y-3">
        <p className="text-ink-3 font-mono text-[12px] leading-snug tracking-[0.02em]">
          {latest ? `${t.runs} ${t.runs === 1 ? "run" : "runs"} · ${t.kept} kept · ${t.reverted} reverted${t.pending ? ` · ${t.pending} pending` : ""} · ${fmtUsd(t.cost)} spent` : "No runs recorded"}
        </p>
        {onRefresh ? <RefreshButton onClick={onRefresh} busy={refreshing} /> : null}
      </Rise>

      {error ? <QuietNote tone="bad">Could not refresh: {error}</QuietNote> : null}

      {latest ? (
        <>
          <Rise i={1} enter={enter}>
            <LatestRun item={latest} metric={metric ?? ""} animate={enter} />
          </Rise>
          {metric ? (
            <Rise i={2} enter={enter}>
              <Trajectory items={items} metric={metric} drawOn={enter} />
            </Rise>
          ) : null}
          <Rise i={3} enter={enter}>
            <RunList items={items} metric={metric ?? ""} />
          </Rise>
        </>
      ) : (
        <Rise i={1} enter={enter}>
          <StateBlock icon={FlaskConical} title="No eval runs yet">
            <p>Record the first one from the repository root and it appears here.</p>
            <p className="mt-3">
              <Code>python -m tutor_evals run ...</Code>
            </p>
          </StateBlock>
        </Rise>
      )}

      {warnings.length > 0 ? (
        <Rise i={4} enter={enter}>
          <Warnings warnings={warnings} />
        </Rise>
      ) : null}
    </div>
  )
}

function Warnings({ warnings }: { warnings: string[] }) {
  return (
    <details className="group border-hairline border-t pt-4">
      <summary className="inline-flex cursor-pointer list-none items-center rounded-md focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-world [&::-webkit-details-marker]:hidden">
        <LedStatus tone="warn" state={`${warnings.length} ${warnings.length === 1 ? "warning" : "warnings"}`} />
        <span className="text-ink-3 ml-3 font-mono text-[11px] group-open:hidden">Show</span>
      </summary>
      <ul className="text-ink-2 mt-3 flex list-none flex-col gap-1.5 p-0 font-mono text-[12px] leading-snug break-words">
        {warnings.map((w, i) => (
          <li key={i}>{w}</li>
        ))}
      </ul>
    </details>
  )
}
