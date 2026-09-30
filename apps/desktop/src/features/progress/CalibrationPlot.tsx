// Calibration plot: confidence level (1 to 4, evenly spaced) against percent correct, with the
// reference diagonal. The chart kit's line chart picks its own x ticks and skips levels here, so this
// small plot draws the four categories directly. Drawn once; no motion beyond a fade.
import { useState } from "react"

import { cn } from "@/lib/utils"
import { SMALL_SAMPLE, expectedPct, type CalibrationRow } from "./model"
import { useWidth } from "./parts"

const H = 240
const M = { l: 44, r: 28, t: 18, b: 40 }
const TICKS = [0, 25, 50, 75, 100]

export function CalibrationPlot({ rows, name }: { rows: CalibrationRow[]; name: string }) {
  const [ref, width] = useWidth<HTMLDivElement>(480)
  const [focus, setFocus] = useState<number | null>(null)
  const plotL = M.l
  const plotR = Math.max(plotL + 120, width - M.r)
  const plotT = M.t
  const plotB = H - M.b
  const pad = Math.min(48, (plotR - plotL) / 8)
  const x = (level: number) => plotL + pad + ((level - 1) / 3) * (plotR - plotL - 2 * pad)
  const y = (pct: number) => plotB - (pct / 100) * (plotB - plotT)
  const pts = rows.flatMap((r) => (r.pct === null ? [] : [{ r, cx: x(r.level), cy: y(r.pct) }]))
  const line = pts.map((p, i) => `${i ? "L" : "M"}${p.cx} ${p.cy}`).join("")
  const compact = width < 400

  return (
    <div ref={ref} className="min-w-0">
      <svg width={width} height={H} viewBox={`0 0 ${width} ${H}`} role="group" aria-label={name} className="block overflow-visible">
        <g aria-hidden="true">
          {TICKS.map((t) => (
            <g key={t}>
              <line x1={plotL} x2={plotR} y1={y(t)} y2={y(t)} stroke={t === 0 ? "var(--hairline-strong)" : "var(--hairline)"} />
              <text x={plotL - 8} y={y(t)} textAnchor="end" dominantBaseline="central" className="fill-ink-3 font-mono text-[11px] tabular-nums">
                {t}%
              </text>
            </g>
          ))}
          {rows.map((r) => (
            <text key={r.level} x={x(r.level)} y={plotB + 18} textAnchor="middle" className="fill-ink-3 font-mono text-[11px]">
              {compact ? r.level : `${r.level} ${r.name}`}
            </text>
          ))}
          <path d={[1, 2, 3, 4].map((l, i) => `${i ? "L" : "M"}${x(l)} ${y(expectedPct(l))}`).join("")} stroke="var(--ink-3)" strokeWidth={2} strokeDasharray="5 4" fill="none" />
          <text x={x(1) + 10} y={y(expectedPct(1)) + 6} textAnchor="start" className="fill-ink-3 font-mono text-[11px]">
            Reference
          </text>
          {line ? <path d={line} stroke="var(--chart-1)" strokeWidth={2} fill="none" strokeLinejoin="round" strokeLinecap="round" /> : null}
        </g>
        {pts.map(({ r, cx, cy }) => {
          const small = r.attempts < SMALL_SAMPLE
          return (
            <g
              key={r.level}
              tabIndex={0}
              role="img"
              aria-label={`Confidence ${r.level}, ${r.name}: ${r.pct}% correct, ${r.correct} of ${r.attempts}${small ? ", small sample" : ""}. Reference ${expectedPct(r.level)}%.`}
              className="chart-focusable"
              onFocus={(e) => e.currentTarget.matches(":focus-visible") && setFocus(r.level)}
              onBlur={() => setFocus(null)}
            >
              <title>{`${r.correct} of ${r.attempts} right${small ? " (small sample)" : ""}`}</title>
              <circle cx={cx} cy={cy} r={14} fill="transparent" stroke={focus === r.level ? "var(--world)" : "none"} strokeWidth={2} />
              <circle cx={cx} cy={cy} r={5.5} fill={small ? "var(--card)" : "var(--chart-1)"} stroke="var(--chart-1)" strokeWidth={2} strokeDasharray={small ? "2.2 2" : undefined} />
              <text x={cx} y={cy - 14} textAnchor="middle" className="fill-foreground font-mono text-[11px] tabular-nums" aria-hidden="true">
                {r.pct}%
              </text>
            </g>
          )
        })}
      </svg>
      <p className={cn("text-ink-3 mt-1 flex items-center gap-2 font-mono text-[11px]")} aria-hidden="true">
        <svg width="12" height="12" viewBox="-6 -6 12 12">
          <circle r={4} fill="var(--card)" stroke="var(--chart-1)" strokeWidth={2} strokeDasharray="2 1.6" />
        </svg>
        Dashed ring: fewer than {SMALL_SAMPLE} attempts
      </p>
    </div>
  )
}
