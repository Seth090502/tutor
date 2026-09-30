import { describe, expect, it } from "vitest"

import { buildDigest, cardable, digestCards, errorKey, type LoggedError } from "./digest"

const err = (over: Partial<LoggedError> = {}): LoggedError => ({
  said: "yo tiene hambre",
  corrected: "yo tengo hambre",
  english: "I am hungry",
  rule: "Tener is irregular: yo tengo.",
  breaksComprehension: false,
  ...over,
})

describe("the digest", () => {
  it("is empty when nothing was logged", () => {
    expect(buildDigest([])).toEqual([])
  })

  it("groups repeats of one error and counts them", () => {
    const d = buildDigest([err(), err({ said: "yo tiene sed", corrected: "Yo tengo hambre." }), err()])
    expect(d).toHaveLength(1)
    expect(d[0].count).toBe(3)
    expect(d[0].said).toEqual(["yo tiene hambre", "yo tiene sed"])
  })

  it("shows at most three, never more", () => {
    const many = Array.from({ length: 7 }, (_, i) => err({ corrected: `frase ${i}`, said: `fraze ${i}` }))
    expect(buildDigest(many)).toHaveLength(3)
    expect(buildDigest(many, 2)).toHaveLength(2)
  })

  it("puts comprehension-breaking errors first, then repeats, then the order heard", () => {
    const d = buildDigest([
      err({ corrected: "a", said: "x" }),
      err({ corrected: "b", said: "y" }),
      err({ corrected: "b", said: "y" }),
      err({ corrected: "c", said: "z", breaksComprehension: true }),
      err({ corrected: "d", said: "w" }),
    ])
    expect(d.map((i) => i.corrected)).toEqual(["c", "b", "a"])
  })

  it("keeps an error that broke comprehension even once it has recurred without doing so", () => {
    const d = buildDigest([err({ breaksComprehension: true }), err()])
    expect(d[0].breaksComprehension).toBe(true)
  })

  it("ignores a correction that is identical to what was said", () => {
    expect(buildDigest([err({ said: "Yo tengo hambre.", corrected: "yo tengo hambre" })])).toEqual([])
  })

  it("fills a missing rule or meaning from a later repeat", () => {
    const d = buildDigest([err({ rule: "", english: "" }), err({ rule: "Use tengo.", english: "I am hungry" })])
    expect(d[0]).toMatchObject({ rule: "Use tengo.", english: "I am hungry", count: 2 })
  })

  it("does not modify its input", () => {
    const input = [err(), err({ corrected: "otra" })]
    const copy = JSON.parse(JSON.stringify(input))
    buildDigest(input)
    expect(input).toEqual(copy)
  })
})

describe("digest cards", () => {
  it("turns items with a meaning into card bodies and skips the rest", () => {
    const d = buildDigest([err(), err({ corrected: "sin significado", said: "sin", english: "" })])
    expect(d.map(cardable)).toEqual([true, false])
    expect(digestCards(d)).toEqual([{ text: "yo tengo hambre", meaning: "I am hungry" }])
  })
})

describe("error keys", () => {
  it("folds case, punctuation, spacing and composition", () => {
    expect(errorKey("  ¿Dónde   está?  ")).toBe("dónde está")
    expect(errorKey("café")).toBe(errorKey("café"))
    expect(errorKey("？！")).toBe("")
  })
})
