// The one seam between the Languages screen and the outside world: the Tauri shell (speech, secrets,
// config). The screen never imports the shell directly, so the preview page and the tests swap in
// fixtures with `setPlatform`. HTTP to the backend goes through `api.ts` and `fetch`.
import { inTauri, shell as appShell, type SttResult, type TtsOptions, type TtsVoice as Voice } from "@/lib/backend"
import type { TargetLanguage } from "@/lib/language"
import { lookupGloss } from "./api"

export type { SttResult, TtsOptions, Voice }

export type Platform = {
  inTauri: boolean
  /** Presence only, never the key. */
  hasElevenLabsKey(): Promise<boolean>
  configValue(key: string): Promise<string | null>
  configSet(key: string, value: string): Promise<void>
  ttsAudio(text: string, lang: string, opts?: TtsOptions): Promise<ArrayBuffer>
  ttsVoices(lang: string): Promise<Voice[]>
  sttTranscribe(audio: Uint8Array, lang: string): Promise<SttResult>
  conversationToken(agentId: string): Promise<string>
  /** An English gloss for a tapped word, shown on screen only; null when no source is wired. */
  gloss(lang: TargetLanguage, word: string, sentence: string): Promise<string | null>
}

const realPlatform: Platform = {
  inTauri,
  hasElevenLabsKey: async () => (await appShell.secretsStatus()).elevenlabs,
  configValue: async (key) => (await appShell.configGet())[key] ?? null,
  configSet: (key, value) => appShell.configSet(key, value),
  ttsAudio: (text, lang, opts) => appShell.ttsAudio(text, lang, opts),
  ttsVoices: async (lang) => (await appShell.ttsVoices(lang)).voices,
  sttTranscribe: (audio, lang) => appShell.sttTranscribe(audio, lang),
  conversationToken: async (agentId) => (await appShell.elevenlabsConversationToken(agentId)).token,
  gloss: lookupGloss,
}

let current: Platform = realPlatform

export const platform = (): Platform => current

/** For the preview page and tests: replace some or all of the platform. Returns a restore function. */
export function setPlatform(patch: Partial<Platform>): () => void {
  const before = current
  current = { ...current, ...patch }
  return () => {
    current = before
  }
}
