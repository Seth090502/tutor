import { describe, expect, it } from "vitest"

import {
  GOVERNOR,
  SUBSTEP,
  createGovernor,
  createSpringEngine,
  isAtRest,
  nextQuality,
  stepSpring,
  type Quality,
  type SpringState,
} from "./motion"

/** A manual requestAnimationFrame: callbacks run only when the test advances the clock. */
function fakeFrames() {
  let id = 0
  let now = 0
  const queue = new Map<number, (t: number) => void>()
  return {
    request(cb: (t: number) => void) {
      id += 1
      queue.set(id, cb)
      return id
    },
    cancel(handle: number) {
      queue.delete(handle)
    },
    get pending() {
      return queue.size
    },
    /** Advance one frame of `ms` milliseconds; returns false when nothing was scheduled. */
    frame(ms: number) {
      if (queue.size === 0) return false
      now += ms
      const callbacks = [...queue.values()]
      queue.clear()
      for (const cb of callbacks) cb(now)
      return true
    },
    run(ms: number, maxFrames = 2000) {
      let frames = 0
      while (frames < maxFrames && this.frame(ms)) frames += 1
      return frames
    },
  }
}

const last = (xs: number[]) => xs[xs.length - 1]

const SIDEBAR = { stiffness: 260, damping: 20 }

describe("spring maths", () => {
  it("matches the analytic damped oscillator within a percent (underdamped)", () => {
    const { stiffness: k, damping: c } = SIDEBAR
    const state: SpringState = { x: 0, v: 0 }
    const t = 0.4
    for (let i = 0; i < Math.round(t / SUBSTEP); i++) stepSpring(state, 1, SIDEBAR)
    // x(t) = 1 - e^(-ct/2) (cos(wd t) + (c / 2wd) sin(wd t)) for a unit step from rest.
    const wd = Math.sqrt(k - (c * c) / 4)
    const exact = 1 - Math.exp((-c * t) / 2) * (Math.cos(wd * t) + (c / (2 * wd)) * Math.sin(wd * t))
    expect(Math.abs(state.x - exact)).toBeLessThan(0.01)
  })

  it("overshoots when underdamped and does not when critically damped", () => {
    const peak = (config: { stiffness: number; damping: number }) => {
      const state: SpringState = { x: 0, v: 0 }
      let max = 0
      for (let i = 0; i < 2 * 240; i++) {
        stepSpring(state, 1, config)
        max = Math.max(max, state.x)
      }
      return max
    }
    const bouncy = peak(SIDEBAR)
    expect(bouncy).toBeGreaterThan(1.03)
    expect(bouncy).toBeLessThan(1.15)
    expect(peak({ stiffness: 100, damping: 20 })).toBeLessThanOrEqual(1.001)
  })

  it("reports rest only when both position and velocity are small", () => {
    expect(isAtRest({ x: 1, v: 0 }, 1, SIDEBAR)).toBe(true)
    expect(isAtRest({ x: 1.0001, v: 0.001 }, 1, SIDEBAR)).toBe(true)
    expect(isAtRest({ x: 1.5, v: 0 }, 1, SIDEBAR)).toBe(false)
    expect(isAtRest({ x: 1, v: 3 }, 1, SIDEBAR)).toBe(false)
    expect(isAtRest({ x: 1.005, v: 0 }, 1, { ...SIDEBAR, precision: 0.001 })).toBe(false)
  })
})

describe("spring engine", () => {
  const make = (suspended = false) => {
    const frames = fakeFrames()
    const flags = { suspended }
    const engine = createSpringEngine({ ...frames, suspended: () => flags.suspended })
    return { frames, flags, engine }
  }

  it("settles exactly on the target, then schedules nothing", () => {
    const { frames, engine } = make()
    const values: number[] = []
    const spring = engine.create(0, SIDEBAR, (v) => values.push(v))
    spring.set(100)
    expect(frames.pending).toBe(1)

    frames.run(16)
    expect(last(values)).toBe(100)
    expect(frames.pending).toBe(0)
    expect(engine.isSleeping()).toBe(true)
    expect(Math.max(...values)).toBeGreaterThan(100) // the underdamped overshoot was visible
  })

  it("shares one ticker between springs and wakes again on a new target", () => {
    const { frames, engine } = make()
    const a: number[] = []
    const b: number[] = []
    const one = engine.create(0, SIDEBAR, (v) => a.push(v))
    const two = engine.create(0, { stiffness: 640, damping: 42 }, (v) => b.push(v))
    one.set(50)
    two.set(-20)
    expect(frames.pending).toBe(1)

    frames.run(16)
    expect(last(a)).toBe(50)
    expect(last(b)).toBe(-20)
    expect(frames.pending).toBe(0)

    one.set(0)
    expect(frames.pending).toBe(1)
    frames.run(16)
    expect(last(a)).toBe(0)
    expect(frames.pending).toBe(0)
  })

  it("does not depend on the frame rate", () => {
    const at = (ms: number) => {
      const { frames, engine } = make()
      let value = 0
      engine.create(0, SIDEBAR, (v) => (value = v)).set(100)
      const target = 300 // ms of simulated time
      for (let t = 0; t < target; t += ms) frames.frame(ms)
      return value
    }
    expect(Math.abs(at(16.667) - at(6.944))).toBeLessThan(1.5)
  })

  it("clamps a long stall instead of exploding", () => {
    const { frames, engine } = make()
    let value = 0
    engine.create(0, SIDEBAR, (v) => (value = v)).set(100)
    frames.frame(16)
    frames.frame(5000) // tab throttled for five seconds
    expect(Number.isFinite(value)).toBe(true)
    expect(Math.abs(value)).toBeLessThan(150)
  })

  it("snaps without scheduling a frame when suspended (reduced motion or hidden)", () => {
    const { frames, flags, engine } = make(true)
    const values: number[] = []
    const spring = engine.create(0, SIDEBAR, (v) => values.push(v))
    spring.set(80)
    expect(values).toEqual([80])
    expect(frames.pending).toBe(0)
    expect(flags.suspended).toBe(true)
  })

  it("snapAll lands every moving spring on its target and cancels the frame", () => {
    const { frames, engine } = make()
    const values: number[] = []
    engine.create(0, SIDEBAR, (v) => values.push(v)).set(10)
    frames.frame(16)
    engine.snapAll()
    expect(last(values)).toBe(10)
    expect(frames.pending).toBe(0)
    expect(engine.isSleeping()).toBe(true)
  })

  it("jump moves instantly, and dispose stops a moving spring", () => {
    const { frames, engine } = make()
    const values: number[] = []
    const spring = engine.create(0, SIDEBAR, (v) => values.push(v))
    spring.jump(42)
    expect(values).toEqual([42])
    expect(frames.pending).toBe(0)

    spring.set(0)
    expect(frames.pending).toBe(1)
    spring.dispose()
    expect(frames.pending).toBe(0)
  })
})

describe("nextQuality", () => {
  it("turns rich only under 24 ms and lite only over 52 ms", () => {
    expect(nextQuality("lite", 16.7, false)).toBe("rich")
    expect(nextQuality("lite", 23.9, false)).toBe("rich")
    expect(nextQuality("lite", 24, false)).toBe("lite")
    expect(nextQuality("rich", 52.1, false)).toBe("lite")
  })

  it("keeps the current state in the 24 to 52 ms band", () => {
    expect(nextQuality("rich", 30, false)).toBe("rich")
    expect(nextQuality("lite", 30, false)).toBe("lite")
    expect(nextQuality("rich", 52, false)).toBe("rich")
  })

  it("is always lite under reduced motion", () => {
    expect(nextQuality("rich", 5, true)).toBe("lite")
  })
})

describe("quality governor", () => {
  const make = (initial: Quality = "lite") => {
    const frames = fakeFrames()
    const env = { hidden: false, reduced: false, quality: initial }
    const governor = createGovernor({
      ...frames,
      isHidden: () => env.hidden,
      isReduced: () => env.reduced,
      getQuality: () => env.quality,
      setQuality: (q) => (env.quality = q),
    })
    return { frames, env, governor }
  }

  it("samples about 90 frames, then stops asking for frames", () => {
    const { frames, env, governor } = make()
    governor.sample()
    const used = frames.run(16.7)
    expect(used).toBe(GOVERNOR.frames + 1) // the first timestamp only starts the measurement
    expect(env.quality).toBe("rich")
    expect(frames.pending).toBe(0)
  })

  it("stays lite on a slow display and steps back from rich above 52 ms", () => {
    const slow = make()
    slow.governor.sample()
    slow.frames.run(33)
    expect(slow.env.quality).toBe("lite")

    const struggling = make("rich")
    struggling.governor.sample()
    struggling.frames.run(60)
    expect(struggling.env.quality).toBe("lite")
  })

  it("does not sample while hidden, and a sample in flight can be cancelled", () => {
    const { frames, env, governor } = make()
    env.hidden = true
    governor.sample()
    expect(frames.pending).toBe(0)

    env.hidden = false
    governor.sample()
    frames.frame(16)
    governor.cancel()
    expect(frames.pending).toBe(0)
    expect(env.quality).toBe("lite")
  })

  it("never runs two samples at once, and can sample again later", () => {
    const { frames, env, governor } = make()
    governor.sample()
    governor.sample()
    expect(frames.pending).toBe(1)
    frames.run(16)
    expect(env.quality).toBe("rich")

    env.quality = "lite"
    governor.sample()
    frames.run(16)
    expect(env.quality).toBe("rich")
  })

  it("goes lite immediately under reduced motion, without sampling", () => {
    const { frames, env, governor } = make("rich")
    env.reduced = true
    governor.sample()
    expect(env.quality).toBe("lite")
    expect(frames.pending).toBe(0)
  })
})
