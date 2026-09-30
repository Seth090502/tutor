import type { ComponentProps, ReactNode } from "react"

import { cn } from "@/lib/utils"
import { usePageVisible } from "./_motion"
import "./kit.css"

export type LedTone = "ok" | "warn" | "bad" | "off"

/** The LED dot alone. Decorative: pair it with a word (see LedStatus). */
export function Led({ tone, pulse = false, className }: { tone: LedTone; pulse?: boolean; className?: string }) {
  const visible = usePageVisible()
  return (
    <span
      aria-hidden
      data-tone={tone}
      data-paused={visible ? undefined : ""}
      className={cn("kit-led", pulse && tone !== "off" && "kit-pulse", className)}
    />
  )
}

type LedStatusProps = Omit<ComponentProps<"span">, "children"> & {
  tone: LedTone
  /** The state word: colour is never the only signal. */
  state: string
  pulse?: boolean
}

/** LED dot followed by its state word. */
export function LedStatus({ tone, state, pulse, className, ...props }: LedStatusProps) {
  return (
    <span className={cn("inline-flex items-center gap-2.5", className)} {...props}>
      <Led tone={tone} pulse={pulse} />
      <span className="text-foreground font-mono text-[11px] tracking-[0.14em] uppercase">{state}</span>
    </span>
  )
}

type ServiceRowProps = Omit<ComponentProps<"div">, "children"> & {
  label: ReactNode
  /** Small secondary text after the label, e.g. an address. */
  detail?: ReactNode
  tone: LedTone
  state: string
  pulse?: boolean
}

/** Dot-leader row: label ..... LED + state. */
export function ServiceRow({ label, detail, tone, state, pulse, className, ...props }: ServiceRowProps) {
  return (
    <div data-slot="kit-service-row" className={cn("flex items-baseline gap-3 py-2.5", className)} {...props}>
      <span className="text-[15px] font-medium">{label}</span>
      {detail ? <span className="text-ink-3 font-mono text-xs">{detail}</span> : null}
      <span
        aria-hidden
        className="border-hairline-strong mb-[5px] h-0 min-w-8 flex-1 self-end border-b-[1.5px] border-dotted"
      />
      <LedStatus tone={tone} state={state} pulse={pulse} className="self-center" />
    </div>
  )
}
