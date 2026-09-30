// The newest run: what changed, the decision, the held-out test number as the hero, and train vs test per metric.
import { useId, type ReactNode } from "react"

import { RangeBars, type RangeRow } from "@/components/charts"
import { DecisionBadge, Eyebrow, Panel, Stat } from "@/components/kit"
import { ciOf, decisionBadge, deltaKind, fmt3, fmtDateTime, fmtDelta, fmtUsd, metricNames, metricOf, orderMetrics, TEST_COLOR, TRAIN_COLOR, type Annotated } from "./model"

/** Rows for the two splits of one run, one row per metric. Test first: it is the one to trust. */
function metricRows(item: Annotated): RangeRow[] {
  const { train, test } = item.run.splits ?? {}
  return orderMetrics([...metricNames(test), ...metricNames(train)]).map((m) => {
    const groups: RangeRow["groups"] = []
    const t = metricOf(test, m)
    const tr = metricOf(train, m)
    if (t !== null) {
      const ci = ciOf(test, m)
      groups.push({ id: "test", label: "Held-out test, trust this", value: t, color: TEST_COLOR, ...(ci ? { lo: ci[0], hi: ci[1] } : {}) })
    }
    if (tr !== null) {
      const ci = ciOf(train, m)
      groups.push({ id: "train", label: "Train, seen while tuning", value: tr, color: TRAIN_COLOR, hatched: true, ...(ci ? { lo: ci[0], hi: ci[1] } : {}) })
    }
    return { id: m, label: m, groups }
  })
}

function FactRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="border-hairline flex items-baseline justify-between gap-4 border-t py-2.5">
      <dt>
        <Eyebrow>{label}</Eyebrow>
      </dt>
      <dd className="m-0 font-mono text-[13px] tabular-nums">{children}</dd>
    </div>
  )
}

export function LatestRun({ item, metric, animate }: { item: Annotated; metric: string; animate: boolean }) {
  const titleId = useId()
  const { run } = item
  const rows = metricRows(item)
  const ci = ciOf(run.splits?.test, metric)
  const gap = item.test !== null && item.train !== null ? item.train - item.test : null
  const delta = item.delta
  const deltaChip = delta
    ? { text: `${deltaKind(delta.value) === "flat" ? "no change" : fmtDelta(delta.value)} vs #${delta.vs}`, tone: (deltaKind(delta.value) === "up" ? "ok" : deltaKind(delta.value) === "down" ? "bad" : "warn") as "ok" | "bad" | "warn" }
    : undefined

  return (
    <Panel className="@container" aria-labelledby={titleId}>
      <div className="mb-6 flex items-start justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-3">
          <Eyebrow rule>
            Latest run · #{item.n} · {fmtDateTime(run.created_at)}
          </Eyebrow>
          <h2 id={titleId} className="font-serif text-[24px] leading-[1.2] tracking-[-0.012em] text-balance">
            {run.change}
          </h2>
        </div>
        <DecisionBadge {...decisionBadge(item.decision)} className="mt-0.5" />
      </div>

      <div className="grid gap-x-12 gap-y-8 @3xl:grid-cols-[16rem_minmax(0,1fr)]">
        <div className="flex min-w-0 flex-col gap-5">
          {item.test !== null ? (
            <div>
              <Stat
                size="xl"
                className="p-0"
                label={`Held-out test · ${metric}`}
                value={item.test}
                decimals={3}
                animate={animate}
                delta={deltaChip}
              />
              {ci ? (
                <p className="text-ink-3 mt-3 font-mono text-[11px] tracking-[0.04em]">
                  95% CI {fmt3(ci[0])} to {fmt3(ci[1])}
                </p>
              ) : null}
            </div>
          ) : (
            <p className="text-ink-2 text-sm">This run recorded no held-out test {metric}.</p>
          )}
          <dl className="m-0 flex flex-col">
            {item.train !== null ? <FactRow label={`Train · ${metric}`}>{fmt3(item.train)}</FactRow> : null}
            {gap !== null ? <FactRow label="Train minus test">{gap < 0 ? "−" : ""}{fmt3(Math.abs(gap))}</FactRow> : null}
            <FactRow label="Questions">
              {run.splits?.test?.n ?? 0} test · {run.splits?.train?.n ?? 0} train
            </FactRow>
            <FactRow label="Cost">{fmtUsd(run.cost_usd)}</FactRow>
          </dl>
        </div>

        <div className="min-w-0">
          <Eyebrow className="mb-4">Every metric, 95% CI whiskers</Eyebrow>
          {rows.length > 0 ? (
            <RangeBars rows={rows} name={`Train and held-out test for run ${item.n}, by metric, with 95% confidence whiskers`} domain={[0, 1]} format={fmt3} barHeight={18} />
          ) : (
            <p className="text-ink-2 text-sm">No metrics recorded for this run.</p>
          )}
        </div>
      </div>
    </Panel>
  )
}
