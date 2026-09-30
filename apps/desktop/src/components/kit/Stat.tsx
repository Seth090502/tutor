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

const NUMERAL: Record<StatSize, string> = {
  md: "text-[40px] font-light",
  lg: "text-[56px] font-extralight",
  xl: "text-[clamp(56px,7vw,92px)] font-extralight",
}

type StatProps = Omit<ComponentProps<"div">, "children"> & {
  label: ReactNode
  value: number | string
  unit?: ReactNode
  delta?: { text: string; tone: Tone }
  size?: StatSize
  decimals?: number
  /** Count numeric values up to their value (default). */
  animate?: boolean
}

/** Big light tabular numeral over a mono caps label, optional unit and delta chip. */
export function Stat({
  label,
  value,
  unit,
  delta,
  size = "lg",
  decimals = 0,
  animate = true,
  className,
  ...props
}: StatProps) {
  return (
    <div data-slot="kit-stat" className={cn("bg-card flex min-w-0 flex-col gap-3.5 px-6 py-5", className)} {...props}>
      <dt>
        <Eyebrow>{label}</Eyebrow>
      </dt>
      <dd className="m-0 flex flex-wrap items-baseline gap-x-2.5 gap-y-2">
        <span className={cn("leading-none tracking-[-0.04em] tabular-nums", NUMERAL[size])}>
          {typeof value === "number" && animate ? <CountUp value={value} decimals={decimals} /> : value}
        </span>
        {unit ? <span className="text-ink-2 font-mono text-xs tracking-[0.06em]">{unit}</span> : null}
        {delta ? <DeltaChip {...delta} className="self-center" /> : null}
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
