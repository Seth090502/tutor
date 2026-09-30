import { EASE_SOFT, useReducedMotion } from "./_motion"
import { useArmed } from "./_hooks"
import { areaPath, linePath, sparklinePoints } from "./geometry"
import { seriesColor, washOf } from "./palette"
import type { TableData } from "./types"

export interface SparklineProps {
  values: number[]
  /** Accessible name, e.g. "Reviews per day". */
  name: string
  width?: number
  height?: number
  /** Any CSS colour. Defaults to the world accent (--chart-1). */
  color?: string
  /** 10% wash under the line. */
  area?: boolean
  drawOn?: boolean
  className?: string
}

const fmt = (v: number) => String(+v.toFixed(2))

export function sparklineTable(values: number[], name = "Value", labels?: string[]): TableData {
  return { caption: name, columns: ["Point", name], rows: values.map((v, i) => [labels?.[i] ?? String(i + 1), fmt(v)]) }
}

/** Tiny trend line with an end dot. No axes; the accessible name carries the latest, low and high values. */
export function Sparkline({ values, name, width = 96, height = 28, color = seriesColor(0), area = false, drawOn = true, className }: SparklineProps) {
  const reduced = useReducedMotion()
  const armed = useArmed(drawOn)
  const drawn = armed || reduced || !drawOn
  const ms = reduced || !drawOn ? 0 : 900
  const pts = sparklinePoints(values, width, height, 3)
  const last = pts[pts.length - 1]
  const label = values.length
    ? `${name}: latest ${fmt(values[values.length - 1])}, low ${fmt(Math.min(...values))}, high ${fmt(Math.max(...values))}, ${values.length} points`
    : `${name}: no data`
  return (
    <svg role="img" aria-label={label} width={width} height={height} viewBox={`0 0 ${width} ${height}`} className={className}>
      {area && pts.length > 1 ? <path d={areaPath(pts, height - 3)} fill={washOf(color, 10)} style={{ opacity: drawn ? 1 : 0, transition: ms ? `opacity ${ms * 0.6}ms ${EASE_SOFT} ${ms * 0.5}ms` : "none" }} /> : null}
      {pts.length > 1 ? (
        <path
          d={linePath(pts)}
          fill="none"
          stroke={color}
          strokeWidth={2}
          strokeLinejoin="round"
          strokeLinecap="round"
          pathLength={1}
          strokeDasharray="1 2"
          strokeDashoffset={drawn ? 0 : 1}
          style={{ transition: ms ? `stroke-dashoffset ${ms}ms ${EASE_SOFT}` : "none" }}
        />
      ) : null}
      {last ? (
        <g style={{ opacity: drawn ? 1 : 0, transition: ms ? `opacity ${ms * 0.4}ms ${EASE_SOFT} ${ms * 0.8}ms` : "none" }}>
          <circle cx={last.x} cy={last.y} r={4.5} fill="var(--card)" />
          <circle cx={last.x} cy={last.y} r={2.75} fill={color} />
        </g>
      ) : null}
    </svg>
  )
}
