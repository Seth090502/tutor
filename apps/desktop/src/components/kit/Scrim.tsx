import type { ComponentProps, CSSProperties } from "react"

import { cn } from "@/lib/utils"

type ScrimProps = ComponentProps<"div"> & {
  /** Peak opacity of the scrim in percent (0-100). */
  strength?: number
}

/** Soft radial scrim behind its children, for text over busy visuals. */
export function Scrim({ strength = 70, className, style, children, ...props }: ScrimProps) {
  return (
    <div
      data-slot="kit-scrim"
      className={cn("relative isolate", className)}
      style={{ "--scrim": `${strength}%`, ...style } as CSSProperties}
      {...props}
    >
      <span
        aria-hidden
        className="pointer-events-none absolute -inset-x-[16%] -inset-y-[40%] -z-10 [background:radial-gradient(ellipse_closest-side,color-mix(in_oklab,var(--background)_var(--scrim),transparent)_0%,color-mix(in_oklab,var(--background)_calc(var(--scrim)*0.78),transparent)_48%,transparent_100%)]"
      />
      {children}
    </div>
  )
}
