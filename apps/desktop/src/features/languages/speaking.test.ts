import { describe, expect, it } from "vitest"

import { SPEAKING_TARGETS, compareSpoken, probeSummary, units } from "./speaking"

describe("units", () => {
  it("folds case and punctuation for spaced scripts", () => {
    expect(units("¿Dónde está la estación?", "es")).toEqual(["dónde", "está", "la", "estación"])
    expect(units("Je m'appelle Marc.", "fr")).toEqual(["je", "m'appelle", "marc"])
  })

  it("uses characters for Japanese and Mandarin, ignoring punctuation and width", () => {
    expect(units("駅はどこですか。", "ja")).toEqual(["駅", "は", "ど", "こ", "で", "す", "か"])
    expect(units("火车站在哪里？", "zh").join("")).toBe("火车站在哪里")
    expect(units("ＡＢＣ", "ja")).toEqual(["a", "b", "c"])
  })
})

describe("comparing what was heard with the target", () => {
  it("matches an exact hearing", () => {
    const c = compareSpoken("Me llamo Ana.", "me llamo ana", "es")
    expect(c).toMatchObject({ total: 3, matched: 3, exact: true, extra: [] })
    expect(c.target.every((t) => t.heard)).toBe(true)
  })

  it("marks the words that were not heard, in target order", () => {
    const c = compareSpoken("Quisiera un café con leche", "quiero un café leche", "es")
    expect(c.target.map((t) => [t.text, t.heard])).toEqual([
      ["quisiera", false],
      ["un", true],
      ["café", true],
      ["con", false],
      ["leche", true],
    ])
    expect(c.extra).toEqual(["quiero"])
    expect(c.exact).toBe(false)
  })

  it("counts order: the same words in the wrong order are not all matched", () => {
    const c = compareSpoken("uno dos tres", "tres dos uno", "es")
    expect(c.matched).toBe(1)
    expect(c.exact).toBe(false)
  })

  it("is not exact when the recogniser added words", () => {
    const c = compareSpoken("hola", "hola hola", "es")
    expect(c.matched).toBe(1)
    expect(c.extra).toEqual(["hola"])
    expect(c.exact).toBe(false)
  })

  it("handles nothing heard and an empty target", () => {
    expect(compareSpoken("hola", "", "es")).toMatchObject({ matched: 0, total: 1, exact: false })
    expect(compareSpoken("", "hola", "es")).toMatchObject({ total: 0, exact: false, extra: ["hola"] })
  })

  it("compares Mandarin by character", () => {
    const c = compareSpoken("我想要一杯咖啡。", "我想要一杯咖啡", "zh")
    expect(c.exact).toBe(true)
    const miss = compareSpoken("我想要一杯咖啡。", "我想要一杯加非", "zh")
    expect(miss.matched).toBe(5)
    expect(miss.total).toBe(7)
  })
})

describe("the summary never grades", () => {
  it("states what matched without a score", () => {
    const exact = compareSpoken("hola", "hola", "es")
    expect(probeSummary(exact, "hola", "es")).toMatch(/smoothed over slips/)
    const miss = compareSpoken("uno dos tres", "uno tres", "es")
    const text = probeSummary(miss, "uno tres", "es")
    expect(text).toBe("The recogniser did not match 1 of the 3 words. That can be your pronunciation, the recording or the recogniser itself.")
    expect(text).not.toMatch(/%|score|good|great|perfect/i)
    expect(probeSummary(compareSpoken("hola", "", "es"), "", "es")).toMatch(/heard nothing/)
    expect(probeSummary(compareSpoken("我想", "我", "zh"), "我", "zh")).toMatch(/1 of the 2 characters\./)
  })

  it("never offers a percentage or grade in any of its messages", () => {
    const heard = compareSpoken("uno dos tres cuatro", "uno", "es")
    expect(probeSummary(heard, "uno", "es")).not.toMatch(/\d+\s?%/)
  })
})

describe("the built-in sentences", () => {
  it("has a few for every language", () => {
    for (const lang of ["es", "fr", "de", "ja", "zh"] as const) expect(SPEAKING_TARGETS[lang].length).toBeGreaterThanOrEqual(3)
  })
})
