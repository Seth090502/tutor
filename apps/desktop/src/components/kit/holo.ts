import { clamp01, smoothing } from "./_motion"

/** Holo card maths (pure). Pointer positions are normalised to the card: 0..1 on both axes. */

export const MAX_TILT_DEG = 7
/** Glare never gets above this opacity, so text underneath stays readable. */
export const GLARE_MAX_OPACITY = 0.4
export const HOLO_SMOOTHING = 13

export type HoloPose = {
  nx: number
  ny: number
  /** Hover mix: 1 while the pointer is over the card, 0 when it is away. Scales every layer. */
  hm: number
}

export const RESTING_POSE: HoloPose = { nx: 0.5, ny: 0.5, hm: 0 }

export function tiltDegrees(nx: number, ny: number, max = MAX_TILT_DEG): { rx: number; ry: number } {
  return { rx: (0.5 - clamp01(ny)) * 2 * max, ry: (clamp01(nx) - 0.5) * 2 * max }
}

type Offset = [x: number, y: number]

/**
 * Layer offsets in percent of each layer's own (oversized) box. Rainbow, diffraction and sparkle
 * drift at different rates and directions; the glare follows the pointer (d / 2.2, because its
 * box is 220% of the card).
 */
export function layerOffsets(nx: number, ny: number) {
  const dx = clamp01(nx) - 0.5
  const dy = clamp01(ny) - 0.5
  return {
    rainbow: [-dx * 20, -dy * 20] as Offset,
    diffraction: [dx * 10, dy * 10] as Offset,
    sparkle: [dx * 14, -dy * 14] as Offset,
    sheen: [-dx * 36, -dy * 12] as Offset,
    glare: [(dx * 100) / 2.2, (dy * 100) / 2.2] as Offset,
  }
}

const GAIN = { rainbow: 0.22, diffraction: 0.12, sparkle: 0.3, sheen: 0.4, glare: 0.38 }

/** Layer opacities: base gain times hover mix, stronger away from the card centre. */
export function layerOpacities(nx: number, ny: number, hm: number) {
  const dist = Math.min(1, Math.hypot(clamp01(nx) - 0.5, clamp01(ny) - 0.5) * 2)
  const f = clamp01(hm) * (0.55 + 0.45 * dist)
  return {
    rainbow: GAIN.rainbow * f,
    diffraction: GAIN.diffraction * f,
    sparkle: GAIN.sparkle * f,
    sheen: GAIN.sheen * f,
    glare: Math.min(GLARE_MAX_OPACITY, GAIN.glare * f),
  }
}

/** One smoothing step toward the target pose (frame-rate independent, 1 - exp(-dt * 13)). */
export function stepPose(cur: HoloPose, target: HoloPose, dt: number): HoloPose {
  const k = smoothing(dt, HOLO_SMOOTHING)
  return {
    nx: cur.nx + (target.nx - cur.nx) * k,
    ny: cur.ny + (target.ny - cur.ny) * k,
    hm: cur.hm + (target.hm - cur.hm) * k,
  }
}

export function isSettled(cur: HoloPose, target: HoloPose, eps = 0.0008): boolean {
  return Math.abs(cur.nx - target.nx) < eps && Math.abs(cur.ny - target.ny) < eps && Math.abs(cur.hm - target.hm) < eps
}
