import { describe, expect, it } from "vitest"

import { barRatio, barTotals, shareText } from "./bar-list-logic"
import { countPlan } from "./countup-logic"
import { commitInt, radioTarget } from "./form-logic"
import { typedChars } from "./log-logic"

describe("bar list", () => {
  it("clamps the ratio and is zero without a maximum", () => {
    expect(barRatio(5, 0)).toBe(0)
    expect(barRatio(5, 10)).toBe(0.5)
    expect(barRatio(12, 10)).toBe(1)
    expect(barRatio(-3, 10)).toBe(0)
    expect(barRatio(Number.NaN, 10)).toBe(0)
  })

  it("prints shares with one decimal and a dash for an empty whole", () => {
    expect(shareText(72, 184)).toBe("39.1%")
    expect(shareText(1, 3, 0)).toBe("33%")
    expect(shareText(5, 0)).toBe("—")
  })

  it("totals and finds the longest bar", () => {
    expect(barTotals([{ label: "a", value: 3 }, { label: "b", value: 9 }])).toEqual({ total: 12, max: 9 })
    expect(barTotals([])).toEqual({ total: 0, max: 0 })
  })
})

describe("typedChars", () => {
  it("counts the printed line, padded to the state column", () => {
    expect(typedChars("16:39:24", "done", "ok")).toBe("16:39:24  done     ok".length)
  })
  it("caps long lines and never returns zero", () => {
    expect(typedChars("16:39:24", "running", "x".repeat(200))).toBe(64)
    expect(typedChars("", "", "", 0)).toBe(1)
  })
})

describe("commitInt", () => {
  it("clamps and rounds", () => {
    expect(commitInt("15", 12, 20)).toBe(15)
    expect(commitInt("1", 12, 20)).toBe(12)
    expect(commitInt("99", 12, 20)).toBe(20)
    expect(commitInt("3.6", 1, 9)).toBe(4)
  })
  it("is null while the draft is not a number", () => {
    expect(commitInt("", 1, 9)).toBeNull()
    expect(commitInt("  ", 1, 9)).toBeNull()
    expect(commitInt("abc", 1, 9)).toBeNull()
  })
})

describe("radioTarget", () => {
  it("wraps with the arrow keys", () => {
    expect(radioTarget("ArrowRight", 2, 3)).toBe(0)
    expect(radioTarget("ArrowDown", 0, 3)).toBe(1)
    expect(radioTarget("ArrowLeft", 0, 3)).toBe(2)
    expect(radioTarget("ArrowUp", 2, 3)).toBe(1)
  })
  it("jumps with Home and End and starts from the first when nothing is chosen", () => {
    expect(radioTarget("Home", 2, 4)).toBe(0)
    expect(radioTarget("End", 0, 4)).toBe(3)
    expect(radioTarget("ArrowRight", -1, 4)).toBe(1)
  })
  it("ignores other keys and empty groups", () => {
    expect(radioTarget("a", 0, 3)).toBeNull()
    expect(radioTarget("ArrowRight", 0, 0)).toBeNull()
  })
})

describe("countPlan", () => {
  const base = { once: false, ran: false, reduced: false, duration: 900, shown: 0, value: 10 }
  it("tweens by default, and again on every new value", () => {
    expect(countPlan(base)).toBe("tween")
    expect(countPlan({ ...base, ran: true })).toBe("tween")
  })
  it("in once mode tweens only the first run", () => {
    expect(countPlan({ ...base, once: true })).toBe("tween")
    expect(countPlan({ ...base, once: true, ran: true })).toBe("snap")
  })
  it("snaps under reduced motion, with no duration, or when already there", () => {
    expect(countPlan({ ...base, reduced: true })).toBe("snap")
    expect(countPlan({ ...base, duration: 0 })).toBe("snap")
    expect(countPlan({ ...base, shown: 10 })).toBe("snap")
  })
})
