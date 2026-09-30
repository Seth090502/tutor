// The last 30 days: moon-phase discs for reviews per day and an area chart of questions, mistakes
// and reviews, each with a table view.
import { useMemo } from "react"

import { AreaChart, ChartFrame, PhaseDiscs, lineTable } from "@/components/charts"
import { countDomain, dailySeries, fmtDayLong, fmtInt, localIsoDay, peakReviews, phaseDays, phaseTableFor, type Analytics } from "./model"
import { Quiet, useWidth } from "./parts"

export function ReviewDiscs({ analytics }: { analytics: Analytics }) {
  const daily = analytics.daily ?? []
  const today = analytics.end_date ?? localIsoDay(new Date())
  const days = useMemo(() => phaseDays(daily, today), [daily, today])
  const peak = peakReviews(daily)
  const [ref, width] = useWidth<HTMLDivElement>(640)
  const columns = width >= 820 ? 15 : width >= 420 ? 10 : 6
  const period = analytics.days || daily.length

  return (
    <ChartFrame
      title={`Reviews, last ${period} days`}
      subtitle={
        peak > 0
          ? `Each disc is one day, lit in proportion to its reviews compared with your busiest day (${fmtInt(peak)} reviews). In the legend, 100% of "goal" means that busiest day, not a target you set. Today is ringed.`
          : "Each disc is one day. Nothing has been reviewed in this window yet, so every disc is dark. Today is ringed."
      }
      table={phaseTableFor(daily, days, peak)}
    >
      <div ref={ref}>
        <PhaseDiscs
          name={`Reviews for each of the last ${period} days, relative to the busiest day`}
          days={days}
          columns={columns}
          unit="of your busiest day"
          label="of busiest day"
        />
      </div>
    </ChartFrame>
  )
}

export function ActivityChart({ analytics }: { analytics: Analytics }) {
  const daily = analytics.daily ?? []
  const series = useMemo(() => dailySeries(daily), [daily])
  const domain = useMemo(() => countDomain(series), [series])
  const any = series.some((s) => s.points.some((p) => p.y > 0))
  const totals = analytics.totals

  return (
    <ChartFrame
      title="Activity"
      subtitle={
        totals
          ? `${fmtInt(totals.questions)} questions added, ${fmtInt(totals.mistakes)} first mistakes and ${fmtInt(totals.reviews)} reviews in the last ${analytics.days || daily.length} days.`
          : "Questions added, first mistakes and reviews per day."
      }
      table={lineTable(series, { xTitle: "Day", xFormat: fmtDayLong, yFormat: fmtInt, caption: "Questions, mistakes and reviews per day" })}
    >
      {any ? (
        <AreaChart name="Questions, mistakes and reviews per day" series={series} xKind="time" yDomain={domain} yFormat={fmtInt} height={280} />
      ) : (
        <Quiet>No practice activity in this period. Questions, mistakes and reviews will appear here as you practise.</Quiet>
      )}
    </ChartFrame>
  )
}
