import { useEffect, useState, type ComponentProps } from "react"

import { cn } from "@/lib/utils"

/** Milliseconds left before `now` counts as idle; 0 or less means idle already. */
export function idleRemaining(lastInput: number, now: number, ms: number): number {
  return lastInput + ms - now
}

const INPUT_EVENTS = ["pointermove", "pointerdown", "keydown", "wheel"] as const

/**
 * True after `ms` (default 5 s) without pointer or keyboard input, false again on the next
 * input. One timer, re-armed only when it fires early; input events only stamp a timestamp.
 */
export function useIdle(ms = 5000): boolean {
  const [idle, setIdle] = useState(false)

  useEffect(() => {
    let last = performance.now()
    let isIdle = false
    let timer = 0

    const arm = (delay: number) => {
      timer = window.setTimeout(check, delay)
    }
    const check = () => {
      const left = idleRemaining(last, performance.now(), ms)
      if (left > 0) {
        arm(left)
      } else {
        isIdle = true
        setIdle(true)
      }
    }
    const onInput = () => {
      last = performance.now()
      if (isIdle) {
        isIdle = false
        setIdle(false)
        arm(ms)
      }
    }

    arm(ms)
    for (const type of INPUT_EVENTS) window.addEventListener(type, onInput, { passive: true })
    return () => {
      window.clearTimeout(timer)
      for (const type of INPUT_EVENTS) window.removeEventListener(type, onInput)
    }
  }, [ms])

  return idle
}

/**
 * Classes for children that should dim while an ancestor IdleDim is idle: 40% over 1.8 s, then
 * back instantly on input (the base state carries no transition). Under reduced motion the dim
 * still applies, but without a fade.
 */
export const idleDimClass =
  "group-data-[idle]/idle:opacity-40 group-data-[idle]/idle:transition-opacity group-data-[idle]/idle:duration-(--idle-dur) group-data-[idle]/idle:ease-soft"

type IdleDimProps = ComponentProps<"div"> & {
  /** Idle threshold in milliseconds. */
  ms?: number
}

/** Wrapper that sets `data-idle` after no input, for children using `idleDimClass`. */
export function IdleDim({ ms = 5000, className, children, ...props }: IdleDimProps) {
  const idle = useIdle(ms)
  return (
    <div
      data-slot="kit-idle-dim"
      data-idle={idle ? "" : undefined}
      className={cn("group/idle [--idle-dur:1800ms] motion-reduce:[--idle-dur:1ms]", className)}
      {...props}
    >
      {children}
    </div>
  )
}
