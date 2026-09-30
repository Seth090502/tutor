// Progress: DeepTutor practice summary, daily analytics and mastery topics.
// Never call /api/mastery-paths/progress/{id}/map or /board: they create paths as a side effect.
import { useCallback, useEffect, useState } from "react"
import { Clock, RefreshCw, ServerOff } from "lucide-react"

import { Button } from "@/components/ui/button"
import { BACKEND_URL, HttpError, fetchJson } from "@/lib/backend"
import {
  LegendDot,
  Notice,
  Page,
  Panel,
  ScreenBoundary,
  StackedBar,
  Stat,
  StateMessage,
  errorText,
  fmtDateTime,
  fmtInt,
  niceStep,
  type Segment,
} from "./ui"

type Summary = {
  total: number
  mistakes: number
  due: number
  overdue: number
  reviewed_today: number
  next_due_at: number | null
}
type Counts = { questions: number; mistakes: number; reviews: number }
type Analytics = { days: number; daily: (Counts & { date: string })[]; totals: Counts }
type Topic = {
  path_id: string
  name: string
  metadata?: { emoji?: string }
  map?: { counts?: { mastered: number; learning: number; new: number; total: number }; due_reviews?: number }
}

const DAYS = 30
const SERIES: { key: keyof Counts; label: string; color: string }[] = [
  { key: "questions", label: "Questions", color: "var(--viz-1)" },
  { key: "mistakes", label: "Mistakes", color: "var(--viz-2)" },
  { key: "reviews", label: "Reviews", color: "var(--viz-3)" },
]

type Loaded = {
  summary: Summary | null
  analytics: Analytics | null
  topics: Topic[] | null
  errors: string[]
}

export function ProgressScreen() {
  return (
    <ScreenBoundary>
      <ProgressView />
    </ScreenBoundary>
  )
}

function ProgressView() {
  const [data, setData] = useState<Loaded | null>(null)
  const [offline, setOffline] = useState(false)
  const [loading, setLoading] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    const tz = encodeURIComponent(Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC")
    const [summary, analytics, topics] = await Promise.allSettled([
      fetchJson<Summary>(BACKEND_URL, `/api/question-notebook/practice/summary?timezone=${tz}`),
      fetchJson<Analytics>(BACKEND_URL, `/api/question-notebook/practice/analytics?days=${DAYS}&timezone=${tz}`),
      fetchJson<{ topics: Topic[] }>(BACKEND_URL, "/api/mastery-paths/topics"),
    ])
    const failures = [summary, analytics, topics].flatMap((r) => (r.status === "rejected" ? [r.reason as unknown] : []))
    // A rejection that is not an HTTP status means nothing answered on the port.
    setOffline(failures.some((f) => !(f instanceof HttpError)))
    setData({
      summary: summary.status === "fulfilled" ? summary.value : null,
      analytics: analytics.status === "fulfilled" ? analytics.value : null,
      topics: topics.status === "fulfilled" ? (topics.value.topics ?? []) : null,
      errors: failures.map(errorText),
    })
    setLoading(false)
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  if (offline) return <StateMessage icon={ServerOff} title="Backend not running; start it from Settings" />
  if (!data) return <StateMessage title="Loading…" />

  const { summary, analytics, topics, errors } = data
  return (
    <Page>
      <div className="flex justify-end">
        <Button variant="outline" size="sm" onClick={() => void load()} disabled={loading}>
          <RefreshCw />
          Refresh
        </Button>
      </div>
      {errors.length > 0 && (
        <Notice>
          <ul className="space-y-1">
            {errors.map((e, i) => (
              <li key={i}>{e}</li>
            ))}
          </ul>
        </Notice>
      )}

      {summary && (
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-5">
            <Stat label="Due" value={fmtInt(summary.due)} />
            <Stat label="Overdue" value={fmtInt(summary.overdue)} />
            <Stat label="Reviewed today" value={fmtInt(summary.reviewed_today)} />
            <Stat label="Total" value={fmtInt(summary.total)} />
            <Stat label="Mistakes" value={fmtInt(summary.mistakes)} />
          </div>
          <p className="text-muted-foreground flex items-center gap-1.5 text-sm">
            <Clock className="size-4" />
            {summary.next_due_at ? `Next due ${fmtDateTime(summary.next_due_at)}` : "Nothing scheduled"}
          </p>
        </div>
      )}

      {analytics && (
        <Panel title={`Last ${analytics.days || DAYS} days`}>
          <DailyChart analytics={analytics} />
        </Panel>
      )}

      {topics && (
        <Panel title="Mastery topics">
          {topics.length === 0 ? (
            <p className="text-muted-foreground text-sm">No mastery topics yet.</p>
          ) : (
            <ul className="divide-y">
              {topics.map((t) => (
                <TopicRow key={t.path_id} topic={t} />
              ))}
            </ul>
          )}
        </Panel>
      )}
    </Page>
  )
}

function DailyChart({ analytics }: { analytics: Analytics }) {
  const daily = analytics.daily ?? []
  const W = 760
  const H = 220
  const m = { l: 40, r: 16, t: 12, b: 28 }
  const iw = W - m.l - m.r
  const ih = H - m.t - m.b
  const peak = Math.max(0, ...daily.flatMap((d) => SERIES.map((s) => d[s.key] ?? 0)))
  const step = peak <= 5 ? 1 : niceStep(peak / 4)
  const yMax = Math.max(step, Math.ceil(peak / step) * step)
  const ticks: number[] = []
  for (let t = 0; t <= yMax; t += step) ticks.push(t)
  const x = (i: number) => m.l + (daily.length <= 1 ? iw / 2 : (i * iw) / (daily.length - 1))
  const y = (v: number) => m.t + ih - (v / yMax) * ih
  const col = daily.length <= 1 ? iw : iw / (daily.length - 1)
  const labelEvery = Math.max(1, Math.ceil(daily.length / 7))

  return (
    <div>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label="Questions, mistakes and reviews per day">
        {ticks.map((t) => (
          <g key={t}>
            <line x1={m.l} x2={W - m.r} y1={y(t)} y2={y(t)} style={{ stroke: "var(--border)" }} />
            <text x={m.l - 8} y={y(t)} textAnchor="end" dominantBaseline="middle" className="fill-muted-foreground text-[11px]">
              {t}
            </text>
          </g>
        ))}
        {SERIES.map((s) => (
          <polyline
            key={s.key}
            points={daily.map((d, i) => `${x(i)},${y(d[s.key] ?? 0)}`).join(" ")}
            fill="none"
            strokeWidth={2}
            strokeLinejoin="round"
            strokeLinecap="round"
            style={{ stroke: s.color }}
          />
        ))}
        {daily.map((d, i) => (
          <g key={d.date}>
            {i % labelEvery === 0 && (
              <text x={x(i)} y={H - 8} textAnchor="middle" className="fill-muted-foreground text-[11px]">
                {d.date.slice(5)}
              </text>
            )}
            <rect x={x(i) - col / 2} y={m.t} width={col} height={ih} fill="transparent">
              <title>{`${d.date}: ${d.questions} questions, ${d.mistakes} mistakes, ${d.reviews} reviews`}</title>
            </rect>
          </g>
        ))}
      </svg>
      <ul className="mt-2 flex flex-wrap gap-x-6 gap-y-1">
        {SERIES.map((s) => (
          <LegendDot key={s.key} color={s.color} label={s.label} value={fmtInt(analytics.totals?.[s.key])} />
        ))}
      </ul>
      {peak === 0 && <p className="text-muted-foreground mt-3 text-sm">No practice activity in this period.</p>}
    </div>
  )
}

function TopicRow({ topic }: { topic: Topic }) {
  const c = topic.map?.counts ?? { mastered: 0, learning: 0, new: 0, total: 0 }
  const due = topic.map?.due_reviews ?? 0
  const segments: Segment[] = [
    { key: "mastered", label: "Mastered", value: c.mastered, color: "var(--viz-3)" },
    { key: "learning", label: "Learning", value: c.learning, color: "var(--viz-1)" },
    { key: "new", label: "New", value: c.new, color: "var(--viz-neutral)" },
  ]
  return (
    <li className="grid gap-x-6 gap-y-2 py-3 first:pt-0 last:pb-0 md:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_7.5rem] md:items-center">
      <div className="min-w-0 truncate text-sm font-medium">
        {topic.metadata?.emoji && <span className="mr-1.5">{topic.metadata.emoji}</span>}
        {topic.name}
      </div>
      <div>
        <StackedBar segments={segments} label={`${topic.name}: mastery`} height={10} />
        <p className="text-muted-foreground mt-1.5 text-xs tabular-nums">
          {c.total > 0 ? `${c.mastered} mastered · ${c.learning} learning · ${c.new} new` : "No objectives yet"}
        </p>
      </div>
      <div className="flex items-center gap-1.5 text-xs md:justify-end">
        <Clock className="size-3.5" style={due > 0 ? { color: "var(--viz-warn)" } : undefined} />
        <span className={due > 0 ? "" : "text-muted-foreground"}>{due > 0 ? `${due} due` : "No reviews due"}</span>
      </div>
    </li>
  )
}
