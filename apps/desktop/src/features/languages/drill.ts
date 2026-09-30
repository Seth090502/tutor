// Pure helpers for the Practice tab over C12 study sessions. The session (src/lib/study.ts) is the state
// machine: the server decides each attempt's status and what may be shown. Nothing here computes a verdict
// or a rating; it reads the accepted snapshot and words it. The language tags (`lang:<code>`,
// `card:production|recognition|listening`) are contract C10.
import type { StudyAttempt, StudyCheck, StudyQueueItem } from "@/lib/study"

export type CardKind = "production" | "recognition" | "listening" | "unknown"

/** The highest hint rung the screen will ask for: 1 nudge, 2 hint, 3 key step, 4 worked example. */
export const MAX_RUNG = 4

// ---- cards from the queue ----------------------------------------------------------------------

export function cardMeta(item: Pick<StudyQueueItem, "tags">): { kind: CardKind } {
  const kind = item.tags
    .map((t) => t.trim().toLowerCase())
    .find((t) => t.startsWith("card:"))
    ?.slice(5)
  return { kind: kind === "production" || kind === "recognition" || kind === "listening" ? kind : "unknown" }
}

const KIND_ORDER: Record<CardKind, number> = { production: 0, recognition: 1, listening: 2, unknown: 3 }

/** Production cards first (the default format), otherwise in queue order. The server already chose the language. */
export function orderQueue<T extends Pick<StudyQueueItem, "tags">>(items: T[]): T[] {
  return items
    .map((item, i) => ({ item, i, kind: cardMeta(item).kind }))
    .sort((x, y) => KIND_ORDER[x.kind] - KIND_ORDER[y.kind] || x.i - y.i)
    .map((x) => x.item)
}

/** The next card to open: the first in queue order that this tab has not finished and is not on screen. */
export function nextEntry(queue: StudyQueueItem[], done: readonly number[], shown: StudyAttempt | null): StudyQueueItem | undefined {
  return queue.find((q) => !done.includes(q.entry_id) && q.entry_id !== shown?.entry_id)
}

// ---- reading an attempt ------------------------------------------------------------------------

/** The ladder rungs given so far; other assistance (a gloss, a source) has no rung. */
export const hintsGiven = (a: StudyAttempt) => a.assistance.filter((x) => x.rung !== null)

const hintCap = (a: StudyAttempt) => Math.min(MAX_RUNG, a.item.rungs_available)

/** Hints open once the first answer has been checked and missed; the first answer is already sealed by then. */
export const canHint = (a: StudyAttempt) => a.status === "remediating" && hintsGiven(a).length < hintCap(a)

/** Why the hint button is off, in words; null when it is on or does not apply. */
export function hintBlocker(a: StudyAttempt): string | null {
  if (a.status !== "remediating") return null
  if (a.item.rungs_available === 0) return "This card has no hints."
  if (hintsGiven(a).length >= hintCap(a)) return "No more hints for this card."
  return null
}

export const latestCheck = (a: StudyAttempt): StudyCheck | null => a.checks[a.checks.length - 1] ?? null

/** The target-language sentence to speak once the answer is released. */
export const targetSentence = (kind: CardKind, prompt: string, answer: string) => (kind === "production" ? answer : prompt)

// ---- feedback ----------------------------------------------------------------------------------

export type Segment = { text: string; differs: boolean }

/**
 * Marks the characters of `answer` that differ from `target` when the two have the same length (an
 * accent-only slip never changes the length once composed). Different lengths give one unmarked segment.
 */
export function diffChars(answer: string, target: string): Segment[] {
  const a = Array.from(answer.normalize("NFC"))
  const t = Array.from(target.normalize("NFC"))
  if (a.length !== t.length) return [{ text: answer, differs: false }]
  const out: Segment[] = []
  a.forEach((ch, i) => {
    const differs = ch !== t[i]
    const last = out[out.length - 1]
    if (last && last.differs === differs) last.text += ch
    else out.push({ text: ch, differs })
  })
  return out
}

/** One plain sentence naming what the check found. The reference answer is not released while a miss is open. */
export function feedbackLine(check: Pick<StudyCheck, "verdict" | "diagnostic">): string {
  if (check.verdict === "correct") return "Your answer matches the reference."
  if (check.verdict === "incorrect" && check.diagnostic === "accent")
    return "Check your accents: the letters are right but an accent or diacritic is wrong."
  if (check.verdict === "incorrect") return "Yours does not match. Try again from what you know, ask for a hint, or give up to see the answer."
  return "This one cannot be checked automatically. Compare it with the reference and say whether you had it."
}

export function verdictWord(check: Pick<StudyCheck, "verdict" | "diagnostic">): string {
  if (check.verdict === "correct") return "Correct"
  if (check.verdict === "incorrect") return check.diagnostic === "accent" ? "Accent slip" : "Not quite"
  return "Compare"
}

/** How a closed attempt ended, as a word and whether it counts as a success (null: neither). */
export function outcomeOf(a: StudyAttempt): { word: string; ok: boolean | null } {
  const reason = a.closure?.reason
  if (reason === "correct") return { word: a.checks[0]?.verdict === "correct" ? "Correct" : "Correct on a repair", ok: true }
  if (reason === "revealed") return { word: "Answer shown", ok: false }
  if (reason === "self_assessed") {
    const had = a.self_assessment?.judged === "correct"
    return { word: had ? "You had it" : "You missed it", ok: had }
  }
  if (reason === "invalidated") return { word: "Set aside", ok: null }
  return { word: "Skipped", ok: null }
}

const RATING_WORD: Record<string, string> = { good: "Good", again: "Again" }

const PROVENANCE: Record<string, string> = {
  verified: "The app checked your first answer and you had no help before it.",
  assisted: "You had help before answering, so this counts as assisted.",
  self_report: "The app could not check this answer, so the rating is your own judgement.",
}

/** The rating and where it came from, in words. */
export function ratingLine(a: StudyAttempt): { rating: string | null; why: string } {
  const d = a.closure?.scheduler_disposition
  if (!d || d.rating === null) return { rating: null, why: "No review was recorded for this card." }
  return { rating: RATING_WORD[d.rating] ?? d.rating, why: (d.provenance && PROVENANCE[d.provenance]) || "" }
}

/** The next review time from an applied review (epoch seconds), in the learner's time zone. */
export function formatDue(epochSeconds: number | null | undefined): string | null {
  if (epochSeconds == null) return null
  return new Date(epochSeconds * 1000).toLocaleString("en-US", {
    timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    dateStyle: "medium",
    timeStyle: "short",
  })
}

/** The hint-ladder note shown beside the rating. */
export function assistanceNote(hints: number): string {
  if (hints === 0) return "No hints used."
  return `${hints} hint${hints === 1 ? "" : "s"} used after your first answer.`
}
