// The end-of-session digest for the role-play partner (LANGUAGES.md item 5, fluency mode): while the
// learner talks the partner logs errors silently through the `log_error` client tool, and when the
// call ends the screen shows at most three of them, each of which can become a card.
//
// Ranking: an error that broke comprehension first, then the one that recurred most, then the one
// heard first. The digest never claims the session was error-free: an empty log means the partner
// logged nothing, not that there was nothing to log.

export type LoggedError = {
  /** What the learner said, as the recogniser heard it. */
  said: string
  /** The corrected form. */
  corrected: string
  /** What the corrected form means, in English. */
  english: string
  /** One short rule in English; may be empty. */
  rule: string
  breaksComprehension: boolean
}

export type DigestItem = {
  corrected: string
  english: string
  rule: string
  /** Up to two distinct things the learner said. */
  said: string[]
  count: number
  breaksComprehension: boolean
}

export const MAX_DIGEST = 3

/** Case, accents' composition, punctuation and spacing folded away, so repeats of one error group together. */
export function errorKey(text: string): string {
  return text
    .normalize("NFC")
    .toLowerCase()
    .replace(/[\p{P}\p{S}]+/gu, " ")
    .replace(/\s+/g, " ")
    .trim()
}

export function buildDigest(errors: LoggedError[], max = MAX_DIGEST): DigestItem[] {
  const groups = new Map<string, DigestItem & { first: number }>()
  errors.forEach((e, i) => {
    const key = errorKey(e.corrected)
    // A "correction" identical to what was said is not an error.
    if (!key || key === errorKey(e.said)) return
    const g = groups.get(key)
    if (!g) {
      groups.set(key, {
        corrected: e.corrected,
        english: e.english,
        rule: e.rule,
        said: e.said ? [e.said] : [],
        count: 1,
        breaksComprehension: e.breaksComprehension,
        first: i,
      })
      return
    }
    g.count++
    g.breaksComprehension ||= e.breaksComprehension
    if (!g.rule && e.rule) g.rule = e.rule
    if (!g.english && e.english) g.english = e.english
    if (e.said && g.said.length < 2 && !g.said.some((s) => errorKey(s) === errorKey(e.said))) g.said.push(e.said)
  })
  return [...groups.values()]
    .sort((x, y) => Number(y.breaksComprehension) - Number(x.breaksComprehension) || y.count - x.count || x.first - y.first)
    .slice(0, Math.max(0, max))
    .map(({ first: _first, ...item }) => item)
}

/** A digest item can become a card only when its meaning is known (the cards route requires one). */
export const cardable = (item: DigestItem) => item.corrected.trim() !== "" && item.english.trim() !== ""

export function digestCards(items: DigestItem[]): { text: string; meaning: string }[] {
  return items.filter(cardable).map((i) => ({ text: i.corrected.trim(), meaning: i.english.trim() }))
}

/** The line under an empty digest: honest about what the log can and cannot say. */
export const EMPTY_DIGEST =
  "The partner logged no errors this time. That is not proof there were none: it only flags what it noticed."
