import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import toolsIndex from "../../../agent/tools.json"
import { CHOICE, DUE_AT, ESSAY, type FakeItem, type FakeStudy, SECRETS, TYPED, fakeStudyServer } from "./fakeStudy"
import { type PanelAction, STORAGE_KEY, type VoicePractice, createVoicePractice, panelActions } from "./practice"
import { type TutorTools, createTutorTools, screenMessage } from "./tools"

const STUDY = "http://127.0.0.1:8001/api/study"
const PRACTICE = "http://127.0.0.1:8001/api/question-notebook/practice"
const LIBRARY = "http://127.0.0.1:8001/api/library/search"
const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
const expectedDue = new Date(DUE_AT * 1000).toLocaleString("en-US", { timeZone: tz, dateStyle: "medium", timeStyle: "short" })
/** Hints the tests never ask for before they check for them. */
const UNASKED_HINTS = /HINT-TWO-IBIS|HINT-ONE-HERON/

let server: FakeStudy
let storage: Map<string, string>
let practice: VoicePractice
let tools: TutorTools

/** A fresh screen (as after a restart) over the same server and storage. */
function newScreen() {
  practice = createVoicePractice()
  tools = createTutorTools(practice)
}

function use(items?: FakeItem[]) {
  server = fakeStudyServer(items)
  vi.stubGlobal("fetch", server.fetch)
  newScreen()
}

beforeEach(() => {
  storage = new Map()
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => storage.get(k) ?? null,
    setItem: (k: string, v: string) => void storage.set(k, v),
    removeItem: (k: string) => void storage.delete(k),
  })
  vi.spyOn(console, "warn").mockImplementation(() => {})
  use()
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

/** The typed controls, as the learner uses them on screen. */
const screen = () => panelActions(practice.session!)
const sent = () => server.commands().map((c) => [c.body?.kind, c.source])
const kinds = () => server.commands().map((c) => c.body?.kind)
const lastCommand = () => server.commands()[server.commands().length - 1]
const status = () => practice.session?.state.attempt?.status

/** Opens the first due item, answers it on screen and records confidence by voice. */
async function readyToCheck(response: string) {
  await tools.get_next_question({})
  await screen().answer(response)
  expect(await tools.set_confidence({ level: 3 })).toBe('{"recorded":true,"level":3}')
}

describe("get_next_question", () => {
  it("starts the session, opens the next due item as voice_tool and returns only public fields", async () => {
    const out = await tools.get_next_question({})

    expect(JSON.parse(out)).toEqual({
      attempt_id: practice.session?.state.attempt?.attempt_id,
      question_id: 7,
      question: CHOICE.prompt,
      question_type: "single_choice",
      options: CHOICE.options,
      rungs_available: 2,
    })
    expect(out).not.toMatch(SECRETS)
    expect(out).not.toMatch(/HINT-|Think about/)
    expect(server.calls.map((c) => [c.method, c.url.replace(STUDY, ""), c.source])).toEqual([
      ["POST", "/commands", "ui"],
      ["GET", "/queue?limit=20", null],
      ["POST", "/commands", "voice_tool"],
    ])
    expect(kinds()).toEqual(["start_session", "open_item"])
    expect(server.commands()[0].body).toMatchObject({ kind: "start_session", payload: { mode: "practice" } })
    expect(lastCommand().body).toMatchObject({ payload: { entry_id: 7 } })
    expect(storage.get(STORAGE_KEY)).toBe(practice.session?.state.session.session_id)
  })

  it("asks the queue for the domain's tag", async () => {
    expect(JSON.parse(await tools.get_next_question({ domain: "math" })).question_id).toBe(8)
    expect(server.calls[1].url).toBe(`${STUDY}/queue?limit=20&tag=math`)
  })

  it("says so when nothing is due, without opening an item", async () => {
    expect(await tools.get_next_question({ domain: "history" })).toBe("No history practice questions are due right now.")
    use([])
    expect(await tools.get_next_question({})).toBe("No practice questions are due right now.")
    expect(kinds()).toEqual(["start_session"])
  })

  it("asks to finish a live question first and opens nothing", async () => {
    await tools.get_next_question({})
    expect(await tools.get_next_question({})).toBe(
      "Finish the current question first: it is waiting for the learner's answer, confirmed on screen (propose_answer for a spoken one).",
    )
    expect(kinds()).toEqual(["start_session", "open_item"])
  })

  it("resumes the stored session after a restart instead of starting another", async () => {
    await tools.get_next_question({})
    newScreen()

    expect(await tools.get_next_question({})).toMatch(/^Finish the current question first/)
    expect(kinds()).toEqual(["start_session", "open_item"])
    expect(server.calls[server.calls.length - 1].url).toBe(`${STUDY}/sessions/${storage.get(STORAGE_KEY)}?after_seq=0`)
    expect(practice.getSnapshot()?.attempt?.status).toBe("open")
  })

  it("starts fresh when the stored session has closed", async () => {
    await tools.get_next_question({})
    const first = storage.get(STORAGE_KEY)
    server.closeSession()
    newScreen()

    expect(await practice.current()).toBeNull()
    expect(storage.has(STORAGE_KEY)).toBe(false)
    expect(JSON.parse(await tools.get_next_question({})).question_id).toBe(7)
    expect(kinds()).toEqual(["start_session", "open_item", "start_session", "open_item"])
    expect(storage.get(STORAGE_KEY)).not.toBe(first)
  })

  it("ignores a stored id that is not a session id", async () => {
    storage.set(STORAGE_KEY, "not-a-session")
    expect(await practice.current()).toBeNull()
    expect(server.calls).toHaveLength(0)
  })
})

describe("propose_answer", () => {
  it("proposes the learner's exact words for them to confirm, and never commits", async () => {
    await tools.get_next_question({})

    const out = await tools.propose_answer({ transcript: " carbon dioxide " })

    expect(out).toMatch(/confirm/)
    expect(out).toMatch(/"carbon dioxide"/)
    expect(lastCommand()).toMatchObject({
      source: "voice_tool",
      body: { kind: "propose_response", payload: { raw_transcript: "carbon dioxide", input_mode: "spoken" } },
    })
    expect(status()).toBe("open")
    expect(practice.session?.state.attempt?.pending_proposal?.raw_transcript).toBe("carbon dioxide")
    expect(kinds()).not.toContain("commit_response")
  })

  it("needs a transcript and an open question", async () => {
    expect(await tools.propose_answer({})).toBe("Error: transcript is required.")
    expect(await tools.propose_answer({ transcript: "B" })).toBe(
      "Error: there is no open question; call get_next_question first.",
    )
    expect(server.calls).toHaveLength(0)
  })
})

describe("set_confidence", () => {
  it("is refused until the learner has confirmed an answer on screen", async () => {
    await tools.get_next_question({})
    await tools.propose_answer({ transcript: "bee" })

    expect(await tools.set_confidence({ level: 3 })).toBe(
      "Error: confidence can be set only after the learner has confirmed an answer on screen; the question is waiting for the learner's answer, confirmed on screen (propose_answer for a spoken one).",
    )
    expect(kinds()).not.toContain("record_confidence")

    await screen().confirm("bee", "B")
    expect(JSON.parse(await tools.set_confidence({ level: "4" }))).toEqual({ recorded: true, level: 4 })
    expect(lastCommand()).toMatchObject({ source: "voice_tool", body: { kind: "record_confidence", payload: { level: 4 } } })
    expect(status()).toBe("ready")
  })

  it("rejects levels outside 1-4 without calling the backend", async () => {
    for (const level of [0, 5, 2.5, "sure", undefined]) {
      expect(await tools.set_confidence({ level })).toBe("Error: level must be a whole number from 1 to 4.")
    }
    expect(server.calls).toHaveLength(0)
  })
})

describe("get_hint", () => {
  it("returns only the rung asked for, one per call", async () => {
    await tools.get_next_question({})

    const first = await tools.get_hint({})
    expect(JSON.parse(first)).toEqual({ rung: 1, text: CHOICE.hints[0] })
    expect(first).not.toMatch(UNASKED_HINTS)
    expect(first).not.toMatch(SECRETS)
    expect(lastCommand()).toMatchObject({ source: "voice_tool", body: { kind: "request_assistance", payload: { kind: "hint" } } })

    expect(JSON.parse(await tools.get_hint({}))).toEqual({ rung: 2, text: "HINT-TWO-IBIS" })
  })

  it("tells the tutor what to do for rungs with no written text, and when the ladder is used up", async () => {
    use([{ ...CHOICE, hints: [] }])
    await tools.get_next_question({})

    const nudge = JSON.parse(await tools.get_hint({}))
    expect(nudge).toMatchObject({ rung: 1, text: null })
    expect(nudge.instruction).toMatch(/short conceptual nudge/)
    expect(nudge.instruction).toMatch(/without (stating|giving) the answer/)
    await tools.get_hint({})
    await tools.get_hint({})
    const example = JSON.parse(await tools.get_hint({}))
    expect(example.rung).toBe(4)
    expect(example.instruction).toMatch(/parallel problem/)
    expect(example.instruction).toMatch(/not this one/)

    expect(await tools.get_hint({})).toBe(
      "Error: that step is not possible at this point; the question is waiting for the learner's answer, confirmed on screen (propose_answer for a spoken one).",
    )
  })
})

describe("check_answer", () => {
  it("sends no answer, and keeps the answer hidden after a wrong one", async () => {
    await readyToCheck("A")

    const out = await tools.check_answer({})

    expect(lastCommand()).toMatchObject({ source: "voice_tool", body: { kind: "check", payload: {} } })
    const result = JSON.parse(out)
    expect(result).toMatchObject({ verdict: "incorrect", diagnostic: "Not the expected answer." })
    expect(result).not.toHaveProperty("correct_answer")
    expect(result).not.toHaveProperty("explanation")
    expect(result.instruction).toMatch(/try again/)
    expect(result.instruction).toMatch(/hint/)
    expect(result.instruction).toMatch(/give up/)
    expect(out).not.toMatch(SECRETS)
    expect(out).not.toMatch(UNASKED_HINTS)
    expect(status()).toBe("remediating")
  })

  it("returns the released answer, explanation, misconception and the review after a correct one", async () => {
    await readyToCheck("B")

    expect(JSON.parse(await tools.check_answer({}))).toEqual({
      verdict: "correct",
      closed: "correct",
      correct_answer: "B",
      explanation: CHOICE.explanation,
      misconception: CHOICE.misconception,
      review: "Good, correct without help",
      next_due: expectedDue,
    })
  })

  it("hands an unverifiable answer to the learner's judgement on screen, with the reference answer", async () => {
    use([ESSAY])
    await readyToCheck("Water moves to the saltier side.")

    const result = JSON.parse(await tools.check_answer({}))

    expect(result).toMatchObject({ verdict: "unverifiable", correct_answer: ESSAY.key, explanation: ESSAY.explanation })
    expect(result.instruction).toMatch(/judges their answer/)
    expect(result.instruction).toMatch(/on screen/)
    expect(result).not.toHaveProperty("closed")
    expect(status()).toBe("awaiting_self_assessment")
  })

  it("says what the question is waiting for when asked too early", async () => {
    await tools.get_next_question({})
    await screen().answer("B")

    expect(await tools.check_answer({})).toBe(
      "Error: that step is not possible at this point; the question is waiting for the learner's confidence (set_confidence).",
    )
  })
})

describe("give_up", () => {
  it("reveals the answer only once the server releases it", async () => {
    const results: string[] = []
    await tools.get_next_question({})
    results.push(await tools.get_hint({}))
    await screen().answer("A")
    await tools.set_confidence({ level: 2 })
    results.push(await tools.check_answer({}))
    for (const r of results) expect(r).not.toMatch(SECRETS)

    expect(JSON.parse(await tools.give_up({}))).toEqual({
      verdict: "incorrect",
      diagnostic: "Not the expected answer.",
      closed: "revealed",
      correct_answer: "B",
      explanation: CHOICE.explanation,
      misconception: CHOICE.misconception,
      review: "Again, because the first answer was not correct",
      next_due: expectedDue,
    })
    expect(lastCommand()).toMatchObject({ source: "voice_tool", body: { kind: "give_up" } })
  })

  it("counts giving up before any answer as not knowing it", async () => {
    use([TYPED])
    await tools.get_next_question({})

    expect(JSON.parse(await tools.give_up({}))).toMatchObject({
      closed: "revealed",
      correct_answer: TYPED.key,
      review: "Again, because you did not know it",
    })
  })
})

describe("sources", () => {
  it("sends every tool command as voice_tool; only the screen commits", async () => {
    await tools.get_next_question({})
    await tools.get_hint({})
    await tools.propose_answer({ transcript: "oxygen" })
    await screen().confirm("oxygen", "A")
    await tools.set_confidence({ level: 4 })
    await tools.check_answer({})
    await tools.give_up({})

    expect(sent()).toEqual([
      ["start_session", "ui"],
      ["open_item", "voice_tool"],
      ["request_assistance", "voice_tool"],
      ["propose_response", "voice_tool"],
      ["commit_response", "ui"],
      ["record_confidence", "voice_tool"],
      ["check", "voice_tool"],
      ["give_up", "voice_tool"],
    ])
  })
})

describe("screenMessage", () => {
  it("tells the tutor what the learner did on screen, with no answer before its release", async () => {
    const heard: [PanelAction, string][] = []
    await tools.get_next_question({})
    const actions = panelActions(practice.session!, (action, attempt) => heard.push([action, screenMessage(action, attempt)]))

    await actions.hint()
    await actions.answer("C")
    await actions.confidence(4)
    await actions.check()
    await actions.repair("A")
    await actions.giveUp()

    const before = heard.slice(0, -1)
    expect(before.map(([a]) => a)).toEqual(["hint", "answer", "confidence", "check", "repair"])
    for (const [, message] of before) {
      expect(message).toMatch(/^On screen: /)
      expect(message).not.toMatch(SECRETS)
      expect(message).not.toMatch(UNASKED_HINTS)
    }
    expect(before[1][1]).toBe('On screen: the learner confirmed their answer: "C".')
    expect(before[2][1]).toBe("On screen: confidence 4, certain.")
    const [action, last] = heard[heard.length - 1]
    expect(action).toBe("give_up")
    expect(last).toMatch(/EXPLANATION-OCTOPUS/)
  })
})

describe("errors", () => {
  it("refreshes after a conflict and asks to try again", async () => {
    await tools.get_next_question({})
    server.failNext("conflict", 409)

    expect(await tools.get_hint({})).toBe("Error: the session changed; state refreshed, try again.")
    expect(server.calls[server.calls.length - 1].url).toMatch(`${STUDY}/sessions/`)
  })

  it("reports an unreachable or unavailable practice service", async () => {
    await tools.get_next_question({})
    server.dropNext(3)
    expect(await tools.get_hint({})).toBe("Error: the practice service is not reachable.")
    server.failNext("unavailable", 503)
    expect(await tools.get_hint({})).toBe("Error: the practice service is unavailable right now.")
  })

  it("never throws, even when the tutor calls a tool with no parameters object", async () => {
    await expect(tools.check_answer(undefined as unknown as Record<string, unknown>)).resolves.toBe(
      "Error: there is no open question; call get_next_question first.",
    )
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
    ranks: { bm25: 1, vector: null, final: 1 },
    section: ["[c:c] short text"],
    ...extra,
  })
  const packOf = (hits: unknown[]) => ({ body: { pack: hits, queries: ["limits"], mode_used: "bm25", withheld: 0 } })
  const searches = () => server.calls.filter((c) => c.url === LIBRARY).map((c) => c.body)

  it("posts the query to DeepTutor's library search and maps the pack, truncating excerpts to 600 chars", async () => {
    server.replies.push(packOf([hit({}), hit({ page_end: 14, text: "x".repeat(900) }), hit({ page_start: null, page_end: null })]))

    const out = JSON.parse(await tools.search_library({ query: "limits", domain: "math" }))

    expect(searches()).toEqual([{ query: "limits", k: 5, domains: ["math"] }])
    expect(out.hits[0]).toEqual({ title: "Calculus", domain: "math", source_type: "book", pages: "12", excerpt: "short text" })
    expect(out.hits[1].pages).toBe("12-14")
    expect(out.hits[1].excerpt).toHaveLength(600)
    expect(out.hits[1].excerpt.endsWith("…")).toBe(true)
    expect(out.hits[2].pages).toBe("")
  })

  it("sends the live attempt_id, and stops once the attempt has closed", async () => {
    server.replies.push(packOf([]), packOf([]), packOf([]))
    await tools.search_library({ query: "gases" })
    await readyToCheck("B")
    const attemptId = practice.session?.state.attempt?.attempt_id
    await tools.search_library({ query: "gases" })
    await tools.check_answer({})
    await tools.search_library({ query: "gases" })

    expect(searches()).toEqual([
      { query: "gases", k: 5 },
      { query: "gases", k: 5, attempt_id: attemptId },
      { query: "gases", k: 5 },
    ])
  })

  it("reports an unreachable library", async () => {
    server.replies.push(new TypeError("Failed to fetch"))
    expect(await tools.search_library({ query: "x" })).toBe("Error: the library search is not reachable.")
    expect(await tools.search_library({})).toBe("Error: query is required.")
  })
})

describe("get_progress", () => {
  const summary = { total: 10, mistakes: 2, due: 3, overdue: 1, reviewed_today: 4, next_due_at: null, day_end: 0, timezone: tz }

  it("reads the summary and the 30-day calibration for the local timezone", async () => {
    const calibration = [
      { level: 1, attempts: 2, correct: 0 },
      { level: 4, attempts: 5, correct: 4 },
    ]
    server.replies.push({ body: summary }, { body: { timezone: tz, days: 30, calibration, assistance: {} } })

    expect(JSON.parse(await tools.get_progress({}))).toEqual({
      total: 10,
      due: 3,
      overdue: 1,
      mistakes: 2,
      reviewed_today: 4,
      next_due: null,
      calibration_30_days: calibration,
    })
    expect(server.calls.map((c) => c.url)).toEqual([
      `${PRACTICE}/summary?timezone=${encodeURIComponent(tz)}`,
      `${PRACTICE}/analytics?timezone=${encodeURIComponent(tz)}&days=30`,
    ])
  })

  it("leaves calibration out when the backend does not report it", async () => {
    server.replies.push({ body: summary }, { body: { timezone: tz, days: 30 } })
    expect(JSON.parse(await tools.get_progress({}))).not.toHaveProperty("calibration_30_days")
  })

  it("reports an unreachable backend", async () => {
    server.replies.push(new TypeError("Failed to fetch"))
    expect(await tools.get_progress({})).toBe("Error: the practice service is not reachable.")
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
    propose_answer: { properties: { transcript: "string" }, required: ["transcript"] },
    set_confidence: { properties: { level: "integer" }, required: ["level"] },
    get_hint: { properties: {}, required: [] },
    check_answer: { properties: {}, required: [] },
    give_up: { properties: {}, required: [] },
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
    expect(configs.map((c) => c.name).sort()).toEqual(Object.keys(tools).sort())
    for (const c of configs) {
      expect(c).toMatchObject({ type: "client", expects_response: true })
      const types = Object.fromEntries(Object.entries(c.parameters.properties).map(([k, v]) => [k, v.type]))
      expect({ properties: types, required: c.parameters.required }).toEqual(expected[c.name])
    }
  })

  it("has no record_review tool: the server schedules the review when the question closes", () => {
    expect(Object.keys(tools)).not.toContain("record_review")
    expect(Object.keys(files).some((f) => f.endsWith("/record_review.json"))).toBe(false)
  })
})
