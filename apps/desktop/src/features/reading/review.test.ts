import { describe, expect, it } from "vitest"

import { CONFIDENCE_P, calibration, shortText } from "./review"

describe("calibration of a summary judgement against what it covered", () => {
  it("uses the C-wide 1-4 scale: 0.25, 0.5, 0.75, 0.95", () => {
    expect(CONFIDENCE_P).toEqual([0.25, 0.5, 0.75, 0.95])
  })
  it("calls a judgement close when the covered share is within 0.2 of it", () => {
    expect(calibration(3, 5, 6)).toMatchObject({ verdict: "close", expected: 0.75 })
    expect(calibration(2, 3, 6)).toMatchObject({ verdict: "close", actual: 0.5 })
  })
  it("names an overestimate and an underestimate", () => {
    expect(calibration(4, 2, 6).verdict).toBe("over")
    expect(calibration(1, 5, 6).verdict).toBe("under")
  })
  it("has no verdict when there were no idea units", () => {
    expect(calibration(3, 0, 0).verdict).toBe("none")
  })
})

describe("shortText", () => {
  it("cuts at a word boundary and marks the cut", () => {
    expect(shortText("one two three four", 10)).toEqual({ text: "one two…", cut: true })
    expect(shortText("short", 10)).toEqual({ text: "short", cut: false })
  })
})
