// History: the training curve (trained task only), transfer checks listed apart, the weekly dose
// reminder and CSV export.
import { Button } from "@/components/ui/button"
import { WEEK_SESSIONS, WEEK_SOFT_CAP_MIN, rollingMedian } from "./engine/history"
import { Section } from "./parts"
import type { Mode, NBackSummary } from "./types"

const W = 640
const H = 200
const PAD = { l: 36, r: 12, t: 12, b: 28 }
const WINDOW = 5

const fmtDate = (t: number) => new Date(t * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric" })
const fmtD = (d: number | null) => (d === null ? "—" : d.toFixed(2))

export function TrainingCurve({ sessions, mode }: { sessions: NBackSummary[]; mode: Exclude<Mode, "transfer"> }) {
  const points = sessions.filter((s) => s.mode === mode).sort((a, b) => a.started_at - b.started_at)
  const comparable = points.filter((s) => s.comparable)
  if (comparable.length < 2) {
    return (
      <p className="text-ink-3 text-sm">
        {comparable.length === 0 ? "No comparable" : "One comparable"} {mode} session so far; the curve starts at two.
      </p>
    )
  }
  const bands = rollingMedian(
    comparable.map((s) => s.mean_n),
    WINDOW,
  )
  const maxN = Math.max(2, ...points.map((s) => s.max_n))
  const x = (i: number) => PAD.l + (points.length === 1 ? 0 : (i / (points.length - 1)) * (W - PAD.l - PAD.r))
  const y = (n: number) => PAD.t + (1 - (n - 1) / (maxN - 1 || 1)) * (H - PAD.t - PAD.b)
  const idx = (s: NBackSummary) => points.indexOf(s)
  const median = comparable.map((s, k) => `${k ? "L" : "M"}${x(idx(s)).toFixed(1)},${y(bands[k].median).toFixed(1)}`).join("")
  const band =
    comparable.map((s, k) => `${k ? "L" : "M"}${x(idx(s)).toFixed(1)},${y(bands[k].hi).toFixed(1)}`).join("") +
    [...comparable]
      .reverse()
      .map((s, k) => `L${x(idx(s)).toFixed(1)},${y(bands[comparable.length - 1 - k].lo).toFixed(1)}`)
      .join("") +
    "Z"
  const ticks = Array.from({ length: maxN }, (_, k) => k + 1)
  return (
    <figure>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label={`Mean level per ${mode} session`}>
        {ticks.map((n) => (
          <g key={n}>
            <line x1={PAD.l} x2={W - PAD.r} y1={y(n)} y2={y(n)} stroke="var(--hairline)" />
            <text x={PAD.l - 8} y={y(n) + 4} textAnchor="end" fontSize="11" fill="var(--ink-3)" fontFamily="var(--font-mono)">
              {n}
            </text>
          </g>
        ))}
        <path d={band} fill="var(--world-soft)" />
        <path d={median} fill="none" stroke="var(--world)" strokeWidth="2" />
        {points.map((s, i) => (
          <circle
            key={s.id}
            cx={x(i)}
            cy={y(s.mean_n)}
            r="3.5"
            fill={s.comparable ? "var(--world)" : "var(--card)"}
            stroke="var(--world)"
            strokeWidth="1.5"
          >
            <title>
              {fmtDate(s.started_at)}: mean n {s.mean_n.toFixed(2)}, max {s.max_n}
              {s.comparable ? "" : " (not comparable)"}
            </title>
          </circle>
        ))}
        <text x={PAD.l} y={H - 8} fontSize="11" fill="var(--ink-3)">
          {fmtDate(points[0].started_at)}
        </text>
        <text x={W - PAD.r} y={H - 8} textAnchor="end" fontSize="11" fill="var(--ink-3)">
          {fmtDate(points[points.length - 1].started_at)}
        </text>
      </svg>
      <figcaption className="text-ink-3 mt-1 text-xs">
        Each dot is a session's mean level (hollow: not comparable). Line: rolling median of the last {WINDOW} comparable
        sessions; shaded: their range. n-back scores are noisy and do not measure intelligence.
      </figcaption>
    </figure>
  )
}

export function HistoryPanel({
  sessions,
  week,
  error,
  exported,
  onExport,
}: {
  sessions: NBackSummary[]
  week: { minutes: number; sessions: number } | null
  error: string | null
  exported: string | null
  onExport: () => void
}) {
  const transfers = sessions.filter((s) => s.mode === "transfer")
  const trainedModes = (["dual", "position", "letter"] as const).filter((m) => sessions.some((s) => s.mode === m))
  return (
    <Section title="History">
      {error && <p className="text-bad mb-3 text-sm">{error}</p>}
      {week && (
        <p className={week.minutes >= WEEK_SOFT_CAP_MIN ? "text-warn mb-4 text-sm" : "text-ink-2 mb-4 text-sm"}>
          Last 7 days: {week.minutes} min in {week.sessions} session{week.sessions === 1 ? "" : "s"} (soft limit{" "}
          {WEEK_SOFT_CAP_MIN} min, up to {WEEK_SESSIONS} sessions).
          {week.minutes >= WEEK_SOFT_CAP_MIN &&
            " You are past the weekly soft limit. More time has no documented benefit and takes time from review."}
        </p>
      )}

      <h3 className="text-ink-2 mb-2 text-xs font-semibold">Training curve (trained task only)</h3>
      {trainedModes.length === 0 && <p className="text-ink-3 text-sm">No training sessions yet.</p>}
      <div className="flex flex-col gap-4">
        {trainedModes.map((m) => (
          <div key={m}>
            {trainedModes.length > 1 && <p className="text-ink-3 mb-1 text-xs capitalize">{m}</p>}
            <TrainingCurve sessions={sessions} mode={m} />
          </div>
        ))}
      </div>

      <h3 className="text-ink-2 mt-6 mb-1 text-xs font-semibold">Transfer checks (measurement)</h3>
      <p className="text-ink-3 mb-2 text-xs">
        Transfer is measured separately, with different stimuli at a fixed level, and never mixed into the curve above.
      </p>
      {transfers.length === 0 ? (
        <p className="text-ink-3 text-sm">None yet.</p>
      ) : (
        <table className="text-sm">
          <thead className="text-ink-3 text-left text-xs">
            <tr>
              <th className="pr-4 font-medium">Date</th>
              <th className="pr-4 font-medium">n</th>
              <th className="pr-4 font-medium">d′ shape</th>
              <th className="pr-4 font-medium">d′ tone</th>
              <th className="font-medium">Void blocks</th>
            </tr>
          </thead>
          <tbody className="font-mono tabular-nums">
            {transfers.map((s) => (
              <tr key={s.id} className="border-hairline border-t">
                <td className="pr-4 font-sans">{fmtDate(s.started_at)}</td>
                <td className="pr-4">{s.start_n}</td>
                <td className="pr-4">{fmtD(s.d_prime.position)}</td>
                <td className="pr-4">{fmtD(s.d_prime.letter)}</td>
                <td>{s.void_blocks}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div className="mt-6 flex flex-wrap items-center gap-3">
        <Button variant="outline" size="sm" onClick={onExport} disabled={sessions.length === 0}>
          Export trials (CSV)
        </Button>
        {exported && <span className="text-ink-3 font-mono text-xs break-all">{exported}</span>}
      </div>
    </Section>
  )
}
