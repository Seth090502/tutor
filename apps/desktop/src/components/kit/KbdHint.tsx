import type { ComponentProps } from "react"

import { cn } from "@/lib/utils"

/** A key chip for use inside KbdHint. */
export function Kbd({ className, ...props }: ComponentProps<"kbd">) {
  return (
    <kbd
      className={cn(
        "border-hairline-strong bg-card text-foreground mx-0.5 inline-block min-w-[1.6em] rounded-[5px] border border-b-2 px-1.5 py-px text-center align-baseline font-mono text-[11px] leading-[1.5] not-italic",
        className,
      )}
      {...props}
    />
  )
}

/** Italic hint line, e.g. Press A to D to choose, Enter to submit, with Kbd chips for the keys. */
export function KbdHint({ className, ...props }: ComponentProps<"p">) {
  return (
    <p
      data-slot="kit-kbd-hint"
      className={cn("text-ink-2 font-serif text-[15px] leading-relaxed italic", className)}
      {...props}
    />
  )
}
