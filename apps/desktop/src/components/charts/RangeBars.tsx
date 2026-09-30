import { useId, useState } from "react"

import { cn } from "@/lib/utils"
import { EASE_SOFT } from "./_motion"
import { useElementWidth } from "./_hooks"
import { ChartTooltip } from "./ChartTooltip"
import { linearScale, MONO_CHAR_W, niceTicks, truncateLabel } from "./geometry"
import { outlineOf, seriesColor } from "./palette"
import type { TableData } from "./types"
import "./charts.css"

export interface RangeGroup {
  id: string
  label: string
  value: number
  /** Confidence interval, drawn as a whisker. */
  lo?: number
  hi?: number
  /** Any CSS colour. Defaults to --chart-1..5 by the group's position in its row. */
  color?: string
  /** Diagonal hatch on top of the fill, so held-out data differs from training data without colour. */
  hatched?: boolean
}

export interface RangeRow {
  id: string
  label: string
  groups: RangeGroup[]
}

export interface RangeBarsProps {
  rows: RangeRow[]
  /** Accessible name. */
  name: string
  /** Value axis range. Defaults to nice ticks from zero. */
  domain?: [number, number]
  format?: (v: number) => string
  barHeight?: number
  className?: string
}

const fmtNum = (v: number) => String(+v.toFixed(3))
const GROUP_GAP = 5
const ROW_GAP = 26
const PAD_T = 8
const VALUE_W = 64

export function rangeTable(rows: RangeRow[], format: (v: number) => string = fmtNum, caption?: string): TableData {
  const groups: { id: string; label: string }[] = []
  for (const r of rows) for (const g of r.groups) if (!groups.some((x) => x.id === g.id)) groups.push({ id: g.id, label: g.label })
  return {
    caption,
    columns: ["Metric", ...groups.map((g) => g.label)],
    rows: rows.map((r) => [
      r.label,
      ...groups.map((g) => {
        const m = r.groups.find((x) => x.id === g.id)
        if (!m) return "—"
        return m.lo !== undefined && m.hi !== undefined ? `${format(m.value)} (${format(m.lo)}–${format(m.hi)})` : format(m.value)
      }),
    ]),
  }
}

function LegendSwatch({ color, hatched, patternId }: { color: string; hatched?: boolean; patternId: string }) {
  return (
    <svg width="18" height="12" viewBox="0 0 18 12" aria-hidden="true">
      <rect x="0.75" y="0.75" width="16.5" height="10.5" rx="3" fill={color} stroke={outlineOf(color)} strokeWidth={1.5} />
      {hatched ? <rect x="0.75" y="0.75" width="16.5" height="10.5" rx="3" fill={`url(#${patternId})`} /> : null}
    </svg>
  )
}

export function RangeBars({ rows, name, domain, format = fmtNum, barHeight = 14, className }: RangeBarsProps) {
  const uid = useId()
  const hatchId = `${uid}-hatch`
  const [wrapRef, width] = useElementWidth<HTMLDivElement>(640)
  const [tip, setTip] = useState<{ row: number; x: number; y: number } | null>(null)
  const [focusRing, setFocusRing] = useState<number | null>(null)
  const [lastRow, setLastRow] = useState(0)

  const vals = rows.flatMap((r) => r.groups.flatMap((g) => [g.value, g.lo, g.hi])).filter((v): v is number => typeof v === "number" && Number.isFinite(v))
  const nice = niceTicks(Math.min(0, ...vals), Math.max(...vals, 0), 5)
  const [d0, d1] = domain ?? [nice.min, nice.max]
  const ticks = niceTicks(d0, d1, 5).ticks.filter((t) => t >= d0 - 1e-9 && t <= d1 + 1e-9)

  const labelW = Math.min(width * 0.34, Math.max(6, ...rows.map((r) => r.label.length)) * MONO_CHAR_W + 18)
  const plotL = labelW
  const plotR = Math.max(plotL + 60, width - VALUE_W)
  const x = linearScale(d0, d1, plotL, plotR)
  const maxGroups = Math.max(1, ...rows.map((r) => r.groups.length))
  const rowH = maxGroups * barHeight + (maxGroups - 1) * GROUP_GAP
  const pitch = rowH + ROW_GAP
  const plotB = PAD_T + rows.length * pitch - ROW_GAP + 10
  const height = plotB + 26

  const legend: RangeGroup[] = []
  for (const r of rows) for (const g of r.groups) if (!legend.some((l) => l.id === g.id)) legend.push(g)
  const legendColor = (g: RangeGroup) => g.color ?? seriesColor(Math.max(0, rows.find((r) => r.groups.some((x) => x.id === g.id))?.groups.findIndex((x) => x.id === g.id) ?? 0))

  const active = tip?.row ?? null
  const shownRow = rows[tip?.row ?? lastRow]

  return (
    <div className={cn("min-w-0", className)}>
      <ul className="text-ink-2 mb-3 flex flex-wrap gap-x-5 gap-y-1 font-mono text-[11px]" aria-label="Legend">
        {legend.map((g) => (
          <li key={g.id} className="inline-flex items-center gap-2">
            <LegendSwatch color={legendColor(g)} hatched={g.hatched} patternId={hatchId} />
            {g.label}
          </li>
        ))}
      </ul>
      <div ref={wrapRef} role="group" aria-label={name} className="relative select-none" style={{ height }}>
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="block overflow-visible">
          <defs>
            <pattern id={hatchId} width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line x1="0" y1="0" x2="0" y2="5" stroke="var(--card)" strokeWidth="2" opacity="0.55" />
            </pattern>
          </defs>
          <g aria-hidden="true">
            {ticks.map((t) => (
              <g key={t}>
                <line x1={x(t)} x2={x(t)} y1={PAD_T - 4} y2={plotB} stroke={t === d0 ? "var(--hairline-strong)" : "var(--hairline)"} strokeWidth={1} />
                <text x={x(t)} y={plotB + 18} textAnchor="middle" className="fill-ink-3 font-mono text-[11px] tabular-nums">
                  {format(t)}
                </text>
              </g>
            ))}
          </g>
          {rows.map((r, ri) => {
            const y0 = PAD_T + ri * pitch
            const h = r.groups.length * barHeight + (r.groups.length - 1) * GROUP_GAP
            const dim = active !== null && active !== ri
            const label = `${r.label}: ${r.groups.map((g) => `${g.label} ${format(g.value)}${g.lo !== undefined && g.hi !== undefined ? ` (interval ${format(g.lo)} to ${format(g.hi)})` : ""}`).join(", ")}`
            return (
              <g
                key={r.id}
                role="img"
                tabIndex={0}
                aria-label={label}
                className="chart-focusable"
                style={{ opacity: dim ? 0.35 : 1, transition: `opacity var(--dur-base) ${EASE_SOFT}` }}
                onPointerMove={(e) => {
                  const rect = wrapRef.current?.getBoundingClientRect()
                  if (!rect) return
                  setLastRow(ri)
                  setTip({ row: ri, x: e.clientX - rect.left, y: y0 - 6 })
                }}
                onPointerLeave={() => setTip(null)}
                onFocus={(e) => {
                  setLastRow(ri)
                  setTip({ row: ri, x: plotL + 24, y: y0 - 6 })
                  setFocusRing(e.currentTarget.matches(":focus-visible") ? ri : null)
                }}
                onBlur={() => {
                  setTip(null)
                  setFocusRing(null)
                }}
              >
                <rect x={0} y={y0 - ROW_GAP / 2 + 2} width={width} height={h + ROW_GAP - 4} rx={8} fill="transparent" stroke={focusRing === ri ? "var(--world)" : "none"} strokeWidth={2} />
                <text x={2} y={y0 + h / 2} dominantBaseline="central" className="fill-foreground font-mono text-[12px]">
                  {truncateLabel(r.label, Math.floor((labelW - 10) / MONO_CHAR_W))}
                </text>
                {r.groups.map((g, gi) => {
                  const color = g.color ?? seriesColor(gi)
                  const y = y0 + gi * (barHeight + GROUP_GAP)
                  const yc = y + barHeight / 2
                  const w = Math.max(0, x(g.value) - plotL)
                  const delay = `${(ri * 2 + gi) * 60}ms`
                  const hasCi = g.lo !== undefined && g.hi !== undefined
                  const end = hasCi ? Math.max(x(g.hi!), x(g.value)) : x(g.value)
                  return (
                    <g key={g.id}>
                      <g className="chart-grow-x" style={{ animationDelay: delay }}>
                        <rect x={plotL} y={y} width={w} height={barHeight} rx={3.5} fill={color} stroke={outlineOf(color)} strokeWidth={1.5} vectorEffect="non-scaling-stroke" />
                        {g.hatched ? <rect x={plotL} y={y} width={w} height={barHeight} rx={3.5} fill={`url(#${hatchId})`} /> : null}
                      </g>
                      <g className="chart-fade" style={{ animationDelay: `${(ri * 2 + gi) * 60 + 380}ms` }}>
                        {hasCi ? (
                          <g>
                            <path d={`M${x(g.lo!)} ${yc}H${x(g.hi!)}M${x(g.lo!)} ${yc - 4}V${yc + 4}M${x(g.hi!)} ${yc - 4}V${yc + 4}`} stroke="var(--card)" strokeWidth={4.5} strokeLinecap="round" fill="none" />
                            <path d={`M${x(g.lo!)} ${yc}H${x(g.hi!)}M${x(g.lo!)} ${yc - 4}V${yc + 4}M${x(g.hi!)} ${yc - 4}V${yc + 4}`} stroke="var(--foreground)" strokeWidth={1.5} strokeLinecap="round" fill="none" />
                          </g>
                        ) : null}
                        <text x={end + 9} y={yc} dominantBaseline="central" className="fill-foreground font-mono text-[11px] tabular-nums">
                          {format(g.value)}
                        </text>
                      </g>
                    </g>
                  )
                })}
              </g>
            )
          })}
        </svg>
        <ChartTooltip x={tip?.x ?? 0} y={tip?.y ?? 0} containerW={width} containerH={height} visible={tip !== null} placement="above">
          {shownRow ? (
            <>
              <div className="text-ink-3 mb-1 tracking-[0.12em] uppercase">{shownRow.label}</div>
              {shownRow.groups.map((g, gi) => (
                <div key={g.id} className="flex items-center gap-2 py-px">
                  <LegendSwatch color={g.color ?? seriesColor(gi)} hatched={g.hatched} patternId={hatchId} />
                  <span className="text-ink-2">{g.label}</span>
                  <span className="text-foreground ml-auto pl-4 whitespace-nowrap tabular-nums">
                    {format(g.value)}
                    {g.lo !== undefined && g.hi !== undefined ? (
                      <span className="text-ink-3">
                        {" "}
                        {format(g.lo)}&ndash;{format(g.hi)}
                      </span>
                    ) : null}
                  </span>
                </div>
              ))}
            </>
          ) : null}
        </ChartTooltip>
      </div>
    </div>
  )
}
