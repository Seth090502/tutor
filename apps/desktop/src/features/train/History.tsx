// History: the training curve (trained task only, comparable sessions), d' per stream, CSV export, and
// the transfer checks in a separate panel. Nothing here is a score of ability.
import { useMemo, useState, type ReactNode } from "react"

import { ChartFrame, LineChart, lineTable, type Series } from "@/components/charts"
import { Eyebrow, LedStatus, Panel, Segmented, riseStyle } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { CURVE_WINDOW, trainingCurve, transferStatus } from "./view"
import type { NBackSummary } from "./types"
import type { TrainMode } from "./Setup"
import "./train.css"

const HONEST = "n-back scores are noisy and do not measure intelligence."
const MODE_LABEL: Record<TrainMode, string> = { dual: "Dual", position: "Position only", letter: "Letter only" }
const FRAME = "border-0 bg-transparent p-0 [box-shadow:none]"

const fmtDate = (t: number) => new Date(t * 1000).toLocaleDateString("en-US", { month: "short", day: "numeric" })
const fmtN = (v: number) => String(+v.toFixed(2))
const fmtD = (d: number | null) => (d === null ? "—" : d.toFixed(2))
const session = (x: number) => `S${x}`

function Curves({ sessions, mode }: { sessions: NBackSummary[]; mode: TrainMode }) {
  const { points, notComparable } = useMemo(() => trainingCurve(sessions, mode), [sessions, mode])
  const aside =
    notComparable > 0
      ? `${notComparable} session${notComparable === 1 ? "" : "s"} with changed settings ${notComparable === 1 ? "is" : "are"} logged but shown apart from this curve.`
      : null

  if (points.length < 2) {
    return (
      <p className="text-ink-2 text-sm leading-relaxed">
        {points.length === 0 ? "No comparable" : "One comparable"} {MODE_LABEL[mode].toLowerCase()} session so far; the curve starts at two.
        {aside && ` ${aside}`}
      </p>
    )
  }

  const level: Series[] = [
    {
      id: "level",
      label: "Level",
      color: "var(--ink-3)",
      dashed: true,
      points: points.map((p) => ({ x: p.session, y: p.level, marker: "circle", note: fmtDate(p.startedAt) })),
    },
    {
      id: "median",
      label: "Median",
      color: "var(--world)",
      points: points.map((p) => ({ x: p.session, y: p.median, lo: p.lo, hi: p.hi })),
    },
  ]
  const d: Series[] = [
    { id: "position", label: "Position", points: points.flatMap((p) => (p.dPosition === null ? [] : [{ x: p.session, y: p.dPosition }])) },
    { id: "letter", label: "Letter", points: points.flatMap((p) => (p.dLetter === null ? [] : [{ x: p.session, y: p.dLetter }])) },
  ].filter((s) => s.points.length > 0)

  return (
    <div className="flex flex-col gap-8">
      <ChartFrame
        className={FRAME}
        title="Level per session"
        subtitle={`Mean level of each comparable session, with the rolling median of the last ${CURVE_WINDOW} and their range.`}
        table={lineTable(level, { xTitle: "Session", xFormat: session, yFormat: fmtN, caption: `${MODE_LABEL[mode]}: level per session` })}
        footer={aside}
      >
        <LineChart
          name={`${MODE_LABEL[mode]}: level per session`}
          series={level}
          xFormat={session}
          yFormat={fmtN}
          height={240}
          bandNote={`Shaded: range of the last ${CURVE_WINDOW} sessions`}
        />
      </ChartFrame>
      <ChartFrame
        className={FRAME}
        title="Sensitivity (d′) per stream"
        subtitle="Higher means matches and non-matches were told apart better. Counts sit in each session summary."
        table={lineTable(d, { xTitle: "Session", xFormat: session, yFormat: (v) => v.toFixed(2), caption: `${MODE_LABEL[mode]}: d-prime per stream` })}
      >
        <LineChart
          name={`${MODE_LABEL[mode]}: d-prime per stream`}
          series={d}
          xFormat={session}
          yFormat={(v) => v.toFixed(1)}
          height={200}
        />
      </ChartFrame>
    </div>
  )
}

export function HistoryPanel({
  sessions,
  error,
  exported,
  onExport,
}: {
  sessions: NBackSummary[]
  error: string | null
  exported: string | null
  onExport: () => void
}) {
  const trained = (["dual", "position", "letter"] as const).filter((m) => sessions.some((s) => s.mode === m))
  const [picked, setPicked] = useState<TrainMode | null>(null)
  const mode = picked && trained.includes(picked) ? picked : (trained[0] ?? "dual")

  return (
    <Panel
      eyebrow="History"
      title="Training curve"
      className="kit-rise"
      style={riseStyle(4)}
      actions={
        <Button variant="outline" size="sm" onClick={onExport} disabled={sessions.length === 0}>
          Export trials (CSV)
        </Button>
      }
    >
      {error && (
        <p className="text-bad mb-4 text-sm" role="alert">
          {error}
        </p>
      )}
      <div className="flex flex-col gap-6">
        {trained.length === 0 ? (
          <p className="text-ink-2 text-sm">No training sessions yet. The curve appears after your second comparable session.</p>
        ) : (
          <>
            {trained.length > 1 && (
              <Segmented
                label="Mode"
                value={mode}
                onChange={setPicked}
                options={trained.map((m) => ({ value: m, label: MODE_LABEL[m] }))}
                className="w-fit"
              />
            )}
            <Curves sessions={sessions} mode={mode} />
          </>
        )}
        <div className="border-hairline flex flex-wrap items-baseline justify-between gap-x-6 gap-y-2 border-t pt-4">
          <p className="text-ink-2 font-serif text-[15px] italic">{HONEST}</p>
          {exported && <span className="text-ink-3 font-mono text-xs break-all">Exported to {exported}</span>}
        </div>
      </div>
    </Panel>
  )
}

/** Transfer checks, kept apart from the training curve: different stimuli, a fixed level, every 4 weeks. */
export function TransferChecks({
  checks,
  days,
  available,
  children,
}: {
  checks: NBackSummary[]
  days: number | null
  /** History needs the desktop app; without it the list says so. */
  available: boolean
  /** The start control. */
  children: ReactNode
}) {
  const status = transferStatus(days)
  const sorted = [...checks].sort((a, b) => b.started_at - a.started_at)
  const th = "text-ink-3 px-3 py-2 text-right font-mono text-[11px] font-normal tracking-[0.14em] uppercase"
  return (
    <Panel
      eyebrow="Measurement"
      title="Transfer check"
      actions={<LedStatus tone={status.tone} state={status.state} />}
      className="kit-rise"
      style={riseStyle(5)}
    >
      {children}
      <div className="border-hairline mt-6 border-t pt-5">
        <Eyebrow as="h3" rule className="mb-3">
          Past checks
        </Eyebrow>
        {!available ? (
          <p className="text-ink-2 text-sm">Saved checks are listed in the desktop app.</p>
        ) : sorted.length === 0 ? (
          <p className="text-ink-2 text-sm">None yet. Transfer is measured apart from the curve above and never mixed into it.</p>
        ) : (
          <table className="w-full border-collapse text-sm">
            <caption className="sr-only">Transfer checks, newest first</caption>
            <thead>
              <tr>
                <th scope="col" className={cn(th, "pl-0 text-left")}>Date</th>
                <th scope="col" className={th}>n</th>
                <th scope="col" className={cn(th, "normal-case tracking-normal")}>d′ shape</th>
                <th scope="col" className={cn(th, "normal-case tracking-normal")}>d′ tone</th>
                <th scope="col" className={cn(th, "pr-0")}>Void blocks</th>
              </tr>
            </thead>
            <tbody className="font-mono tabular-nums">
              {sorted.map((s) => (
                <tr key={s.id} className="border-hairline border-t">
                  <th scope="row" className="py-2.5 pr-3 text-left font-sans font-normal">
                    {fmtDate(s.started_at)}
                  </th>
                  <td className="px-3 py-2.5 text-right">{s.start_n}</td>
                  <td className="px-3 py-2.5 text-right">{fmtD(s.d_prime.position)}</td>
                  <td className="px-3 py-2.5 text-right">{fmtD(s.d_prime.letter)}</td>
                  <td className="py-2.5 pl-3 text-right">{s.void_blocks}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </Panel>
  )
}
