// PENDING, REMOVE AT INTEGRATION. Stand-ins for the Tauri speech commands of contract C10
// (`tts_audio`, `tts_voices`, `stt_transcribe`), which the Rust worker builds, plus a read of the
// per-language config keys (`agent_<lang>`, `voice_<lang>`). The lead switches `../platform.ts` to
// the real `shell.ttsAudio`, `shell.ttsVoices`, `shell.sttTranscribe` and deletes this file.
//
// Every call goes through `../platform.ts`, so a signature difference is fixed in that one file.
import { invoke } from "@tauri-apps/api/core"

export type Voice = {
  voice_id: string
  name: string
  labels: Record<string, string>
  preview_url: string | null
}
export type SttWord = { text: string; start: number; end: number }
export type SttResult = { text: string; words: SttWord[] }
export type TtsOptions = { voiceId?: string; model?: string; speed?: number }

export const shell = {
  /** Raw MP3 bytes; cached by the shell at <app data>/tts-cache. */
  ttsAudio: (text: string, lang: string, opts: TtsOptions = {}) =>
    invoke<ArrayBuffer | number[]>("tts_audio", {
      text,
      lang,
      voiceId: opts.voiceId,
      model: opts.model,
      speed: opts.speed,
    }),
  ttsVoices: (lang: string) => invoke<{ voices: Voice[] }>("tts_voices", { lang }),
  /** webm/opus or wav bytes. The binary-argument convention is the Rust worker's to settle. */
  sttTranscribe: (audio: Uint8Array, lang: string) =>
    invoke<SttResult>("stt_transcribe", { audio: Array.from(audio), lang }),
  /** `config_get` today returns only `elevenlabs_agent_id`; C10 adds `agent_<lang>` and `voice_<lang>`. */
  configValue: async (key: string): Promise<string | null> => {
    const config = await invoke<Record<string, string | null>>("config_get")
    return config[key] ?? null
  },
}
