// What the Practice tab does with a study session (C12): find or start this language's session, and the
// learner's moves on the card. Every move is guarded by the status the server last accepted, so a control
// that does not belong to the current status sends nothing (confidence before the answer is committed, for
// one). All of it goes through StudySession, which sends `X-Tutor-Source: ui`.
import type { TargetLanguage } from "@/lib/language"
import { StudySession, studyQueue, type AttemptStatus, type ConfidenceLevel, type StudyAttempt, type StudyQueueItem } from "@/lib/study"
import { canHint, nextEntry, orderQueue } from "./drill"

const QUEUE_LIMIT = 100
/** The reason recorded when the learner skips the confidence question. */
export const SKIPPED_REASON = "learner_skipped"

export const sessionKey = (lang: TargetLanguage) => `tutor.study.languages.${lang}`

function storedSessionId(lang: TargetLanguage): string | null {
  try {
    return window.localStorage.getItem(sessionKey(lang))
  } catch {
    return null
  }
}

function storeSessionId(lang: TargetLanguage, id: string | null) {
  try {
    if (id) window.localStorage.setItem(sessionKey(lang), id)
    else window.localStorage.removeItem(sessionKey(lang))
  } catch {
    // Storage blocked: the session starts fresh next time.
  }
}

export type Opened = { session: StudySession | null; queue: StudyQueueItem[] }

async function load(lang: TargetLanguage, fetchImpl?: typeof fetch): Promise<Opened> {
  const queue = orderQueue(await studyQueue({ tag: `lang:${lang}`, limit: QUEUE_LIMIT }, fetchImpl))
  const id = storedSessionId(lang)
  let session = id ? await StudySession.resume(id, fetchImpl) : null
  // A session is started only when there is something to practise, so an empty deck leaves none behind.
  if (!session && queue.length > 0) session = await StudySession.start("practice", fetchImpl)
  storeSessionId(lang, session?.state.session.session_id ?? null)
  return { session, queue }
}

const loading = new Map<TargetLanguage, Promise<Opened>>()

/**
 * This language's due cards (filtered by the server) and its practice session: the stored one resumed, or a
 * new one. Calls made while one is in flight share it, so a remount never starts a second session.
 */
export function openPractice(lang: TargetLanguage, fetchImpl?: typeof fetch): Promise<Opened> {
  const held = loading.get(lang)
  if (held) return held
  const p = load(lang, fetchImpl).finally(() => loading.delete(lang))
  loading.set(lang, p)
  return p
}

/** The moves on a card. Each returns null, having sent nothing, when the current status does not allow it. */
export function cardActions(session: StudySession) {
  const when = (...statuses: AttemptStatus[]): StudyAttempt | null => {
    const a = session.state.attempt
    return a && statuses.includes(a.status) ? a : null
  }
  return {
    answer: (text: string) => (when("open") && text.trim() ? session.commit(text.trim(), "typed") : null),
    dontKnow: () => (when("open") ? session.commit("", "dont_know") : null),
    confidence: (level: ConfidenceLevel) => (when("committed") ? session.confidence(level) : null),
    skipConfidence: () => (when("committed") ? session.confidence(null, SKIPPED_REASON) : null),
    check: () => (when("ready") ? session.check() : null),
    selfAssess: (judged: "correct" | "incorrect") => (when("awaiting_self_assessment") ? session.selfAssess(judged) : null),
    repair: (text: string) => (when("remediating") && text.trim() ? session.repair(text.trim(), "typed") : null),
    hint: () => {
      const a = when("remediating")
      return a && canHint(a) ? session.hint() : null
    },
    giveUp: () => (when("remediating") ? session.giveUp() : null),
  }
}

export type CardActions = ReturnType<typeof cardActions>

/** Opens the next card that is still to do; null when there is none. */
export function openNext(session: StudySession, queue: StudyQueueItem[], done: readonly number[]) {
  const item = nextEntry(queue, done, session.state.attempt)
  return item ? session.open(item.entry_id) : null
}
