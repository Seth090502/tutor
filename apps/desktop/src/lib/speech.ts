// Speech for the Languages world (contract C10). Text to speech plays MP3 bytes from the shell,
// which caches every clip on disk; this keeps a small in-memory LRU of object URLs on top so a
// repeated sentence plays at once. Speech to text records WebM/Opus with MediaRecorder and sends
// it to the shell, which calls Scribe with the language pinned.

import { useCallback, useEffect, useRef, useState } from "react"

import { shell, type SttResult, type TtsOptions } from "./backend"
import { languageMeta, type TargetLanguage } from "./language"

/** A least-recently-used map. `onEvict` receives each value pushed out by a newer one. */
export class Lru<V> {
  private readonly map = new Map<string, V>()

  constructor(
    readonly capacity: number,
    private readonly onEvict: (value: V) => void = () => {},
  ) {}

  get size() {
    return this.map.size
  }

  get(key: string): V | undefined {
    const value = this.map.get(key)
    if (value !== undefined) {
      this.map.delete(key)
      this.map.set(key, value)
    }
    return value
  }

  set(key: string, value: V) {
    this.map.delete(key)
    this.map.set(key, value)
    for (const [oldest, evicted] of this.map) {
      if (this.map.size <= this.capacity) break
      this.map.delete(oldest)
      this.onEvict(evicted)
    }
  }

  delete(key: string) {
    this.map.delete(key)
  }
}

type ClipCacheDeps = {
  fetchAudio: (text: string, lang: string, options: TtsOptions) => Promise<ArrayBuffer>
  createUrl?: (blob: Blob) => string
  revokeUrl?: (url: string) => void
  capacity?: number
}

/** Object URLs for TTS clips, fetched once per (text, lang, voice, model, speed). */
export function createClipCache({
  fetchAudio,
  createUrl = (blob) => URL.createObjectURL(blob),
  revokeUrl = (url) => URL.revokeObjectURL(url),
  capacity = 32,
}: ClipCacheDeps) {
  // Promises, so concurrent requests for one clip share a single call.
  const clips = new Lru<Promise<string>>(capacity, (clip) => void clip.then(revokeUrl, () => {}))
  return {
    url(text: string, lang: string, options: TtsOptions = {}): Promise<string> {
      const key = JSON.stringify([text, lang, options.voiceId ?? null, options.model ?? null, options.speed ?? null])
      const cached = clips.get(key)
      if (cached) return cached
      const clip = fetchAudio(text, lang, options).then((bytes) =>
        createUrl(new Blob([bytes], { type: "audio/mpeg" })),
      )
      clips.set(key, clip)
      clip.catch(() => {
        if (clips.get(key) === clip) clips.delete(key)
      })
      return clip
    },
  }
}

const clips = createClipCache({ fetchAudio: (text, lang, options) => shell.ttsAudio(text, lang, options) })
let playing: HTMLAudioElement | null = null

/** Plays `text` in the target language, stopping any clip still playing. Resolves once it starts. */
export async function speak(text: string, lang: TargetLanguage, options: TtsOptions = {}): Promise<HTMLAudioElement> {
  const url = await clips.url(text, lang, options)
  stopSpeaking()
  const audio = new Audio(url)
  playing = audio
  await audio.play()
  return audio
}

export function stopSpeaking() {
  playing?.pause()
  playing = null
}

/** Scribe transcript of a recording, with the target language pinned. */
export function transcribe(audio: Uint8Array, lang: TargetLanguage): Promise<SttResult> {
  return shell.sttTranscribe(audio, languageMeta(lang).scribe)
}

const RECORDING_TYPES = ["audio/webm;codecs=opus", "audio/webm"]

/** The first recording type the browser supports, or undefined for the recorder's default. */
export function pickRecordingType(isSupported: (type: string) => boolean): string | undefined {
  return RECORDING_TYPES.find(isSupported)
}

export type RecorderState = "idle" | "recording" | "error"

type Session = { recorder: MediaRecorder; stream: MediaStream; chunks: Blob[] }

/**
 * Records the microphone with MediaRecorder. `start()` asks for the microphone and begins;
 * `stop()` resolves with the recording's bytes (null when nothing was recording). The microphone
 * is released on stop and on unmount.
 */
export function useRecorder() {
  const [state, setState] = useState<RecorderState>("idle")
  const [error, setError] = useState<string | null>(null)
  const session = useRef<Session | null>(null)
  const starting = useRef(false)

  const release = useCallback(() => {
    session.current?.stream.getTracks().forEach((track) => track.stop())
    session.current = null
  }, [])

  const start = useCallback(async () => {
    if (session.current || starting.current) return
    starting.current = true
    setError(null)
    let stream: MediaStream | null = null
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const mimeType = pickRecordingType((type) => MediaRecorder.isTypeSupported(type))
      const recorder = new MediaRecorder(stream, mimeType ? { mimeType } : undefined)
      const chunks: Blob[] = []
      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunks.push(e.data)
      }
      session.current = { recorder, stream, chunks }
      recorder.start()
      setState("recording")
    } catch (e) {
      stream?.getTracks().forEach((track) => track.stop())
      session.current = null
      setError(e instanceof Error ? e.message : String(e))
      setState("error")
    } finally {
      starting.current = false
    }
  }, [])

  const stop = useCallback(async (): Promise<Uint8Array | null> => {
    const current = session.current
    if (!current) return null
    const stopped = new Promise<void>((resolve) =>
      current.recorder.addEventListener("stop", () => resolve(), { once: true }),
    )
    current.recorder.stop()
    await stopped
    release()
    setState("idle")
    const blob = new Blob(current.chunks, { type: current.recorder.mimeType })
    return new Uint8Array(await blob.arrayBuffer())
  }, [release])

  useEffect(
    () => () => {
      if (session.current?.recorder.state === "recording") session.current.recorder.stop()
      release()
    },
    [release],
  )

  return { state, error, start, stop }
}
