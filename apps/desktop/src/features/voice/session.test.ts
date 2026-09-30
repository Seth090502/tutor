import { describe, expect, it } from "vitest"

import { SESSION_MINUTES, formatClock, sessionClock, stageState } from "./session"

describe("formatClock", () => {
  it("shows mm:ss under an hour and h:mm:ss after", () => {
    expect(formatClock(0)).toBe("00:00")
    expect(formatClock(61_000)).toBe("01:01")
    expect(formatClock(45 * 60_000)).toBe("45:00")
    expect(formatClock(3_725_000)).toBe("1:02:05")
  })

  it("treats negative time as zero", () => {
    expect(formatClock(-5000)).toBe("00:00")
  })
})

describe("sessionClock", () => {
  it("suggests a break at exactly 45 minutes and not a second before", () => {
    expect(SESSION_MINUTES).toBe(45)
    expect(sessionClock(45 * 60_000 - 1000).breakDue).toBe(false)
    expect(sessionClock(45 * 60_000).breakDue).toBe(true)
  })

  it("fills the hairline across the block and stops at full", () => {
    expect(sessionClock(0).progress).toBe(0)
    expect(sessionClock(22.5 * 60_000).progress).toBeCloseTo(0.5)
    expect(sessionClock(90 * 60_000).progress).toBe(1)
  })
})

describe("stageState", () => {
  it("names the four states the chip shows", () => {
    expect(stageState(null)).toBe("idle")
    expect(stageState("listening")).toBe("listening")
    expect(stageState("thinking")).toBe("thinking")
    expect(stageState("talking")).toBe("speaking")
  })
})
