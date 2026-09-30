import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import toolsIndex from "../../../agent/tools.json"
import { type TutorEvent, onTutorEvent, tutorTools } from "./tools"

type Call = { method: string; url: string; body: unknown }
type Reply = { status?: number; body: unknown } | Error

const BACKEND = "http://127.0.0.1:8001/api/question-notebook/practice"
const LIBRARY = "http://127.0.0.1:8001/api/library/search"
const tz = Intl.DateTimeFormat().resolvedOptions().timeZone

let calls: Call[]
let replies: Reply[]
let events: TutorEvent[]
let unsubscribe: () => void

/** Queue the fetch replies in order; every request is recorded in `calls`. */
function reply(...r: Reply[]) {
  replies.push(...r)
}

beforeEach(() => {
  calls = []
  replies = []
  events = []
  unsubscribe = onTutorEvent((e) => events.push(e))
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
  unsubscribe()
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
  misconception: "Counts the starting number twice.",
  hints: ["Count on from two.", "What is one more than three?"],
  categories: [],
  ...extra,
})

const dueAt = 1_790_000_000
const expectedDue = new Date(dueAt * 1000).toLocaleString("en-US", {
  timeZone: tz,
  dateStyle: "medium",
  timeStyle: "short",
})

describe("get_next_question", () => {
  it("opens an attempt and returns only the question fields, never the answer key", async () => {
    reply(
      { body: [{ entry: entry(7), state: { version: 3 } }] },
      { body: { attempt_id: "att-1", rungs_available: 2 } },
    )

    const out = await tutorTools.get_next_question({})

    expect(JSON.parse(out)).toEqual({
      attempt_id: "att-1",
      question_id: 7,
      question: "Question 7?",
      question_type: "single_choice",
      options: { A: "3", B: "4" },
      rungs_available: 2,
    })
    expect(out).not.toMatch(/correct_answer|explanation|misconception|hints|Two plus two|Counts the|Count on/)
    expect(calls).toEqual([
      {
        method: "GET",
        url: `${BACKEND}/queue?limit=20&timezone=${encodeURIComponent(tz)}`,
        body: undefined,
      },
      { method: "POST", url: `${BACKEND}/questions/7/attempts`, body: undefined },
    ])
    expect(events).toEqual([
      {
        type: "question",
        attempt_id: "att-1",
        question_id: 7,
        question: "Question 7?",
        question_type: "single_choice",
        options: { A: "3", B: "4" },
      },
    ])
    expect(JSON.stringify(events)).not.toMatch(/Two plus two|Counts the|Count on/)
  })

  it("prefers an entry whose tags contain the domain, else the first", async () => {
    const queue = [
      { entry: entry(1, { categories: [{ id: 1, name: "Math" }] }), state: {} },
      { entry: entry(2, { categories: [{ id: 2, name: "Chemistry" }] }), state: {} },
    ]
    const attempt = { body: { attempt_id: "a", rungs_available: 0 } }
    reply({ body: queue }, attempt, { body: queue }, attempt)

    expect(JSON.parse(await tutorTools.get_next_question({ domain: "chemistry" })).question_id).toBe(2)
    expect(calls[1].url).toBe(`${BACKEND}/questions/2/attempts`)
    expect(JSON.parse(await tutorTools.get_next_question({ domain: "history" })).question_id).toBe(1)
    expect(calls[3].url).toBe(`${BACKEND}/questions/1/attempts`)
  })

  it("says so when nothing is due, without opening an attempt", async () => {
    reply({ body: [] })
    expect(await tutorTools.get_next_question({})).toBe("No practice questions are due right now.")
    expect(calls).toHaveLength(1)
    expect(events).toEqual([])
  })

  it("emits no event when the attempt cannot be opened", async () => {
    reply({ body: [{ entry: entry(7), state: {} }] }, { status: 500, body: {} })
    expect(await tutorTools.get_next_question({})).toBe("Error: the practice service returned HTTP 500.")
    expect(events).toEqual([])
  })
})

describe("set_confidence", () => {
  it("posts the level to the attempt and emits a confidence event", async () => {
    reply({ body: { ok: true } })

    const out = JSON.parse(await tutorTools.set_confidence({ attempt_id: "att-1", level: 3 }))

    expect(calls).toEqual([{ method: "POST", url: `${BACKEND}/attempts/att-1/confidence`, body: { level: 3 } }])
    expect(out).toEqual({ recorded: true, level: 3 })
    expect(events).toEqual([{ type: "confidence", attempt_id: "att-1", level: 3 }])
  })

  it("accepts a numeric string and rejects levels outside 1-4", async () => {
    reply({ body: { ok: true } })
    await tutorTools.set_confidence({ attempt_id: "att-1", level: "4" })
    expect(calls[0].body).toEqual({ level: 4 })

    for (const level of [0, 5, 2.5, "sure", undefined]) {
      expect(await tutorTools.set_confidence({ attempt_id: "att-1", level })).toBe(
        "Error: level must be a whole number from 1 to 4.",
      )
    }
    expect(calls).toHaveLength(1)
  })

  it("escapes the attempt id in the path", async () => {
    reply({ body: { ok: true } })
    await tutorTools.set_confidence({ attempt_id: "a/b", level: 1 })
    expect(calls[0].url).toBe(`${BACKEND}/attempts/a%2Fb/confidence`)
  })
})

describe("get_hint", () => {
  it("returns the server's rung and text and emits a hint event", async () => {
    reply({ body: { rung: 1, text: "Count on from two.", is_answer: false } })

    const out = JSON.parse(await tutorTools.get_hint({ attempt_id: "att-1" }))

    expect(calls).toEqual([{ method: "POST", url: `${BACKEND}/attempts/att-1/hint`, body: undefined }])
    expect(out).toEqual({ rung: 1, text: "Count on from two.", is_answer: false })
    expect(events).toEqual([
      { type: "hint", attempt_id: "att-1", rung: 1, text: "Count on from two.", is_answer: false },
    ])
  })

  it("tells the tutor to write a conceptual nudge without the answer when a rung has no text", async () => {
    reply({ body: { rung: 2, text: null, is_answer: false } })

    const out = JSON.parse(await tutorTools.get_hint({ attempt_id: "att-1" }))

    expect(out.rung).toBe(2)
    expect(out.text).toBeNull()
    expect(out.is_answer).toBe(false)
    expect(out.instruction).toMatch(/short conceptual nudge/)
    expect(out.instruction).toMatch(/without (stating|giving) the answer/)
    expect(events).toEqual([{ type: "hint", attempt_id: "att-1", rung: 2, text: null, is_answer: false }])
  })

  it("tells the tutor to work a parallel problem when the worked-example rung has no text", async () => {
    reply({ body: { rung: 4, text: null, is_answer: false } })

    const out = JSON.parse(await tutorTools.get_hint({ attempt_id: "att-1" }))

    expect(out.instruction).toMatch(/parallel problem/)
    expect(out.instruction).toMatch(/not this one/)
  })

  it("reports a closed attempt as an error sentence", async () => {
    reply({ status: 409, body: { detail: "attempt already checked" } })
    expect(await tutorTools.get_hint({ attempt_id: "att-1" })).toBe("Error: the practice service returned HTTP 409.")
    expect(events).toEqual([])
  })
})

describe("check_answer", () => {
  it("checks the attempt and returns the verdict with the answer key", async () => {
    reply({
      body: {
        correct: false,
        rating: "again",
        correct_answer: "B",
        explanation: "Two plus two is four.",
        misconception: "Counts the starting number twice.",
      },
    })

    const out = JSON.parse(await tutorTools.check_answer({ attempt_id: "att-1", answer: " A " }))

    expect(calls).toEqual([{ method: "POST", url: `${BACKEND}/attempts/att-1/check`, body: { answer: "A" } }])
    expect(out).toEqual({
      correct: false,
      correct_answer: "B",
      explanation: "Two plus two is four.",
      misconception: "Counts the starting number twice.",
    })
    expect(events).toEqual([
      {
        type: "checked",
        attempt_id: "att-1",
        correct: false,
        correct_answer: "B",
        explanation: "Two plus two is four.",
        misconception: "Counts the starting number twice.",
      },
    ])
  })

  it("omits a missing misconception for the model and sends null in the event", async () => {
    reply({
      body: { correct: null, rating: null, correct_answer: "Light to chemical energy.", explanation: "", misconception: null },
    })

    const out = JSON.parse(await tutorTools.check_answer({ attempt_id: "att-1", answer: "It makes sugar" }))

    expect(out).toEqual({ correct: null, correct_answer: "Light to chemical energy.", explanation: "" })
    expect(events[0]).toMatchObject({ type: "checked", correct: null, misconception: null })
  })
})

describe("record_review", () => {
  it("reads the version and posts the review with the attempt, never a client rating", async () => {
    reply(
      { body: { entry: entry(7), state: { version: 4 } } },
      { body: { rating: "hard", due_at: dueAt, correct: true } },
    )

    const out = JSON.parse(await tutorTools.record_review({ attempt_id: "att-1", question_id: 7, answer: "B" }))

    expect(calls.map((c) => [c.method, c.url])).toEqual([
      ["GET", `${BACKEND}/questions/7`],
      ["POST", `${BACKEND}/questions/7/review`],
    ])
    const body = calls[1].body as Record<string, unknown>
    expect(Object.keys(body).sort()).toEqual(["answer", "attempt_id", "request_id", "version"])
    expect(body).toMatchObject({ attempt_id: "att-1", answer: "B", version: 4 })
    expect(body.request_id).toMatch(/^[A-Za-z0-9]{32}$/)
    expect(out).toEqual({ rating: "hard", next_due: expectedDue })
    expect(events).toEqual([{ type: "reviewed", attempt_id: "att-1", rating: "hard", next_due: expectedDue }])
  })

  it("sends judged_correct when given and treats a missing version as 0", async () => {
    reply({ body: { entry: entry(7), state: {} } }, { body: { rating: "again", due_at: null } })

    const out = JSON.parse(
      await tutorTools.record_review({ attempt_id: "att-1", question_id: "7", answer: "It makes sugar", judged_correct: "false" }),
    )

    expect(calls[1].body).toMatchObject({ version: 0, judged_correct: false })
    expect(calls[1].body).not.toHaveProperty("rating")
    expect(out).toEqual({ rating: "again", next_due: null })
  })

  it("re-reads the version once after a 409 and retries with the same request id", async () => {
    reply(
      { body: { entry: entry(7), state: { version: 1 } } },
      { status: 409, body: { detail: "changed" } },
      { body: { entry: entry(7), state: { version: 2 } } },
      { body: { rating: "good", due_at: dueAt } },
    )

    const out = JSON.parse(await tutorTools.record_review({ attempt_id: "att-1", question_id: 7, answer: "B" }))

    expect(calls.map((c) => c.method)).toEqual(["GET", "POST", "GET", "POST"])
    const [first, second] = [calls[1].body, calls[3].body] as Record<string, unknown>[]
    expect(first.version).toBe(1)
    expect(second.version).toBe(2)
    expect(second.request_id).toBe(first.request_id)
    expect(second.attempt_id).toBe("att-1")
    expect(out.rating).toBe("good")
    expect(events).toHaveLength(1)
  })

  it("gives up after one retry", async () => {
    reply(
      { body: { entry: entry(7), state: { version: 1 } } },
      { status: 409, body: {} },
      { body: { entry: entry(7), state: { version: 2 } } },
      { status: 409, body: {} },
    )

    expect(await tutorTools.record_review({ attempt_id: "att-1", question_id: 7, answer: "B" })).toBe(
      "Error: the practice service returned HTTP 409.",
    )
    expect(calls.filter((c) => c.method === "POST")).toHaveLength(2)
    expect(events).toEqual([])
  })

  it("rejects bad arguments without calling the backend", async () => {
    expect(
      await tutorTools.record_review({ attempt_id: "att-1", question_id: 7, answer: "B", judged_correct: "maybe" }),
    ).toBe("Error: judged_correct must be true or false.")
    expect(await tutorTools.record_review({ question_id: 7, answer: "B" })).toBe(
      "Error: attempt_id must be the attempt_id returned by get_next_question.",
    )
    expect(calls).toHaveLength(0)
  })
})

describe("events", () => {
  it("are emitted in the order the tools ran, and stop after unsubscribing", async () => {
    reply(
      { body: [{ entry: entry(7), state: {} }] },
      { body: { attempt_id: "att-1", rungs_available: 2 } },
      { body: { rung: 1, text: "Count on from two.", is_answer: false } },
      { body: { ok: true } },
      { body: { correct: true, rating: "good", correct_answer: "B", explanation: "e", misconception: null } },
      { body: { entry: entry(7), state: { version: 1 } } },
      { body: { rating: "good", due_at: dueAt } },
      { body: { ok: true } },
    )

    await tutorTools.get_next_question({})
    await tutorTools.get_hint({ attempt_id: "att-1" })
    await tutorTools.set_confidence({ attempt_id: "att-1", level: 2 })
    await tutorTools.check_answer({ attempt_id: "att-1", answer: "B" })
    await tutorTools.record_review({ attempt_id: "att-1", question_id: 7, answer: "B" })
    unsubscribe()
    await tutorTools.set_confidence({ attempt_id: "att-1", level: 2 })

    expect(events.map((e) => e.type)).toEqual(["question", "hint", "confidence", "checked", "reviewed"])
  })

  it("keeps the tool result when a listener throws", async () => {
    const off = onTutorEvent(() => {
      throw new Error("render failed")
    })
    reply({ body: { ok: true } })

    expect(JSON.parse(await tutorTools.set_confidence({ attempt_id: "att-1", level: 1 }))).toEqual({
      recorded: true,
      level: 1,
    })
    expect(events).toHaveLength(1)
    off()
  })
})

describe("search_library", () => {
  // A fresh module per test: the open attempt is module state that other tests leave behind.
  let tools: typeof tutorTools
  beforeEach(async () => {
    vi.resetModules()
    tools = (await import("./tools")).tutorTools
  })

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
    ranks: { bm25: 1, vector: null, final: 1 },
    section: ["[c:c] short text"],
    ...extra,
  })
  const packOf = (hits: unknown[]) => ({ pack: hits, queries: ["limits"], mode_used: "bm25", withheld: 0 })

  it("posts the query to DeepTutor's library search and maps the pack, truncating excerpts to 600 chars", async () => {
    const long = "x".repeat(900)
    reply({
      body: packOf([hit({}), hit({ page_end: 14, text: long }), hit({ page_start: null, page_end: null })]),
    })

    const out = JSON.parse(await tools.search_library({ query: "limits", domain: "math" }))

    expect(calls).toEqual([
      { method: "POST", url: LIBRARY, body: { query: "limits", k: 5, domains: ["math"] } },
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

  it("leaves out domains and attempt_id when there are none", async () => {
    reply({ body: packOf([]) })

    expect(JSON.parse(await tools.search_library({ query: "limits" }))).toEqual({ hits: [] })
    expect(calls[0].body).toEqual({ query: "limits", k: 5 })
  })

  it("sends the open attempt_id while an attempt is open, and stops after check_answer closes it", async () => {
    reply(
      { body: [{ entry: entry(7), state: { version: 3 } }] },
      { body: { attempt_id: "att-1", rungs_available: 2 } },
      { body: packOf([]) },
      { body: { correct: true, correct_answer: "B", explanation: "", misconception: null } },
      { body: packOf([]) },
    )

    await tools.get_next_question({})
    await tools.search_library({ query: "addition" })
    await tools.check_answer({ attempt_id: "att-1", answer: "B" })
    await tools.search_library({ query: "addition" })

    const searches = calls.filter((c) => c.url === LIBRARY).map((c) => c.body)
    expect(searches).toEqual([
      { query: "addition", k: 5, attempt_id: "att-1" },
      { query: "addition", k: 5 },
    ])
  })

  it("keeps the open attempt when a different attempt is checked", async () => {
    reply(
      { body: [{ entry: entry(7), state: { version: 3 } }] },
      { body: { attempt_id: "att-2", rungs_available: 0 } },
      { body: { correct: false, correct_answer: "B", explanation: "", misconception: null } },
      { body: packOf([]) },
    )

    await tools.get_next_question({})
    await tools.check_answer({ attempt_id: "att-old", answer: "A" })
    await tools.search_library({ query: "addition" })

    expect(calls[calls.length - 1].body).toEqual({ query: "addition", k: 5, attempt_id: "att-2" })
  })
})

describe("get_progress", () => {
  const summary = {
    total: 10,
    mistakes: 2,
    due: 3,
    overdue: 1,
    reviewed_today: 4,
    next_due_at: null,
    day_end: 0,
    timezone: tz,
  }

  it("reads the summary and the 30-day calibration for the local timezone", async () => {
    const calibration = [
      { level: 1, attempts: 2, correct: 0 },
      { level: 4, attempts: 5, correct: 4 },
    ]
    reply({ body: summary }, { body: { timezone: tz, days: 30, calibration, assistance: {} } })

    expect(JSON.parse(await tutorTools.get_progress({}))).toEqual({
      total: 10,
      due: 3,
      overdue: 1,
      mistakes: 2,
      reviewed_today: 4,
      next_due: null,
      calibration_30_days: calibration,
    })
    expect(calls.map((c) => c.url)).toEqual([
      `${BACKEND}/summary?timezone=${encodeURIComponent(tz)}`,
      `${BACKEND}/analytics?timezone=${encodeURIComponent(tz)}&days=30`,
    ])
  })

  it("leaves calibration out when the backend does not report it", async () => {
    reply({ body: summary }, { body: { timezone: tz, days: 30 } })

    expect(JSON.parse(await tutorTools.get_progress({}))).not.toHaveProperty("calibration_30_days")
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
    set_confidence: { properties: { attempt_id: "string", level: "integer" }, required: ["attempt_id", "level"] },
    get_hint: { properties: { attempt_id: "string" }, required: ["attempt_id"] },
    check_answer: { properties: { attempt_id: "string", answer: "string" }, required: ["attempt_id", "answer"] },
    record_review: {
      properties: { attempt_id: "string", question_id: "integer", answer: "string", judged_correct: "boolean" },
      required: ["attempt_id", "question_id", "answer"],
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

  it("reports a missing attempt", async () => {
    reply({ status: 404, body: { detail: "Attempt not found" } })
    expect(await tutorTools.check_answer({ attempt_id: "gone", answer: "A" })).toBe(
      "Error: the practice service could not find that.",
    )
  })

  it("rejects bad arguments without calling any service", async () => {
    expect(await tutorTools.check_answer({ attempt_id: "  ", answer: "A" })).toBe(
      "Error: attempt_id must be the attempt_id returned by get_next_question.",
    )
    expect(await tutorTools.check_answer({ attempt_id: "att-1" })).toBe("Error: answer is required.")
    expect(await tutorTools.get_hint({})).toBe(
      "Error: attempt_id must be the attempt_id returned by get_next_question.",
    )
    expect(await tutorTools.record_review({ attempt_id: "att-1", question_id: "abc", answer: "B" })).toBe(
      "Error: question_id must be the number returned by get_next_question.",
    )
    expect(await tutorTools.search_library({})).toBe("Error: query is required.")
    expect(calls).toHaveLength(0)
    expect(events).toEqual([])
  })
})
