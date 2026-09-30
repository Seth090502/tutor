// Small shared pieces of the Evals screen: decision badge, entrance wrapper, state block, quiet notes.
import type { ComponentType, CSSProperties, ReactNode } from "react"

import { MarkerGlyph } from "@/components/charts"
import { Led } from "@/components/kit"
import { cn } from "@/lib/utils"
import { DECISION_META, type Decision } from "./model"
import "./evals.css"

/** Shape plus word, so the decision never rests on colour. The shape matches the chart marker. */
export function DecisionBadge({ decision, className }: { decision: Decision; className?: string }) {
  const meta = DECISION_META[decision]
  return (
    <span
      className={cn(
        "border-hairline-strong text-foreground inline-flex shrink-0 items-center gap-2 rounded-full border px-2.5 py-1.5 font-mono text-[11px] leading-none tracking-[0.12em] uppercase",
        className,
      )}
    >
      <svg width="12" height="12" viewBox="-6 -6 12 12" aria-hidden="true">
        <MarkerGlyph shape={meta.shape} color={meta.tone} size={3.6} filled />
      </svg>
      {meta.label}
    </span>
  )
}

/** Wraps a block in the one-shot entrance. `i` is its place in the stagger. */
export function Rise({ i, enter, className, children }: { i: number; enter: boolean; className?: string; children: ReactNode }) {
  return (
    <div className={cn("min-w-0", enter && "ev-rise", className)} style={{ "--ev-i": i } as CSSProperties}>
      {children}
    </div>
  )
}

/** Centred message for the states that have no data: browser, empty and error. */
export function StateBlock({ icon: Icon, title, children, action }: { icon: ComponentType<{ className?: string }>; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div role="status" className="mx-auto flex min-h-[46vh] max-w-md flex-col items-center justify-center gap-4 px-6 py-12 text-center">
      <span className="border-hairline-strong text-ink-2 grid size-12 place-items-center rounded-full border">
        <Icon className="size-5" />
      </span>
      <h2 className="font-serif text-[26px] leading-tight tracking-[-0.01em] text-balance">{title}</h2>
      {children ? <div className="text-ink-2 text-sm leading-relaxed text-pretty">{children}</div> : null}
      {action}
    </div>
  )
}

/** A quiet one-line note with an LED: used for refresh errors. */
export function QuietNote({ tone, children }: { tone: "warn" | "bad"; children: ReactNode }) {
  return (
    <p role="status" className="text-ink-2 flex items-start gap-3 font-mono text-[12px] leading-snug break-words">
      <Led tone={tone} className="mt-[3px]" />
      <span className="min-w-0">{children}</span>
    </p>
  )
}

export function Code({ children }: { children: ReactNode }) {
  return <code className="bg-muted border-hairline text-foreground inline-block rounded-md border px-2.5 py-1.5 font-mono text-[12.5px] break-words">{children}</code>
}
