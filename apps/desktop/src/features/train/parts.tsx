// Small visual pieces of the Train screen: the 3x3 grid, transfer shapes, keycaps, metrics table.
import type { ReactNode } from "react"

import { cn } from "@/lib/utils"
import type { Metrics } from "./types"

/** Grid cell (0-8, row-major) of a position (0-7): the centre cell is never used. */
export const cellOf = (position: number) => (position < 4 ? position : position + 1)

/** A glyph drawn inside the lit cell so position never depends on colour alone. */
function CellMark() {
  return (
    <svg viewBox="0 0 100 100" className="size-1/2" aria-hidden>
      <rect x="18" y="18" width="64" height="64" rx="10" fill="currentColor" />
    </svg>
  )
}

export function Grid({ active, dim }: { active: number | null; dim?: boolean }) {
  const lit = active === null ? -1 : cellOf(active)
  return (
    <div
      className={cn("grid aspect-square w-full max-w-[min(22rem,60vh)] grid-cols-3 grid-rows-3 gap-2", dim && "opacity-50")}
      role="img"
      aria-label={active === null ? "Grid, no square lit" : `Grid, square ${active + 1} lit`}
    >
      {Array.from({ length: 9 }, (_, g) => (
        <div
          key={g}
          className={cn(
            "flex items-center justify-center rounded-lg border",
            g === 4 ? "border-transparent" : "border-hairline",
            g === lit ? "bg-world text-world-ink border-world" : g !== 4 && "bg-muted/40",
          )}
        >
          {g === lit && <CellMark />}
          {g === 4 && <span className="bg-ink-3 size-1.5 rounded-full" aria-hidden />}
        </div>
      ))}
    </div>
  )
}

const SHAPES: { name: string; el: ReactNode }[] = [
  { name: "circle", el: <circle cx="50" cy="50" r="38" /> },
  { name: "square", el: <rect x="14" y="14" width="72" height="72" /> },
  { name: "triangle", el: <polygon points="50,10 90,86 10,86" /> },
  { name: "diamond", el: <polygon points="50,6 94,50 50,94 6,50" /> },
  {
    name: "star",
    el: <polygon points="50,6 61,38 95,38 67,58 78,92 50,71 22,92 33,58 5,38 39,38" />,
  },
  { name: "hexagon", el: <polygon points="28,10 72,10 94,50 72,90 28,90 6,50" /> },
  { name: "plus", el: <polygon points="36,8 64,8 64,36 92,36 92,64 64,64 64,92 36,92 36,64 8,64 8,36 36,36" /> },
  { name: "chevron", el: <polygon points="10,20 50,50 90,20 90,52 50,82 10,52" /> },
]

/** Transfer-mode visual stimulus: one of 8 shapes in a single frame. */
export function ShapeStage({ shape }: { shape: number | null }) {
  return (
    <div
      className="border-hairline bg-muted/40 flex aspect-square w-full max-w-[min(22rem,60vh)] items-center justify-center rounded-xl border"
      role="img"
      aria-label={shape === null ? "No shape" : SHAPES[shape].name}
    >
      {shape === null ? (
        <span className="bg-ink-3 size-1.5 rounded-full" aria-hidden />
      ) : (
        <svg viewBox="0 0 100 100" className="text-world size-1/2" aria-hidden>
          <g fill="currentColor">{SHAPES[shape].el}</g>
        </svg>
      )}
    </div>
  )
}

export const keyLabel = (code: string) => code.replace(/^(Key|Digit)/, "").replace(/^Arrow/, "Arrow ")

export function Keycap({
  code,
  label,
  pressed,
  mark,
  onPress,
}: {
  code: string
  label: string
  pressed: boolean
  mark: boolean | null // in-trial feedback: correct, wrong, or none
  onPress: (timeStamp: number) => void
}) {
  return (
    <button
      type="button"
      onPointerDown={(e) => {
        e.preventDefault()
        onPress(e.timeStamp)
      }}
      className={cn(
        "border-hairline-strong bg-card flex min-w-36 flex-col items-center gap-1 rounded-xl border border-b-4 px-5 py-3 transition-transform select-none",
        pressed && "translate-y-0.5 border-b-2",
        mark === true && "border-ok",
        mark === false && "border-bad",
      )}
    >
      <span className="font-mono text-2xl leading-none font-semibold">{keyLabel(code)}</span>
      <span className="text-ink-2 text-xs">
        {label}
        {mark === true && <span className="text-ok"> · correct</span>}
        {mark === false && <span className="text-bad"> · wrong</span>}
      </span>
    </button>
  )
}

const fmt2 = (x: number) => (x >= 0 ? " " : "") + x.toFixed(2)

export function MetricsTable({ rows }: { rows: { label: string; m: Metrics | null }[] }) {
  const shown = rows.filter((r): r is { label: string; m: Metrics } => r.m !== null)
  if (shown.length === 0) return null
  return (
    <table className="w-full text-sm">
      <thead className="text-ink-3 text-left text-xs">
        <tr>
          <th className="py-1 pr-3 font-medium">Stream</th>
          <th className="px-2 text-right font-medium">Hits</th>
          <th className="px-2 text-right font-medium">Misses</th>
          <th className="px-2 text-right font-medium">False alarms</th>
          <th className="px-2 text-right font-medium">Correct rejections</th>
          <th className="px-2 text-right font-medium">d′</th>
          <th className="pl-2 text-right font-medium">c</th>
        </tr>
      </thead>
      <tbody className="font-mono tabular-nums">
        {shown.map(({ label, m }) => (
          <tr key={label} className="border-hairline border-t">
            <td className="py-1 pr-3 font-sans">{label}</td>
            <td className="px-2 text-right">{m.hits}</td>
            <td className="px-2 text-right">{m.misses}</td>
            <td className="px-2 text-right">{m.false_alarms}</td>
            <td className="px-2 text-right">{m.correct_rejections}</td>
            <td className="px-2 text-right whitespace-pre">{fmt2(m.d_prime)}</td>
            <td className="pl-2 text-right whitespace-pre">{fmt2(m.c)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

export function Section({ title, children, className }: { title: string; children: ReactNode; className?: string }) {
  return (
    <section className={cn("border-hairline bg-card rounded-xl border p-5", className)}>
      <h2 className="mb-3 text-sm font-semibold">{title}</h2>
      {children}
    </section>
  )
}

/** Stream labels by mode: transfer uses shapes and tones. */
export const streamLabels = (transfer: boolean) =>
  transfer ? { position: "Shape", letter: "Tone" } : { position: "Position", letter: "Letter" }
