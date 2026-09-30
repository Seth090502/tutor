// The typed drill over language cards: a pure state machine for one card, plus the small helpers
// around it. The routes are contract C7 (attempts, confidence, hint ladder, check, review); the
// language tags and the accent diagnostic are contract C10.
//
// The rules it enforces, from PEDAGOGY.md and LANGUAGES.md:
//  - attempt first: a hint needs a typed attempt, and each further rung needs a new one;
//  - the answer rung (5) is never requested from the screen;
//  - confidence is chosen before the check and cannot change after it;
//  - the learner's self-judgement is asked for only when the check could not decide (correct: null);
//  - the rating is never computed here: the server returns it.

export type Confidence = 1 | 2 | 3 | 4
export type CardKind = "production" | "recognition" | "listening" | "unknown"

/** The highest hint rung the screen will ask for: 1 nudge, 2 hint, 3 key step, 4 worked example. */
export const MAX_RUNG = 4

export type Card = {
  questionId: number
  attemptId: string
  prompt: string
  kind: CardKind
  /** From the server when the attempt opened (0-4). */
  rungsAvailable: number
}

export type HintRung = { rung: number; text: string | null; isAnswer: boolean }

export type CheckResult = {
  correct: boolean | null
  correctAnswer: string
  explanation: string
  misconception: string | null
  /** "accent" when only accents or diacritics differ (contract C10). */
  diagnostic: string | null
}

export type ReviewResult = { rating: string; nextDue: string | null }

export type DrillState = {
  phase: "attempt" | "checked" | "done"
  card: Card
  draft: string
  confidence: Confidence | null
  hints: HintRung[]
  /** The attempt that was on screen when the last hint was asked for. */
  hintDraft: string | null
  pending: "hint" | "check" | "review" | null
  error: string | null
  check: CheckResult | null
  /** The learner's own verdict, asked for only when `check.correct` is null. */
  judged: boolean | null
  review: ReviewResult | null
}

export type DrillAction =
  | { type: "draft"; text: string }
  | { type: "confidence"; level: Confidence }
  | { type: "hint:start" }
  | { type: "hint:done"; hint: HintRung }
  | { type: "check:start" }
  | { type: "check:done"; result: CheckResult }
  | { type: "judge"; correct: boolean }
  | { type: "review:start" }
  | { type: "review:done"; result: ReviewResult }
  | { type: "failed"; message: string }

export function newDrill(card: Card): DrillState {
  return {
    phase: "attempt",
    card,
    draft: "",
    confidence: null,
    hints: [],
    hintDraft: null,
    pending: null,
    error: null,
    check: null,
    judged: null,
    review: null,
  }
}

const attemptText = (s: DrillState) => s.draft.trim()

export function canHint(s: DrillState): boolean {
  if (s.phase !== "attempt" || s.pending) return false
  if (s.hints.length >= Math.min(MAX_RUNG, s.card.rungsAvailable)) return false
  if (s.hints.some((h) => h.isAnswer)) return false
  const attempt = attemptText(s)
  return attempt !== "" && attempt !== s.hintDraft
}

/** Why the hint button is off, in words; null when it is on. */
export function hintBlocker(s: DrillState): string | null {
  if (s.phase !== "attempt") return null
  if (s.card.rungsAvailable === 0) return "This card has no hints."
  if (s.hints.length >= Math.min(MAX_RUNG, s.card.rungsAvailable)) return "No more hints for this card."
  if (attemptText(s) === "") return "Type an attempt first."
  if (attemptText(s) === s.hintDraft) return "Change your attempt before the next hint."
  return null
}

export function canCheck(s: DrillState): boolean {
  return s.phase === "attempt" && !s.pending && attemptText(s) !== "" && s.confidence !== null
}

/** The check could not decide, so the learner says whether they had it. */
export function needsJudgement(s: DrillState): boolean {
  return s.phase === "checked" && s.check !== null && s.check.correct === null && s.judged === null
}

export function canReview(s: DrillState): boolean {
  return s.phase === "checked" && !s.pending && s.check !== null && (s.check.correct !== null || s.judged !== null)
}

/** Any hint makes the attempt assisted: FSRS learns only from unassisted retrieval. */
export const assisted = (s: DrillState) => s.hints.length > 0

export function drillReducer(s: DrillState, a: DrillAction): DrillState {
  switch (a.type) {
    case "draft":
      return s.phase === "attempt" ? { ...s, draft: a.text, error: null } : s
    case "confidence":
      return s.phase === "attempt" && !s.pending ? { ...s, confidence: a.level } : s
    case "hint:start":
      return canHint(s) ? { ...s, pending: "hint", hintDraft: attemptText(s), error: null } : s
    case "hint:done":
      return s.pending === "hint" ? { ...s, pending: null, hints: [...s.hints, a.hint] } : s
    case "check:start":
      return canCheck(s) ? { ...s, pending: "check", error: null } : s
    case "check:done":
      return s.pending === "check" ? { ...s, pending: null, phase: "checked", check: a.result } : s
    case "judge":
      return s.phase === "checked" && s.check?.correct === null && s.judged === null ? { ...s, judged: a.correct } : s
    case "review:start":
      return canReview(s) ? { ...s, pending: "review", error: null } : s
    case "review:done":
      return s.pending === "review" ? { ...s, pending: null, phase: "done", review: a.result } : s
    case "failed":
      return { ...s, pending: null, error: a.message }
    default:
      return s
  }
}

// ---- cards from the queue ----------------------------------------------------------------------

type Tagged = {
  categories?: ({ name?: string } | string)[] | null
  tags?: ({ name?: string } | string)[] | null
}

/** Tag names (`lang:es`, `card:production`) from an entry's categories or tags. */
function tagNames(entry: Tagged): string[] {
  const names: string[] = []
  for (const list of [entry.categories, entry.tags]) {
    for (const t of list ?? []) {
      const name = typeof t === "string" ? t : t?.name
      if (typeof name === "string") names.push(name.trim().toLowerCase())
    }
  }
  return names
}

export function cardMeta(entry: Tagged): { lang: string | null; kind: CardKind } {
  const names = tagNames(entry)
  const lang = names.find((n) => n.startsWith("lang:"))?.slice(5) ?? null
  const kind = names.find((n) => n.startsWith("card:"))?.slice(5)
  return {
    lang,
    kind: kind === "production" || kind === "recognition" || kind === "listening" ? kind : "unknown",
  }
}

const KIND_ORDER: Record<CardKind, number> = { production: 0, recognition: 1, listening: 2, unknown: 3 }

/** The language's cards only, production first (the default format), otherwise in queue order. */
export function orderQueue<T extends { entry: Tagged }>(items: T[], lang: string): T[] {
  return items
    .map((item, i) => ({ item, i, meta: cardMeta(item.entry) }))
    .filter((x) => x.meta.lang === lang)
    .sort((x, y) => KIND_ORDER[x.meta.kind] - KIND_ORDER[y.meta.kind] || x.i - y.i)
    .map((x) => x.item)
}

/** The target-language sentence to speak once the card is checked. */
export function targetSentence(card: Card, check: CheckResult): string {
  return card.kind === "production" ? check.correctAnswer : card.prompt
}

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

/** One plain sentence naming what the check found. */
export function feedbackLine(check: CheckResult): string {
  if (check.correct === true) return "Your answer matches the reference."
  if (check.correct === false && check.diagnostic === "accent")
    return "The letters are right but an accent or diacritic is wrong."
  if (check.correct === false) return "Yours does not match the reference."
  return "This one cannot be checked automatically. Compare it with the reference and say whether you had it."
}

/** The hint-ladder note shown beside the rating. */
export function assistanceNote(hints: number): string {
  if (hints === 0) return "No hints used."
  return `${hints} hint${hints === 1 ? "" : "s"} used. The schedule learns only from unassisted recall, so the rating is lower.`
}
