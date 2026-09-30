// Every run, newest first, as compact rows. A row opens to the run's per-domain train vs test bars.
import { useId, useState } from "react"
import { ArrowDown, ArrowUp, ChevronRight, Minus } from "lucide-react"

import { RangeBars, type RangeRow } from "@/components/charts"
import { DecisionBadge, Eyebrow, Panel } from "@/components/kit"
import { cn } from "@/lib/utils"
import type { EvalRun } from "@/lib/backend"
import { ciOf, deltaKind, fmt3, fmtDateTime, fmtDelta, fmtShortDate, fmtUsd, metricNames, metricOf, orderMetrics, decisionBadge, TEST_COLOR, TRAIN_COLOR, type Annotated } from "./model"

const VISIBLE = 10
const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

function Delta({ item }: { item: Annotated }) {
  const d = item.delta
  if (!d) return <span aria-hidden="true" />
  const kind = deltaKind(d.value)
  const Icon = kind === "up" ? ArrowUp : kind === "down" ? ArrowDown : Minus
  return (
    <span
      title={`Against #${d.vs}, the last kept run`}
      className={cn("inline-flex items-center justify-end gap-1 font-mono text-[12px] tabular-nums", kind === "up" ? "text-ok" : kind === "down" ? "text-bad" : "text-ink-3")}
    >
      <Icon aria-hidden="true" className="size-3" strokeWidth={2.5} />
      {kind === "flat" ? "0.000" : fmtDelta(d.value)}
      <span className="sr-only"> against run {d.vs}</span>
    </span>
  )
}

function domainRows(run: EvalRun, metric: string): RangeRow[] {
  return Object.entries(run.per_domain ?? {})
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, d]) => {
      const groups: RangeRow["groups"] = []
      const t = metricOf(d?.test, metric)
      const tr = metricOf(d?.train, metric)
      if (t !== null) {
        const ci = ciOf(d?.test, metric)
        groups.push({ id: "test", label: "Held-out test, trust this", value: t, color: TEST_COLOR, ...(ci ? { lo: ci[0], hi: ci[1] } : {}) })
      }
      if (tr !== null) {
        const ci = ciOf(d?.train, metric)
        groups.push({ id: "train", label: "Train, seen while tuning", value: tr, color: TRAIN_COLOR, hatched: true, ...(ci ? { lo: ci[0], hi: ci[1] } : {}) })
      }
      return { id: name, label: name, groups }
    })
    .filter((r) => r.groups.length > 0)
}

const show = (v: unknown) => (typeof v === "object" && v !== null ? JSON.stringify(v) : String(v))

function RunDetail({ id, item, metric }: { id: string; item: Annotated; metric: string }) {
  const { run } = item
  const domains = Object.entries(run.per_domain ?? {}).sort(([a], [b]) => a.localeCompare(b))
  const names = orderMetrics(domains.flatMap(([, d]) => [...metricNames(d?.test), ...metricNames(d?.train)]))
  const [picked, setPicked] = useState<string | null>(null)
  const active = picked && names.includes(picked) ? picked : names.includes(metric) ? metric : names[0]
  const rows = active ? domainRows(run, active) : []
  const config = Object.entries(run.config ?? {})

  return (
    <div id={id} role="region" aria-label={`Run ${item.n} detail`} className="kit-rise @container px-2 pt-2 pb-7">
      <dl className="text-ink-3 m-0 mb-5 flex flex-wrap gap-x-6 gap-y-1.5 font-mono text-[11px] leading-snug">
        <div className="min-w-0">
          <dt className="sr-only">Run id</dt>
          <dd className="m-0 break-all">{run.run_id}</dd>
        </div>
        <div>
          <dt className="inline">Cost </dt>
          <dd className="text-ink-2 m-0 inline">{fmtUsd(run.cost_usd)}</dd>
        </div>
        {config.length > 0 ? (
          <div className="min-w-0">
            <dt className="inline">Config </dt>
            <dd className="text-ink-2 m-0 inline break-words">{config.map(([k, v]) => `${k}=${show(v)}`).join("  ")}</dd>
          </div>
        ) : null}
      </dl>

      {rows.length === 0 || !active ? (
        <p className="text-ink-2 text-sm">This run has no per-domain results.</p>
      ) : (
        <>
          <div className="mb-4 flex flex-wrap items-center gap-x-4 gap-y-2">
            <Eyebrow>Per domain</Eyebrow>
            {names.length > 1 ? (
              <div role="group" aria-label="Metric" className="flex flex-wrap gap-1.5">
                {names.map((m) => (
                  <button
                    key={m}
                    type="button"
                    aria-pressed={m === active}
                    onClick={() => setPicked(m)}
                    className={cn(
                      "rounded-full border px-3 py-1.5 font-mono text-[11px] leading-none transition-colors",
                      FOCUS,
                      m === active ? "bg-foreground text-background border-foreground" : "border-hairline-strong text-ink-2 hover:text-foreground hover:bg-accent",
                    )}
                    style={{ transitionDuration: "var(--dur-quick)" }}
                  >
                    {m}
                  </button>
                ))}
              </div>
            ) : (
              <span className="text-ink-2 font-mono text-[11px]">{active}</span>
            )}
          </div>
          <RangeBars rows={rows} name={`${active} by domain for run ${item.n}, train and held-out test`} domain={[0, 1]} format={fmt3} />
          <p className="text-ink-3 mt-3 font-mono text-[11px] leading-snug">
            Questions per domain, test / train: {domains.map(([name, d]) => `${name} ${d?.test?.n ?? 0}/${d?.train?.n ?? 0}`).join(" · ")}
          </p>
        </>
      )}
    </div>
  )
}

function RunRow({ item, metric, open, onToggle, showEval }: { item: Annotated; metric: string; open: boolean; onToggle: () => void; showEval: boolean }) {
  const detailId = useId()
  const { run } = item
  return (
    <li className="border-hairline border-b last:border-b-0">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={open ? detailId : undefined}
        onClick={onToggle}
        className={cn(
          "hover:bg-accent grid w-full grid-cols-[1rem_2rem_minmax(0,1fr)_8.25rem] items-center gap-x-3 rounded-lg px-2 py-3 text-left transition-colors @2xl:grid-cols-[1rem_2rem_4.5rem_minmax(0,1fr)_6.75rem_8.25rem]",
          FOCUS,
        )}
        style={{ transitionDuration: "var(--dur-quick)" }}
      >
        <ChevronRight aria-hidden="true" className="text-ink-3 size-4" style={{ transform: open ? "rotate(90deg)" : "none", transition: "transform var(--dur-base) var(--ease-soft)" }} />
        <span className="text-ink-3 font-mono text-[12px] tabular-nums">#{item.n}</span>
        <span className="text-ink-2 hidden font-mono text-[12px] @2xl:block">{fmtShortDate(run.created_at)}</span>
        <span className="min-w-0">
          <span className="line-clamp-2 text-[14px] leading-snug @2xl:line-clamp-1">{run.change}</span>
          <span className="mt-1.5 flex min-w-0 items-center gap-2.5 @2xl:hidden">
            <DecisionBadge {...decisionBadge(item.decision)} />
            <span className="text-ink-3 truncate font-mono text-[11px]">
              {fmtDateTime(run.created_at)}
              {showEval ? ` · ${run.eval}` : ""}
            </span>
          </span>
          {showEval ? <span className="text-ink-3 mt-1 hidden truncate font-mono text-[11px] @2xl:block">{run.eval}</span> : null}
        </span>
        <DecisionBadge {...decisionBadge(item.decision)} className="hidden justify-self-start @2xl:inline-flex" />
        <span className="flex items-baseline justify-end gap-3">
          <span className="w-[3.25rem] text-right font-mono text-[15px] tabular-nums">{item.test !== null ? fmt3(item.test) : <span className="text-ink-3">{"—"}</span>}</span>
          <span className="flex w-[4.25rem] justify-end">
            <Delta item={item} />
          </span>
        </span>
      </button>
      {open ? <RunDetail id={detailId} item={item} metric={metric} /> : null}
    </li>
  )
}

export function RunList({ items, metric }: { items: Annotated[]; metric: string }) {
  const newestFirst = [...items].reverse()
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set())
  const [all, setAll] = useState(false)
  const shown = all ? newestFirst : newestFirst.slice(0, VISIBLE)
  const showEval = new Set(items.map((a) => a.run.eval)).size > 1
  const toggle = (id: string) =>
    setOpen((prev) => {
      const next = new Set(prev)
      if (!next.delete(id)) next.add(id)
      return next
    })

  return (
    <Panel
      className="@container"
      eyebrow={`All runs · ${items.length}`}
      actions={<span className="text-ink-3 hidden font-mono text-[11px] @lg:inline">{`Held-out test ${metric}, change vs last kept run`}</span>}
    >
      <ul className="m-0 -mx-2 list-none p-0">
        {shown.map((item) => (
          <RunRow key={item.run.run_id} item={item} metric={metric} open={open.has(item.run.run_id)} onToggle={() => toggle(item.run.run_id)} showEval={showEval} />
        ))}
      </ul>
      {items.length > VISIBLE ? (
        <button
          type="button"
          onClick={() => setAll((v) => !v)}
          className={cn("text-ink-2 hover:text-foreground mt-3 rounded-md font-mono text-[11px] tracking-[0.12em] uppercase transition-colors", FOCUS)}
        >
          {all ? "Show fewer" : `Show ${items.length - VISIBLE} older ${items.length - VISIBLE === 1 ? "run" : "runs"}`}
        </button>
      ) : null}
    </Panel>
  )
}
