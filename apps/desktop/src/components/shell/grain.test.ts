import { describe, expect, it } from "vitest"

import { grainPixels } from "./grain"

describe("grainPixels", () => {
  const pixels = grainPixels(160)

  it("fills a 160 px RGBA tile", () => {
    expect(pixels.length).toBe(160 * 160 * 4)
  })

  it("keeps alpha between 2 and 4 percent, so text contrast is untouched", () => {
    let total = 0
    for (let i = 3; i < pixels.length; i += 4) {
      expect(pixels[i]).toBeGreaterThanOrEqual(5)
      expect(pixels[i]).toBeLessThanOrEqual(10)
      total += pixels[i]
    }
    const meanPercent = (total / (pixels.length / 4) / 255) * 100
    expect(meanPercent).toBeGreaterThan(2.5)
    expect(meanPercent).toBeLessThan(3.6)
  })

  it("uses both dark and light specks and is the same on every launch", () => {
    const values = new Set<number>()
    for (let i = 0; i < pixels.length; i += 4) values.add(pixels[i])
    expect([...values].sort()).toEqual([0, 255])
    expect(grainPixels(160)).toEqual(pixels)
  })
})
