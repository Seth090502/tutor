import { useSyncExternalStore } from "react"

// Local motion helpers for the kit. Kept free of browser globals at import time so the pure
// functions are testable under node.

const REDUCED_QUERY = "(prefers-reduced-motion: reduce)"

export function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia(REDUCED_QUERY).matches
  )
}

function subscribeReducedMotion(onChange: () => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return () => {}
  const mq = window.matchMedia(REDUCED_QUERY)
  mq.addEventListener("change", onChange)
  return () => mq.removeEventListener("change", onChange)
}

export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribeReducedMotion, prefersReducedMotion, () => false)
}

function subscribeVisibility(onChange: () => void): () => void {
  document.addEventListener("visibilitychange", onChange)
  return () => document.removeEventListener("visibilitychange", onChange)
}

/** False while the tab is hidden; loops (pulses, drifts) pause on it. */
export function usePageVisible(): boolean {
  return useSyncExternalStore(
    subscribeVisibility,
    () => document.visibilityState !== "hidden",
    () => true,
  )
}

export const easeOutCubic = (t: number): number => 1 - Math.pow(1 - t, 3)

export const clamp01 = (v: number): number => (v < 0 ? 0 : v > 1 ? 1 : v)

/** Frame-rate independent smoothing factor: cur += (target - cur) * smoothing(dt). */
export function smoothing(dt: number, rate = 13): number {
  return 1 - Math.exp(-dt * rate)
}

export type FrameEnv = {
  raf: (cb: (ts: number) => void) => number
  caf: (id: number) => void
}

const browserEnv: FrameEnv = {
  raf: (cb) => requestAnimationFrame(cb),
  caf: (id) => cancelAnimationFrame(id),
}

export type TweenOptions = {
  from: number
  to: number
  /** Milliseconds. */
  duration: number
  ease?: (t: number) => number
  onFrame: (value: number) => void
  onDone?: () => void
}

/**
 * Runs a value from `from` to `to` on animation frames and returns a cancel function. After
 * cancel no further frame is delivered. The final frame is exactly `to`.
 */
export function tween(o: TweenOptions, env: FrameEnv = browserEnv): () => void {
  const ease = o.ease ?? easeOutCubic
  let id = 0
  let start: number | null = null
  let finished = false

  const step = (ts: number) => {
    if (finished) return
    if (start === null) start = ts
    const t = o.duration <= 0 ? 1 : Math.min(1, (ts - start) / o.duration)
    o.onFrame(t >= 1 ? o.to : o.from + (o.to - o.from) * ease(t))
    if (t >= 1) {
      finished = true
      o.onDone?.()
    } else {
      id = env.raf(step)
    }
  }

  id = env.raf(step)
  return () => {
    if (finished) return
    finished = true
    env.caf(id)
  }
}
