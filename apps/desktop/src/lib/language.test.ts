import { createElement } from "react"
import { renderToString } from "react-dom/server"
import { describe, expect, it } from "vitest"

import {
  LANGUAGES,
  LANGUAGE_STORAGE_KEY,
  isTargetLanguage,
  languageMeta,
  readStoredLanguage,
  useLanguage,
  writeStoredLanguage,
} from "./language"

const memoryStorage = (initial: Record<string, string> = {}) => {
  const data = new Map(Object.entries(initial))
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
  }
}

const throwing = {
  getItem: () => {
    throw new Error("SecurityError")
  },
  setItem: () => {
    throw new Error("QuotaExceededError")
  },
}

describe("languages", () => {
  it("lists the five target languages with complete metadata", () => {
    expect(LANGUAGES.map((l) => l.id)).toEqual(["es", "fr", "de", "ja", "zh"])
    for (const l of LANGUAGES) {
      expect(l.english.length).toBeGreaterThan(0)
      expect(l.native.length).toBeGreaterThan(0)
      expect(l.script).toMatch(/^[A-Z][a-z]{3}$/) // ISO 15924
      expect(l.bcp47.startsWith(l.id)).toBe(true)
      expect(l.scribe).toMatch(/^[a-z]{3}$/) // ISO 639-3, as Scribe lists its languages
    }
  })

  it("carries the codes the speech services expect", () => {
    expect(languageMeta("es")).toMatchObject({ english: "Spanish", native: "Español", scribe: "spa", script: "Latn" })
    expect(languageMeta("ja")).toMatchObject({ native: "日本語", scribe: "jpn", script: "Jpan" })
    expect(languageMeta("zh")).toMatchObject({ bcp47: "zh-Hans", scribe: "zho", script: "Hans" })
  })

  it("accepts only known ids", () => {
    expect(isTargetLanguage("de")).toBe(true)
    expect(isTargetLanguage("it")).toBe(false)
    expect(isTargetLanguage("ES")).toBe(false)
    expect(isTargetLanguage(undefined)).toBe(false)
  })
})

describe("remembering the language", () => {
  it("round-trips through storage", () => {
    const storage = memoryStorage()
    expect(readStoredLanguage(storage)).toBe("es")
    writeStoredLanguage("ja", storage)
    expect(storage.getItem(LANGUAGE_STORAGE_KEY)).toBe("ja")
    expect(readStoredLanguage(storage)).toBe("ja")
  })

  it("ignores a stored value that is not a target language", () => {
    expect(readStoredLanguage(memoryStorage({ [LANGUAGE_STORAGE_KEY]: "klingon" }))).toBe("es")
  })

  it("works when storage is missing or throws", () => {
    expect(readStoredLanguage(null)).toBe("es")
    expect(readStoredLanguage(throwing)).toBe("es")
    expect(() => writeStoredLanguage("fr", throwing)).not.toThrow()
    expect(() => writeStoredLanguage("fr", null)).not.toThrow()
  })
})

describe("useLanguage", () => {
  it("refuses to run outside a LanguageProvider", () => {
    const Probe = () => (useLanguage(), null)
    expect(() => renderToString(createElement(Probe))).toThrow(/LanguageProvider/)
  })
})
