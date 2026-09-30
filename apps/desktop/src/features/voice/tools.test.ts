import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import toolsIndex from "../../../agent/tools.json"
import { tutorTools } from "./tools"

type Call = { method: string; url: string; body: unknown }
type Reply = { status?: number; body: unknown } | Error

const BACKEND = "http://127.0.0.1:8001/api/question-notebook/practice"
const RAG = "http://127.0.0.1:8811"
const tz = Intl.DateTimeFormat().resolvedOptions().timeZone

let calls: Call[]
let replies: Reply[]

/** Queue the fetch replies in order; every request is recorded in `calls`. */
function reply(...r: Reply[]) {
  replies.push(...r)
}

beforeEach(() => {
  calls = []
  replies = []
  vi.spyOn(console, "warn").mockImplementation(() => {})
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      calls.push({
        method: init?.method ?? "GET",
        url,
        body: init?.body ? JSON.parse(String(init.body)) : undefined,
      })
      const next = replies.shift()
      if (!next) throw new Error(`unexpected fetch ${url}`)
      if (next instanceof Error) throw next
      return new Response(JSON.stringify(next.body), { status: next.status ?? 200 })
    }),
  )
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

const entry = (id: number, extra: Record<string, unknown> = {}) => ({
  id,
  question: `Question ${id}?`,
  question_type: "single_choice",
  options: { A: "3", B: "4" },
  correct_answer: "B",
  explanation: "Two plus two is four.",
  categories: [],
  ...extra,
})

describe("get_next_question", () => {
  it("returns only the question fields, never the answer key", async () => {
    reply({ body: [{ entry: entry(7, { misconception: "adds wrong" }), state: { version: 3 } }] })

    const out = await tutorTools.get_next_question({})

    expect(JSON.parse(out)).toEqual({
      question_id: 7,
      question: "Question 7?",
      question_type: "single_choice",
      options: { A: "3", B: "4" },
    })
    expect(out).not.toMatch(/correct_answer|explanation|misconception|Two plus two|adds wrong/)
    expect(calls).toEqual([
      {
        method: "GET",
        url: `${BACKEND}/queue?limit=20&timezone=${encodeURIComponent(tz)}`,
        body: undefined,
      },
    ])
  })

  it("prefers an entry whose tags contain the domain, else the first", async () => {
    const queue = [
      { entry: entry(1, { categories: [{ id: 1, name: "Math" }] }), state: {} },
      { entry: entry(2, { categories: [{ id: 2, name: "Chemistry" }] }), state: {} },
    ]
    reply({ body: queue }, { body: queue })

    expect(JSON.parse(await tutorTools.get_next_question({ domain: "chemistry" })).question_id).toBe(2)
    expect(JSON.parse(await tutorTools.get_next_question({ domain: "history" })).question_id).toBe(1)
  })

  it("says so when nothing is due", async () => {
    reply({ body: [] })
    expect(await tutorTools.get_next_question({})).toBe("No practice questions are due right now.")
  })
})

describe("check_answer", () => {
  it("checks the answer before fetching the answer key", async () => {
    reply({ body: { correct: false } }, { body: { entry: entry(7, { misconception: "adds wrong" }), state: {} } })

    const out = JSON.parse(await tutorTools.check_answer({ question_id: 7, answer: "A" }))

    expect(calls.map((c) => [c.method, c.url, c.body])).toEqual([
      ["POST", `${BACKEND}/questions/7/check`, { answer: "A" }],
      ["GET", `${BACKEND}/questions/7`, undefined],
    ])
    expect(out).toEqual({
      correct: false,
      correct_answer: "B",
      explanation: "Two plus two is four.",
      misconception: "adds wrong",
    })
  })

  it("never fetches the answer key when the check fails", async () => {
    reply({ status: 500, body: { detail: "boom" } })

    expect(await tutorTools.check_answer({ question_id: "7", answer: "A" })).toBe(
      "Error: the practice service returned HTTP 500.",
    )
    expect(calls).toHaveLength(1)
  })

  it("omits misconception when the entry has none", async () => {
    reply({ body: { correct: true } }, { body: { entry: entry(7), state: {} } })

    const out = JSON.parse(await tutorTools.check_answer({ question_id: 7, answer: "B" }))

    expect(out).toEqual({ correct: true, correct_answer: "B", explanation: "Two plus two is four." })
  })
})

describe("record_review", () => {
  const dueAt = 1_790_000_000
  const expectedDue = new Date(dueAt * 1000).toLocaleString("en-US", {
    timeZone: tz,
    dateStyle: "medium",
    timeStyle: "short",
  })

  it("reads the version, posts the review, and returns the next due time", async () => {
    reply({ body: { entry: entry(7), state: { version: 4 } } }, { body: { due_at: dueAt } })

    const out = JSON.parse(await tutorTools.record_review({ question_id: 7, answer: "B", correct: true }))

    expect(calls.map((c) => [c.method, c.url])).toEqual([
      ["GET", `${BACKEND}/questions/7`],
      ["POST", `${BACKEND}/questions/7/review`],
    ])
    const body = calls[1].body as Record<string, unknown>
    expect(body).toMatchObject({ answer: "B", self_report: false, version: 4, rating: "good" })
    expect(body.request_id).toMatch(/^[A-Za-z0-9]{32}$/)
    expect(out).toEqual({ rating: "good", next_due: expectedDue })
  })

  it("rates an incorrect answer 'again' and treats a missing version as 0", async () => {
    reply({ body: { entry: entry(7), state: {} } }, { body: { due_at: dueAt } })

    await tutorTools.record_review({ question_id: 7, answer: "A", correct: false })

    expect(calls[1].body).toMatchObject({ version: 0, rating: "again" })
  })

  it("re-reads the version once after a 409 and retries with the same request id", async () => {
    reply(
      { body: { entry: entry(7), state: { version: 1 } } },
      { status: 409, body: { detail: "changed" } },
      { body: { entry: entry(7), state: { version: 2 } } },
      { body: { due_at: dueAt } },
    )

    const out = JSON.parse(await tutorTools.record_review({ question_id: 7, answer: "B", correct: true }))

    expect(calls.map((c) => c.method)).toEqual(["GET", "POST", "GET", "POST"])
    const [first, second] = [calls[1].body, calls[3].body] as Record<string, unknown>[]
    expect(first.version).toBe(1)
    expect(second.version).toBe(2)
    expect(second.request_id).toBe(first.request_id)
    expect(out.rating).toBe("good")
  })

  it("gives up after one retry", async () => {
    reply(
      { body: { entry: entry(7), state: { version: 1 } } },
      { status: 409, body: {} },
      { body: { entry: entry(7), state: { version: 2 } } },
      { status: 409, body: {} },
    )

    expect(await tutorTools.record_review({ question_id: 7, answer: "B", correct: true })).toBe(
      "Error: the practice service returned HTTP 409.",
    )
    expect(calls.filter((c) => c.method === "POST")).toHaveLength(2)
  })

  it("rejects a non-boolean correct without calling the backend", async () => {
    expect(await tutorTools.record_review({ question_id: 7, answer: "B", correct: "maybe" })).toBe(
      "Error: correct must be true or false.",
    )
    expect(calls).toHaveLength(0)
  })
})

describe("search_library", () => {
  const hit = (extra: Record<string, unknown>) => ({
    chunk_id: "c",
    doc_id: "d",
    path: "corpus/math/books/x.pdf",
    title: "Calculus",
    domain: "math",
    source_type: "book",
    page_start: 12,
    page_end: 12,
    heading_path: [],
    text: "short text",
    score: 1,
    ranks: { bm25: 1, vector: null },
    ...extra,
  })

  it("posts the query and maps hits for citation, truncating excerpts to 600 chars", async () => {
    const long = "x".repeat(900)
    reply({
      body: {
        mode_used: "bm25",
        hits: [hit({}), hit({ page_end: 14, text: long }), hit({ page_start: null, page_end: null })],
      },
    })

    const out = JSON.parse(await tutorTools.search_library({ query: "limits", domain: "math" }))

    expect(calls).toEqual([
      { method: "POST", url: `${RAG}/search`, body: { query: "limits", k: 5, domains: ["math"] } },
    ])
    expect(out.hits[0]).toEqual({
      title: "Calculus",
      domain: "math",
      source_type: "book",
      pages: "12",
      excerpt: "short text",
    })
    expect(out.hits[1].pages).toBe("12-14")
    expect(out.hits[1].excerpt).toHaveLength(600)
    expect(out.hits[1].excerpt.endsWith("…")).toBe(true)
    expect(out.hits[2].pages).toBe("")
  })

  it("leaves out domains when none is given", async () => {
    reply({ body: { mode_used: "bm25", hits: [] } })

    expect(JSON.parse(await tutorTools.search_library({ query: "limits" }))).toEqual({ hits: [] })
    expect(calls[0].body).toEqual({ query: "limits", k: 5 })
  })
})

describe("get_progress", () => {
  it("reads the summary for the local timezone", async () => {
    reply({
      body: {
        total: 10,
        mistakes: 2,
        due: 3,
        overdue: 1,
        reviewed_today: 4,
        next_due_at: null,
        day_end: 0,
        timezone: tz,
      },
    })

    expect(JSON.parse(await tutorTools.get_progress({}))).toEqual({
      total: 10,
      due: 3,
      overdue: 1,
      mistakes: 2,
      reviewed_today: 4,
      next_due: null,
    })
    expect(calls[0].url).toBe(`${BACKEND}/summary?timezone=${encodeURIComponent(tz)}`)
  })
})

describe("agent tool configs (apps/desktop/agent)", () => {
  type ToolConfig = {
    name: string
    type: string
    expects_response: boolean
    parameters: { properties: Record<string, { type: string }>; required: string[] }
  }
  const files = import.meta.glob<ToolConfig>("../../../agent/tool_configs/*.json", { eager: true, import: "default" })
  // The arguments each tool in tools.ts reads, and which of them it requires.
  const expected: Record<string, { properties: Record<string, string>; required: string[] }> = {
    get_next_question: { properties: { domain: "string" }, required: [] },
    check_answer: { properties: { question_id: "integer", answer: "string" }, required: ["question_id", "answer"] },
    record_review: {
      properties: { question_id: "integer", answer: "string", correct: "boolean" },
      required: ["question_id", "answer", "correct"],
    },
    search_library: { properties: { query: "string", domain: "string" }, required: ["query"] },
    get_progress: { properties: {}, required: [] },
  }

  it("declares exactly the client tools implemented here, with matching parameters", () => {
    const configs = toolsIndex.tools.map((t) => {
      expect(t.type).toBe("client")
      const config = files[`../../../agent/${t.config}`]
      expect(config, t.config).toBeDefined()
      return config
    })
    expect(configs.map((c) => c.name).sort()).toEqual(Object.keys(tutorTools).sort())
    for (const c of configs) {
      expect(c).toMatchObject({ type: "client", expects_response: true })
      const types = Object.fromEntries(Object.entries(c.parameters.properties).map(([k, v]) => [k, v.type]))
      expect({ properties: types, required: c.parameters.required }).toEqual(expected[c.name])
    }
  })
})

describe("error strings", () => {
  it("reports an unreachable backend", async () => {
    reply(new TypeError("Failed to fetch"))
    expect(await tutorTools.get_progress({})).toBe("Error: the practice service is not reachable.")
  })

  it("reports an unreachable library", async () => {
    reply(new TypeError("Failed to fetch"))
    expect(await tutorTools.search_library({ query: "x" })).toBe("Error: the library search is not reachable.")
  })

  it("reports a missing question", async () => {
    reply({ status: 404, body: { detail: "Question not found" } })
    expect(await tutorTools.check_answer({ question_id: 99, answer: "A" })).toBe(
      "Error: the practice service could not find that.",
    )
  })

  it("rejects bad arguments without calling any service", async () => {
    expect(await tutorTools.check_answer({ question_id: "abc", answer: "A" })).toBe(
      "Error: question_id must be the number returned by get_next_question.",
    )
    expect(await tutorTools.check_answer({ question_id: 7 })).toBe("Error: answer is required.")
    expect(await tutorTools.search_library({})).toBe("Error: query is required.")
    expect(calls).toHaveLength(0)
  })
})
