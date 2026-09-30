import { describe, expect, it } from "vitest"

import { cardItem, coverageBand, coverageWords, foldWord, isNew, tokenize, unknownSet } from "./reading"

const words = (text: string, lang: Parameters<typeof tokenize>[1]) => tokenize(text, lang).filter((p) => p.word).map((p) => p.text)

describe("coverage bands", () => {
  it("puts 95 to 98 percent in the target", () => {
    expect(coverageBand(0.94)).toBe("below")
    expect(coverageBand(0.95)).toBe("target")
    expect(coverageBand(0.97)).toBe("target")
    expect(coverageBand(0.98)).toBe("target")
    expect(coverageBand(0.99)).toBe("above")
  })

  it("says it in words, with the count of new words", () => {
    expect(coverageWords(0.97, 1)).toBe("In the 95 to 98% target, with 1 new word.")
    expect(coverageWords(0.9, 4)).toMatch(/^Below the 95 to 98% target, with 4 new words/)
    expect(coverageWords(1, 0)).toMatch(/^Above the target, with 0 new words/)
  })
})

describe("tokenising", () => {
  it("keeps every character of the sentence", () => {
    for (const [text, lang] of [
      ["¿Dónde está la estación, por favor?", "es"],
      ["L'homme qu'il a vu; « oui » !", "fr"],
      ["Gestern sind wir zum Markt gegangen.", "de"],
      ["昨日、市場に行きました。", "ja"],
      ["我叫王明，住在北京。", "zh"],
    ] as const) {
      expect(tokenize(text, lang).map((p) => p.text).join("")).toBe(text)
    }
  })

  it("finds the words in spaced scripts and leaves punctuation out", () => {
    expect(words("¿Dónde está la estación?", "es")).toEqual(["Dónde", "está", "la", "estación"])
    expect(words("Ich heiße Anna, und du?", "de")).toEqual(["Ich", "heiße", "Anna", "und", "du"])
    expect(words("Mi abuelo vive en Buenos-Aires.", "es")).toEqual(["Mi", "abuelo", "vive", "en", "Buenos-Aires"])
  })

  it("splits French elision into two words", () => {
    expect(words("L'homme qu'il a vu", "fr")).toEqual(["L'", "homme", "qu'", "il", "a", "vu"])
    expect(words("aujourd'hui", "fr")).toEqual(["aujourd'hui"]) // not an elision
  })

  it("segments Japanese and Mandarin into tappable words", () => {
    const ja = words("昨日、市場に行きました。", "ja")
    expect(ja.length).toBeGreaterThan(2)
    expect(ja.join("")).toBe("昨日市場に行きました")
    const zh = words("我叫王明，住在北京。", "zh")
    expect(zh.length).toBeGreaterThan(2)
    expect(zh.join("")).toBe("我叫王明住在北京")
  })
})

describe("new words and card items", () => {
  it("matches the server's unknown list regardless of case and composition", () => {
    const unknown = unknownSet(["Estación", "rio"])
    expect(isNew("estación", unknown)).toBe(true)
    expect(isNew("ESTACIÓN", unknown)).toBe(true)
    expect(isNew("estacion", unknown)).toBe(false) // a missing accent is a different word
    expect(isNew("Rio", unknown)).toBe(true)
    expect(isNew("café", unknownSet(["café"]))).toBe(true)
    expect(foldWord("'hola-")).toBe("hola")
  })

  it("builds a card item only once there is a meaning", () => {
    expect(cardItem("estación", "", "La estación.")).toBeNull()
    expect(cardItem("  ", "station", "")).toBeNull()
    expect(cardItem(" estación ", " station ", " ¿Dónde está la estación? ")).toEqual({
      text: "estación",
      meaning: "station",
      example: "¿Dónde está la estación?",
    })
    expect(cardItem("estación", "station", "  ")).toEqual({ text: "estación", meaning: "station" })
  })
})
