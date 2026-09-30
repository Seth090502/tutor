import { describe, expect, it } from "vitest"

import { idleDimClass, idleRemaining } from "./IdleDim"

describe("idleRemaining", () => {
  it("counts down from the last input", () => {
    expect(idleRemaining(1000, 1000, 5000)).toBe(5000)
    expect(idleRemaining(1000, 3500, 5000)).toBe(2500)
  })

  it("is zero or negative once the threshold has passed", () => {
    expect(idleRemaining(1000, 6000, 5000)).toBe(0)
    expect(idleRemaining(1000, 9000, 5000)).toBeLessThan(0)
  })
})

describe("idleDimClass", () => {
  it("dims to 40% and only transitions while idle, so restore is instant", () => {
    expect(idleDimClass).toContain("group-data-[idle]/idle:opacity-40")
    expect(idleDimClass).toContain("group-data-[idle]/idle:duration-(--idle-dur)")
    // No ungated transition class: the un-idle state must not animate.
    expect(idleDimClass.split(" ").filter((c) => c.startsWith("transition") || c.startsWith("duration"))).toEqual([])
  })
})
