import { useEffect, useState } from "react"

import { useReducedMotion } from "@/lib/motion"

export type Phase = "idle" | "out" | "in"

/** A CSS duration custom property (such as --dur-quick) in milliseconds. */
function cssMs(name: string, fallback: number) {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  const value = parseFloat(raw)
  if (Number.isNaN(value)) return fallback
  return raw.endsWith("ms") ? value : value * 1000
}

/**
 * Two-step section change: the outgoing view fades out for --dur-quick, then `shown` switches and
 * the incoming view plays its rise-and-fade. Under reduced motion `shown` follows `active` at once.
 */
export function useSectionPhase(active: string) {
  const reduced = useReducedMotion()
  const [shown, setShown] = useState(active)
  const [phase, setPhase] = useState<Phase>("in")

  useEffect(() => {
    if (active === shown) {
      setPhase((p) => (p === "out" ? "idle" : p))
      return
    }
    if (reduced) {
      setShown(active)
      setPhase("idle")
      return
    }
    setPhase("out")
    const timer = window.setTimeout(() => {
      setShown(active)
      setPhase("in")
    }, cssMs("--dur-quick", 140))
    return () => window.clearTimeout(timer)
    // `shown` is read as of the render where `active` changed; it only changes inside this effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, reduced])

  return { shown, phase: reduced ? "idle" : phase }
}
