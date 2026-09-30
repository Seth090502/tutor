import { useState, type CSSProperties } from "react"

import { cn } from "@/lib/utils"
import { phasePath } from "./geometry"
import type { TableData } from "./types"
import "./charts.css"

export interface PhaseDay {
  id: string
  /** Short label under the disc, e.g. the day of the month. */
  label: string
  /** Long form for the caption and the table, e.g. "Mon, Sep 14". */
  title: string
  /** Share of the daily goal, 0..1 (above 1 is capped). null for a day that has not happened yet. */
  value: number | null
  isToday?: boolean
}

export interface PhaseDiscsProps {
  days: PhaseDay[]
  /** Accessible name. */
  name: string
  columns?: number
  className?: string
}

const R = 15
const pct = (v: number) => `${Math.round(Math.min(1, Math.max(0, v)) * 100)}%`

export function phaseTable(days: PhaseDay[], caption?: string): TableData {
  return {
    caption,
    columns: ["Day", "Share of daily goal"],
    rows: days.map((d) => [d.isToday ? `${d.title} (today)` : d.title, d.value === null ? "—" : pct(d.value)]),
  }
}

function Disc({ value, today, className, style }: { value: number | null; today?: boolean; className?: string; style?: CSSProperties }) {
  return (
    <svg viewBox="-21 -21 42 42" aria-hidden="true" className={cn("block h-auto w-full", className)} style={style}>
      {value === null ? (
        <circle r={R} fill="none" stroke="var(--hairline-strong)" strokeWidth={1.25} strokeDasharray="2 3.2" />
      ) : (
        <>
          <circle r={R} fill="color-mix(in oklab, var(--ink) 11%, var(--card))" stroke="var(--hairline)" strokeWidth={1} />
          <path d={phasePath(value, R)} fill="var(--chart-1)" />
        </>
      )}
      {today ? <circle r={R + 4.5} fill="none" stroke="var(--foreground)" strokeWidth={1.5} /> : null}
    </svg>
  )
}

/** A grid of moon-phase discs: the lit share of each disc is the share of that day's goal; today wears a ring. */
export function PhaseDiscs({ days, name, columns = 10, className }: PhaseDiscsProps) {
  const [cur, setCur] = useState<number | null>(null)
  const shown = cur !== null ? days[cur] : undefined
  const describe = (d: PhaseDay) => `${d.title}${d.isToday ? " (today)" : ""}: ${d.value === null ? "not yet" : `${pct(d.value)} of the daily goal`}`
  return (
    <div className={cn("min-w-0", className)}>
      <ul
        role="list"
        aria-label={name}
        className="grid gap-x-1 gap-y-2"
        style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 52px))`, justifyContent: "space-between" }}
      >
        {days.map((d, i) => (
          <li
            key={d.id}
            tabIndex={0}
            aria-label={describe(d)}
            onPointerEnter={() => setCur(i)}
            onPointerLeave={() => setCur(null)}
            onFocus={() => setCur(i)}
            onBlur={() => setCur(null)}
            className="focus-visible:outline-world flex flex-col items-center gap-1 rounded-md focus-visible:outline-2 focus-visible:outline-offset-2"
          >
            <Disc value={d.value} today={d.isToday} className="chart-pop" style={{ animationDelay: `${i * 22}ms` }} />
            <span className={cn("font-mono text-[11px] tabular-nums", d.isToday ? "text-foreground font-semibold" : "text-ink-3")}>{d.label}</span>
          </li>
        ))}
      </ul>
      <div className="mt-3 flex flex-wrap items-center justify-between gap-x-6 gap-y-2">
        <p aria-hidden="true" className="text-ink-2 min-h-[18px] font-mono text-[11px] tabular-nums">
          {shown ? describe(shown) : " "}
        </p>
        <div className="text-ink-3 flex items-center gap-2 font-mono text-[11px]" aria-hidden="true">
          <span>0%</span>
          {[0, 0.25, 0.5, 0.75, 1].map((v) => (
            <span key={v} className="w-[18px]">
              <Disc value={v} />
            </span>
          ))}
          <span>100% of goal</span>
        </div>
      </div>
    </div>
  )
}
