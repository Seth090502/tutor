// Test support: an in-memory C12 study server (/api/study) for the voice tests. It drives attempts
// through every status and keeps the answer key back until it releases it. Every public attempt and
// queue item also carries decoy private fields (the key, the explanation, every hint), so a test
// fails if the client forwards anything it did not pick field by field.
import type { ConfidenceLevel, StudyAssistance, StudyAttempt, StudyCheck, StudyResponse, StudySessionState } from "@/lib/study"

export type FakeItem = {
  entry_id: number
  prompt: string
  question_type: string
  options: Record<string, string>
  key: string
  /** False: no checker, so a check releases the reference answer for self-assessment. */
  checkable: boolean
  hints: string[]
  explanation: string
  misconception: string | null
  tags: string[]
}

/** A choice item. Its key "B" is not distinctive; the explanation, misconception and hint 2 are. */
export const CHOICE: FakeItem = {
  entry_id: 7,
  prompt: "Which gas do leaves take in by day?",
  question_type: "single_choice",
  options: { B: "Carbon dioxide", A: "Oxygen", C: "Nitrogen" },
  key: "B",
  checkable: true,
  hints: ["Think about what plants breathe in to make sugar.", "HINT-TWO-IBIS"],
  explanation: "EXPLANATION-OCTOPUS",
  misconception: "MISCONCEPTION-PLATYPUS",
  tags: ["Science"],
}

/** A typed item whose key is a distinctive string. */
export const TYPED: FakeItem = {
  entry_id: 8,
  prompt: "Type the code word.",
  question_type: "short_answer",
  options: {},
  key: "KEY-ZEBRA-42",
  checkable: true,
  hints: ["HINT-ONE-HERON"],
  explanation: "EXPLANATION-MANTA",
  misconception: null,
  tags: ["Math"],
}

/** An item with no checker: the learner judges it against the reference answer. */
export const ESSAY: FakeItem = {
  entry_id: 9,
  prompt: "Explain osmosis in one sentence.",
  question_type: "essay",
  options: {},
  key: "REFERENCE-WALRUS",
  checkable: false,
  hints: [],
  explanation: "EXPLANATION-SQUID",
  misconception: null,
  tags: ["Science"],
}

/** Keys, explanations and misconceptions of all three items: none may show before release. */
export const SECRETS = /KEY-ZEBRA|REFERENCE-WALRUS|EXPLANATION-|MISCONCEPTION-/

export const DUE_AT = 1_790_000_000

export type Call = { method: string; url: string; source: string | null; body: Record<string, unknown> | null }
type Reply = { status?: number; body: unknown } | Error

type Fail = { code: string; status: number }

const hex = (n: number) => n.toString(16).padStart(32, "0")

export function fakeStudyServer(items: FakeItem[] = [CHOICE, TYPED, ESSAY]) {
  const calls: Call[] = []
  /** Replies for anything outside /api/study, in order. */
  const replies: Reply[] = []
  const due = new Set(items.map((i) => i.entry_id))
  const attempts: (StudyAttempt & { released_at: number | null })[] = []
  const sessions: StudySessionState[] = []
  /** The newest session; commands go to it. */
  let session: StudySessionState | null = null
  let failNext: Fail | null = null
  let networkDown = 0
  let clock = 100

  const itemOf = (a: StudyAttempt) => items.find((i) => i.entry_id === a.entry_id)!

  const publicAttempt = (a: (typeof attempts)[number]) => {
    const item = itemOf(a)
    const { released_at, ...rest } = a
    return {
      ...rest,
      item: { ...a.item, hints: item.hints, correct_answer: item.key },
      answer_key: item.key,
      private_explanation: item.explanation,
      released: released_at
        ? { correct_answer: item.key, accepted_answers: [], explanation: item.explanation, misconception: item.misconception }
        : null,
    }
  }

  const error = (status: number, code: string, message: string, extra: Record<string, unknown> = {}) =>
    new Response(JSON.stringify({ error: { code, message, retryable: code === "conflict", ...extra } }), { status })

  class Refusal extends Error {
    constructor(
      readonly status: number,
      readonly code: string,
    ) {
      super(code)
    }
  }
  const refuse = (status: number, code: string): never => {
    throw new Refusal(status, code)
  }
  const require = (a: StudyAttempt, ...statuses: string[]) => {
    if (!statuses.includes(a.status)) refuse(409, "invalid_transition")
  }

  const check = (a: StudyAttempt, r: StudyResponse): StudyCheck => {
    const item = itemOf(a)
    const verdict =
      r.input_mode === "dont_know"
        ? "incorrect"
        : !item.checkable
          ? "unverifiable"
          : r.response.trim().toLowerCase() === item.key.toLowerCase()
            ? "correct"
            : "incorrect"
    const c: StudyCheck = {
      response_id: r.response_id,
      verdict,
      checker_id: r.input_mode === "dont_know" ? "dont_know" : "fake",
      checker_version: "1",
      assumptions: [],
      limitations: [],
      evidence_refs: [],
      checked_at: clock,
      diagnostic: verdict === "incorrect" && r.input_mode !== "dont_know" ? "Not the expected answer." : null,
    }
    a.checks.push(c)
    return c
  }

  const close = (a: (typeof attempts)[number], reason: NonNullable<StudyAttempt["closure"]>["reason"], judged?: string) => {
    const first = a.first_response
    const firstVerdict = a.checks[0]?.verdict
    const outcome = firstVerdict === "unverifiable" ? judged : firstVerdict
    const assisted = Boolean(first?.assisted_before_commit)
    const eligible = Boolean(first) && (outcome === "correct" || outcome === "incorrect") && reason !== "invalidated"
    a.status = "closed"
    a.closure = {
      reason,
      closed_at: clock,
      scheduler_disposition: eligible
        ? {
            eligible_for_fsrs: true,
            rating: outcome === "correct" && !assisted ? "good" : "again",
            first_outcome: outcome,
            provenance: firstVerdict === "unverifiable" ? "self_report" : assisted ? "assisted" : "verified",
            policy_version: "review-policy/2",
            applied: { request_id: `ev:${a.attempt_id}`, due_at: DUE_AT },
          }
        : { eligible_for_fsrs: false, rating: null },
    }
    if (reason === "correct" || reason === "revealed" || reason === "self_assessed") a.released_at = clock
    due.delete(a.entry_id)
  }

  const respond = (payload: Record<string, unknown>, source: string, first: boolean): StudyResponse => {
    if (source !== "ui") refuse(403, "unauthorized")
    const mode = payload.input_mode as StudyResponse["input_mode"]
    const response = String(payload.response ?? "")
    const confirmation = (payload.transcript_confirmation ?? null) as StudyResponse["transcript_confirmation"]
    if (mode === "spoken" && confirmation?.confirmed_text !== response) refuse(422, "invalid_input")
    if (mode !== "spoken" && confirmation) refuse(422, "invalid_input")
    if (mode === "dont_know" && (response || !first)) refuse(422, "invalid_input")
    return { response_id: `r${clock}`, response, input_mode: mode, committed_at: clock, transcript_confirmation: confirmation }
  }

  function command(kind: string, body: Record<string, unknown>, source: string) {
    const payload = (body.payload ?? {}) as Record<string, unknown>
    if (kind === "start_session") {
      session = {
        session_id: "a".repeat(31) + String(sessions.length + 1),
        mode: "practice",
        status: "active",
        policy_version: "review-policy/2",
        curriculum_version: "",
        version: 0,
        last_event_seq: 0,
        started_at: clock,
        closed_at: null,
      }
      sessions.push(session)
      return null
    }
    if (!session || body.session_id !== session.session_id) return refuse(404, "not_found")
    if (kind === "open_item") {
      if (attempts.some((a) => a.session_id === session!.session_id && a.status !== "closed")) refuse(409, "invalid_transition")
      const item = items.find((i) => i.entry_id === payload.entry_id) ?? refuse(404, "not_found")
      const a = {
        attempt_id: hex(attempts.length + 1),
        session_id: session.session_id,
        entry_id: item.entry_id,
        item_version: "v1",
        mode: "practice" as const,
        status: "open" as const,
        opened_at: clock,
        item: {
          prompt: item.prompt,
          question_type: item.question_type,
          options: item.options,
          lang: null,
          rungs_available: Math.min(item.hints.length, 4),
        },
        first_response: null,
        repairs: [],
        confidence: null,
        checks: [],
        assistance: [],
        pending_proposal: null,
        closure: null,
        offer: null,
        released: null,
        released_at: null,
      }
      attempts.push(a)
      return a
    }
    const a = attempts.find((x) => x.attempt_id === body.attempt_id) ?? refuse(404, "not_found")
    switch (kind) {
      case "request_assistance": {
        require(a, "open", "remediating")
        const rung = 1 + a.assistance.filter((x) => x.rung).length
        if (rung > 4) refuse(409, "invalid_transition")
        const hint: StudyAssistance = {
          kind: ["nudge", "hint", "step", "example"][rung - 1],
          rung,
          source: source as StudyAssistance["source"],
          timestamp: clock,
          content_ref: `hint:${rung}`,
          text: itemOf(a).hints[rung - 1] ?? null,
          before_commit: a.first_response === null,
        }
        a.assistance.push(hint)
        break
      }
      case "propose_response":
        require(a, "open", "remediating")
        a.pending_proposal = {
          raw_transcript: String(payload.raw_transcript ?? ""),
          input_mode: payload.input_mode as "spoken" | "dont_know",
          proposed_at: clock,
          source: source as "ui" | "voice_tool",
        }
        break
      case "commit_response":
        require(a, "open")
        a.first_response = {
          ...respond(payload, source, true),
          assisted_before_commit: a.assistance.some((x) => x.before_commit),
        }
        a.pending_proposal = null
        a.status = "committed"
        break
      case "record_confidence":
        require(a, "committed")
        if (payload.level === null && !payload.skipped_reason) refuse(422, "invalid_input")
        a.confidence = {
          level: (payload.level ?? null) as ConfidenceLevel | null,
          recorded_at: clock,
          skipped_reason: (payload.skipped_reason as string | undefined) ?? null,
        }
        a.status = "ready"
        break
      case "check": {
        require(a, "ready")
        const c = check(a, a.first_response!)
        if (c.verdict === "correct") close(a, "correct")
        else if (c.verdict === "incorrect") a.status = "remediating"
        else {
          a.status = "awaiting_self_assessment"
          a.released_at = clock
        }
        break
      }
      case "self_assess":
        require(a, "awaiting_self_assessment")
        if (source !== "ui") refuse(403, "unauthorized")
        a.self_assessment = { judged: payload.judged as "correct" | "incorrect", recorded_at: clock }
        close(a, "self_assessed", String(payload.judged))
        break
      case "submit_repair": {
        require(a, "remediating")
        const r = respond(payload, source, false)
        a.repairs.push(r)
        a.pending_proposal = null
        if (check(a, r).verdict === "correct") close(a, "correct")
        else if (a.repairs.length >= 2 && !a.offer) a.offer = "worked_example_then_fresh_problem"
        break
      }
      case "give_up":
        require(a, "open", "remediating")
        if (!a.first_response) {
          a.first_response = {
            response_id: `r${clock}`,
            response: "",
            input_mode: "dont_know",
            committed_at: clock,
            transcript_confirmation: null,
            assisted_before_commit: a.assistance.some((x) => x.before_commit),
          }
          check(a, a.first_response)
        }
        close(a, "revealed")
        break
      default:
        refuse(422, "invalid_input")
    }
    return a
  }

  const fetchImpl = async (input: string | URL | Request, init?: RequestInit): Promise<Response> => {
    const url = new URL(String(input))
    const headers = (init?.headers ?? {}) as Record<string, string>
    const body = init?.body ? (JSON.parse(String(init.body)) as Record<string, unknown>) : null
    calls.push({ method: init?.method ?? "GET", url: String(input), source: headers["X-Tutor-Source"] ?? null, body })
    clock += 1

    if (!url.pathname.startsWith("/api/study/")) {
      const next = replies.shift()
      if (!next) throw new Error(`unexpected fetch ${String(input)}`)
      if (next instanceof Error) throw next
      return new Response(JSON.stringify(next.body), { status: next.status ?? 200 })
    }
    if (networkDown > 0) {
      networkDown -= 1
      throw new TypeError("Failed to fetch")
    }
    if (url.pathname === "/api/study/queue") {
      const tag = url.searchParams.get("tag")?.toLowerCase()
      const found = items
        .filter((i) => due.has(i.entry_id) && (!tag || i.tags.some((t) => t.toLowerCase() === tag)))
        .map((i) => ({
          entry_id: i.entry_id,
          prompt: i.prompt,
          question_type: i.question_type,
          options: i.options,
          lang: null,
          tags: i.tags,
          due_at: 1,
          correct_answer: i.key,
          explanation: i.explanation,
          hints: i.hints,
        }))
      return new Response(JSON.stringify({ items: found }))
    }
    if (url.pathname.startsWith("/api/study/sessions/")) {
      const found = sessions.find((x) => x.session_id === url.pathname.split("/").pop())
      if (!found) return error(404, "not_found", "No such session")
      const own = attempts.filter((a) => a.session_id === found.session_id).map(publicAttempt)
      return new Response(JSON.stringify({ session: found, attempts: own, events: [], last_event_seq: found.version }))
    }
    // POST /api/study/commands
    if (failNext) {
      const { code, status } = failNext
      failNext = null
      return error(status, code, code, code === "conflict" ? { current_version: session?.version } : {})
    }
    const kind = String(body?.kind)
    if (kind !== "start_session" && session && body?.expected_version !== session.version) {
      return error(409, "conflict", "stale", { current_version: session.version })
    }
    try {
      const attempt = command(kind, body ?? {}, headers["X-Tutor-Source"] ?? "")
      session!.version += 1
      session!.last_event_seq += 1
      return new Response(
        JSON.stringify({
          schema_version: 1,
          command_id: body?.command_id,
          aggregate_version: session!.version,
          last_event_seq: session!.last_event_seq,
          public_state: { session, attempt: attempt ? publicAttempt(attempt as (typeof attempts)[number]) : null },
          events: [],
        }),
      )
    } catch (e) {
      if (e instanceof Refusal) return error(e.status, e.code, e.code)
      throw e
    }
  }

  return {
    fetch: fetchImpl as typeof fetch,
    calls,
    replies,
    /** The commands sent, in order. */
    commands: () => calls.filter((c) => c.url.endsWith("/api/study/commands")),
    /** The next command fails with this error, without changing anything. */
    failNext: (code: string, status: number) => {
      failNext = { code, status }
    },
    /** The next `n` study requests fail as if the network were down. */
    dropNext: (n: number) => {
      networkDown = n
    },
    /** Closes the session, as another client or a restart elsewhere would. */
    closeSession: () => {
      if (session) Object.assign(session, { status: "closed", closed_at: clock })
    },
  }
}

export type FakeStudy = ReturnType<typeof fakeStudyServer>
