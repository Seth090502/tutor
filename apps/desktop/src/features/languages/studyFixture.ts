// A fake of the C12 routes the Practice tab uses, for the preview page (fixtures.ts) and the tests. It keeps
// the rules of the contract that matter to a client: the `X-Tutor-Source` header, versions, statuses, and the
// answer key appearing only in `attempt.released`. It is not the backend and never talks to one.
import type { AttemptStatus, CommandResult, StudyAttempt, StudyCheck, StudySessionState } from "@/lib/study"

/** A fixture card: what the backend would hold for one question, key and hints included. */
export type FixtureCard = {
  id: number
  lang: string
  kind: "production" | "recognition" | "listening"
  question: string
  answer: string
  hints: string[]
  explanation: string
  misconception?: string
  /** No checker: the verdict is `unverifiable` and the learner self-assesses. */
  free?: boolean
}

export type StudyBackend = {
  fetch: typeof fetch
  /** Every request the client made, with lower-cased header names. */
  calls: { method: string; path: string; headers: Record<string, string>; body: Record<string, unknown> | null }[]
}

type Reply = { status: number; body: unknown }
type Source = "ui" | "voice_tool"
type Stored = { attempt: StudyAttempt; card: FixtureCard; misses: number }
type StoredSession = { session: StudySessionState; attempts: Stored[] }

const ok = (body: unknown): Reply => ({ status: 200, body })
const fail = (status: number, code: string, message: string, extra: Record<string, unknown> = {}): Reply => ({
  status,
  body: { error: { code, message, retryable: false, ...extra } },
})

const strip = (s: string) => s.normalize("NFD").replace(/\p{M}+/gu, "")
const norm = (s: string) => s.normalize("NFC").toLowerCase().replace(/[\p{P}\p{S}\s]+/gu, " ").trim()

export function createStudyBackend(cards: FixtureCard[]): StudyBackend {
  const sessions = new Map<string, StoredSession>()
  const calls: StudyBackend["calls"] = []
  let ids = 1
  const newId = () => String(ids++).padStart(32, "0")
  const now = () => Date.now() / 1000

  const release = (s: Stored) => {
    s.attempt.released = {
      correct_answer: s.card.answer,
      accepted_answers: [s.card.answer],
      explanation: s.card.explanation,
      misconception: s.card.misconception ?? null,
    }
  }

  const close = (s: Stored, reason: NonNullable<StudyAttempt["closure"]>["reason"]) => {
    const a = s.attempt
    const assisted = a.assistance.some((x) => x.before_commit)
    const first = a.checks[0]
    const selfReport = first?.verdict === "unverifiable"
    let rating: "good" | "again" | null = null
    let provenance: "verified" | "assisted" | "self_report" | undefined
    if (reason !== "abandoned" && reason !== "invalidated") {
      const good = selfReport ? a.self_assessment?.judged === "correct" : first?.verdict === "correct"
      rating = good && !assisted ? "good" : "again"
      provenance = selfReport ? "self_report" : assisted ? "assisted" : "verified"
    }
    if (reason === "correct" || reason === "revealed" || reason === "self_assessed") release(s)
    a.status = "closed"
    a.closure = {
      reason,
      closed_at: now(),
      scheduler_disposition: {
        eligible_for_fsrs: rating !== null,
        rating,
        ...(provenance
          ? { provenance, policy_version: "review-policy/2", applied: { request_id: `ev:${a.attempt_id}`, due_at: now() + 3 * 86400 } }
          : {}),
      },
    }
  }

  const checkOf = (s: Stored, response: string, mode: string): StudyCheck => {
    const a = s.attempt
    const exact = norm(response) === norm(s.card.answer)
    const accent = !exact && norm(response) !== "" && norm(strip(response)) === norm(strip(s.card.answer))
    return {
      response_id: (a.repairs[a.repairs.length - 1] ?? a.first_response)!.response_id,
      verdict: mode === "dont_know" ? "incorrect" : s.card.free ? "unverifiable" : exact ? "correct" : "incorrect",
      checker_id: mode === "dont_know" ? "dont_know" : s.card.free ? "none" : `language:${s.card.lang}`,
      checker_version: "fixture",
      assumptions: [],
      limitations: [],
      evidence_refs: [],
      checked_at: now(),
      diagnostic: accent ? "accent" : null,
    }
  }

  const command = (source: string, body: Record<string, unknown>): Reply => {
    if (source !== "ui" && source !== "voice_tool") return fail(422, "invalid_input", "X-Tutor-Source must be ui or voice_tool")
    const from = source as Source
    const kind = String(body.kind)
    const payload = (body.payload ?? {}) as Record<string, unknown>
    let held: StoredSession | undefined
    let stored: Stored | undefined
    if (kind === "start_session") {
      const session: StudySessionState = {
        session_id: newId(),
        mode: payload.mode as StudySessionState["mode"],
        status: "active",
        policy_version: "review-policy/2",
        curriculum_version: "",
        version: 0,
        last_event_seq: 0,
        started_at: now(),
        closed_at: null,
      }
      held = { session, attempts: [] }
      sessions.set(session.session_id, held)
    } else {
      held = sessions.get(String(body.session_id))
      if (!held) return fail(404, "not_found", "No such session")
      if (body.expected_version !== held.session.version)
        return fail(409, "conflict", "The session changed", { current_version: held.session.version })
      stored = held.attempts.find((x) => x.attempt.attempt_id === body.attempt_id)
    }

    const need = (...statuses: AttemptStatus[]): Reply | null =>
      stored && statuses.includes(stored.attempt.status)
        ? null
        : fail(409, "invalid_transition", `Not allowed while the attempt is ${stored?.attempt.status ?? "missing"}`)
    const uiOnly = (): Reply | null => (from === "ui" ? null : fail(403, "unauthorized", "The learner confirms answers on screen"))

    let error: Reply | null = null
    const a = stored?.attempt
    switch (kind) {
      case "start_session":
        break
      case "open_item": {
        if (held.attempts.some((x) => x.attempt.status !== "closed")) {
          error = fail(409, "invalid_transition", "Finish, give up or abandon the current item first")
          break
        }
        const card = cards.find((c) => c.id === payload.entry_id)
        if (!card) return fail(404, "not_found", "No such item")
        stored = {
          card,
          misses: 0,
          attempt: {
            attempt_id: newId(),
            session_id: held.session.session_id,
            entry_id: card.id,
            item_version: "fixture",
            mode: held.session.mode,
            status: "open",
            opened_at: now(),
            item: { prompt: card.question, question_type: "short_answer", options: {}, lang: card.lang, rungs_available: Math.min(card.hints.length, 4) },
            first_response: null,
            repairs: [],
            confidence: null,
            checks: [],
            assistance: [],
            pending_proposal: null,
            closure: null,
            offer: null,
            released: null,
          },
        }
        held.attempts.push(stored)
        break
      }
      case "request_assistance": {
        error = need("open", "remediating")
        if (error || !stored || !a) break
        const rung = payload.kind === "hint" ? a.assistance.filter((x) => x.rung !== null).length + 1 : null
        if (rung !== null && rung > a.item.rungs_available) {
          error = fail(409, "invalid_transition", "No more hints; give up to see the answer")
          break
        }
        a.assistance.push({
          kind: rung ? `hint_${rung}` : String(payload.kind),
          rung,
          source: from,
          timestamp: now(),
          content_ref: rung ? `hint:${rung}` : ((payload.content_ref as string | undefined) ?? null),
          text: rung ? stored.card.hints[rung - 1] : null,
          before_commit: a.status === "open",
        })
        break
      }
      case "commit_response": {
        error = need("open") ?? uiOnly()
        if (error || !a) break
        a.first_response = {
          response_id: newId(),
          response: String(payload.response ?? ""),
          input_mode: payload.input_mode as "typed" | "dont_know",
          committed_at: now(),
          transcript_confirmation: null,
          assisted_before_commit: a.assistance.length > 0,
        }
        a.status = "committed"
        break
      }
      case "record_confidence": {
        error = need("committed")
        if (error || !a) break
        const level = payload.level as 1 | 2 | 3 | 4 | null
        if (level === null && !String(payload.skipped_reason ?? "").trim()) {
          error = fail(422, "invalid_input", "A skip needs a reason")
          break
        }
        a.confidence = { level, recorded_at: now(), skipped_reason: level === null ? String(payload.skipped_reason) : null }
        a.status = "ready"
        break
      }
      case "check": {
        error = need("ready")
        if (error || !stored || !a) break
        const c = checkOf(stored, a.first_response!.response, a.first_response!.input_mode)
        a.checks.push(c)
        if (c.verdict === "correct") close(stored, "correct")
        else if (c.verdict === "unverifiable") {
          a.status = "awaiting_self_assessment"
          release(stored)
        } else a.status = "remediating"
        break
      }
      case "self_assess": {
        error = need("awaiting_self_assessment") ?? uiOnly()
        if (error || !stored || !a) break
        a.self_assessment = { judged: payload.judged as "correct" | "incorrect", recorded_at: now() }
        close(stored, "self_assessed")
        break
      }
      case "submit_repair": {
        error = need("remediating") ?? uiOnly()
        if (error || !stored || !a) break
        const text = String(payload.response ?? "")
        a.repairs.push({ response_id: newId(), response: text, input_mode: "typed", committed_at: now(), transcript_confirmation: null })
        const c = checkOf(stored, text, "typed")
        a.checks.push(c)
        if (c.verdict === "correct") close(stored, "correct")
        else if (++stored.misses >= 2) a.offer = "worked_example_then_fresh_problem"
        break
      }
      case "give_up":
        error = need("open", "remediating")
        if (!error && stored) close(stored, "revealed")
        break
      case "abandon":
        error = need("open", "committed", "ready", "remediating", "awaiting_self_assessment")
        if (!error && stored) close(stored, "abandoned")
        break
      default:
        return fail(422, "invalid_input", `The fixture does not implement ${kind}`)
    }
    if (error) return error

    held.session.version += 1
    held.session.last_event_seq += 1
    const result: CommandResult = {
      schema_version: 1,
      command_id: String(body.command_id),
      aggregate_version: held.session.version,
      last_event_seq: held.session.last_event_seq,
      public_state: { session: held.session, attempt: stored?.attempt ?? null },
      events: [
        {
          event_id: newId(),
          seq: held.session.last_event_seq,
          kind,
          attempt_id: stored?.attempt.attempt_id ?? null,
          recorded_at: now(),
          source: from,
          payload: {},
        },
      ],
    }
    return ok(result)
  }

  const route = (method: string, url: URL, headers: Record<string, string>, body: Record<string, unknown> | null): Reply => {
    if (url.pathname === "/api/study/queue") {
      const tag = url.searchParams.get("tag")?.toLowerCase()
      const limit = Number(url.searchParams.get("limit") ?? 20)
      const items = cards
        .filter((c) => !tag || [`lang:${c.lang}`, `card:${c.kind}`].includes(tag))
        .slice(0, limit)
        .map((c) => ({
          entry_id: c.id,
          prompt: c.question,
          question_type: "short_answer",
          options: {},
          lang: c.lang,
          tags: [`lang:${c.lang}`, `card:${c.kind}`],
          due_at: 1,
        }))
      return ok({ items })
    }
    if (url.pathname === "/api/study/commands" && method === "POST") return command(headers["x-tutor-source"] ?? "", body ?? {})
    const view = /^\/api\/study\/sessions\/([^/]+)$/.exec(url.pathname)
    if (view && method === "GET") {
      const held = sessions.get(view[1])
      if (!held) return fail(404, "not_found", "No such session")
      return ok({ session: held.session, attempts: held.attempts.map((x) => x.attempt), events: [], last_event_seq: held.session.last_event_seq })
    }
    return { status: 404, body: { detail: `no fixture for ${method} ${url.pathname}` } }
  }

  const impl = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = new URL(String(input))
    const method = init?.method ?? "GET"
    const headers = Object.fromEntries(new Headers(init?.headers).entries())
    const body = init?.body ? (JSON.parse(String(init.body)) as Record<string, unknown>) : null
    calls.push({ method, path: url.pathname + url.search, headers, body })
    const reply = route(method, url, headers, body)
    return new Response(JSON.stringify(reply.body), { status: reply.status, headers: { "Content-Type": "application/json" } })
  }
  return { fetch: impl as typeof fetch, calls }
}
