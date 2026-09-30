import type { ComponentProps, ReactNode } from "react"
import { Check, TriangleAlert, X } from "lucide-react"

import { cn } from "@/lib/utils"
import { CountUp } from "./CountUp"
import { Eyebrow } from "./Eyebrow"
import "./kit.css"

export type Tone = "ok" | "warn" | "bad"

const TONE: Record<Tone, { icon: typeof Check; cls: string }> = {
  ok: { icon: Check, cls: "border-ok/40 bg-ok/10 [&>svg]:text-ok" },
  warn: { icon: TriangleAlert, cls: "border-warn/45 bg-warn/10 [&>svg]:text-warn" },
  bad: { icon: X, cls: "border-bad/40 bg-bad/10 [&>svg]:text-bad" },
}

/**
 * Small change chip. Colour is never the only signal: a tone glyph is always shown, and `text`
 * should carry a sign or a word ("+12%", "3 overdue").
 */
export function DeltaChip({ text, tone, className }: { text: string; tone: Tone; className?: string }) {
  const { icon: Icon, cls } = TONE[tone]
  return (
    <span
      data-tone={tone}
      className={cn(
        "text-foreground inline-flex items-center gap-1.5 rounded-full border px-2 py-[3px] font-mono text-[11px] leading-none tracking-[0.04em]",
        cls,
        className,
      )}
    >
      <Icon aria-hidden className="size-3" strokeWidth={2.5} />
      {text}
    </span>
  )
}

type StatSize = "md" | "lg" | "xl"

// Sized from the cell's own width (cqi), so a narrow cell shrinks its numeral instead of overflowing it.
// Md reaches its maximum in a cell about 200px wide inside its padding, lg in one about 267px, xl in one about 255px.
const NUMERAL: Record<StatSize, string> = {
  md: "text-[clamp(22px,20cqi,40px)] font-light",
  lg: "text-[clamp(24px,21cqi,56px)] font-extralight",
  xl: "text-[clamp(56px,36cqi,92px)] font-extralight",
}

type StatProps = Omit<ComponentProps<"div">, "children"> & {
  label: ReactNode
  value: number | string
  /** Small mono text beside the numeral. */
  unit?: ReactNode
  /** A line under the numeral, e.g. "of 61,432 · 95.7%". */
  caption?: ReactNode
  delta?: { text: string; tone: Tone }
  size?: StatSize
  decimals?: number
  /** Count numeric values up to their value (default). */
  animate?: boolean
}

/** Big light tabular numeral over a mono caps label, with an optional unit, delta chip and caption. */
export function Stat({
  label,
  value,
  unit,
  caption,
  delta,
  size = "lg",
  decimals = 0,
  animate = true,
  className,
  ...props
}: StatProps) {
  return (
    <div data-slot="kit-stat" className={cn("bg-card @container flex min-w-0 flex-col gap-3.5 px-6 py-5", className)} {...props}>
      <dt>
        <Eyebrow>{label}</Eyebrow>
      </dt>
      <dd className="m-0 flex flex-col gap-2">
        <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-2">
          <span className={cn("leading-none tracking-[-0.04em] tabular-nums", NUMERAL[size])}>
            {typeof value === "number" && animate ? <CountUp value={value} decimals={decimals} /> : value}
          </span>
          {unit ? <span className="text-ink-2 font-mono text-xs tracking-[0.06em]">{unit}</span> : null}
          {delta ? <DeltaChip {...delta} className="self-center" /> : null}
        </div>
        {caption ? <p className="text-ink-2 font-mono text-xs tracking-[0.06em]">{caption}</p> : null}
      </dd>
    </div>
  )
}

/** Hairline-divided cells; wraps to extra rows on narrow widths. */
export function StatRow({ className, children, ...props }: ComponentProps<"dl">) {
  return (
    <dl
      data-slot="kit-stat-row"
      className={cn(
        "border-hairline bg-hairline m-0 grid grid-cols-[repeat(auto-fit,minmax(11rem,1fr))] gap-px overflow-hidden rounded-2xl border",
        className,
      )}
      {...props}
    >
      {children}
    </dl>
  )
}
