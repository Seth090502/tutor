/// <reference types="node" />
import { readFileSync } from "node:fs"
import { describe, expect, it } from "vitest"

import { springLinear } from "./_spring"

const values = (s: string) =>
  s
    .replace(/^linear\(|\)$/g, "")
    .split(",")
    .map((p) => Number(p.trim()))

const keycapCss = readFileSync(new URL("./keycap.css", import.meta.url), "utf8")

describe("springLinear", () => {
  it("starts at 0 and ends at exactly 1", () => {
    const v = values(springLinear(0.5))
    expect(v[0]).toBe(0)
    expect(v[v.length - 1]).toBe(1)
  })

  it("overshoots when underdamped, and more when damping is lower", () => {
    const peak = (z: number) => Math.max(...values(springLinear(z)))
    expect(peak(0.72)).toBeGreaterThan(1)
    expect(peak(0.38)).toBeGreaterThan(peak(0.72))
  })

  it("rejects damping ratios outside (0, 1)", () => {
    expect(() => springLinear(0)).toThrow(RangeError)
    expect(() => springLinear(1)).toThrow(RangeError)
  })

  it("matches the literals committed in keycap.css", () => {
    expect(keycapCss).toContain(`--kit-ease-press: ${springLinear(0.72)};`)
    expect(keycapCss).toContain(`--kit-ease-release: ${springLinear(0.38)};`)
  })
})
