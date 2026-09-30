import type { CSSProperties, ReactNode } from "react"

import { cn } from "@/lib/utils"
import { barRatio, barTotals, shareText, type BarRow } from "./bar-list-logic"
import { formatNumber } from "./CountUp"
import "./kit.css"

export type { BarRow }

/** Neutral bar fill for rows with no colour of their own. */
export const NEUTRAL_BAR = "color-mix(in oklab, var(--ink) 48%, var(--card))"

type BarListProps = {
  rows: BarRow[]
  /** Accessible name of the list. */
  label: string
  /** Shown instead of the list when there are no rows. */
  empty: ReactNode
  /** Capitalise each label (for lower-case keys such as "math"). */
  capitalize?: boolean
  format?: (value: number) => string
  className?: string
}

/**
 * Labelled horizontal bars: label, count and share of the total on one line, a thin flat bar under it
 * that grows in once, staggered by row. The longest bar fills the track.
 */
export function BarList({ rows, label, empty, capitalize = false, format = formatNumber, className }: BarListProps) {
  if (rows.length === 0) return <p className="text-ink-2 text-[13px] leading-relaxed">{empty}</p>
  const { total, max } = barTotals(rows)
  return (
    <ul data-slot="kit-bar-list" aria-label={label} className={cn("m-0 flex list-none flex-col p-0", className)}>
      {rows.map((r, i) => {
        const color = r.color ?? NEUTRAL_BAR
        return (
          <li key={r.label} className="border-hairline border-b py-3 first:pt-0 last:border-b-0 last:pb-0">
            <div className="flex items-baseline gap-3 text-[13px]">
              <span className={cn("text-foreground min-w-0 flex-1 truncate", capitalize && "capitalize")}>{r.label}</span>
              <span className="text-foreground font-mono text-[12px] tabular-nums">{format(r.value)}</span>
              <span className="text-ink-3 w-12 text-right font-mono text-[11px] tabular-nums">{shareText(r.value, total)}</span>
            </div>
            <div aria-hidden className="mt-2 h-2 overflow-hidden rounded-full" style={{ background: "color-mix(in oklab, var(--ink) 7%, transparent)" }}>
              <div
                className="kit-bar h-full rounded-full"
                style={
                  {
                    width: `${barRatio(r.value, max) * 100}%`,
                    background: color,
                    boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${color} 58%, black)`,
                    "--i": i,
                  } as CSSProperties
                }
              />
            </div>
          </li>
        )
      })}
    </ul>
  )
}
