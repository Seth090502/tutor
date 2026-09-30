// PENDING, REMOVE AT INTEGRATION. Contract C10 has no word-lookup route, but the Read tab shows a gloss
// when a learner taps a word. Until the lead decides the source (for example a backend route
// `POST /api/languages/gloss {lang, word, sentence}` returning `{gloss}`), this stand-in returns null and
// the screen asks the learner to write the meaning themselves before adding the card.
import type { TargetLanguage } from "./language"

export async function lookupGloss(_lang: TargetLanguage, _word: string, _sentence: string): Promise<string | null> {
  return null
}
