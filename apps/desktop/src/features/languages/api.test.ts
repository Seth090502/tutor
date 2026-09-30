import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { addCards, gradedText, starterDeckCommand } from "./api"

const BACKEND = "http://127.0.0.1:8001"

type Call = { method: string; url: string; body: unknown }
type Reply = { status?: number; body: unknown } | Error
let calls: Call[]
let replies: Reply[]

beforeEach(() => {
  calls = []
  replies = []
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      calls.push({ method: init?.method ?? "GET", url, body: init?.body ? JSON.parse(String(init.body)) : undefined })
      const next = replies.shift()
      if (!next) throw new Error(`unexpected fetch ${url}`)
      if (next instanceof Error) throw next
      return new Response(JSON.stringify(next.body), { status: next.status ?? 200 })
    }),
  )
})
afterEach(() => vi.unstubAllGlobals())

describe("language routes (C10)", () => {
  it("asks for graded text with the band and an optional topic", async () => {
    replies.push({ body: { text: "t", sentences: ["t"], coverage: 0.97, unknown: [], questions: [] } })
    await gradedText("fr", "B1", "  le marché ")
    expect(calls[0]).toMatchObject({ method: "POST", url: `${BACKEND}/api/languages/graded-text`, body: { lang: "fr", band: "B1", topic: "le marché" } })
    replies.push({ body: { text: "t", sentences: [], coverage: 1, unknown: [], questions: [] } })
    await gradedText("fr", "A1", "  ")
    expect(calls[1].body).toEqual({ lang: "fr", band: "A1" })
  })

  it("adds cards for tapped words", async () => {
    replies.push({ body: { created: 3, entry_ids: [1, 2, 3] } })
    const out = await addCards("es", [{ text: "estación", meaning: "station", example: "La estación." }])
    expect(calls[0]).toMatchObject({ url: `${BACKEND}/api/languages/cards`, body: { lang: "es", items: [{ text: "estación", meaning: "station", example: "La estación." }] } })
    expect(out.created).toBe(3)
  })

  it("names the starter deck command", () => {
    expect(starterDeckCommand("ja")).toBe("python -m deeptutor.services.languages.build_deck --lang ja --size 200")
  })
})
