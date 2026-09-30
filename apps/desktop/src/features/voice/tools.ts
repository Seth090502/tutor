// Client tools for the ElevenLabs voice tutor. The agent config declares the same names and
// parameter schemas (apps/desktop/agent/tool_configs/). Every tool returns a string for the
// LLM and never throws into the SDK: failures come back as a short "Error: ..." sentence.
// Practice runs as C12 study sessions: the tools and the screen's typed controls send commands to
// one server-side attempt through the screen's StudySession (practice.ts), with the source
// voice_tool. Results are built only from the accepted public state, so the answer, explanation
// and misconception reach the model only once the server has released them.
// Library search goes through DeepTutor (contract C11) with the live attempt, so the server
// withholds passages that contain the answer or sit under a solution heading.
import { BACKEND_URL, HttpError, fetchJson } from "@/lib/backend"
import { type AttemptStatus, type CommandResult, type StudyAttempt, StudyError, type StudySession } from "@/lib/study"
import { confidenceWord, hintsGiven, isLive, lastCheck, offerWords, reviewWords } from "./attempt"
import { type PanelAction, type VoicePractice, openNext, studyErrorText } from "./practice"

const PRACTICE = "/api/question-notebook/practice"
const EXCERPT_CHARS = 600
const CALIBRATION_DAYS = 30

type PracticeSummary = {
  total: number
  mistakes: number
  due: number
  overdue: number
  reviewed_today: number
  next_due_at: number | null
}
type Calibration = { level: number; attempts: number; correct: number }
type SearchHit = {
  title: string
  domain: string
  source_type: string
  page_start: number | null
  page_end: number | null
  text: string
}

/** A bad argument from the LLM, or a step out of order; its message is safe to say aloud. */
class ToolInputError extends Error {}

type Params = Record<string, unknown>
export type TutorTool = (params: Params) => Promise<string>

const timezone = () => Intl.DateTimeFormat().resolvedOptions().timeZone

function localTime(epochSeconds: number | null | undefined): string | null {
  if (epochSeconds == null) return null
  return new Date(epochSeconds * 1000).toLocaleString("en-US", {
    timeZone: timezone(),
    dateStyle: "medium",
    timeStyle: "short",
  })
}

function confidence(value: unknown): 1 | 2 | 3 | 4 {
  const level = typeof value === "string" ? Number(value.trim()) : value
  if (level !== 1 && level !== 2 && level !== 3 && level !== 4) {
    throw new ToolInputError("level must be a whole number from 1 to 4.")
  }
  return level
}

function text(value: unknown, name: string): string {
  if (typeof value === "number") return String(value)
  if (typeof value !== "string" || !value.trim()) throw new ToolInputError(`${name} is required.`)
  return value.trim()
}

function optionalText(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined
}

function pages(hit: SearchHit): string {
  if (hit.page_start == null) return ""
  return hit.page_end == null || hit.page_end === hit.page_start
    ? String(hit.page_start)
    : `${hit.page_start}-${hit.page_end}`
}

function excerpt(s: string): string {
  return s.length <= EXCERPT_CHARS ? s : `${s.slice(0, EXCERPT_CHARS - 1)}…`
}

/** What the tutor does when the server has no written text for a rung. */
function missingRungInstruction(rung: number): string {
  return rung >= 4
    ? "No written worked example for this question. Work a parallel problem of the same kind, not this one, step by step, then give the learner a fresh problem of that kind to try."
    : "No written hint for this rung. Give one short conceptual nudge in your own words, without stating the answer or anything that gives it away."
}

/** What a live question needs next, for steps asked out of order. */
const WAITING: Record<AttemptStatus, string> = {
  open: "the learner's answer, confirmed on screen (propose_answer for a spoken one)",
  committed: "the learner's confidence (set_confidence)",
  ready: "check_answer",
  remediating: "a new try, a hint (get_hint) or give_up",
  awaiting_self_assessment: "the learner's own judgement, on screen",
  submitted: "the end of the assessment",
  closed: "get_next_question",
}

const NO_QUESTION = "there is no open question; call get_next_question first."

/** Only public fields, one by one: never the whole attempt. */
function questionResult(a: StudyAttempt) {
  return {
    attempt_id: a.attempt_id,
    question_id: a.entry_id,
    question: a.item.prompt,
    question_type: a.item.question_type,
    options: a.item.options,
    rungs_available: a.item.rungs_available,
  }
}

function hintResult(a: StudyAttempt) {
  const hints = hintsGiven(a)
  const { rung, text: hintText } = hints[hints.length - 1]
  return {
    rung,
    text: hintText,
    ...(hintText == null && rung != null ? { instruction: missingRungInstruction(rung) } : {}),
  }
}

function instruction(a: StudyAttempt): string | null {
  if (a.status === "remediating")
    return "Not correct yet, and the answer stays hidden. The learner may try again (aloud through propose_answer and the screen, or on screen), ask for a hint, or give up."
  if (a.status === "awaiting_self_assessment")
    return "There is no automatic check. The learner judges their answer against the reference answer on screen; wait for their judgement."
  if (a.closure?.reason === "invalidated")
    return "The answer key could not be read, so the question was set aside without a review. Offer the next question."
  return null
}

/** A check, a retry, a give-up or a self-assessment. The answer appears only when released. */
function outcomeResult(a: StudyAttempt) {
  const check = lastCheck(a)
  const { released, closure } = a
  const review = reviewWords(a)
  const nextDue = localTime(closure?.scheduler_disposition.applied?.due_at)
  const todo = instruction(a)
  return {
    ...(check ? { verdict: check.verdict } : {}),
    ...(check?.diagnostic ? { diagnostic: check.diagnostic } : {}),
    ...(closure ? { closed: closure.reason } : {}),
    ...(released
      ? {
          correct_answer: released.correct_answer,
          explanation: released.explanation,
          ...(released.misconception ? { misconception: released.misconception } : {}),
        }
      : {}),
    ...(review ? { review } : {}),
    ...(nextDue ? { next_due: nextDue } : {}),
    ...(a.offer ? { offer: offerWords(a.offer) } : {}),
    ...(todo ? { instruction: todo } : {}),
  }
}

/** The attempt a command left; the server always returns it for attempt commands. */
function attemptOf(result: CommandResult): StudyAttempt {
  if (!result.public_state.attempt) throw new Error(`no attempt in the result of ${result.command_id}`)
  return result.public_state.attempt
}

/** What the learner did on screen, told to the tutor with the same public data a tool would return. */
export function screenMessage(action: PanelAction, a: StudyAttempt): string {
  const json = (value: unknown) => JSON.stringify(value)
  switch (action) {
    case "next":
      return `On screen: the learner opened the next question. ${json(questionResult(a))}`
    case "answer":
      return a.first_response?.input_mode === "dont_know"
        ? `On screen: the learner answered "I don't know".`
        : `On screen: the learner confirmed their answer: "${a.first_response?.response ?? ""}".`
    case "confidence": {
      const level = a.confidence?.level
      return level
        ? `On screen: confidence ${level}, ${confidenceWord(level)}.`
        : `On screen: the learner skipped confidence (${a.confidence?.skipped_reason ?? ""}).`
    }
    case "hint":
      return `On screen: the learner asked for a hint. ${json(hintResult(a))}`
    case "check":
      return `On screen: the learner checked their answer. ${json(outcomeResult(a))}`
    case "repair":
      return `On screen: the learner tried again with "${a.repairs[a.repairs.length - 1]?.response ?? ""}". ${json(outcomeResult(a))}`
    case "give_up":
      return `On screen: the learner gave up. ${json(outcomeResult(a))}`
    case "self_assess":
      return `On screen: the learner judged their answer ${a.self_assessment?.judged ?? ""}. ${json(outcomeResult(a))}`
  }
}

const PRACTICE_SERVICE = "practice service"
const LIBRARY_SERVICE = "library search"

/** The client tools for one Tutor screen, all driving that screen's study session. */
export function createTutorTools(practice: VoicePractice) {
  /** Wraps a tool so any failure becomes a short sentence the agent can say. */
  const safe =
    (name: string, service: string, run: TutorTool): TutorTool =>
    async (params) => {
      try {
        return await run(params ?? {})
      } catch (e) {
        console.warn(`voice tool ${name} failed`, e)
        if (e instanceof ToolInputError) return `Error: ${e.message}`
        if (e instanceof StudyError) {
          const status = practice.session?.state.attempt?.status
          const waiting = e.code === "invalid_transition" && status ? `; the question is waiting for ${WAITING[status]}` : ""
          return `Error: ${studyErrorText(e)}${waiting}.`
        }
        if (e instanceof HttpError) {
          return e.status === 404
            ? `Error: the ${service} could not find that.`
            : `Error: the ${service} returned HTTP ${e.status}.`
        }
        return `Error: the ${service} is not reachable.`
      }
    }

  /** The session and its live question, or a sentence telling the agent to open one. */
  const live = async (): Promise<{ session: StudySession; attempt: StudyAttempt }> => {
    const session = await practice.current()
    const attempt = session?.state.attempt ?? null
    if (!session || !isLive(attempt)) throw new ToolInputError(NO_QUESTION)
    return { session, attempt }
  }

  return {
    get_next_question: safe("get_next_question", PRACTICE_SERVICE, async ({ domain }) => {
      const tag = optionalText(domain)
      const session = await practice.ensure()
      const current = session.state.attempt
      if (isLive(current)) return `Finish the current question first: it is waiting for ${WAITING[current.status]}.`
      const result = await openNext(session, "voice_tool", tag)
      if (!result) return tag ? `No ${tag} practice questions are due right now.` : "No practice questions are due right now."
      return JSON.stringify(questionResult(attemptOf(result)))
    }),

    propose_answer: safe("propose_answer", PRACTICE_SERVICE, async ({ transcript }) => {
      const raw = text(transcript, "transcript")
      const { session } = await live()
      // A proposal only: the learner commits it on screen (source ui), never the tool.
      await session.propose(raw)
      return `Shown on screen for the learner to confirm: "${raw}". It counts only once they confirm or correct it there; wait for that.`
    }),

    set_confidence: safe("set_confidence", PRACTICE_SERVICE, async ({ level }) => {
      const value = confidence(level)
      const { session, attempt } = await live()
      const { status } = attempt
      if (status !== "committed") {
        throw new ToolInputError(
          `confidence can be set only after the learner has confirmed an answer on screen; the question is waiting for ${WAITING[status]}.`,
        )
      }
      await session.confidence(value, undefined, "voice_tool")
      return JSON.stringify({ recorded: true, level: value })
    }),

    get_hint: safe("get_hint", PRACTICE_SERVICE, async () => {
      const { session } = await live()
      return JSON.stringify(hintResult(attemptOf(await session.hint("voice_tool"))))
    }),

    check_answer: safe("check_answer", PRACTICE_SERVICE, async () => {
      const { session } = await live()
      return JSON.stringify(outcomeResult(attemptOf(await session.check("voice_tool"))))
    }),

    give_up: safe("give_up", PRACTICE_SERVICE, async () => {
      const { session } = await live()
      return JSON.stringify(outcomeResult(attemptOf(await session.giveUp("voice_tool"))))
    }),

    search_library: safe("search_library", LIBRARY_SERVICE, async ({ query, domain }) => {
      const q = text(query, "query")
      const d = optionalText(domain)
      const attempt = (await practice.current())?.state.attempt ?? null
      const { pack } = await fetchJson<{ pack: SearchHit[] }>(BACKEND_URL, "/api/library/search", {
        method: "POST",
        body: JSON.stringify({
          query: q,
          k: 5,
          ...(d ? { domains: [d] } : {}),
          ...(isLive(attempt) ? { attempt_id: attempt.attempt_id } : {}),
        }),
      })
      return JSON.stringify({
        hits: pack.map((h) => ({
          title: h.title,
          domain: h.domain,
          source_type: h.source_type,
          pages: pages(h),
          excerpt: excerpt(h.text),
        })),
      })
    }),

    get_progress: safe("get_progress", PRACTICE_SERVICE, async () => {
      const tz = encodeURIComponent(timezone())
      const s = await fetchJson<PracticeSummary>(BACKEND_URL, `${PRACTICE}/summary?timezone=${tz}`)
      const { calibration } = await fetchJson<{ calibration?: Calibration[] }>(
        BACKEND_URL,
        `${PRACTICE}/analytics?timezone=${tz}&days=${CALIBRATION_DAYS}`,
      )
      return JSON.stringify({
        total: s.total,
        due: s.due,
        overdue: s.overdue,
        mistakes: s.mistakes,
        reviewed_today: s.reviewed_today,
        next_due: localTime(s.next_due_at),
        ...(Array.isArray(calibration)
          ? {
              calibration_30_days: calibration.map((c) => ({
                level: c.level,
                attempts: c.attempts,
                correct: c.correct,
              })),
            }
          : {}),
      })
    }),
  } satisfies Record<string, TutorTool>
}

export type TutorTools = ReturnType<typeof createTutorTools>
export type TutorToolName = keyof TutorTools
