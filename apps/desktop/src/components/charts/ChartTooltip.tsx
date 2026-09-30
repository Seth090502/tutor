import { useLayoutEffect, useRef, useState, type ReactNode } from "react"

import { cn } from "@/lib/utils"
import { EASE_SOFT } from "./_motion"
import { placeTooltip, placeTooltipAbove } from "./geometry"

interface ChartTooltipProps {
  /** Anchor inside the container, in px. */
  x: number
  y: number
  containerW: number
  containerH: number
  visible: boolean
  /** "side" sits beside the anchor (scrubbers); "above" sits over it (hovered marks). */
  placement?: "side" | "above"
  className?: string
  children: ReactNode
}

/**
 * Floating readout inside a `position: relative` container. Moves by transform and fades by opacity only;
 * flips to the left of the anchor near the right edge. Never takes pointer events.
 */
export function ChartTooltip({ x, y, containerW, containerH, visible, placement = "side", className, children }: ChartTooltipProps) {
  const ref = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ w: 160, h: 80 })
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const w = el.offsetWidth
    const h = el.offsetHeight
    setSize((s) => (s.w === w && s.h === h ? s : { w, h }))
  })
  const p = (placement === "above" ? placeTooltipAbove : placeTooltip)(x, y, size.w, size.h, containerW, containerH)
  return (
    <div
      ref={ref}
      aria-hidden="true"
      className={cn(
        "border-hairline-strong bg-popover text-popover-foreground pointer-events-none absolute top-0 left-0 z-10 rounded-lg border px-3 py-2 font-mono text-[11px] leading-[1.45] shadow-[0_10px_28px_-14px_color-mix(in_oklab,var(--ink)_55%,transparent)]",
        className,
      )}
      style={{
        transform: `translate(${p.x}px, ${p.y}px)`,
        opacity: visible ? 1 : 0,
        transition: visible
          ? `transform var(--dur-quick) ${EASE_SOFT}, opacity var(--dur-quick) ${EASE_SOFT}`
          : `opacity var(--dur-quick) ${EASE_SOFT}`,
      }}
    >
      {children}
    </div>
  )
}
