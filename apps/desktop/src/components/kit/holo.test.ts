import { describe, expect, it } from "vitest"

import {
  GLARE_MAX_OPACITY,
  MAX_TILT_DEG,
  RESTING_POSE,
  isSettled,
  layerOffsets,
  layerOpacities,
  stepPose,
  tiltDegrees,
} from "./holo"

describe("tiltDegrees", () => {
  it("is flat at the centre and within the 6-8 degree budget at the edges", () => {
    expect(tiltDegrees(0.5, 0.5)).toEqual({ rx: 0, ry: 0 })
    expect(MAX_TILT_DEG).toBeGreaterThanOrEqual(6)
    expect(MAX_TILT_DEG).toBeLessThanOrEqual(8)
    for (const [nx, ny] of [[0, 0], [1, 1], [0, 1], [1, 0], [-3, 9]]) {
      const { rx, ry } = tiltDegrees(nx, ny)
      expect(Math.abs(rx)).toBeLessThanOrEqual(MAX_TILT_DEG)
      expect(Math.abs(ry)).toBeLessThanOrEqual(MAX_TILT_DEG)
    }
  })

  it("leans toward the pointer: right edge rotates +Y, top edge rotates +X", () => {
    expect(tiltDegrees(1, 0.5).ry).toBeGreaterThan(0)
    expect(tiltDegrees(0, 0.5).ry).toBeLessThan(0)
    expect(tiltDegrees(0.5, 0).rx).toBeGreaterThan(0)
    expect(tiltDegrees(0.5, 1).rx).toBeLessThan(0)
  })
})

describe("layerOpacities", () => {
  it("is zero when the pointer is away", () => {
    expect(Object.values(layerOpacities(0.2, 0.8, 0)).every((v) => v === 0)).toBe(true)
  })

  it("never lets the glare exceed 0.4", () => {
    for (let x = 0; x <= 1; x += 0.25)
      for (let y = 0; y <= 1; y += 0.25) {
        expect(layerOpacities(x, y, 1).glare).toBeLessThanOrEqual(GLARE_MAX_OPACITY)
        expect(layerOpacities(x, y, 5).glare).toBeLessThanOrEqual(GLARE_MAX_OPACITY)
      }
  })
})

describe("layerOffsets", () => {
  it("rests centred, and the glare tracks the pointer at d / 2.2", () => {
    expect(layerOffsets(0.5, 0.5).glare).toEqual([0, 0])
    expect(layerOffsets(1, 0.5).glare[0]).toBeCloseTo(50 / 2.2, 6)
    expect(layerOffsets(0, 0.5).glare[0]).toBeCloseTo(-50 / 2.2, 6)
  })

  it("moves the rainbow against the pointer", () => {
    expect(layerOffsets(1, 0.5).rainbow[0]).toBeLessThan(0)
  })
})

describe("stepPose", () => {
  it("moves toward the target without overshooting, and settles", () => {
    const target = { nx: 0.9, ny: 0.1, hm: 1 }
    let pose = RESTING_POSE
    for (let i = 0; i < 400; i++) {
      const next = stepPose(pose, target, 1 / 60)
      expect(next.hm).toBeLessThanOrEqual(1)
      expect(next.hm).toBeGreaterThanOrEqual(pose.hm)
      pose = next
    }
    expect(isSettled(pose, target)).toBe(true)
  })

  it("is frame-rate independent: two half steps equal one full step", () => {
    const target = { nx: 1, ny: 1, hm: 1 }
    const a = stepPose(stepPose(RESTING_POSE, target, 1 / 120), target, 1 / 120)
    const b = stepPose(RESTING_POSE, target, 1 / 60)
    expect(a.nx).toBeCloseTo(b.nx, 6)
    expect(a.hm).toBeCloseTo(b.hm, 6)
  })

  it("a resting pose is settled against itself", () => {
    expect(isSettled(RESTING_POSE, RESTING_POSE)).toBe(true)
    expect(isSettled(RESTING_POSE, { nx: 0.7, ny: 0.5, hm: 1 })).toBe(false)
  })
})
