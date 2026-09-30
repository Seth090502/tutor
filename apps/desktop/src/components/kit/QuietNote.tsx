import type { ComponentProps, ReactNode } from "react"

import { cn } from "@/lib/utils"
import { Led, type LedTone } from "./Led"

type QuietNoteProps = Omit<ComponentProps<"p">, "children"> & {
  tone: LedTone
  children: ReactNode
}

/** A quiet one-line status note led by an LED, for a refresh that failed or a value that is stale. */
export function QuietNote({ tone, className, children, ...props }: QuietNoteProps) {
  return (
    <p
      data-slot="kit-quiet-note"
      role="status"
      className={cn("text-ink-2 flex items-start gap-3 font-mono text-[12px] leading-snug break-words", className)}
      {...props}
    >
      <Led tone={tone} className="mt-[3px]" />
      <span className="min-w-0">{children}</span>
    </p>
  )
}
