// The backend routes the Languages screen calls besides the C12 study sessions (src/lib/study.ts, driven by
// practice.ts): the C10 language routes. Shapes are the contracts in docs/tutor/CONTRACTS.md.
import { BACKEND_URL, HttpError, fetchJson } from "@/lib/backend"
import type { TargetLanguage } from "@/lib/language"
import type { Band } from "./agent"

export const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))
/** A rejection that is not an HTTP status means nothing answered on the port. */
export const isOffline = (e: unknown) => !(e instanceof HttpError)

// ---- C10 language routes -----------------------------------------------------------------------

export type GradedText = {
  text: string
  sentences: string[]
  coverage: number
  unknown: string[]
  questions: { q: string; a: string }[]
}

export const gradedText = (lang: TargetLanguage, band: Band, topic?: string) =>
  fetchJson<GradedText>(BACKEND_URL, "/api/languages/graded-text", {
    method: "POST",
    body: JSON.stringify({ lang, band, ...(topic?.trim() ? { topic: topic.trim() } : {}) }),
  })

/** An English gloss for a tapped word in its sentence, from the lesson LLM; shown on screen only. */
export const lookupGloss = async (lang: TargetLanguage, word: string, sentence: string) =>
  (
    await fetchJson<{ gloss: string | null }>(BACKEND_URL, "/api/languages/gloss", {
      method: "POST",
      body: JSON.stringify({ lang, word, sentence }),
    })
  ).gloss

export const addCards = (lang: TargetLanguage, items: { text: string; meaning: string; example?: string }[]) =>
  fetchJson<{ created: number; entry_ids: number[] }>(BACKEND_URL, "/api/languages/cards", {
    method: "POST",
    body: JSON.stringify({ lang, items }),
  })

/** The command that builds a starter deck (C10); the screen shows it, it does not run it. */
export const starterDeckCommand = (lang: TargetLanguage, size = 200) =>
  `python -m deeptutor.services.languages.build_deck --lang ${lang} --size ${size}`
