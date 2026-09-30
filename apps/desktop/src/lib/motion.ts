// Motion runtime (contract C5): page visibility, reduced motion, the quality governor and the
// shared spring ticker. Nothing here runs a permanent requestAnimationFrame loop: the governor
// samples frames for a moment, and the spring ticker sleeps whenever every spring is at rest.

import { useCallback, useEffect, useLayoutEffect, useRef, useSyncExternalStore } from "react"

export type Quality = "lite" | "rich"

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)"

// ---------------------------------------------------------------------------------------------
// Environment hooks
// ---------------------------------------------------------------------------------------------

const isHidden = () => typeof document !== "undefined" && document.visibilityState === "hidden"

const prefersReducedMotion = () =>
  typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia(REDUCED_MOTION_QUERY).matches

function subscribeVisibility(notify: () => void) {
  document.addEventListener("visibilitychange", notify)
  return () => document.removeEventListener("visibilitychange", notify)
}

function subscribeReducedMotion(notify: () => void) {
  const query = window.matchMedia(REDUCED_MOTION_QUERY)
  query.addEventListener("change", notify)
  return () => query.removeEventListener("change", notify)
}

const readQuality = (): Quality => (document.documentElement.dataset.quality === "rich" ? "rich" : "lite")

function subscribeQuality(notify: () => void) {
  const observer = new MutationObserver(notify)
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-quality"] })
  return () => observer.disconnect()
}

/** False while the tab or window is hidden. */
export function usePageVisible(): boolean {
  return useSyncExternalStore(subscribeVisibility, () => !isHidden(), () => true)
}

/** Live `prefers-reduced-motion: reduce` subscription. */
export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribeReducedMotion, prefersReducedMotion, () => false)
}

/** The governor's verdict, read from `data-quality` on <html>. */
export function useQuality(): Quality {
  return useSyncExternalStore(subscribeQuality, readQuality, () => "lite")
}

// ---------------------------------------------------------------------------------------------
// Quality governor
// ---------------------------------------------------------------------------------------------

export const GOVERNOR = { frames: 90, richBelowMs: 24, liteAboveMs: 52 } as const

/** Rich only below 24 ms average, lite above 52 ms, unchanged in between; always lite when reduced. */
export function nextQuality(current: Quality, averageMs: number, reduced: boolean): Quality {
  if (reduced) return "lite"
  if (averageMs < GOVERNOR.richBelowMs) return "rich"
  if (averageMs > GOVERNOR.liteAboveMs) return "lite"
  return current
}

export type GovernorEnv = {
  request: (cb: (t: number) => void) => number
  cancel: (id: number) => void
  isHidden: () => boolean
  isReduced: () => boolean
  getQuality: () => Quality
  setQuality: (q: Quality) => void
}

/**
 * Samples GOVERNOR.frames frame intervals on demand (never continuously) and sets the quality.
 * `sample()` is a no-op while hidden or already sampling; `cancel()` drops a sample in flight.
 */
export function createGovernor(env: GovernorEnv) {
  let pending: number | null = null

  function cancel() {
    if (pending !== null) env.cancel(pending)
    pending = null
  }

  function sample() {
    if (env.isReduced()) {
      cancel()
      env.setQuality("lite")
      return
    }
    if (env.isHidden() || pending !== null) return

    let previous: number | null = null
    let total = 0
    let count = 0
    const tick = (t: number) => {
      pending = null
      if (previous !== null) {
        total += t - previous
        count += 1
      }
      previous = t
      if (count >= GOVERNOR.frames) {
        env.setQuality(nextQuality(env.getQuality(), total / count, env.isReduced()))
        return
      }
      pending = env.request(tick)
    }
    pending = env.request(tick)
  }

  return { sample, cancel }
}

let stopGovernor: (() => void) | null = null

/** Starts the governor once (call from main.tsx). Returns a function that stops it. */
export function startQualityGovernor(): () => void {
  if (stopGovernor) return stopGovernor
  const root = document.documentElement
  const governor = createGovernor({
    request: (cb) => requestAnimationFrame(cb),
    cancel: (id) => cancelAnimationFrame(id),
    isHidden,
    isReduced: prefersReducedMotion,
    getQuality: readQuality,
    setQuality: (q) => {
      root.dataset.quality = q
    },
  })

  const reduced = window.matchMedia(REDUCED_MOTION_QUERY)
  const onVisibility = () => (isHidden() ? governor.cancel() : governor.sample())
  root.dataset.quality = "lite"
  document.addEventListener("visibilitychange", onVisibility)
  reduced.addEventListener("change", governor.sample)
  governor.sample()

  stopGovernor = () => {
    governor.cancel()
    document.removeEventListener("visibilitychange", onVisibility)
    reduced.removeEventListener("change", governor.sample)
    stopGovernor = null
  }
  return stopGovernor
}

// ---------------------------------------------------------------------------------------------
// Springs
// ---------------------------------------------------------------------------------------------

/** `precision` is the resting tolerance in the spring's own units (default 0.01). */
export type SpringConfig = { stiffness: number; damping: number; precision?: number }
export type SpringState = { x: number; v: number }

/** Fixed integration step, so results do not depend on the display's frame rate. */
export const SUBSTEP = 1 / 240
const MAX_FRAME = 1 / 20
const DEFAULT_PRECISION = 0.01

/** One semi-implicit Euler step for a unit mass: velocity first, then position with the new velocity. */
export function stepSpring(state: SpringState, target: number, config: SpringConfig, dt = SUBSTEP) {
  const acceleration = -config.stiffness * (state.x - target) - config.damping * state.v
  state.v += acceleration * dt
  state.x += state.v * dt
}

export function isAtRest(state: SpringState, target: number, config: SpringConfig) {
  const precision = config.precision ?? DEFAULT_PRECISION
  return Math.abs(state.x - target) < precision && Math.abs(state.v) < precision
}

export type SpringHandle = {
  /** Aim at a new target; snaps instead when the engine is suspended. */
  set: (target: number) => void
  /** Move there now, with no velocity. */
  jump: (value: number) => void
  config: SpringConfig
  dispose: () => void
}

export type SpringEngineEnv = {
  request: (cb: (t: number) => void) => number
  cancel: (id: number) => void
  /** True under reduced motion or while the page is hidden: springs snap instead of moving. */
  suspended: () => boolean
}

type Node = { state: SpringState; target: number; config: SpringConfig; apply: (v: number) => void }

/** One ticker for every spring. It requests a frame only while some spring is moving. */
export function createSpringEngine(env: SpringEngineEnv) {
  const active = new Set<Node>()
  let frame: number | null = null
  let last: number | null = null
  let carry = 0

  function arrive(node: Node) {
    node.state.x = node.target
    node.state.v = 0
    node.apply(node.target)
    active.delete(node)
  }

  function tick(now: number) {
    frame = null
    const dt = last === null ? 0 : Math.min((now - last) / 1000, MAX_FRAME)
    last = now
    carry += dt
    let steps = 0
    while (carry >= SUBSTEP) {
      for (const node of active) stepSpring(node.state, node.target, node.config)
      carry -= SUBSTEP
      steps += 1
    }
    for (const node of [...active]) {
      if (isAtRest(node.state, node.target, node.config)) arrive(node)
      else if (steps > 0) node.apply(node.state.x)
    }
    if (active.size > 0) frame = env.request(tick)
    else {
      last = null
      carry = 0
    }
  }

  function wake(node: Node) {
    active.add(node)
    if (frame === null) frame = env.request(tick)
  }

  /** Stop everything where it is going: used when the page hides or reduced motion turns on. */
  function snapAll() {
    if (frame !== null) env.cancel(frame)
    frame = null
    last = null
    carry = 0
    for (const node of [...active]) arrive(node)
  }

  function create(initial: number, config: SpringConfig, apply: (v: number) => void): SpringHandle {
    const node: Node = { state: { x: initial, v: 0 }, target: initial, config, apply }
    const handle: SpringHandle = {
      set(target) {
        node.target = target
        if (env.suspended()) arrive(node)
        else if (!isAtRest(node.state, target, node.config)) wake(node)
        else arrive(node)
      },
      jump(value) {
        node.target = value
        arrive(node)
      },
      get config() {
        return node.config
      },
      set config(next) {
        node.config = next
      },
      dispose() {
        active.delete(node)
        if (active.size === 0 && frame !== null) {
          env.cancel(frame)
          frame = null
          last = null
          carry = 0
        }
      },
    }
    return handle
  }

  return { create, snapAll, isSleeping: () => frame === null }
}

let sharedEngine: ReturnType<typeof createSpringEngine> | null = null

function engine() {
  if (sharedEngine) return sharedEngine
  const shared = createSpringEngine({
    request: (cb) => requestAnimationFrame(cb),
    cancel: (id) => cancelAnimationFrame(id),
    suspended: () => isHidden() || prefersReducedMotion(),
  })
  document.addEventListener("visibilitychange", () => isHidden() && shared.snapAll())
  window.matchMedia(REDUCED_MOTION_QUERY).addEventListener("change", (e) => e.matches && shared.snapAll())
  sharedEngine = shared
  return shared
}

/**
 * Drives `apply(value)` toward `target` with a spring. All springs share one ticker that sleeps
 * when every spring is at rest, and snap to the target under reduced motion or while hidden.
 * `apply` runs once on mount with the initial target. `jump` moves without animating.
 */
export function useSpring(
  target: number,
  config: { stiffness: number; damping: number; precision?: number },
  apply: (value: number) => void,
): { jump: (value: number) => void } {
  const applyRef = useRef(apply)
  const handleRef = useRef<SpringHandle | null>(null)
  const targetRef = useRef(target)
  const { stiffness, damping, precision } = config

  useLayoutEffect(() => {
    applyRef.current = apply
  })

  useLayoutEffect(() => {
    const handle = engine().create(targetRef.current, { stiffness, damping, precision }, (v) => applyRef.current(v))
    handleRef.current = handle
    applyRef.current(targetRef.current)
    return () => {
      handle.dispose()
      handleRef.current = null
    }
  }, [stiffness, damping, precision])

  useEffect(() => {
    targetRef.current = target
    handleRef.current?.set(target)
  }, [target])

  const jump = useCallback((value: number) => {
    targetRef.current = value
    handleRef.current?.jump(value)
  }, [])

  return { jump }
}
