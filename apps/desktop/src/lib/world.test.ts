import { createElement } from "react"
import { renderToString } from "react-dom/server"
import { describe, expect, it } from "vitest"

import {
  WORLDS,
  WORLD_STORAGE_KEY,
  applyWorld,
  isWorld,
  readStoredWorld,
  useWorld,
  worldMeta,
  writeStoredWorld,
} from "./world"

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

describe("worlds", () => {
  it("lists the six worlds with a label and description each", () => {
    expect(WORLDS.map((w) => w.id)).toEqual(["neutral", "math", "english", "science", "chemistry", "languages"])
    for (const w of WORLDS) {
      expect(w.label.length).toBeGreaterThan(0)
      expect(w.description.length).toBeGreaterThan(0)
    }
    expect(worldMeta("math").label).toBe("Math")
    expect(worldMeta("languages").label).toBe("Languages")
  })

  it("accepts only known ids", () => {
    expect(isWorld("chemistry")).toBe(true)
    expect(isWorld("languages")).toBe(true)
    expect(isWorld("history")).toBe(false)
    expect(isWorld(null)).toBe(false)
  })
})

describe("remembering the world", () => {
  it("round-trips through storage", () => {
    const storage = memoryStorage()
    expect(readStoredWorld(storage)).toBe("neutral")
    writeStoredWorld("science", storage)
    expect(storage.getItem(WORLD_STORAGE_KEY)).toBe("science")
    expect(readStoredWorld(storage)).toBe("science")
  })

  it("ignores a stored value that is not a world", () => {
    expect(readStoredWorld(memoryStorage({ [WORLD_STORAGE_KEY]: "<script>" }))).toBe("neutral")
  })

  it("works when storage is missing or throws", () => {
    expect(readStoredWorld(null)).toBe("neutral")
    expect(readStoredWorld(throwing)).toBe("neutral")
    expect(() => writeStoredWorld("math", throwing)).not.toThrow()
    expect(() => writeStoredWorld("math", null)).not.toThrow()
  })
})

describe("applyWorld", () => {
  it("sets data-world on the root", () => {
    const root = { dataset: {} as DOMStringMap }
    applyWorld(root, "chemistry")
    expect(root.dataset.world).toBe("chemistry")
  })
})

describe("useWorld", () => {
  it("refuses to run outside a WorldProvider", () => {
    const Probe = () => (useWorld(), null)
    expect(() => renderToString(createElement(Probe))).toThrow(/WorldProvider/)
  })
})
