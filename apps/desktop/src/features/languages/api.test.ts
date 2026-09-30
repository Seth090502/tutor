import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { addCards, checkAnswer, gradedText, loadQueue, openCard, requestHint, sendConfidence, starterDeckCommand, submitReview } from "./api"
import type { Card } from "./drill"

const BACKEND = "http://127.0.0.1:8001"
const PRACTICE = `${BACKEND}/api/question-notebook/practice`

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

const card: Card = { questionId: 7, attemptId: "att 1", prompt: "the river", kind: "production", rungsAvailable: 3 }

describe("the queue", () => {
  it("keeps the chosen language's cards, production first", async () => {
    const item = (id: number, ...names: string[]) => ({ entry: { id, question: `q${id}`, categories: names.map((name) => ({ name })) }, state: { version: 1 } })
    replies.push({ body: [item(1, "lang:es", "card:recognition"), item(2, "lang:fr", "card:production"), item(3, "lang:es", "card:production")] })
    const queue = await loadQueue("es")
    expect(queue.map((q) => q.entry.id)).toEqual([3, 1])
    expect(calls[0].url).toMatch(/\/queue\?limit=100&timezone=/)
  })

  it("opens an attempt and reads only the prompt and kind from the entry", async () => {
    replies.push({ body: { attempt_id: "att-9", rungs_available: 2 } })
    const opened = await openCard({
      entry: { id: 5, question: "the river", categories: [{ name: "lang:es" }, { name: "card:production" }], correct_answer: "el río", hints: ["x"] } as never,
      state: { version: 3 },
    })
    expect(calls[0]).toMatchObject({ method: "POST", url: `${PRACTICE}/questions/5/attempts` })
    expect(opened).toEqual({ questionId: 5, attemptId: "att-9", prompt: "the river", kind: "production", rungsAvailable: 2 })
    expect(JSON.stringify(opened)).not.toMatch(/el río|"x"/)
  })
})

describe("attempt routes (C7)", () => {
  it("sends confidence, then a hint request, then the check, each on the attempt", async () => {
    replies.push({ body: { ok: true } }, { body: { rung: 1, text: "Starts with r.", is_answer: false } }, {
      body: { correct: false, correct_answer: "el río", explanation: "Tatoeba", misconception: "", diagnostic: "accent" },
    })
    await sendConfidence("att 1", 3)
    const hint = await requestHint("att 1")
    const check = await checkAnswer("att 1", "el rio")
    expect(calls.map((c) => [c.method, c.url])).toEqual([
      ["POST", `${PRACTICE}/attempts/att%201/confidence`],
      ["POST", `${PRACTICE}/attempts/att%201/hint`],
      ["POST", `${PRACTICE}/attempts/att%201/check`],
    ])
    expect(calls[0].body).toEqual({ level: 3 })
    expect(calls[2].body).toEqual({ answer: "el rio" })
    expect(hint).toEqual({ rung: 1, text: "Starts with r.", isAnswer: false })
    expect(check).toEqual({ correct: false, correctAnswer: "el río", explanation: "Tatoeba", misconception: null, diagnostic: "accent" })
  })

  it("reviews with the attempt id and never a client rating", async () => {
    replies.push({ body: { entry: {}, state: { version: 4 } } }, { body: { rating: "good", due_at: 1790000000 } })
    const out = await submitReview(card, "el río", null)
    expect(calls[0]).toMatchObject({ method: "GET", url: `${PRACTICE}/questions/7` })
    const body = calls[1].body as Record<string, unknown>
    expect(calls[1].url).toBe(`${PRACTICE}/questions/7/review`)
    expect(body).toMatchObject({ attempt_id: "att 1", version: 4, answer: "el río" })
    expect(body.request_id).toMatch(/^[A-Za-z0-9]{32}$/)
    expect(body).not.toHaveProperty("rating")
    expect(body).not.toHaveProperty("judged_correct")
    expect(out.rating).toBe("good")
    expect(out.nextDue).toBeTruthy()
  })

  it("sends the self-judgement only when there is one, and retries once on 409", async () => {
    replies.push(
      { body: { state: { version: 4 } } },
      { status: 409, body: { detail: "stale" } },
      { body: { state: { version: 5 } } },
      { body: { rating: "hard", due_at: null } },
    )
    const out = await submitReview(card, "x", true)
    expect(calls.map((c) => c.method)).toEqual(["GET", "POST", "GET", "POST"])
    expect((calls[1].body as Record<string, unknown>).judged_correct).toBe(true)
    expect((calls[3].body as Record<string, unknown>).version).toBe(5)
    expect(out).toEqual({ rating: "hard", nextDue: null })
  })

  it("does not retry other failures", async () => {
    replies.push({ body: { state: { version: 4 } } }, { status: 422, body: { detail: "rating rejected" } })
    await expect(submitReview(card, "x", null)).rejects.toThrow(/422/)
    expect(calls).toHaveLength(2)
  })
})

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
