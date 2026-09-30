import { useId, useMemo, useState, type KeyboardEvent, type PointerEvent } from "react"

import { cn } from "@/lib/utils"
import { EASE_SOFT, useReducedMotion } from "./_motion"
import { useArmed, useElementWidth } from "./_hooks"
import { ChartTooltip } from "./ChartTooltip"
import {
  areaPath,
  bandPath,
  linePath,
  linearScale,
  loneEntries,
  markerPath,
  MONO_CHAR_W,
  nearestIndex,
  niceTicks,
  niceTimeTicks,
  repelLabels,
  runsOf,
  textWidth,
  tickStride,
  truncateLabel,
  whiskerPath,
  type MarkerShape,
} from "./geometry"
import { seriesColor, washOf } from "./palette"
import type { TableData } from "./types"
import "./charts.css"

export interface SeriesPoint {
  x: number
  y: number
  /** Lower and upper edge of a confidence interval. Drawn as a 10% band when both are present; a point with no
   *  neighbour to join a band to, and every point of a category axis, is drawn as a whisker instead. */
  lo?: number
  hi?: number
  /** Shape drawn on this point, e.g. one per decision type. The shape is the signal, not the colour. */
  marker?: MarkerShape
  /** Extra line in the tooltip and the table, e.g. "reverted". */
  note?: string
}

export interface Series {
  id: string
  label: string
  /** Any CSS colour. Defaults to --chart-1..5 by position. */
  color?: string
  /** Dashed series fade in instead of drawing on. */
  dashed?: boolean
  points: SeriesPoint[]
}

export interface MarkerKeyItem {
  shape: MarkerShape
  label: string
}

export interface LineChartProps {
  series: Series[]
  /** Accessible name. */
  name: string
  /** "category": x is the position (0, 1, 2 ...) of a named tick. Every tick is labelled (labels thin out when
   *  they would touch), every point is a dot, and an interval is a whisker per point, since there is nothing
   *  between categories to shade. Give the names in `xLabels`. */
  xKind?: "index" | "time" | "category"
  /** The name of each category tick, by position. */
  xLabels?: string[]
  /** x axis label text. Time defaults to a short date. */
  xFormat?: (x: number) => string
  yFormat?: (y: number) => string
  /** Fixed y range. Otherwise nice ticks around the data (and zero for area charts). */
  yDomain?: [number, number]
  height?: number
  /** 10% wash under each line. */
  area?: boolean
  /** Draw each line on once, on first render. */
  drawOn?: boolean
  /** Explains the shaded band, e.g. "Shaded band: 95% confidence interval". */
  bandNote?: string
  /** Explains the marker shapes. */
  markerKey?: MarkerKeyItem[]
  className?: string
}

const DRAW_MS = 1150
const PAD_T = 12
const PAD_B = 28
const FOCUS_RING = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

const fmtNum = (v: number) => String(+v.toFixed(3))
const fmtDay = (t: number) => new Date(t).toLocaleDateString("en-US", { month: "short", day: "numeric" })
const fmtDayLong = (t: number) => new Date(t).toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" })
const fmtHour = (t: number) => new Date(t).toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })

export function MarkerGlyph({ shape, color, size = 4.5, filled = false }: { shape: MarkerShape; color: string; size?: number; filled?: boolean }) {
  return (
    <path
      d={markerPath(shape, size)}
      fill={shape === "cross" ? "none" : filled ? color : "var(--card)"}
      stroke={color}
      strokeWidth={2}
      strokeLinejoin="round"
      strokeLinecap="round"
    />
  )
}

function Swatch({ shape = "circle", color }: { shape?: MarkerShape; color: string }) {
  return (
    <svg width="12" height="12" viewBox="-6 -6 12 12" aria-hidden="true" className="shrink-0">
      <MarkerGlyph shape={shape} color={color} size={3.6} filled />
    </svg>
  )
}

/** The same data as a table: one row per x, one column per series (with the band when present). */
export function lineTable(series: Series[], opts: { xTitle?: string; xFormat?: (x: number) => string; yFormat?: (y: number) => string; caption?: string } = {}): TableData {
  const xf = opts.xFormat ?? fmtNum
  const yf = opts.yFormat ?? fmtNum
  const xs = [...new Set(series.flatMap((s) => s.points.map((p) => p.x)))].sort((a, b) => a - b)
  const maps = series.map((s) => new Map(s.points.map((p) => [p.x, p])))
  return {
    caption: opts.caption,
    columns: [opts.xTitle ?? "x", ...series.map((s) => s.label)],
    rows: xs.map((x) => [
      xf(x),
      ...maps.map((m) => {
        const p = m.get(x)
        if (!p) return "—"
        const band = p.lo !== undefined && p.hi !== undefined ? ` (${yf(p.lo)}–${yf(p.hi)})` : ""
        return `${yf(p.y)}${band}${p.note ? ` ${p.note}` : ""}`
      }),
    ]),
  }
}

export function LineChart({
  series,
  name,
  xKind = "index",
  xLabels,
  xFormat,
  yFormat = fmtNum,
  yDomain,
  height = 280,
  area = false,
  drawOn = true,
  bandNote,
  markerKey,
  className,
}: LineChartProps) {
  const uid = useId()
  const reduced = useReducedMotion()
  const armed = useArmed(drawOn)
  const drawn = armed || reduced || !drawOn
  const drawMs = reduced || !drawOn ? 0 : DRAW_MS
  const [wrapRef, width] = useElementWidth<HTMLDivElement>(640)

  const [hoverId, setHoverId] = useState<string | null>(null)
  const [focusId, setFocusId] = useState<string | null>(null)
  const [focusVisibleId, setFocusVisibleId] = useState<string | null>(null)
  const [pinId, setPinId] = useState<string | null>(null)
  const [scrub, setScrub] = useState<number | null>(null)
  const [lastScrub, setLastScrub] = useState(0)

  // Domains and ticks depend only on the data.
  const model = useMemo(() => {
    const xs = [...new Set(series.flatMap((s) => s.points.map((p) => p.x)))].sort((a, b) => a - b)
    const ys = series.flatMap((s) => s.points.flatMap((p) => [p.y, p.lo, p.hi])).filter((v): v is number => typeof v === "number" && Number.isFinite(v))
    const xMin = xs[0] ?? 0
    const xMax = xs[xs.length - 1] ?? 1
    let yMin = ys.length ? Math.min(...ys) : 0
    const yMax = ys.length ? Math.max(...ys) : 1
    if (area && yMin > 0) yMin = 0
    const nice = yDomain ? { ...niceTicks(yDomain[0], yDomain[1], 5), min: yDomain[0], max: yDomain[1] } : niceTicks(yMin, yMax, 5)
    const yTicks = nice.ticks.filter((t) => t >= nice.min - 1e-9 && t <= nice.max + 1e-9)
    let xTicks: number[]
    let unit: "hour" | "day" = "day"
    if (xKind === "time") {
      const t = niceTimeTicks(xMin, xMax, 6)
      xTicks = t.ticks
      unit = t.unit
    } else if (xKind === "category") {
      xTicks = xs
    } else {
      xTicks = niceTicks(xMin, xMax, Math.min(8, Math.max(2, xs.length))).ticks.filter((t) => Number.isInteger(t) && t >= xMin && t <= xMax)
    }
    const pad = xKind === "category" ? 0.5 : 0
    return { xs, xMin, xMax, dMin: xMin - pad, dMax: xMax + pad, yMin: nice.min, yMax: nice.max, yTicks, xTicks, unit }
  }, [series, yDomain, area, xKind])

  const fmtCategory = (x: number) => xLabels?.[x] ?? fmtNum(x)
  const fmtX = xFormat ?? (xKind === "time" ? (model.unit === "hour" ? fmtHour : fmtDay) : xKind === "category" ? fmtCategory : fmtNum)
  const fmtXLong = xFormat ?? (xKind === "time" ? (model.unit === "hour" ? (t: number) => `${fmtDay(t)}, ${fmtHour(t)}` : fmtDayLong) : xKind === "category" ? fmtCategory : fmtNum)

  const compact = width < 480
  const endText = (s: Series) => {
    const last = s.points[s.points.length - 1]
    const label = truncateLabel(s.label, compact ? 16 : 20)
    return compact || !last ? label : `${label} ${yFormat(last.y)}`
  }
  const left = Math.max(1, ...model.yTicks.map((t) => yFormat(t).length)) * MONO_CHAR_W + 14
  const right = Math.min(width * 0.4, Math.max(24, ...series.map((s) => textWidth(endText(s)))) + 30)
  const plotL = left
  const plotR = Math.max(plotL + 40, width - right)
  const plotT = PAD_T
  const plotB = height - PAD_B
  const xScale = linearScale(model.dMin, model.dMax, plotL + 6, plotR - 6)
  const yScale = linearScale(model.yMin, model.yMax, plotB, plotT)
  const xsPx = model.xs.map(xScale)
  const stride = xKind === "category" ? tickStride(model.xTicks.length + 1, plotR - plotL - 12, Math.max(0, ...model.xTicks.map((t) => textWidth(fmtX(t))))) : 1

  const laid = series.map((s, i) => {
    const color = s.color ?? seriesColor(i)
    const pts = [...s.points].sort((a, b) => a.x - b.x)
    const xy = pts.map((p) => ({ x: xScale(p.x), y: yScale(p.y) }))
    const runs = runsOf(pts, (p) => p.lo !== undefined && p.hi !== undefined)
    const band = (xKind === "category" ? [] : runs)
      .map((r) =>
        bandPath(
          r.map((p) => ({ x: xScale(p.x), y: yScale(p.hi!) })),
          r.map((p) => ({ x: xScale(p.x), y: yScale(p.lo!) })),
        ),
      )
      .join("")
    const whiskers = (xKind === "category" ? runs.flat() : loneEntries(runs))
      .map((p) => whiskerPath(xScale(p.x), yScale(p.lo!), yScale(p.hi!)))
      .join("")
    return { s, i, color, pts, xy, d: linePath(xy), band, whiskers, areaD: area ? areaPath(xy, plotB) : "" }
  })
  const labelYs = repelLabels(
    laid.map((l) => l.xy[l.xy.length - 1]?.y ?? plotB),
    16,
    plotT + 6,
    plotB,
  )

  const active = hoverId ?? focusId ?? pinId
  const dimmed = (id: string) => active !== null && active !== id
  const shown = scrub ?? lastScrub
  const shownX = model.xs[shown]
  const sx = xsPx[shown] ?? plotL

  const setScrubIdx = (i: number | null) => {
    setScrub(i)
    if (i !== null) setLastScrub(i)
  }
  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const px = e.clientX - rect.left
    const py = e.clientY - rect.top
    if (px < plotL - 12 || px > plotR + 12 || py < plotT - 8 || py > plotB + 8) setScrubIdx(null)
    else setScrubIdx(nearestIndex(xsPx, px))
  }
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget || model.xs.length === 0) return
    const cur = scrub ?? model.xs.length - 1
    let next: number | null | undefined
    if (e.key === "ArrowRight") next = Math.min(model.xs.length - 1, cur + 1)
    else if (e.key === "ArrowLeft") next = Math.max(0, cur - 1)
    else if (e.key === "Home") next = 0
    else if (e.key === "End") next = model.xs.length - 1
    else if (e.key === "Escape") next = null
    if (next === undefined) return
    e.preventDefault()
    setScrubIdx(next)
  }

  const valuesAt = (x: number | undefined) =>
    laid.map((l) => ({ l, p: x === undefined ? undefined : l.pts.find((p) => p.x === x) }))
  const announce =
    scrub !== null && shownX !== undefined
      ? `${fmtXLong(shownX)}: ${valuesAt(shownX)
          .map(({ l, p }) => `${l.s.label} ${p ? yFormat(p.y) : "no value"}`)
          .join(", ")}`
      : ""
  const summary = `${series.length} series, ${model.xs.length} points${
    model.xs.length ? ` from ${fmtX(model.xMin)} to ${fmtX(model.xMax)}` : ""
  }. Latest: ${laid.map((l) => `${l.s.label} ${l.pts.length ? yFormat(l.pts[l.pts.length - 1].y) : "none"}`).join(", ")}. Use the left and right arrow keys to read values at each point. The Table view lists all the data.`

  return (
    <div className={cn("min-w-0", className)}>
      <div
        ref={wrapRef}
        role="group"
        aria-label={name}
        aria-describedby={`${uid}-d`}
        tabIndex={0}
        onKeyDown={onKeyDown}
        onFocus={(e) => {
          if (e.target === e.currentTarget && e.currentTarget.matches(":focus-visible") && scrub === null) setScrubIdx(model.xs.length - 1)
        }}
        onBlur={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setScrub(null)
        }}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setScrub(null)}
        className={cn("relative rounded-md select-none", FOCUS_RING)}
        style={{ height, touchAction: "pan-y" }}
      >
        <p id={`${uid}-d`} className="sr-only">
          {summary}
        </p>
        <p className="sr-only" aria-live="polite">
          {announce}
        </p>
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="block overflow-visible">
          <g aria-hidden="true">
            {model.yTicks.map((t) => (
              <g key={t}>
                <line x1={plotL} x2={plotR} y1={yScale(t)} y2={yScale(t)} stroke={t === model.yMin ? "var(--hairline-strong)" : "var(--hairline)"} strokeWidth={1} />
                <text x={plotL - 8} y={yScale(t)} textAnchor="end" dominantBaseline="central" className="fill-ink-3 font-mono text-[11px] tabular-nums">
                  {yFormat(t)}
                </text>
              </g>
            ))}
            {model.xTicks.map((t, ti) => {
              if (ti % stride !== 0) return null
              const label = fmtX(t)
              const half = textWidth(label) / 2
              const x = Math.min(Math.max(xScale(t), half), width - half)
              return (
                <text key={t} x={x} y={plotB + 18} textAnchor="middle" className="fill-ink-3 font-mono text-[11px] tabular-nums">
                  {label}
                </text>
              )
            })}
          </g>

          {laid.map((l) => {
            const dim = dimmed(l.s.id)
            const reveal = drawn ? 1 : 0
            return (
              <g
                key={l.s.id}
                aria-hidden="true"
                style={{ opacity: dim ? 0.14 : 1, transition: `opacity var(--dur-base) ${EASE_SOFT}` }}
              >
                {l.band ? (
                  <path d={l.band} fill={washOf(l.color, 10)} style={{ opacity: reveal, transition: drawMs ? `opacity ${drawMs * 0.5}ms ${EASE_SOFT} ${drawMs * 0.6}ms` : "none" }} />
                ) : null}
                {l.whiskers ? (
                  <path d={l.whiskers} fill="none" stroke={l.color} strokeOpacity={0.6} strokeWidth={2} strokeLinecap="round" style={{ opacity: reveal, transition: drawMs ? `opacity ${drawMs * 0.5}ms ${EASE_SOFT} ${drawMs * 0.6}ms` : "none" }} />
                ) : null}
                {l.areaD ? (
                  <path d={l.areaD} fill={washOf(l.color, 10)} style={{ opacity: reveal, transition: drawMs ? `opacity ${drawMs * 0.5}ms ${EASE_SOFT} ${drawMs * 0.5}ms` : "none" }} />
                ) : null}
                <path
                  d={l.d}
                  fill="none"
                  stroke={l.color}
                  strokeWidth={2}
                  strokeLinejoin="round"
                  strokeLinecap={l.s.dashed ? "butt" : "round"}
                  {...(l.s.dashed
                    ? { strokeDasharray: "5 4", style: { opacity: reveal, transition: drawMs ? `opacity ${drawMs}ms ${EASE_SOFT} ${l.i * 90}ms` : "none" } }
                    : {
                        pathLength: 1,
                        strokeDasharray: "1 2",
                        strokeDashoffset: drawn ? 0 : 1,
                        style: { transition: drawMs ? `stroke-dashoffset ${drawMs}ms ${EASE_SOFT} ${l.i * 90}ms` : "none" },
                      })}
                />
                <path
                  d={l.d}
                  fill="none"
                  stroke="transparent"
                  strokeWidth={14}
                  strokeLinejoin="round"
                  onPointerEnter={() => setHoverId(l.s.id)}
                  onPointerLeave={() => setHoverId(null)}
                />
                <g style={{ opacity: reveal, transition: drawMs ? `opacity ${drawMs * 0.4}ms ${EASE_SOFT} ${drawMs * 0.8}ms` : "none" }}>
                  {l.pts.map((p, k) =>
                    p.marker ? (
                      <g key={p.x} transform={`translate(${l.xy[k].x} ${l.xy[k].y})`}>
                        <MarkerGlyph shape={p.marker} color={l.color} />
                      </g>
                    ) : null,
                  )}
                  {xKind === "category" && !l.s.dashed ? l.xy.map((p, k) => <circle key={l.pts[k].x} cx={p.x} cy={p.y} r={3.5} fill={l.color} />) : null}
                  {l.xy.length ? <circle cx={l.xy[l.xy.length - 1].x} cy={l.xy[l.xy.length - 1].y} r={3.5} fill={l.color} /> : null}
                </g>
              </g>
            )
          })}

          <g aria-hidden="true" style={{ opacity: scrub !== null ? 1 : 0, transition: `opacity var(--dur-quick) ${EASE_SOFT}` }} pointerEvents="none">
            <line
              x1={0}
              x2={0}
              y1={plotT}
              y2={plotB}
              stroke="var(--hairline-strong)"
              strokeWidth={1}
              style={{ transform: `translateX(${sx}px)`, transition: `transform var(--dur-quick) ${EASE_SOFT}` }}
            />
            {valuesAt(shownX).map(({ l, p }) =>
              p ? (
                <g
                  key={l.s.id}
                  style={{
                    transform: `translate(${sx}px, ${yScale(p.y)}px)`,
                    opacity: dimmed(l.s.id) ? 0.14 : 1,
                    transition: `transform var(--dur-quick) ${EASE_SOFT}, opacity var(--dur-base) ${EASE_SOFT}`,
                  }}
                >
                  <circle r={4.5} fill="var(--card)" stroke={l.color} strokeWidth={2} />
                </g>
              ) : null,
            )}
          </g>

          {laid.map((l, k) => {
            const last = l.xy[l.xy.length - 1]
            if (!last) return null
            const text = endText(l.s)
            const y = labelYs[k]
            const lx = plotR + 12
            const w = textWidth(text) + 18
            const moved = Math.abs(y - last.y) > 1
            const pinned = pinId === l.s.id
            const value = l.pts[l.pts.length - 1].y
            return (
              <g
                key={l.s.id}
                role="button"
                tabIndex={0}
                aria-pressed={pinned}
                aria-label={`${l.s.label}, latest ${yFormat(value)}. Press Enter to ${pinned ? "show all series" : "isolate this series"}.`}
                className="chart-focusable cursor-pointer"
                style={{ opacity: dimmed(l.s.id) ? 0.14 : 1, transition: `opacity var(--dur-base) ${EASE_SOFT}` }}
                onPointerEnter={() => setHoverId(l.s.id)}
                onPointerLeave={() => setHoverId(null)}
                onFocus={(e) => {
                  setFocusId(l.s.id)
                  setFocusVisibleId(e.currentTarget.matches(":focus-visible") ? l.s.id : null)
                }}
                onBlur={() => {
                  setFocusId(null)
                  setFocusVisibleId(null)
                }}
                onClick={() => setPinId(pinned ? null : l.s.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault()
                    setPinId(pinned ? null : l.s.id)
                  }
                }}
              >
                {moved ? <path d={`M${last.x + 6} ${last.y}L${lx - 4} ${y}`} stroke={l.color} strokeWidth={1} fill="none" opacity={0.6} /> : null}
                <rect x={lx - 6} y={y - 10} width={w} height={20} rx={6} fill="transparent" stroke={focusVisibleId === l.s.id ? "var(--world)" : "none"} strokeWidth={2} />
                <g transform={`translate(${lx + 1} ${y})`} aria-hidden="true">
                  <MarkerGlyph shape="circle" color={l.color} size={3.5} filled />
                </g>
                <text x={lx + 12} y={y} dominantBaseline="central" className="fill-ink-2 font-mono text-[11px]" style={{ fontWeight: pinned ? 600 : 400 }}>
                  {text}
                </text>
              </g>
            )
          })}
        </svg>

        <ChartTooltip x={sx} y={plotT + 4} containerW={width} containerH={height} visible={scrub !== null}>
          {shownX !== undefined ? (
            <>
              <div className="text-ink-3 mb-1 tracking-[0.12em] uppercase">{fmtXLong(shownX)}</div>
              {valuesAt(shownX).map(({ l, p }) => (
                <div key={l.s.id} className="flex items-center gap-2 py-px">
                  <Swatch shape={p?.marker ?? "circle"} color={l.color} />
                  <span className="text-ink-2">{l.s.label}</span>
                  <span className="text-foreground ml-auto pl-4 whitespace-nowrap tabular-nums">
                    {p ? yFormat(p.y) : "—"}
                    {p && p.lo !== undefined && p.hi !== undefined ? (
                      <span className="text-ink-3">
                        {" "}
                        {yFormat(p.lo)}&ndash;{yFormat(p.hi)}
                      </span>
                    ) : null}
                  </span>
                </div>
              ))}
              {valuesAt(shownX)
                .filter(({ p }) => p?.note)
                .map(({ l, p }) => (
                  <div key={l.s.id} className="text-ink-3 mt-0.5">
                    {l.s.label}: {p!.note}
                  </div>
                ))}
            </>
          ) : null}
        </ChartTooltip>
      </div>
      {bandNote || markerKey?.length ? (
        <div className="text-ink-3 mt-2 flex flex-wrap items-center gap-x-5 gap-y-1 font-mono text-[11px]">
          {markerKey?.map((m) => (
            <span key={m.shape + m.label} className="inline-flex items-center gap-1.5">
              <svg width="12" height="12" viewBox="-6 -6 12 12" aria-hidden="true">
                <MarkerGlyph shape={m.shape} color="var(--ink-2)" size={3.6} />
              </svg>
              {m.label}
            </span>
          ))}
          {bandNote ? (
            <span className="inline-flex items-center gap-1.5">
              <span aria-hidden="true" className="inline-block h-2.5 w-4 rounded-[3px]" style={{ background: washOf("var(--ink-2)", 22) }} />
              {bandNote}
            </span>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}

export function AreaChart(props: Omit<LineChartProps, "area">) {
  return <LineChart {...props} area />
}
