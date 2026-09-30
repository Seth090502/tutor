// The top of the screen: a gauge for what is due today (overdue as the secondary figure) beside a
// hairline-divided stat row.
import { Gauge, type GaugeTone } from "@/components/charts"
import { CountUp, DeltaChip, Eyebrow, Panel, Stat, StatRow } from "@/components/kit"
import { gaugeMax, nextDue, type Summary } from "./model"
import { Rise, useIntroCount } from "./parts"

export function Hero({ summary }: { summary: Summary }) {
  const due = Math.max(0, summary.due)
  const overdue = Math.max(0, summary.overdue)
  const shown = useIntroCount(due)
  const tone: GaugeTone = overdue > 0 ? "warn" : due === 0 ? "ok" : "world"
  const next = nextDue(summary.next_due_at, Date.now())

  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)]">
      <Rise index={0}>
        <Panel eyebrow="Reviews" aria-label="Reviews due today" className="flex h-full flex-col">
          <div className="flex flex-1 items-center justify-center py-2">
            <Gauge value={due} max={gaugeMax(due)} label="Due today" valueLabel={String(shown)} tone={tone} />
          </div>
          <div className="border-hairline mt-4 flex items-center justify-between gap-4 border-t pt-4">
            <div className="flex items-baseline gap-3">
              <Eyebrow>Overdue</Eyebrow>
              <span className="text-[28px] leading-none font-light tracking-[-0.03em] tabular-nums">
                <CountUp value={overdue} />
              </span>
            </div>
            {overdue > 0 ? <DeltaChip tone="warn" text="Behind" /> : <DeltaChip tone="ok" text="Caught up" />}
          </div>
        </Panel>
      </Rise>

      <Rise index={1} className="flex">
        <StatRow className="w-full flex-1 grid-cols-2">
          <Stat size="md" label="Reviewed today" value={summary.reviewed_today} />
          <Stat size="md" label="Total" value={summary.total} unit={summary.total === 1 ? "question" : "questions"} />
          <Stat size="md" label="Mistakes" value={summary.mistakes} />
          <Stat size="md" label={next.label} value={next.value} unit={next.unit} />
        </StatRow>
      </Rise>
    </div>
  )
}
