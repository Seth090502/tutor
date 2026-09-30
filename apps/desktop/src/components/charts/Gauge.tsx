import type { ReactNode } from "react"

import { cn } from "@/lib/utils"
import { EASE_SOFT, EASE_SPRING } from "./_motion"
import { useArmed } from "./_hooks"
import { gaugeTicks, GAUGE_TICKS, needleAngle } from "./geometry"
import type { TableData } from "./types"

export type GaugeTone = "world" | "ok" | "warn" | "bad"

export interface GaugeProps {
  value: number
  min?: number
  max: number
  /** Mono caps label under the number, e.g. "Due today". */
  label: string
  /** Accessible name. Defaults to the label. */
  name?: string
  /** The big number: text, or a node such as a CountUp. Defaults to the value. */
  valueLabel?: ReactNode
  /** The number as text, for the meter's spoken value. Defaults to the value; set it when `valueLabel` is a node. */
  valueText?: string
  /** Colours the needle and lit ticks. The number and label carry the meaning too. */
  tone?: GaugeTone
  className?: string
}

const TONE: Record<GaugeTone, string> = { world: "var(--world)", ok: "var(--ok)", warn: "var(--warn)", bad: "var(--bad)" }

const W = 240
const H = 128
const CX = W / 2
const CY = 112
const R_OUT = 100
const R_IN = 88

export function gaugeTable(g: Pick<GaugeProps, "value" | "min" | "max" | "label">): TableData {
  return { caption: g.label, columns: ["Measure", "Value", "Min", "Max"], rows: [[g.label, g.value, g.min ?? 0, g.max]] }
}

/** Watch-style power-reserve gauge: 21 ticks over 124 degrees and a needle that overshoots a little before it settles. */
export function Gauge({ value, min = 0, max, label, name, valueLabel, valueText, tone = "world", className }: GaugeProps) {
  const armed = useArmed(true)
  const color = TONE[tone]
  const ticks = gaugeTicks(CX, CY, R_IN, R_OUT)
  const target = needleAngle(value, min, max)
  const t = max === min ? 0 : Math.min(1, Math.max(0, (value - min) / (max - min)))
  const litUpTo = Math.round(t * (GAUGE_TICKS - 1))
  const angle = armed ? target : needleAngle(min, min, max)
  const text = valueLabel ?? String(value)
  const spoken = valueText ?? (typeof valueLabel === "string" ? valueLabel : String(value))
  return (
    <div
      role="meter"
      aria-label={name ?? label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-valuetext={`${spoken} ${label}`}
      className={cn("inline-flex flex-col items-center", className)}
    >
      <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} aria-hidden="true" className="block max-w-full overflow-visible">
        {ticks.map((k) => (
          <line key={k.i} x1={k.x1} y1={k.y1} x2={k.x2} y2={k.y2} stroke="var(--hairline-strong)" strokeWidth={k.major ? 2 : 1.5} strokeLinecap="round" />
        ))}
        {ticks.map((k) => (
          <line
            key={`lit-${k.i}`}
            x1={k.x1}
            y1={k.y1}
            x2={k.x2}
            y2={k.y2}
            stroke={color}
            strokeWidth={k.major ? 2 : 1.5}
            strokeLinecap="round"
            style={{
              opacity: armed && k.i <= litUpTo ? 1 : 0,
              transition: `opacity var(--dur-base) ${EASE_SOFT} calc(${k.i} * 28ms)`,
            }}
          />
        ))}
        <text x={ticks[0].x1 - 2} y={ticks[0].y1 + 18} textAnchor="middle" className="fill-ink-3 font-mono text-[11px] tabular-nums">
          {min}
        </text>
        <text x={ticks[GAUGE_TICKS - 1].x1 + 2} y={ticks[GAUGE_TICKS - 1].y1 + 18} textAnchor="middle" className="fill-ink-3 font-mono text-[11px] tabular-nums">
          {max}
        </text>
        <g style={{ transform: `rotate(${angle}deg)`, transformOrigin: `${CX}px ${CY}px`, transition: `transform calc(var(--dur-slow) * 2) ${EASE_SPRING}` }}>
          <path d={`M${CX} ${CY + 14}L${CX} ${CY - (R_IN - 6)}`} stroke={color} strokeWidth={2.5} strokeLinecap="round" fill="none" />
          <circle cx={CX} cy={CY} r={6} fill="var(--card)" stroke={color} strokeWidth={2.5} />
          <circle cx={CX} cy={CY} r={1.75} fill={color} />
        </g>
      </svg>
      <div className="-mt-1 text-center">
        <div className="text-foreground text-[44px] leading-none font-extralight tracking-[-0.04em] tabular-nums">{text}</div>
        <div className="text-ink-3 mt-2 font-mono text-[11px] tracking-[0.18em] uppercase">{label}</div>
      </div>
    </div>
  )
}
