import { useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react"

import { cn } from "@/lib/utils"
import { Eyebrow } from "../kit/Eyebrow"
import { HEADER, SURFACE } from "../kit/surface"
import { EASE_SPRING } from "./_motion"
import type { TableData } from "./types"
import "./charts.css"

type View = "chart" | "table"

const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

function ViewToggle({ view, onChange, label }: { view: View; onChange: (v: View) => void; label: string }) {
  const refs = useRef<Record<View, HTMLButtonElement | null>>({ chart: null, table: null })
  const views: View[] = ["chart", "table"]
  const onKey = (e: KeyboardEvent) => {
    let next: View | null = null
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next = view === "chart" ? "table" : "chart"
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = view === "table" ? "chart" : "table"
    else if (e.key === "Home") next = "chart"
    else if (e.key === "End") next = "table"
    if (!next) return
    e.preventDefault()
    onChange(next)
    refs.current[next]?.focus()
  }
  return (
    <div
      role="radiogroup"
      aria-label={label}
      onKeyDown={onKey}
      className="bg-muted border-hairline relative inline-grid shrink-0 grid-cols-2 rounded-full border p-0.5"
    >
      <span
        aria-hidden="true"
        className="bg-card border-hairline-strong absolute inset-y-0.5 left-0.5 w-[calc(50%-2px)] rounded-full border shadow-[0_1px_0_color-mix(in_oklab,var(--ink)_8%,transparent)]"
        style={{
          transform: view === "chart" ? "translateX(0)" : "translateX(100%)",
          transition: `transform var(--dur-slow) ${EASE_SPRING}`,
        }}
      />
      {views.map((v) => (
        <button
          key={v}
          ref={(el) => {
            refs.current[v] = el
          }}
          type="button"
          role="radio"
          aria-checked={view === v}
          tabIndex={view === v ? 0 : -1}
          onClick={() => onChange(v)}
          className={cn(
            "relative min-w-[68px] rounded-full px-3 py-1 font-mono text-[11px] tracking-[0.08em] uppercase transition-colors",
            FOCUS,
            view === v ? "text-foreground" : "text-ink-3 hover:text-foreground",
          )}
          style={{ transitionDuration: "var(--dur-base)" }}
        >
          {v === "chart" ? "Chart" : "Table"}
        </button>
      ))}
    </div>
  )
}

export function DataTable({ table, label }: { table: TableData; label: string }) {
  const align = (i: number) => table.align?.[i] ?? (i === 0 ? "left" : "right")
  return (
    <div
      role="region"
      aria-label={`${label} data table`}
      tabIndex={0}
      className={cn("max-h-[420px] overflow-auto rounded-md", FOCUS)}
    >
      <table className="w-full border-collapse text-[13px]">
        <caption className="sr-only">{table.caption ?? label}</caption>
        <thead>
          <tr>
            {table.columns.map((c, i) => (
              <th
                key={c + i}
                scope="col"
                className={cn(
                  "bg-card border-hairline-strong text-ink-3 sticky top-0 border-b px-3 py-2 font-mono text-[11px] font-normal tracking-[0.12em] whitespace-nowrap uppercase",
                  align(i) === "right" ? "text-right" : "text-left",
                )}
              >
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {table.rows.map((row, r) => (
            <tr key={r} className="hover:bg-accent transition-colors" style={{ transitionDuration: "var(--dur-quick)" }}>
              {row.map((cell, i) =>
                i === 0 ? (
                  <th key={i} scope="row" className="border-hairline text-foreground border-b px-3 py-2 text-left font-normal">
                    {cell}
                  </th>
                ) : (
                  <td
                    key={i}
                    className={cn(
                      "border-hairline text-foreground border-b px-3 py-2 font-mono text-[12px] tabular-nums",
                      align(i) === "right" ? "text-right" : "text-left",
                    )}
                  >
                    {cell}
                  </td>
                ),
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

export interface ChartFrameProps {
  title: string
  subtitle?: string
  /** The same data as the chart, rendered as a real table in the Table view. */
  table: TableData
  children: ReactNode
  defaultView?: View
  /** Small note under the chart, e.g. what the band means. */
  footer?: ReactNode
  className?: string
}

/** Card around a chart with a Chart / Table toggle. Both views stay mounted so the chart keeps its state. */
export function ChartFrame({ title, subtitle, table, children, defaultView = "chart", footer, className }: ChartFrameProps) {
  const [view, setView] = useState<View>(defaultView)
  const id = useId()
  return (
    <section aria-labelledby={`${id}-t`} className={cn(SURFACE, "min-w-0", className)}>
      <header className={HEADER}>
        <div className="min-w-0">
          <Eyebrow as="h3" rule id={`${id}-t`} className="flex">
            {title}
          </Eyebrow>
          {subtitle ? <p className="text-ink-2 mt-2 max-w-[60ch] text-sm leading-snug text-pretty">{subtitle}</p> : null}
        </div>
        <ViewToggle view={view} onChange={setView} label={`${title} view`} />
      </header>
      <div hidden={view !== "chart"} className="chart-rise">
        {children}
      </div>
      <div hidden={view !== "table"} className="chart-rise">
        <DataTable table={table} label={title} />
      </div>
      {footer && view === "chart" ? <div className="text-ink-3 mt-3 font-mono text-[11px] leading-snug">{footer}</div> : null}
    </section>
  )
}
