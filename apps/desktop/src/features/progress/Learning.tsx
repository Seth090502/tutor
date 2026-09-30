// Calibration and assistance: how well confidence matches results, and how much help attempts needed.
// Both come from contract C7 fields and are only rendered when the backend sends them.
import { useMemo } from "react"

import { ChartFrame, LineChart, StackedBar, stackTable } from "@/components/charts"
import { CountUp, Eyebrow, Stat, StatRow } from "@/components/kit"
import {
  SMALL_SAMPLE,
  assistanceSegments,
  calibrationRows,
  calibrationSeries,
  calibrationTable,
  fmtInt,
  type Assistance,
  type Calibration,
} from "./model"
import { Quiet } from "./parts"

export function CalibrationPanel({ calibration, days }: { calibration: Calibration[]; days: number }) {
  const rows = useMemo(() => calibrationRows(calibration), [calibration])
  const plot = useMemo(() => calibrationSeries(rows), [rows])
  const total = rows.reduce((a, r) => a + r.attempts, 0)
  const small = rows.some((r) => r.attempts > 0 && r.attempts < SMALL_SAMPLE)

  return (
    <ChartFrame
      title="Calibration"
      subtitle={
        total === 0
          ? `Percent correct at each confidence level over the last ${days} days.`
          : `Percent correct at each confidence level, from ${fmtInt(total)} checked attempts in the last ${days} days. Attempts marked by hand are left out.`
      }
      table={calibrationTable(rows)}
      footer={
        total === 0 ? undefined : (
          <span>
            Dashed line: what each level would imply (25, 50, 75, 95%). It is a reference, not a target. Above it you did better than you felt; below it, worse. Whiskers show the 95% confidence interval.
            {small ? ` Levels with fewer than ${SMALL_SAMPLE} attempts are noisy.` : ""}
          </span>
        )
      }
    >
      {total === 0 ? (
        <Quiet>No checked attempts with a confidence rating yet. Rate how sure you are before each answer and this chart fills in.</Quiet>
      ) : (
        <div className="flex flex-col gap-5">
          <LineChart
            name="Percent correct by confidence level, against a reference line"
            series={plot.series}
            xKind="category"
            xLabels={plot.labels}
            yDomain={[0, 100]}
            yFormat={(v) => `${Math.round(v)}%`}
            height={240}
          />
          <StatRow className="grid-cols-2">
            {rows.map((r) => (
              <Stat
                key={r.level}
                size="md"
                label={`${r.level} · ${r.name}`}
                value={r.attempts}
                unit={r.pct === null ? "attempts" : `attempts · ${r.pct}% right`}
              />
            ))}
          </StatRow>
        </div>
      )}
    </ChartFrame>
  )
}

export function AssistancePanel({ assistance, days }: { assistance: Assistance; days: number }) {
  const segments = useMemo(() => assistanceSegments(assistance), [assistance])
  const unassisted = segments[0].value
  const pct = assistance.attempts > 0 ? Math.round((100 * unassisted) / assistance.attempts) : 0

  return (
    <ChartFrame
      title="Assistance"
      subtitle={
        assistance.attempts === 0
          ? `Attempts reached alone, with a hint, or after the answer was shown, over the last ${days} days.`
          : `How the ${fmtInt(assistance.attempts)} checked attempts in the last ${days} days were reached: alone, with a hint, or after the answer was shown.`
      }
      table={stackTable(segments, "Attempts", undefined, "Attempts by level of assistance")}
      footer={assistance.attempts === 0 ? undefined : "A revealed answer is always rated again, so it comes back for review."}
    >
      {assistance.attempts === 0 ? (
        <Quiet>No checked attempts yet in this window.</Quiet>
      ) : (
        <div className="flex flex-col gap-6">
          <div className="flex items-baseline gap-3">
            <span className="text-[56px] leading-none font-extralight tracking-[-0.04em] tabular-nums">
              <CountUp value={pct} />
              <span className="text-ink-2 ml-0.5 text-[28px] font-light">%</span>
            </span>
            <Eyebrow>Unassisted</Eyebrow>
          </div>
          <StackedBar name="Attempts by level of assistance" segments={segments} unit="attempts" height={18} />
        </div>
      )}
    </ChartFrame>
  )
}

