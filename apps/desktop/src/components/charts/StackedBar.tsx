import { useState } from "react"

import { cn } from "@/lib/utils"
import { EASE_SOFT } from "./_motion"
import { useElementWidth } from "./_hooks"
import { percentages, stackLayout } from "./geometry"
import { outlineOf, seriesColor } from "./palette"
import type { TableData } from "./types"
import "./charts.css"

export interface StackSegment {
  id: string
  label: string
  value: number
  /** Any CSS colour. Defaults to --chart-1..5 by position. */
  color?: string
}

export interface StackedBarProps {
  segments: StackSegment[]
  /** Accessible name. */
  name: string
  /** What the values count, e.g. "pages". Shown in the legend and the table. */
  unit?: string
  height?: number
  format?: (v: number) => string
  className?: string
}

const fmtInt = (v: number) => v.toLocaleString("en-US")

export function stackTable(segments: StackSegment[], unit = "Count", format: (v: number) => string = fmtInt, caption?: string): TableData {
  const pct = percentages(segments.map((s) => s.value), 1)
  return {
    caption,
    columns: ["Segment", unit.charAt(0).toUpperCase() + unit.slice(1), "Share"],
    rows: segments.map((s, i) => [s.label, format(s.value), `${pct[i].toFixed(1)}%`]),
  }
}

export function StackedBar({ segments, name, unit, height = 32, format = fmtInt, className }: StackedBarProps) {
  const [wrapRef, width] = useElementWidth<HTMLDivElement>(640)
  const [hover, setHover] = useState<string | null>(null)
  const layout = stackLayout(segments.map((s) => s.value), width, 2)
  const pct = percentages(segments.map((s) => s.value), 0)
  const total = segments.reduce((a, s) => a + Math.max(0, s.value), 0)
  const dim = (id: string) => hover !== null && hover !== id
  const on = (id: string) => ({ onPointerEnter: () => setHover(id), onPointerLeave: () => setHover(null), onFocus: () => setHover(id), onBlur: () => setHover(null) })

  return (
    <div className={cn("min-w-0", className)}>
      <div ref={wrapRef} role="group" aria-label={`${name}, ${format(total)}${unit ? ` ${unit}` : ""} in total`}>
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="block">
          {segments.map((s, i) => {
            const color = s.color ?? seriesColor(i)
            const l = layout[i]
            if (l.w <= 0) return null
            return (
              <g
                key={s.id}
                role="img"
                tabIndex={0}
                aria-label={`${s.label}: ${format(s.value)}${unit ? ` ${unit}` : ""}, ${pct[i]}%`}
                className="chart-focusable chart-grow-x"
                style={{
                  opacity: dim(s.id) ? 0.3 : 1,
                  transition: `opacity var(--dur-base) ${EASE_SOFT}`,
                  animationDelay: `${i * 70}ms`,
                }}
                {...on(s.id)}
              >
                <rect
                  x={l.x + 0.75}
                  y={0.75}
                  width={Math.max(0, l.w - 1.5)}
                  height={height - 1.5}
                  rx={5}
                  fill={color}
                  stroke={outlineOf(color)}
                  strokeWidth={1.5}
                  vectorEffect="non-scaling-stroke"
                />
              </g>
            )
          })}
        </svg>
      </div>
      <ul className="mt-4 grid gap-x-8 gap-y-1 [grid-template-columns:repeat(auto-fit,minmax(220px,1fr))]" aria-label="Legend">
        {segments.map((s, i) => {
          const color = s.color ?? seriesColor(i)
          return (
            <li
              key={s.id}
              className="border-hairline flex items-center gap-3 border-b py-2 text-[13px]"
              style={{ opacity: dim(s.id) ? 0.4 : 1, transition: `opacity var(--dur-base) ${EASE_SOFT}` }}
              onPointerEnter={() => setHover(s.id)}
              onPointerLeave={() => setHover(null)}
            >
              <span aria-hidden="true" className="inline-block h-3 w-3 shrink-0 rounded-[4px]" style={{ background: color, boxShadow: `inset 0 0 0 1.5px ${outlineOf(color)}` }} />
              <span className="text-foreground min-w-0 flex-1 truncate">{s.label}</span>
              <span className="text-foreground font-mono text-[12px] tabular-nums">{format(s.value)}</span>
              <span className="text-ink-3 w-10 text-right font-mono text-[11px] tabular-nums">{pct[i]}%</span>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
