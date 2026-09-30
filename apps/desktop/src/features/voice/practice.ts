// The Tutor screen's one study session (contract C12). The voice tools and the typed controls share it,
// so both drive the same server-side attempt, and the screen shows only what the server accepted.
import {
  type CommandResult,
  type ConfidenceLevel,
  type Snapshot,
  type StudyAttempt,
  type StudyErrorCode,
  StudyError,
  StudySession,
  type StudySource,
  studyQueue,
} from "@/lib/study"

/** Where the screen keeps its session id between restarts. */
export const STORAGE_KEY = "tutor.study.voice"
/** The server's session id format; anything else in storage is ignored. */
const SESSION_ID = /^[0-9a-f]{32}$/

function storedId(): string | null {
  try {
    const id = localStorage.getItem(STORAGE_KEY)
    return id && SESSION_ID.test(id) ? id : null
  } catch {
    return null
  }
}

function storeId(id: string | null) {
  try {
    if (id) localStorage.setItem(STORAGE_KEY, id)
    else localStorage.removeItem(STORAGE_KEY)
  } catch {
    // No storage (private window, tests): the session still works until the screen closes.
  }
}

export type VoicePractice = ReturnType<typeof createVoicePractice>

/** One per Tutor screen. The session starts when practice first starts, not on mount. */
export function createVoicePractice() {
  let session: StudySession | null = null
  let snapshot: Snapshot | null = null
  let resuming: Promise<StudySession | null> | null = null
  let starting: Promise<StudySession> | null = null
  const listeners = new Set<() => void>()

  const notify = () => {
    for (const listener of [...listeners]) listener()
  }

  const adopt = (s: StudySession) => {
    session = s
    snapshot = s.state
    storeId(s.state.session.session_id)
    s.subscribe((next) => {
      snapshot = next
      notify()
    })
    notify()
    return s
  }

  /** The session on screen, or the stored one resumed; null when there is none. Never starts one. */
  const current = (): Promise<StudySession | null> => {
    if (session) return Promise.resolve(session)
    const id = storedId()
    if (!id) return Promise.resolve(null)
    resuming ??= StudySession.resume(id)
      .then((s) => {
        if (s) return session ?? adopt(s)
        storeId(null)
        return null
      })
      .finally(() => {
        resuming = null
      })
    return resuming
  }

  /** The session, starting one in practice mode the first time practice starts. */
  const ensure = async (): Promise<StudySession> => {
    const existing = await current()
    if (existing) return existing
    if (session) return session
    starting ??= StudySession.start("practice")
      .then(adopt)
      .finally(() => {
        starting = null
      })
    return starting
  }

  return {
    /** The adopted session, if any, without touching the network. */
    get session() {
      return session
    },
    /** For useSyncExternalStore: the accepted snapshot, or null before a session exists. */
    getSnapshot: () => snapshot,
    subscribe: (listener: () => void) => {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    current,
    ensure,
  }
}

/** Opens the next due item, optionally one tagged `tag`; null when nothing is due. */
export async function openNext(session: StudySession, source: StudySource, tag?: string): Promise<CommandResult | null> {
  const [item] = await studyQueue({ tag })
  return item ? session.open(item.entry_id, source) : null
}

const ERROR_TEXT: Record<StudyErrorCode, string> = {
  conflict: "the session changed; state refreshed, try again",
  invalid_transition: "that step is not possible at this point",
  invalid_input: "the practice service did not accept that",
  not_found: "the practice service could not find that question or session",
  unauthorized: "only the learner can do that, on screen",
  budget_exceeded: "the practice budget is used up",
  unavailable: "the practice service is unavailable right now",
  unverified: "the practice service could not verify that",
  network: "the practice service is not reachable",
}

/** A StudyError as a readable phrase, without a leading capital or a full stop. */
export function studyErrorText(e: unknown): string {
  if (e instanceof StudyError) return ERROR_TEXT[e.code] ?? `the practice service refused that (${e.code})`
  return "something went wrong in the app"
}

/** What an accepted typed action was, for telling the voice tutor about it. */
export type PanelAction = "next" | "answer" | "confidence" | "check" | "hint" | "repair" | "give_up" | "self_assess"

/**
 * The typed controls (source ui). `onAccepted` runs after the server accepted an action, with the
 * attempt as accepted; a refused action throws a StudyError instead.
 */
export function panelActions(
  session: StudySession,
  onAccepted: (action: PanelAction, attempt: StudyAttempt) => void = () => {},
) {
  const accepted = async (action: PanelAction, sent: Promise<CommandResult>) => {
    const result = await sent
    if (result.public_state.attempt) onAccepted(action, result.public_state.attempt)
    return result
  }
  return {
    /** Null when nothing is due. */
    next: async () => {
      const result = await openNext(session, "ui")
      if (result?.public_state.attempt) onAccepted("next", result.public_state.attempt)
      return result
    },
    answer: (response: string) => accepted("answer", session.commit(response, "typed")),
    dontKnow: () => accepted("answer", session.commit("", "dont_know")),
    /** What the tutor heard, as heard or as edited: the first answer while open, a retry while remediating. */
    confirm: (rawTranscript: string, confirmedText: string) => {
      const confirmation = { raw_transcript: rawTranscript, confirmed_text: confirmedText }
      return session.state.attempt?.status === "remediating"
        ? accepted("repair", session.repair(confirmedText, "spoken", confirmation))
        : accepted("answer", session.commit(confirmedText, "spoken", confirmation))
    },
    confidence: (level: ConfidenceLevel) => accepted("confidence", session.confidence(level, undefined, "ui")),
    skipConfidence: (reason: string) => accepted("confidence", session.confidence(null, reason, "ui")),
    check: () => accepted("check", session.check("ui")),
    hint: () => accepted("hint", session.hint("ui")),
    repair: (response: string) => accepted("repair", session.repair(response, "typed")),
    giveUp: () => accepted("give_up", session.giveUp("ui")),
    selfAssess: (judged: "correct" | "incorrect") => accepted("self_assess", session.selfAssess(judged)),
  }
}

export type PanelActions = ReturnType<typeof panelActions>
