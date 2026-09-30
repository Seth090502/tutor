import { afterEach, describe, expect, it, vi } from "vitest"

import { DEFAULT_FILTERS } from "./docs"
import { PREPARED, SECTION } from "./fixtures"
import { ServiceFault, createApi, pageImageUrl, toFault } from "./api"
import { HttpError } from "@/lib/backend"

type Call = { url: string; method: string; body: unknown }

/** Stubs fetch: each request is recorded and answered by `reply(url)`; a thrown error stands for no answer at all. */
function stubFetch(reply: (url: string) => Response | Error) {
  const calls: Call[] = []
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      calls.push({ url, method: init?.method ?? "GET", body: init?.body ? JSON.parse(String(init.body)) : undefined })
      const r = reply(url)
      if (r instanceof Error) throw r
      return r
    }),
  )
  return calls
}
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status })

afterEach(() => vi.unstubAllGlobals())

const api = createApi({ engine: "http://e", backend: "http://b" })

describe("rag-engine calls (C9)", () => {
  it("lists documents with the filter query", async () => {
    const calls = stubFetch(() => json({ total: 0, documents: [] }))
    await api.documents({ ...DEFAULT_FILTERS, q: "limits", domain: "math" }, 50)
    expect(calls[0].url).toBe("http://e/documents?domain=math&q=limits&sort=title&limit=50&offset=50")
    expect(calls[0].method).toBe("GET")
  })
  it("reads one document and a page of chunks, encoding the id", async () => {
    const calls = stubFetch(() => json({}))
    await api.document("a/b c")
    await api.chunks("a/b c", 41, 20)
    expect(calls.map((c) => c.url)).toEqual(["http://e/documents/a%2Fb%20c", "http://e/documents/a%2Fb%20c/chunks?from=41&limit=20"])
  })
  it("builds a page image URL with the width and the quote to mark", () => {
    expect(pageImageUrl("d1", 12, {}, "http://e")).toBe("http://e/documents/d1/pages/12.png?w=1200")
    const u = new URL(pageImageUrl("d 1", 3, { highlight: " the limit is unique ", width: 800 }, "http://e"))
    expect(u.pathname).toBe("/documents/d%201/pages/3.png")
    expect(u.searchParams.get("w")).toBe("800")
    expect(u.searchParams.get("highlight")).toBe("the limit is unique")
  })
  it("keeps the quote short enough for a URL", () => {
    const u = new URL(pageImageUrl("d", 1, { highlight: "x".repeat(900) }, "http://e"))
    expect(u.searchParams.get("highlight")).toHaveLength(240)
  })
})

describe("reading calls (C11)", () => {
  it("opens a session from a chunk", async () => {
    const calls = stubFetch(() => json({ session_id: "s1", section: SECTION }))
    const opened = await api.openSession("d-analysis", "d-analysis:41")
    expect(calls[0]).toMatchObject({ url: "http://b/api/reading/sessions", method: "POST", body: { doc_id: "d-analysis", from_chunk: "d-analysis:41" } })
    expect(opened.section.chunk_ids).toHaveLength(6)
  })
  it("prepares with a bodyless POST", async () => {
    const calls = stubFetch(() => json(PREPARED))
    const prep = await api.prepare("s1")
    expect(calls[0]).toMatchObject({ url: "http://b/api/reading/sessions/s1/prepare", method: "POST", body: undefined })
    expect(prep.prequestions.map((q) => q.id)).toEqual(["q1", "q2"])
  })
  it("sends an answer with its confidence, to the prequestion's own route", async () => {
    const calls = stubFetch(() => json({ correct: false }))
    const r = await api.answer("s 1", 7, "a guess", 3)
    expect(calls[0]).toMatchObject({ url: "http://b/api/reading/sessions/s%201/prequestions/7/answer", method: "POST", body: { answer: "a guess", confidence: 3 } })
    expect(r.correct).toBe(false)
  })
  it("sends recall text, then the summary with its 1-4 judgement, then the accepted atom ids", async () => {
    const calls = stubFetch((url) => (url.endsWith("/summary") ? json({ covered: 4, missed: [{ unit: "u", chunk_id: "c" }] }) : url.endsWith("/accept") ? json({ entry_ids: [11] }) : json({ ok: true })))
    await api.recall("s1", "what I remember")
    const s = await api.summary("s1", "my summary", 2)
    const a = await api.acceptAtoms("s1", ["a1", 3])
    expect(calls.map((c) => [c.url.replace("http://b/api/reading/sessions/s1", ""), c.body])).toEqual([
      ["/recall", { text: "what I remember" }],
      ["/summary", { text: "my summary", judgement: 2 }],
      ["/atoms/accept", { ids: ["a1", 3] }],
    ])
    expect(s).toEqual({ covered: 4, missed: [{ unit: "u", chunk_id: "c" }] })
    expect(a.entry_ids).toEqual([11])
  })
  it("verifies citations through the library route", async () => {
    const calls = stubFetch(() => json({ results: [{ ok: true, reason: null }] }))
    const r = await api.verify([{ chunk_id: "c1", quote: "q", page: 3 }])
    expect(calls[0]).toMatchObject({ url: "http://b/api/library/verify", body: { citations: [{ chunk_id: "c1", quote: "q", page: 3 }] } })
    expect(r.results[0].ok).toBe(true)
  })
})

describe("faults", () => {
  it("separates a service that is not running from one that answered with an error", async () => {
    stubFetch((url) => (url.startsWith("http://e") ? new TypeError("Failed to fetch") : json({ detail: "no API key configured" }, 503)))
    const engineDown = await api.document("d").catch((e) => e)
    const backendErr = await api.prepare("s1").catch((e) => e)
    expect(engineDown).toBeInstanceOf(ServiceFault)
    expect(engineDown).toMatchObject({ service: "rag-engine", offline: true, status: null })
    expect(backendErr).toMatchObject({ service: "backend", offline: false, status: 503, message: "no API key configured" })
  })
  it("names the backend when it is the one that does not answer", async () => {
    stubFetch(() => new TypeError("Failed to fetch"))
    expect(await api.openSession("d", "c").catch((e) => e)).toMatchObject({ service: "backend", offline: true })
  })
  it("leaves an existing fault alone", () => {
    const f = new ServiceFault("backend", false, 500, "x")
    expect(toFault("rag-engine", f)).toBe(f)
    expect(toFault("rag-engine", new HttpError(404, "GET /d: 404")).status).toBe(404)
  })
})
