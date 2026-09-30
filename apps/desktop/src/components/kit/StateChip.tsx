import type { ComponentProps } from "react"

import { cn } from "@/lib/utils"
import { usePageVisible } from "./_motion"
import "./kit.css"

type StateChipProps = Omit<ComponentProps<"span">, "children"> & {
  /** The state word, shown uppercase. Always present: the dot is decoration. */
  state: string
  tone?: "world" | "ok" | "warn" | "bad"
  /** "glass" is dark and translucent, for text over busy visuals such as the Orb. "light" is a card-coloured
   *  chip for a plain surface. */
  variant?: "glass" | "light"
  /** Pulsing dot. Stops under reduced motion and while the page is hidden. */
  pulse?: boolean
  dot?: boolean
}

/** State word with an optional dot. Dark glass by default (for text over the Orb); `variant="light"` for plain surfaces. */
export function StateChip({ state, tone = "world", variant = "glass", pulse = false, dot = true, className, ...props }: StateChipProps) {
  const visible = usePageVisible()
  return (
    <span data-slot="kit-state-chip" data-tone={tone} data-variant={variant} className={cn("kit-chip", className)} {...props}>
      {dot ? (
        <span
          aria-hidden
          data-paused={visible ? undefined : ""}
          className={cn("kit-chip-dot", pulse && "kit-pulse")}
        />
      ) : null}
      {state}
    </span>
  )
}
