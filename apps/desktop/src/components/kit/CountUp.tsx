import { useEffect, useRef, useState, type ComponentProps } from "react"

import { cn } from "@/lib/utils"
import { prefersReducedMotion, tween } from "./_motion"

export function formatNumber(value: number, decimals = 0): string {
  return value.toLocaleString("en-US", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })
}

type CountUpProps = Omit<ComponentProps<"span">, "children"> & {
  value: number
  /** Milliseconds; 900 by default. */
  duration?: number
  decimals?: number
  /** Starting value on first mount. */
  from?: number
  format?: (value: number) => string
}

/**
 * Animates a number to `value` (ease-out cubic, 900 ms). A new value restarts from whatever is
 * currently shown; the previous run is cancelled. Instant under reduced motion. Frames write
 * straight to the text node, so nothing re-renders while counting.
 */
export function CountUp({ value, duration = 900, decimals = 0, from = 0, format, className, ...props }: CountUpProps) {
  const fmt = useRef((v: number) => v.toString())
  fmt.current = (v: number) => (format ? format(v) : formatNumber(v, decimals))

  const textRef = useRef<HTMLSpanElement>(null)
  const shown = useRef(from)
  const [initial] = useState(() => fmt.current(prefersReducedMotion() ? value : from))

  useEffect(() => {
    const node = textRef.current
    if (!node) return
    if (prefersReducedMotion() || duration <= 0 || shown.current === value) {
      shown.current = value
      node.textContent = fmt.current(value)
      return
    }
    return tween({
      from: shown.current,
      to: value,
      duration,
      onFrame: (v) => {
        shown.current = v
        node.textContent = fmt.current(v)
      },
    })
  }, [value, duration])

  return (
    <span className={cn("tabular-nums", className)} {...props}>
      <span className="sr-only">{fmt.current(value)}</span>
      <span aria-hidden ref={textRef}>
        {initial}
      </span>
    </span>
  )
}
