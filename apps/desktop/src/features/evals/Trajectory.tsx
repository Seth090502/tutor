// Test and train across runs: one line each with 95% bands, a decision marker on every test point, a scrubber
// and the chart/table toggle from the chart kit.
import { useMemo } from "react"

import { ChartFrame, LineChart, lineTable, type MarkerKeyItem } from "@/components/charts"
import { DECISION_META, fmt3, trajectorySeries, type Annotated, type Decision } from "./model"

const KEY: MarkerKeyItem[] = (["baseline", "keep", "revert", "pending"] as Decision[]).map((d) => ({ shape: DECISION_META[d].shape, label: DECISION_META[d].label }))
const runLabel = (n: number) => `#${n}`

export function Trajectory({ items, metric, drawOn }: { items: Annotated[]; metric: string; drawOn: boolean }) {
  const evalName = items[items.length - 1]?.run.eval ?? ""
  const series = useMemo(() => trajectorySeries(items.filter((a) => a.run.eval === evalName), metric), [items, metric, evalName])
  const count = series[1].points.length
  const table = useMemo(() => lineTable(series, { xTitle: "Run", xFormat: runLabel, yFormat: fmt3, caption: `${metric} by run, train and held-out test, with 95% intervals and the decision for each test point` }), [series, metric])

  return (
    <ChartFrame
      title="Trajectory"
      subtitle={`${metric} across ${count} ${evalName} ${count === 1 ? "run" : "runs"}. Follow the solid held-out line; the dashed train line is what tuning could see.`}
      table={table}
      className="p-5 [box-shadow:inset_0_1px_0_var(--kit-highlight),var(--kit-shadow)] sm:p-6"
    >
      {count === 0 ? (
        <p className="text-ink-2 text-sm">No {evalName} run has a held-out {metric} yet.</p>
      ) : (
        <LineChart series={series} name={`${metric} for train and held-out test by run`} xFormat={runLabel} yFormat={fmt3} height={268} drawOn={drawOn} bandNote="95% confidence interval" markerKey={KEY} />
      )}
    </ChartFrame>
  )
}
