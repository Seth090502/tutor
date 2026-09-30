import { describe, expect, it } from "vitest"

import { blockerFor, NO_KEY, NO_SHELL } from "./hooks"
import { appendLog, readLog } from "./logs"

function store(initial: Record<string, string> = {}) {
  const data = { ...initial }
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => void (data[k] = v) }
}

describe("local logs", () => {
  it("appends entries in order and keeps the newest when over the cap", () => {
    const s = store()
    for (let i = 0; i < 5; i++) appendLog("k", { i }, 3, s)
    expect(readLog<{ i: number }>("k", s)).toEqual([{ i: 2 }, { i: 3 }, { i: 4 }])
  })

  it("reads an empty log from missing, corrupt or non-array storage", () => {
    expect(readLog("k", store())).toEqual([])
    expect(readLog("k", store({ k: "{not json" }))).toEqual([])
    expect(readLog("k", store({ k: '{"a":1}' }))).toEqual([])
    expect(readLog("k", null)).toEqual([])
  })

  it("reports a failed write instead of throwing", () => {
    const full = { getItem: () => null, setItem: () => { throw new DOMException("full", "QuotaExceededError") } }
    expect(appendLog("k", { a: 1 }, 10, full)).toBe(false)
    expect(appendLog("k", { a: 1 }, 10, null)).toBe(false)
    expect(appendLog("k", { a: 1 }, 10, store())).toBe(true)
  })
})

describe("why audio is off", () => {
  it("names the missing piece in order: the desktop shell first, then the key", () => {
    expect(blockerFor(false, false)).toBe(NO_SHELL)
    expect(blockerFor(false, true)).toBe(NO_SHELL)
    expect(blockerFor(true, false)).toBe(NO_KEY)
    expect(blockerFor(true, true)).toBeNull()
  })
})
