// Small shared pieces for the tracker screens: layout, formatting, error text, SVG bars.
import { Component, useId, type ErrorInfo, type ReactNode } from "react"
import { TriangleAlert, type LucideIcon } from "lucide-react"

import { HttpError } from "@/lib/backend"
import { cn } from "@/lib/utils"

// Chart colours: categorical slots 1-4 and the status hues of the dataviz reference palette.
// Dark values apply under the app's `.dark` class, like the shadcn tokens.
const CHART_CSS = `
.viz{--viz-1:#2a78d6;--viz-2:#eb6834;--viz-3:#1baf7a;--viz-4:#eda100;--viz-good:#0ca30c;--viz-bad:#d03b3b;--viz-warn:#fab219;--viz-neutral:var(--muted-foreground)}
.dark .viz,.dark.viz{--viz-1:#3987e5;--viz-2:#d95926;--viz-3:#199e70;--viz-4:#c98500}
`

export const fill = (color: string) => ({ fill: color })

/** Round a raw axis step up to 1, 2, 5 or 10 times a power of ten. */
export function niceStep(raw: number) {
  const pow = 10 ** Math.floor(Math.log10(raw))
  const f = raw / pow
  return (f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10) * pow
}

// ---------- formatting ----------

const INT = new Intl.NumberFormat("en-US")
const COMPACT = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 2 })
const finite = (n: number | null | undefined): n is number => typeof n === "number" && Number.isFinite(n)

export const fmtInt = (n: number | null | undefined) => (finite(n) ? INT.format(Math.round(n)) : "—")
export const fmtCompact = (n: number | null | undefined) => (finite(n) ? COMPACT.format(n) : "—")
export const fmtPct = (part: number, whole: number) => (whole > 0 ? `${((100 * part) / whole).toFixed(1)}%` : "—")

export function fmtUsd(n: number | null | undefined) {
  if (!finite(n)) return "—"
  return n.toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: n > 0 && n < 1 ? 4 : 2,
  })
}

/** Decimal units, matching the survey's GB = 1e9 bytes. */
export function fmtBytes(n: number | null | undefined) {
  if (!finite(n)) return "—"
  const units = ["B", "kB", "MB", "GB", "TB"]
  let v = n
  let i = 0
  while (Math.abs(v) >= 1000 && i < units.length - 1) {
    v /= 1000
    i++
  }
  return `${i === 0 ? v : v.toFixed(v < 10 ? 2 : 1)} ${units[i]}`
}

export function fmtDateTime(epochSeconds: number | null | undefined) {
  if (!finite(epochSeconds)) return "—"
  const d = new Date(epochSeconds * 1000)
  return Number.isNaN(d.getTime()) ? "—" : d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })
}

/** Text for any thrown value; prefers the `error`/`detail` field of a service's JSON error body. */
export function errorText(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e)
  if (!(e instanceof HttpError)) return msg
  const at = msg.indexOf("{")
  if (at >= 0) {
    try {
      const body = JSON.parse(msg.slice(at)) as { error?: unknown; detail?: unknown }
      if (typeof body.error === "string") return body.error
      if (typeof body.detail === "string") return body.detail
    } catch {
      // not JSON: fall through to the raw message
    }
  }
  return msg
}

// ---------- layout ----------

export function Page({ children }: { children: ReactNode }) {
  return (
    <div className="viz mx-auto flex max-w-5xl flex-col gap-6 p-8">
      <style>{CHART_CSS}</style>
      {children}
    </div>
  )
}

export function Panel({
  title,
  action,
  children,
  className,
}: {
  title?: ReactNode
  action?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <section className={cn("bg-card rounded-xl border p-5", className)}>
      {(title || action) && (
        <div className="mb-4 flex items-center justify-between gap-3">
          <h2 className="text-sm font-medium">{title}</h2>
          {action}
        </div>
      )}
      {children}
    </section>
  )
}

export function Stat({ label, value, hint }: { label: string; value: ReactNode; hint?: ReactNode }) {
  return (
    <div className="bg-card rounded-xl border p-4">
      <div className="text-muted-foreground text-xs">{label}</div>
      <div className="mt-1 text-2xl font-semibold tracking-tight tabular-nums">{value}</div>
      {hint && <div className="text-muted-foreground mt-1 text-xs">{hint}</div>}
    </div>
  )
}

export function StateMessage({ icon: Icon, title, detail }: { icon?: LucideIcon; title: string; detail?: string }) {
  return (
    <div className="mx-auto flex h-full max-w-md flex-col items-center justify-center gap-2 p-8 text-center">
      {Icon && <Icon className="text-muted-foreground size-6" />}
      <p className="text-sm font-medium">{title}</p>
      {detail && <p className="text-muted-foreground text-xs break-words">{detail}</p>}
    </div>
  )
}

export function Notice({ children }: { children: ReactNode }) {
  return (
    <div role="alert" className="bg-card flex items-start gap-2 rounded-xl border p-3 text-sm">
      <TriangleAlert className="mt-0.5 size-4 shrink-0" style={{ color: "var(--viz-warn)" }} />
      <div className="min-w-0 break-words">{children}</div>
    </div>
  )
}

/** Keeps a render bug in one screen from blanking the whole app. */
export class ScreenBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }
  static getDerivedStateFromError(error: Error) {
    return { error }
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("tracker screen failed to render", error, info.componentStack)
  }
  render() {
    const { error } = this.state
    if (error) return <StateMessage icon={TriangleAlert} title="This screen failed to render" detail={String(error.message ?? error)} />
    return this.props.children
  }
}

// ---------- SVG bars ----------

export type Segment = { key: string; label: string; value: number; color: string }

/** One horizontal bar split into proportional segments, separated by 2px surface gaps. */
export function StackedBar({ segments, label, height = 14 }: { segments: Segment[]; label: string; height?: number }) {
  const clip = useId()
  const total = segments.reduce((sum, s) => sum + s.value, 0)
  let x = 0
  return (
    <svg width="100%" height={height} role="img" aria-label={label} className="block">
      <defs>
        <clipPath id={clip}>
          <rect width="100%" height={height} rx={4} />
        </clipPath>
      </defs>
      <g clipPath={`url(#${clip})`}>
        <rect width="100%" height={height} style={fill("var(--muted)")} />
        {total > 0 &&
          segments
            .filter((s) => s.value > 0)
            .map((s) => {
              const w = (100 * s.value) / total
              const rect = (
                <rect key={s.key} x={`${x}%`} width={`${w}%`} height={height} style={{ fill: s.color, stroke: "var(--card)", strokeWidth: 2 }}>
                  <title>{`${s.label}: ${fmtInt(s.value)}`}</title>
                </rect>
              )
              x += w
              return rect
            })}
      </g>
    </svg>
  )
}

export function LegendDot({ color, label, value }: { color: string; label: string; value?: ReactNode }) {
  return (
    <li className="flex items-center gap-2 text-sm">
      <span className="size-2.5 shrink-0 rounded-sm" style={{ background: color }} />
      <span>{label}</span>
      {value !== undefined && <span className="text-muted-foreground ml-auto tabular-nums">{value}</span>}
    </li>
  )
}

/** Labelled horizontal bars scaled to the largest value. */
export function BarList({ rows, color }: { rows: { label: string; value: number }[]; color: string }) {
  const max = Math.max(0, ...rows.map((r) => r.value))
  if (rows.length === 0) return <p className="text-muted-foreground text-sm">None yet</p>
  return (
    <ul className="space-y-3">
      {rows.map((r) => (
        <li key={r.label}>
          <div className="mb-1 flex justify-between gap-3 text-sm">
            <span className="truncate">{r.label}</span>
            <span className="text-muted-foreground tabular-nums">{fmtInt(r.value)}</span>
          </div>
          <svg width="100%" height={6} role="presentation" className="block">
            <rect width="100%" height={6} rx={3} style={fill("var(--muted)")} />
            <rect width={`${max > 0 ? (100 * r.value) / max : 0}%`} height={6} rx={3} style={fill(color)} />
          </svg>
        </li>
      ))}
    </ul>
  )
}
