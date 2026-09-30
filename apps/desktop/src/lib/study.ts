// Study sessions (contract C12): the one client for /api/study. Typed controls and the voice tools share a
// StudySession, which sends each command with the version it last saw, one at a time, and tells
// subscribers about accepted server results only, never an optimistic guess. The answer key reaches the
// client only in `attempt.released`, after the server releases it.
import { BACKEND_URL } from "./backend"

const STUDY = "/api/study"
/** Network retries per command. They reuse the command id, so the server applies it at most once. */
const NETWORK_RETRIES = 2

export type StudySource = "ui" | "voice_tool"
export type StudyMode = "practice" | "assessment" | "instruction"
export type AttemptStatus =
  | "open"
  | "committed"
  | "ready"
  | "remediating"
  | "awaiting_self_assessment"
  | "submitted"
  | "closed"
export type Verdict = "correct" | "incorrect" | "unverifiable" | "invalid_item" | "recorded"
export type InputMode = "typed" | "spoken" | "dont_know"
export type ConfidenceLevel = 1 | 2 | 3 | 4

export type TranscriptConfirmation = { raw_transcript: string; confirmed_text: string }

export type StudyResponse = {
  response_id: string
  response: string
  input_mode: InputMode
  committed_at: number
  transcript_confirmation: TranscriptConfirmation | null
  assisted_before_commit?: boolean
}

export type StudyCheck = {
  response_id: string
  verdict: Verdict
  checker_id: string
  checker_version: string
  assumptions: string[]
  limitations: string[]
  evidence_refs: string[]
  checked_at: number
  diagnostic: string | null
}

export type StudyAssistance = {
  kind: string
  rung: number | null
  source: StudySource
  timestamp: number
  content_ref: string | null
  text: string | null
  before_commit: boolean
}

export type SchedulerDisposition = {
  eligible_for_fsrs: boolean
  rating: "good" | "again" | null
  first_outcome?: string | null
  provenance?: "verified" | "assisted" | "self_report"
  policy_version?: string
  applied?: { request_id: string; due_at: number } | null
}

export type StudyAttempt = {
  attempt_id: string
  session_id: string
  entry_id: number
  item_version: string
  mode: StudyMode
  status: AttemptStatus
  opened_at: number
  item: {
    prompt: string
    question_type: string
    options: Record<string, string>
    lang: string | null
    rungs_available: number
  }
  first_response: StudyResponse | null
  repairs: StudyResponse[]
  confidence: { level: ConfidenceLevel | null; recorded_at: number; skipped_reason: string | null } | null
  checks: StudyCheck[]
  assistance: StudyAssistance[]
  pending_proposal: {
    raw_transcript: string
    input_mode: "spoken" | "dont_know"
    proposed_at: number
    source: StudySource
  } | null
  closure: {
    reason: "correct" | "revealed" | "self_assessed" | "abandoned" | "invalidated"
    closed_at: number
    scheduler_disposition: SchedulerDisposition
  } | null
  offer: string | null
  released: {
    correct_answer: string
    accepted_answers: string[]
    explanation: string
    misconception: string | null
  } | null
  self_assessment?: { judged: "correct" | "incorrect"; recorded_at: number }
  measurement_eligibility?: { eligible: boolean; reasons: string[] }
}

export type StudySessionState = {
  session_id: string
  mode: StudyMode
  status: "active" | "closed"
  policy_version: string
  curriculum_version: string
  version: number
  last_event_seq: number
  started_at: number
  closed_at: number | null
}

export type StudyEvent = {
  event_id: string
  seq: number
  kind: string
  attempt_id: string | null
  recorded_at: number
  source: StudySource
  payload: Record<string, unknown>
}

export type CommandResult = {
  schema_version: 1
  command_id: string
  aggregate_version: number
  last_event_seq: number
  public_state: { session: StudySessionState; attempt: StudyAttempt | null }
  events: StudyEvent[]
}

export type SessionView = {
  session: StudySessionState
  attempts: StudyAttempt[]
  events: StudyEvent[]
  last_event_seq: number
}

export type StudyQueueItem = {
  entry_id: number
  prompt: string
  question_type: string
  options: Record<string, string>
  lang: string | null
  tags: string[]
  due_at: number
}

export type StudyErrorCode =
  | "conflict"
  | "invalid_transition"
  | "invalid_input"
  | "not_found"
  | "unauthorized"
  | "budget_exceeded"
  | "unavailable"
  | "unverified"
  | "network"

export class StudyError extends Error {
  constructor(
    readonly code: StudyErrorCode,
    message: string,
    readonly retryable = false,
    readonly currentVersion: number | null = null,
  ) {
    super(message)
  }
}

/** What a subscriber sees: the session and the attempt on screen, as the server last accepted them. */
export type Snapshot = { session: StudySessionState; attempt: StudyAttempt | null }

type Fetch = typeof fetch

async function request<T>(path: string, init: RequestInit = {}, fetchImpl: Fetch = fetch): Promise<T> {
  let res: Response
  try {
    res = await fetchImpl(BACKEND_URL + path, {
      ...init,
      headers: { "Content-Type": "application/json", ...init.headers },
    })
  } catch (e) {
    throw new StudyError("network", e instanceof Error ? e.message : String(e), true)
  }
  const text = await res.text()
  let body: unknown = null
  try {
    body = text ? JSON.parse(text) : null
  } catch {
    body = null
  }
  if (!res.ok) {
    const error = (body as { error?: { code?: string; message?: string; retryable?: boolean; current_version?: number } })
      ?.error
    if (error?.code) {
      throw new StudyError(
        error.code as StudyErrorCode,
        error.message ?? `HTTP ${res.status}`,
        Boolean(error.retryable),
        error.current_version ?? null,
      )
    }
    throw new StudyError(res.status >= 500 ? "unavailable" : "invalid_input", `HTTP ${res.status} ${text}`.trim())
  }
  return body as T
}

export const studyQueue = (opts: { limit?: number; tag?: string } = {}, fetchImpl?: Fetch) => {
  const params = new URLSearchParams({ limit: String(opts.limit ?? 20) })
  if (opts.tag) params.set("tag", opts.tag)
  return request<{ items: StudyQueueItem[] }>(`${STUDY}/queue?${params}`, {}, fetchImpl).then((r) => r.items)
}

export const sessionView = (sessionId: string, afterSeq = 0, fetchImpl?: Fetch) =>
  request<SessionView>(`${STUDY}/sessions/${sessionId}?after_seq=${afterSeq}`, {}, fetchImpl)

/** The attempt a screen should show: the newest one, open or just closed. */
const currentAttempt = (attempts: StudyAttempt[]): StudyAttempt | null =>
  attempts.reduce<StudyAttempt | null>((latest, a) => (!latest || a.opened_at >= latest.opened_at ? a : latest), null)

type SendOptions = { source?: StudySource; attempt?: boolean }

export class StudySession {
  private snapshot: Snapshot
  private readonly listeners = new Set<(snapshot: Snapshot, events: StudyEvent[]) => void>()
  private tail: Promise<unknown> = Promise.resolve()

  private constructor(
    snapshot: Snapshot,
    private readonly fetchImpl: Fetch,
  ) {
    this.snapshot = snapshot
  }

  static async start(mode: StudyMode, fetchImpl: Fetch = fetch): Promise<StudySession> {
    const result = await sendCommand(
      { session_id: null, attempt_id: null, expected_version: null, kind: "start_session", payload: { mode } },
      "ui",
      fetchImpl,
    )
    return new StudySession(result.public_state, fetchImpl)
  }

  /** Reopens a session after a restart or reconnect from durable server state; null when it has closed. */
  static async resume(sessionId: string, fetchImpl: Fetch = fetch): Promise<StudySession | null> {
    try {
      const view = await sessionView(sessionId, 0, fetchImpl)
      if (view.session.status !== "active") return null
      return new StudySession({ session: view.session, attempt: currentAttempt(view.attempts) }, fetchImpl)
    } catch (e) {
      if (e instanceof StudyError && e.code === "not_found") return null
      throw e
    }
  }

  get state(): Snapshot {
    return this.snapshot
  }

  subscribe(listener: (snapshot: Snapshot, events: StudyEvent[]) => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private publish(events: StudyEvent[]) {
    for (const listener of [...this.listeners]) {
      try {
        listener(this.snapshot, events)
      } catch (e) {
        console.warn("study listener failed", e)
      }
    }
  }

  /** Re-reads the session from the server (after a conflict, or on reconnect). */
  async refresh(): Promise<Snapshot> {
    const view = await sessionView(this.snapshot.session.session_id, 0, this.fetchImpl)
    this.snapshot = { session: view.session, attempt: currentAttempt(view.attempts) }
    this.publish([])
    return this.snapshot
  }

  /**
   * Sends one command with the last version seen. Commands from this client run one at a time, so the
   * voice tools and the typed controls never race each other. A conflict refreshes the state and is
   * rethrown for the caller to decide.
   */
  send(kind: string, payload: Record<string, unknown> = {}, { source = "ui", attempt = true }: SendOptions = {}) {
    const run = async (): Promise<CommandResult> => {
      const { session, attempt: current } = this.snapshot
      try {
        const result = await sendCommand(
          {
            session_id: session.session_id,
            attempt_id: attempt ? (current?.attempt_id ?? null) : null,
            expected_version: session.version,
            kind,
            payload,
          },
          source,
          this.fetchImpl,
        )
        this.snapshot = {
          session: result.public_state.session,
          attempt: result.public_state.attempt ?? (kind === "close_session" ? null : current),
        }
        this.publish(result.events)
        return result
      } catch (e) {
        if (e instanceof StudyError && e.code === "conflict") await this.refresh().catch(() => {})
        throw e
      }
    }
    const next = this.tail.then(run, run)
    this.tail = next.catch(() => {})
    return next
  }

  open = (entryId: number, source?: StudySource) =>
    this.send("open_item", { entry_id: entryId }, { source, attempt: false })
  hint = (source?: StudySource) => this.send("request_assistance", { kind: "hint" }, { source })
  noteAssistance = (kind: "gloss" | "source_open" | "other", contentRef?: string) =>
    this.send("request_assistance", { kind, ...(contentRef ? { content_ref: contentRef } : {}) })
  propose = (rawTranscript: string, inputMode: "spoken" | "dont_know" = "spoken") =>
    this.send("propose_response", { raw_transcript: rawTranscript, input_mode: inputMode }, { source: "voice_tool" })
  commit = (response: string, inputMode: InputMode, confirmation?: TranscriptConfirmation) =>
    this.send("commit_response", {
      response,
      input_mode: inputMode,
      ...(confirmation ? { transcript_confirmation: confirmation } : {}),
    })
  confidence = (level: ConfidenceLevel | null, skippedReason?: string, source?: StudySource) =>
    this.send("record_confidence", level === null ? { level, skipped_reason: skippedReason ?? "" } : { level }, {
      source,
    })
  check = (source?: StudySource) => this.send("check", {}, { source })
  selfAssess = (judged: "correct" | "incorrect") => this.send("self_assess", { judged })
  repair = (response: string, inputMode: "typed" | "spoken", confirmation?: TranscriptConfirmation) =>
    this.send("submit_repair", {
      response,
      input_mode: inputMode,
      ...(confirmation ? { transcript_confirmation: confirmation } : {}),
    })
  giveUp = (source?: StudySource) => this.send("give_up", {}, { source })
  abandon = (source?: StudySource) => this.send("abandon", {}, { source })
  invalidate = (reason: string) => this.send("invalidate_item", { reason })
  close = () => this.send("close_session", {}, { attempt: false })
}

type Envelope = {
  session_id: string | null
  attempt_id: string | null
  expected_version: number | null
  kind: string
  payload: Record<string, unknown>
}

/** One C12 command. Network failures are retried with the same command id; nothing else is retried. */
export async function sendCommand(envelope: Envelope, source: StudySource, fetchImpl: Fetch = fetch) {
  const body = JSON.stringify({ schema_version: 1, command_id: crypto.randomUUID(), ...envelope })
  for (let attempt = 0; ; attempt++) {
    try {
      return await request<CommandResult>(
        `${STUDY}/commands`,
        { method: "POST", body, headers: { "X-Tutor-Source": source } },
        fetchImpl,
      )
    } catch (e) {
      if (!(e instanceof StudyError && e.code === "network") || attempt >= NETWORK_RETRIES) throw e
    }
  }
}

/** True when the key or reference answer may be shown for this attempt. */
export const isReleased = (attempt: StudyAttempt | null): attempt is StudyAttempt & { released: NonNullable<StudyAttempt["released"]> } =>
  Boolean(attempt?.released)
