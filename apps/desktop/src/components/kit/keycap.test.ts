import { describe, expect, it } from "vitest"

import { hotkeyIndex, isEditableTarget, stepIndex } from "./keycap-logic"

describe("hotkeyIndex", () => {
  it("maps A-D and 1-4 to positions, in either case", () => {
    expect(["a", "B", "c", "D"].map((k) => hotkeyIndex(k, 4))).toEqual([0, 1, 2, 3])
    expect(["1", "2", "3", "4"].map((k) => hotkeyIndex(k, 4))).toEqual([0, 1, 2, 3])
  })

  it("ignores keys beyond the number of choices", () => {
    expect(hotkeyIndex("d", 3)).toBeNull()
    expect(hotkeyIndex("4", 3)).toBeNull()
    expect(hotkeyIndex("c", 3)).toBe(2)
  })

  it("ignores other keys and named keys", () => {
    for (const k of ["e", "5", "0", "Enter", "ArrowDown", " ", ""]) expect(hotkeyIndex(k, 4)).toBeNull()
  })
})

describe("stepIndex", () => {
  it("wraps in both directions", () => {
    expect(stepIndex(3, 1, 4)).toBe(0)
    expect(stepIndex(0, -1, 4)).toBe(3)
    expect(stepIndex(1, 1, 4)).toBe(2)
  })
})

describe("isEditableTarget", () => {
  it("is true for inputs, textareas, selects and contenteditable", () => {
    expect(isEditableTarget({ tagName: "INPUT" } as unknown as EventTarget)).toBe(true)
    expect(isEditableTarget({ tagName: "TEXTAREA" } as unknown as EventTarget)).toBe(true)
    expect(isEditableTarget({ tagName: "SELECT" } as unknown as EventTarget)).toBe(true)
    expect(isEditableTarget({ tagName: "DIV", isContentEditable: true } as unknown as EventTarget)).toBe(true)
  })

  it("is false for buttons, plain elements and null", () => {
    expect(isEditableTarget({ tagName: "BUTTON" } as unknown as EventTarget)).toBe(false)
    expect(isEditableTarget({ tagName: "DIV" } as unknown as EventTarget)).toBe(false)
    expect(isEditableTarget(null)).toBe(false)
  })
})
