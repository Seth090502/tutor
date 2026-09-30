import { describe, expect, it, vi } from "vitest"

import { type CommandResult, type StudyAttempt, StudyError, StudySession, isReleased } from "./study"

type Call = { url: string; method: string; headers: Record<string, string>; body: Record<string, unknown> | null }

const SESSION = "a".repeat(32)
const ATTEMPT = "b".repeat(32)

function session(version: number, status: "active" | "closed" = "active") {
  return {
    session_id: SESSION,
    mode: "practice" as const,
    status,
    policy_version: "review-policy/2",
    curriculum_version: "",
    version,
    last_event_seq: version,
    started_at: 1,
    closed_at: null,
  }
}

function attempt(status: StudyAttempt["status"], openedAt = 2): StudyAttempt {
  return {
    attempt_id: ATTEMPT,
    session_id: SESSION,
    entry_id: 7,
    item_version: "v",
    mode: "practice",
    status,
    opened_at: openedAt,
    item: { prompt: "2 + 2?", question_type: "single_choice", options: { A: "3", B: "4" }, lang: null, rungs_available: 4 },
    first_response: null,
    repairs: [],
    confidence: null,
    checks: [],
    assistance: [],
    pending_proposal: null,
    closure: null,
    offer: null,
    released: null,
  }
}

function result(version: number, att: StudyAttempt | null, kinds: string[] = []): CommandResult {
  return {
    schema_version: 1,
    command_id: "x",
    aggregate_version: version,
    last_event_seq: version,
    public_state: { session: session(version), attempt: att },
    events: kinds.map((kind, i) => ({
      event_id: `e${i}`,
      seq: i + 1,
      kind,
      attempt_id: null,
      recorded_at: 1,
      source: "ui",
      payload: {},
    })),
  }
}

/** A fetch that records calls and plays back replies: a body, [status, body], or an Error. */
function fakeFetch(...replies: (unknown | [number, unknown] | Error)[]) {
  const calls: Call[] = []
  const impl = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({
      url: String(url),
      method: init?.method ?? "GET",
      headers: (init?.headers ?? {}) as Record<string, string>,
      body: init?.body ? JSON.parse(String(init.body)) : null,
    })
    const next = replies.shift()
    if (next === undefined) throw new Error(`unexpected fetch ${String(url)}`)
    if (next instanceof Error) throw next
    const [status, body] = Array.isArray(next) ? next : [200, next]
    return new Response(JSON.stringify(body), { status })
  }) as unknown as typeof fetch
  return { impl, calls }
}

describe("StudySession", () => {
  it("starts with a null version and chains each command on the last accepted version", async () => {
    const { impl, calls } = fakeFetch(result(1, null, ["session_started"]), result(2, attempt("open")), result(3, attempt("committed")))
    const study = await StudySession.start("practice", impl)
    expect(calls[0].body).toMatchObject({ schema_version: 1, kind: "start_session", session_id: null, expected_version: null })
    expect(calls[0].headers["X-Tutor-Source"]).toBe("ui")
    // Two sends at once still go one after the other, each on the version the previous one produced.
    await Promise.all([study.open(7), study.commit("B", "typed")])
    expect(calls[1].body).toMatchObject({ kind: "open_item", expected_version: 1, attempt_id: null, payload: { entry_id: 7 } })
    expect(calls[2].body).toMatchObject({ kind: "commit_response", expected_version: 2, attempt_id: ATTEMPT })
    expect(study.state.session.version).toBe(3)
    expect(study.state.attempt?.status).toBe("committed")
  })

  it("retries a network failure with the same command id and publishes once", async () => {
    const { impl, calls } = fakeFetch(result(1, null), new TypeError("Failed to fetch"), result(2, attempt("open"), ["item_opened"]))
    const study = await StudySession.start("practice", impl)
    const seen: string[][] = []
    study.subscribe((_, events) => seen.push(events.map((e) => e.kind)))
    await study.open(7)
    expect(calls[1].body?.command_id).toBe(calls[2].body?.command_id)
    expect(seen).toEqual([["item_opened"]])
  })

  it("gives up after the network retries with a retryable network error", async () => {
    const down = () => new TypeError("Failed to fetch")
    const { impl, calls } = fakeFetch(result(1, null), down(), down(), down())
    const study = await StudySession.start("practice", impl)
    await expect(study.open(7)).rejects.toMatchObject({ code: "network", retryable: true })
    expect(new Set(calls.slice(1).map((c) => c.body?.command_id)).size).toBe(1)
  })

  it("refreshes on a conflict, rethrows it and publishes only the refreshed state", async () => {
    const conflict = { error: { code: "conflict", message: "changed", retryable: true, current_version: 5 } }
    const view = { session: session(5), attempts: [attempt("remediating")], events: [], last_event_seq: 5 }
    const { impl, calls } = fakeFetch(result(1, null), [409, conflict], view)
    const study = await StudySession.start("practice", impl)
    const seen: string[] = []
    study.subscribe((snapshot, events) => seen.push(`${snapshot.session.version}:${events.length}`))
    const error = await study.check().catch((e) => e)
    expect(error).toBeInstanceOf(StudyError)
    expect(error).toMatchObject({ code: "conflict", currentVersion: 5 })
    expect(calls[2].url).toContain(`/api/study/sessions/${SESSION}`)
    expect(study.state.attempt?.status).toBe("remediating")
    expect(seen).toEqual(["5:0"])
  })

  it("keeps state and publishes nothing when a command is refused", async () => {
    const refused = { error: { code: "invalid_transition", message: "Not allowed", retryable: false } }
    const { impl } = fakeFetch(result(1, null), [409, refused])
    const study = await StudySession.start("practice", impl)
    const listener = vi.fn()
    study.subscribe(listener)
    await expect(study.check()).rejects.toMatchObject({ code: "invalid_transition" })
    expect(listener).not.toHaveBeenCalled()
    expect(study.state.session.version).toBe(1)
  })

  it("marks voice proposals and lets callers pick the source", async () => {
    const { impl, calls } = fakeFetch(result(1, attempt("open")), result(2, attempt("open")), result(3, attempt("open")))
    const study = await StudySession.start("practice", impl)
    await study.propose("for")
    await study.hint("voice_tool")
    expect(calls[1].headers["X-Tutor-Source"]).toBe("voice_tool")
    expect(calls[1].body).toMatchObject({ kind: "propose_response", payload: { raw_transcript: "for", input_mode: "spoken" } })
    expect(calls[2].headers["X-Tutor-Source"]).toBe("voice_tool")
  })

  it("resumes an active session at its newest attempt and not a closed one", async () => {
    const older = { ...attempt("closed", 1), attempt_id: "c".repeat(32) }
    const view = { session: session(4), attempts: [older, attempt("open", 3)], events: [], last_event_seq: 4 }
    const { impl } = fakeFetch(
      view,
      { ...view, session: session(9, "closed") },
      [404, { error: { code: "not_found", message: "gone" } }],
    )
    const resumed = await StudySession.resume(SESSION, impl)
    expect(resumed?.state.attempt?.attempt_id).toBe(ATTEMPT)
    expect(await StudySession.resume(SESSION, impl)).toBeNull()
    expect(await StudySession.resume(SESSION, impl)).toBeNull()
  })
})

describe("isReleased", () => {
  it("is true only when the server released the answer", () => {
    expect(isReleased(null)).toBe(false)
    expect(isReleased(attempt("remediating"))).toBe(false)
    const released = { ...attempt("closed"), released: { correct_answer: "B", accepted_answers: [], explanation: "", misconception: null } }
    expect(isReleased(released)).toBe(true)
  })
})
