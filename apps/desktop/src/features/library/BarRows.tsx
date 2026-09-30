// Labelled horizontal bars: label, count and share on one line, a thin flat bar under it.
import type { CSSProperties } from "react"

import { cn } from "@/lib/utils"
import { fmtInt, fmtPct } from "./format"
import { ratio, type Row } from "./logic"
import "./library.css"

/** Neutral bar fill for rows with no colour of their own. */
export const NEUTRAL_BAR = "color-mix(in oklab, var(--ink) 48%, var(--card))"

export function BarRows({
  rows,
  colorFor,
  empty,
  label,
  className,
}: {
  rows: Row[]
  /** Fill for a row; return undefined for the neutral fill. */
  colorFor?: (label: string) => string | undefined
  empty: string
  label: string
  className?: string
}) {
  if (rows.length === 0) return <p className="text-ink-2 text-[13px] leading-relaxed">{empty}</p>
  const total = rows.reduce((sum, r) => sum + r.value, 0)
  const max = Math.max(...rows.map((r) => r.value))
  return (
    <ul aria-label={label} className={cn("m-0 flex list-none flex-col p-0", className)}>
      {rows.map((r, i) => {
        const color = colorFor?.(r.label) ?? NEUTRAL_BAR
        return (
          <li key={r.label} className="border-hairline border-b py-3 first:pt-0 last:border-b-0 last:pb-0">
            <div className="flex items-baseline gap-3 text-[13px]">
              <span className="text-foreground min-w-0 flex-1 truncate capitalize">{r.label}</span>
              <span className="text-foreground font-mono text-[12px] tabular-nums">{fmtInt(r.value)}</span>
              <span className="text-ink-3 w-12 text-right font-mono text-[11px] tabular-nums">{fmtPct(r.value, total)}</span>
            </div>
            <div aria-hidden className="mt-2 h-2 overflow-hidden rounded-full" style={{ background: "color-mix(in oklab, var(--ink) 7%, transparent)" }}>
              <div
                className="lib-bar h-full rounded-full"
                style={
                  {
                    width: `${ratio(r.value, max) * 100}%`,
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
