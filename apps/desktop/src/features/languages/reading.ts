// Helpers for the Read tab: splitting a sentence into tappable words, and the coverage meter's bands.
import type { TargetLanguage } from "./_pending/language"

/** Known-word coverage to aim for when reading (LANGUAGES.md item 7). */
export const COVERAGE_LOW = 0.95
export const COVERAGE_HIGH = 0.98

export type CoverageBand = "below" | "target" | "above"

export function coverageBand(coverage: number): CoverageBand {
  if (coverage < COVERAGE_LOW) return "below"
  return coverage > COVERAGE_HIGH ? "above" : "target"
}

/** The meter's caption in words, so the colour is never the only signal. */
export function coverageWords(coverage: number, unknown: number): string {
  const band = coverageBand(coverage)
  const new_ = `${unknown} new word${unknown === 1 ? "" : "s"}`
  if (band === "below") return `Below the 95 to 98% target, with ${new_}. Expect to stop often.`
  if (band === "above") return `Above the target, with ${new_}. Easy reading: good for speed, less new vocabulary.`
  return `In the 95 to 98% target, with ${new_}.`
}

export type Part = { text: string; word: boolean }

// Latin scripts split on anything that is not a letter, mark, digit or an inner apostrophe or hyphen.
const LATIN_WORD = /[\p{L}\p{M}\p{N}]+(?:[-'’][\p{L}\p{M}\p{N}]+)*/gu
// French elision (l'homme, qu'il) is two words.
const ELISION = /^(\p{L}{1,2}['’])(.+)$/u

function latinParts(text: string, lang: TargetLanguage): Part[] {
  const parts: Part[] = []
  let last = 0
  for (const m of text.matchAll(LATIN_WORD)) {
    const start = m.index ?? 0
    if (start > last) parts.push({ text: text.slice(last, start), word: false })
    const elided = lang === "fr" ? ELISION.exec(m[0]) : null
    if (elided) {
      parts.push({ text: elided[1], word: true }, { text: elided[2], word: true })
    } else {
      parts.push({ text: m[0], word: true })
    }
    last = start + m[0].length
  }
  if (last < text.length) parts.push({ text: text.slice(last), word: false })
  return parts
}

// Intl.Segmenter is in every Chromium the app runs in, but not in this project's ES2020 lib typings.
type SegmenterCtor = new (
  locale: string,
  options: { granularity: "word" },
) => { segment(text: string): Iterable<{ segment: string; isWordLike?: boolean }> }

/** Splits a sentence into words and the gaps between them, keeping every character. */
export function tokenize(text: string, lang: TargetLanguage): Part[] {
  if (lang === "ja" || lang === "zh") {
    const Segmenter = (Intl as unknown as { Segmenter?: SegmenterCtor }).Segmenter
    if (Segmenter) {
      const seg = new Segmenter(lang === "ja" ? "ja" : "zh", { granularity: "word" })
      return [...seg.segment(text)].map((s) => ({ text: s.segment, word: Boolean(s.isWordLike) }))
    }
    // No segmenter: fall back to single characters, which is tappable if crude.
    return Array.from(text, (ch) => ({ text: ch, word: /[\p{L}\p{N}]/u.test(ch) }))
  }
  return latinParts(text, lang)
}

/** Case and composition folded, for comparing a tapped word with the server's unknown list. */
export const foldWord = (w: string) => w.normalize("NFC").toLowerCase().replace(/^['’-]+|['’-]+$/g, "")

export function isNew(word: string, unknown: Set<string>): boolean {
  return unknown.has(foldWord(word))
}

export const unknownSet = (unknown: string[]) => new Set(unknown.map(foldWord))

export type CardItem = { text: string; meaning: string; example?: string }

/** The body item for POST /api/languages/cards; null until the learner has written a meaning. */
export function cardItem(word: string, meaning: string, sentence: string): CardItem | null {
  const text = word.trim()
  const m = meaning.trim()
  if (!text || !m) return null
  const example = sentence.trim()
  return { text, meaning: m, ...(example ? { example } : {}) }
}
