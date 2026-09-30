// Pure data shaping for the Progress screen: API shapes (DeepTutor practice /summary and /analytics,
// contract C7 additions, /api/mastery-paths/topics) in, chart-kit inputs out. No React here.
import type { PhaseDay, Series, StackSegment, SunburstDatum, TableData } from "@/components/charts"
import { HttpError } from "@/lib/backend"

export type Summary = {
  total: number
  mistakes: number
  due: number
  overdue: number
  reviewed_today: number
  next_due_at: number | null
}
export type Counts = { questions: number; mistakes: number; reviews: number }
export type Calibration = { level: number; attempts: number; correct: number }
export type Assistance = { attempts: number; hinted: number; revealed: number }
export type Analytics = {
  days: number
  end_date?: string
  daily: (Counts & { date: string })[]
  totals: Counts
  /** Contract C7. Absent on a backend that predates it. */
  calibration?: Calibration[]
  assistance?: Assistance
}
export type KnowledgePoint = { id: string; name: string; status?: string; mastery?: number }
export type Module = { id: string; name: string; knowledge_points?: KnowledgePoint[] }
export type TopicCounts = { mastered: number; learning: number; new: number; total: number }
export type Topic = {
  path_id: string
  name: string
  map?: { counts?: TopicCounts; due_reviews?: number; modules?: Module[] }
}

export const DAYS = 30

// ---------- formatting ----------

const INT = new Intl.NumberFormat("en-US")
export const fmtInt = (n: number) => INT.format(Math.round(n))

/** Message for a failed request: the service's `detail` or `error` field when its body is JSON. */
export function errorText(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e)
  if (!(e instanceof HttpError)) return msg
  const at = msg.indexOf("{")
  if (at >= 0) {
    try {
      const body = JSON.parse(msg.slice(at)) as { error?: unknown; detail?: unknown }
      if (typeof body.error === "string") return body.error
      if (typeof body.detail === "string") return body.detail
    } catch {
      // Not JSON: fall through to the raw message.
    }
  }
  return msg
}

// ---------- hero ----------

/** The gauge scale: the smallest round number at or above the due count (at least 10). */
export function gaugeMax(due: number): number {
  for (const m of [10, 20, 50, 100, 200, 500, 1000, 2000, 5000]) if (due <= m) return m
  return Math.ceil(due / 1000) * 1000
}

export type NextDue = { label: string; value: number | string; unit: string }

const pad = (n: number) => String(n).padStart(2, "0")
const timeOf = (d: Date) => d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })

/** "Next due in 3 hours · Thu 2:30 PM", split into the label, a numeral and a unit for a Stat. */
export function nextDue(epochSeconds: number | null | undefined, nowMs: number): NextDue {
  if (typeof epochSeconds !== "number" || !Number.isFinite(epochSeconds)) {
    return { label: "Next due", value: "None", unit: "nothing scheduled" }
  }
  const at = new Date(epochSeconds * 1000)
  const diff = at.getTime() - nowMs
  if (diff <= 0) return { label: "Next due", value: "Now", unit: "waiting for you" }
  const minutes = Math.round(diff / 60_000)
  if (minutes < 60) return { label: "Next due in", value: Math.max(1, minutes), unit: `min · ${timeOf(at)}` }
  const hours = Math.round(diff / 3_600_000)
  if (hours < 48) {
    const day = at.toLocaleDateString("en-US", { weekday: "short" })
    return { label: "Next due in", value: hours, unit: `${hours === 1 ? "hour" : "hours"} · ${day} ${timeOf(at)}` }
  }
  const days = Math.round(diff / 86_400_000)
  const date = at.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" })
  return { label: "Next due in", value: days, unit: `days · ${date}` }
}

// ---------- 30 days ----------

/** "YYYY-MM-DD" as a local date, so the label never shifts a day through a UTC parse. */
function parseDay(iso: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso)
  if (!m) return null
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]))
  return Number.isNaN(d.getTime()) ? null : d
}

export function localIsoDay(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

export const fmtDayLong = (t: number) => new Date(t).toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" })

export const peakReviews = (daily: Analytics["daily"]) => Math.max(0, ...daily.map((d) => d.reviews ?? 0))

/**
 * One disc per day. The lit share is that day's reviews divided by the busiest day in the window, so
 * the busiest day is a full moon. It is relative to the period, not to a goal the learner set.
 */
export function phaseDays(daily: Analytics["daily"], todayIso: string): PhaseDay[] {
  const peak = peakReviews(daily)
  return daily.map((d) => {
    const at = parseDay(d.date)
    return {
      id: d.date,
      label: at ? String(at.getDate()) : d.date.slice(-2),
      title: at ? fmtDayLong(at.getTime()) : d.date,
      value: peak > 0 ? Math.min(1, (d.reviews ?? 0) / peak) : 0,
      isToday: d.date === todayIso,
    }
  })
}

export function phaseTableFor(daily: Analytics["daily"], days: PhaseDay[], peak: number): TableData {
  return {
    caption: "Reviews per day, and each day's share of the busiest day",
    columns: ["Day", "Reviews", "Share of busiest day"],
    rows: days.map((d, i) => [
      d.isToday ? `${d.title} (today)` : d.title,
      daily[i].reviews ?? 0,
      peak > 0 ? `${Math.round((d.value ?? 0) * 100)}%` : "—",
    ]),
  }
}

export const DAILY_SERIES: { key: keyof Counts; label: string }[] = [
  { key: "questions", label: "Questions" },
  { key: "mistakes", label: "Mistakes" },
  { key: "reviews", label: "Reviews" },
]

export function dailySeries(daily: Analytics["daily"]): Series[] {
  const points = daily.flatMap((d) => {
    const at = parseDay(d.date)
    return at ? [{ at: at.getTime(), d }] : []
  })
  return DAILY_SERIES.map((s) => ({
    id: s.key,
    label: s.label,
    points: points.map(({ at, d }) => ({ x: at, y: d[s.key] ?? 0 })),
  }))
}

/** A y axis in whole counts: at least 0 to 4, and a multiple of 4 above that, so ticks stay integers. */
export function countDomain(series: Series[]): [number, number] {
  const peak = Math.max(0, ...series.flatMap((s) => s.points.map((p) => p.y)))
  return [0, Math.max(4, Math.ceil(peak / 4) * 4)]
}

// ---------- calibration (C7) ----------

export const LEVEL_NAMES = ["Guess", "Unsure", "Fairly sure", "Certain"]
/** Level 1 to 4 read as 25, 50, 75 and 95 percent (PEDAGOGY.md): a reference line, not a target. */
export const expectedPct = (level: number) => [25, 50, 75, 95][level - 1] ?? 0

/** 95% Wilson score interval for a share, in percent (0 to 100). Null with no attempts. Sound for small counts, unlike a normal approximation. */
export function wilsonPct(correct: number, attempts: number): [number, number] | null {
  if (attempts <= 0) return null
  const z = 1.96
  const p = Math.min(1, Math.max(0, correct / attempts))
  const z2n = (z * z) / attempts
  const centre = (p + z2n / 2) / (1 + z2n)
  const half = (z * Math.sqrt((p * (1 - p)) / attempts + (z * z) / (4 * attempts * attempts))) / (1 + z2n)
  return [Math.max(0, 100 * (centre - half)), Math.min(100, 100 * (centre + half))]
}

export type CalibrationRow = { level: number; name: string; attempts: number; correct: number; pct: number | null }

export function calibrationRows(cal: Calibration[]): CalibrationRow[] {
  return [1, 2, 3, 4].map((level) => {
    const row = cal.find((c) => c.level === level)
    const attempts = Math.max(0, row?.attempts ?? 0)
    const correct = Math.min(attempts, Math.max(0, row?.correct ?? 0))
    return { level, name: LEVEL_NAMES[level - 1], attempts, correct, pct: attempts > 0 ? Math.round((100 * correct) / attempts) : null }
  })
}

/** Attempts below this are plotted but flagged as a small sample. */
export const SMALL_SAMPLE = 5

/**
 * The calibration plot as chart-kit input: one category per confidence level, the percent correct with its
 * 95% Wilson interval, and the dashed reference line. A level with no attempts has no point.
 */
export function calibrationSeries(rows: CalibrationRow[]): { series: Series[]; labels: string[] } {
  const actual = rows.flatMap((r) => {
    if (r.pct === null) return []
    const ci = wilsonPct(r.correct, r.attempts)
    return [
      {
        x: r.level - 1,
        y: r.pct,
        ...(ci ? { lo: ci[0], hi: ci[1] } : {}),
        note: `${r.correct} of ${r.attempts} right${r.attempts < SMALL_SAMPLE ? ", small sample" : ""}`,
      },
    ]
  })
  return {
    labels: rows.map((r) => r.name),
    series: [
      { id: "actual", label: "Correct", points: actual },
      { id: "reference", label: "Reference", color: "var(--ink-3)", dashed: true, points: rows.map((r) => ({ x: r.level - 1, y: expectedPct(r.level) })) },
    ],
  }
}

export function calibrationTable(rows: CalibrationRow[]): TableData {
  return {
    caption: "Percent correct at each confidence level",
    columns: ["Confidence", "Attempts", "Correct", "Percent correct", "Reference"],
    rows: rows.map((r) => [`${r.level} · ${r.name}`, r.attempts, r.correct, r.pct === null ? "—" : `${r.pct}%`, `${expectedPct(r.level)}%`]),
  }
}

// ---------- assistance (C7) ----------

/** `hinted` counts every attempt that used a rung, revealed ones included, so they are separated here. */
export function assistanceSegments(a: Assistance): StackSegment[] {
  const revealed = Math.min(Math.max(0, a.revealed), a.attempts)
  const hinted = Math.min(Math.max(0, a.hinted), a.attempts)
  const hintedOnly = Math.max(0, hinted - revealed)
  const unassisted = Math.max(0, a.attempts - Math.max(hinted, revealed))
  return [
    { id: "unassisted", label: "Unassisted", value: unassisted, color: "var(--ok)" },
    { id: "hinted", label: "Hinted", value: hintedOnly, color: "var(--warn)" },
    { id: "revealed", label: "Answer revealed", value: revealed, color: "var(--bad)" },
  ]
}

// ---------- mastery ----------

const finiteCount = (n: unknown) => (typeof n === "number" && Number.isFinite(n) && n > 0 ? n : 0)

export function topicCounts(t: Topic): TopicCounts {
  const c = t.map?.counts
  return { mastered: finiteCount(c?.mastered), learning: finiteCount(c?.learning), new: finiteCount(c?.new), total: finiteCount(c?.total) }
}

export const topicDue = (t: Topic) => finiteCount(t.map?.due_reviews)

export function totalCounts(topics: Topic[]): TopicCounts {
  return topics.reduce(
    (acc, t) => {
      const c = topicCounts(t)
      return { mastered: acc.mastered + c.mastered, learning: acc.learning + c.learning, new: acc.new + c.new, total: acc.total + c.total }
    },
    { mastered: 0, learning: 0, new: 0, total: 0 },
  )
}

export function masterySegments(c: Pick<TopicCounts, "mastered" | "learning" | "new">): StackSegment[] {
  return [
    { id: "mastered", label: "Mastered", value: c.mastered, color: "var(--ok)" },
    { id: "learning", label: "Learning", value: c.learning, color: "color-mix(in oklab, var(--ink) 56%, var(--card))" },
    { id: "new", label: "New", value: c.new, color: "color-mix(in oklab, var(--ink) 20%, var(--card))" },
  ]
}

/**
 * Subject (a mastery path) > topic (its modules) > skill (their knowledge points, shaded by the
 * server's mastery value). A path whose payload has no module detail becomes one leaf sized by its
 * objective count and shaded by the share mastered, so every path with objectives still shows.
 */
export function masteryTree(topics: Topic[]): SunburstDatum[] {
  return topics.flatMap((t): SunburstDatum[] => {
    const c = topicCounts(t)
    const modules = (t.map?.modules ?? []).flatMap((m): SunburstDatum[] => {
      const kps = (m.knowledge_points ?? []).map(
        (k): SunburstDatum => ({ id: `${t.path_id}/${m.id}/${k.id}`, label: k.name, value: 1, mastery: typeof k.mastery === "number" ? k.mastery : k.status === "mastered" ? 1 : 0 }),
      )
      return kps.length ? [{ id: `${t.path_id}/${m.id}`, label: m.name, children: kps }] : []
    })
    if (modules.length) return [{ id: t.path_id, label: t.name, children: modules }]
    if (c.total > 0) return [{ id: t.path_id, label: t.name, value: c.total, mastery: c.mastered / c.total }]
    return []
  })
}

/** Most urgent first: due reviews descending, then the name. */
export function sortTopics(topics: Topic[]): Topic[] {
  return [...topics].sort((a, b) => topicDue(b) - topicDue(a) || a.name.localeCompare(b.name))
}
