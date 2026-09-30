import { useSyncExternalStore } from "react"

// Local motion helpers for the chart kit. The app-wide hooks live in src/lib/motion.ts (another owner), so
// the charts read prefers-reduced-motion and run their own short tweens here.

/** Same curves as the --ease-* tokens, for inline styles where a CSS variable is not guaranteed. */
export const EASE_SOFT = "cubic-bezier(.2,.8,.2,1)"
export const EASE_SPRING = "cubic-bezier(.3,1.35,.4,1)"
export const EASE_INOUT = "cubic-bezier(.76,0,.16,1)"

const QUERY = "(prefers-reduced-motion: reduce)"

export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia(QUERY).matches
}

function subscribe(onChange: () => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return () => {}
  const mq = window.matchMedia(QUERY)
  mq.addEventListener("change", onChange)
  return () => mq.removeEventListener("change", onChange)
}

export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, prefersReducedMotion, () => false)
}

/**
 * Run `onFrame(eased)` from 0 to 1 over `duration` ms on requestAnimationFrame, then `onDone`. Under reduced
 * motion it jumps straight to 1. rAF stops by itself while the page is hidden, so nothing loops in the
 * background. Returns a cancel function.
 */
export function tween(duration: number, ease: (t: number) => number, onFrame: (eased: number) => void, onDone?: () => void): () => void {
  if (prefersReducedMotion() || duration <= 0 || typeof requestAnimationFrame !== "function") {
    onFrame(1)
    onDone?.()
    return () => {}
  }
  let raf = 0
  let start = 0
  let cancelled = false
  const step = (now: number) => {
    if (cancelled) return
    if (!start) start = now
    const t = Math.min(1, (now - start) / duration)
    onFrame(ease(t))
    if (t < 1) raf = requestAnimationFrame(step)
    else onDone?.()
  }
  raf = requestAnimationFrame(step)
  return () => {
    cancelled = true
    cancelAnimationFrame(raf)
  }
}
