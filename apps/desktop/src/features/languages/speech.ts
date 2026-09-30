// Audio in and out for the Languages screen: playing ElevenLabs speech through the shell (the key never
// reaches the webview) and recording the learner's voice for the speaking check. Everything goes through
// `platform()`, which the preview page and the tests replace.
import { useCallback, useEffect, useRef, useState } from "react"

import { platform, type TtsOptions } from "./platform"

const MAX_CACHED = 160
const urls = new Map<string, string>()

const keyOf = (text: string, lang: string, opts: TtsOptions) =>
  JSON.stringify([text, lang, opts.voiceId ?? "", opts.model ?? "", opts.speed ?? ""])

/** An object URL for the spoken text. Cached in memory for the life of the page; the shell caches the MP3 on disk. */
export async function audioUrl(text: string, lang: string, opts: TtsOptions = {}): Promise<string> {
  const key = keyOf(text, lang, opts)
  const hit = urls.get(key)
  if (hit) return hit
  const bytes = await platform().ttsAudio(text, lang, opts)
  const url = URL.createObjectURL(new Blob([bytes], { type: "audio/mpeg" }))
  urls.set(key, url)
  if (urls.size > MAX_CACHED) {
    const oldest = urls.keys().next().value as string
    URL.revokeObjectURL(urls.get(oldest) as string)
    urls.delete(oldest)
  }
  return url
}

/** Fetches ahead so the next trial starts without a wait. Failures surface when the trial plays it. */
export function prefetch(text: string, lang: string, opts: TtsOptions = {}) {
  audioUrl(text, lang, opts).catch(() => {})
}

let current: HTMLAudioElement | null = null

export function stopAudio() {
  if (!current) return
  current.pause()
  current = null
}

/** Plays a URL, stopping whatever was playing. Resolves when it ends; rejects if it cannot play. */
export function playUrl(url: string): Promise<void> {
  stopAudio()
  const el = new Audio(url)
  current = el
  return new Promise((resolve, reject) => {
    el.onended = () => {
      if (current === el) current = null
      resolve()
    }
    el.onerror = () => {
      if (current === el) current = null
      reject(new Error("The audio could not be played."))
    }
    el.play().catch((e: unknown) => {
      if (current === el) current = null
      // A pause from stopAudio() rejects play(); that is not an error.
      if (e instanceof DOMException && e.name === "AbortError") resolve()
      else reject(e instanceof Error ? e : new Error(String(e)))
    })
  })
}

export type SayState = { status: "idle" | "loading" | "playing" | "error"; error: string | null; playing: string | null }

/** Text to speech for buttons: `say(text)` plays it, one at a time, with a readable error. */
export function useSay(lang: string, opts: TtsOptions = {}) {
  const [state, setState] = useState<SayState>({ status: "idle", error: null, playing: null })
  const token = useRef(0)
  const voice = opts.voiceId
  const model = opts.model
  const speed = opts.speed

  useEffect(
    () => () => {
      token.current++
      stopAudio()
    },
    [],
  )

  const say = useCallback(
    async (text: string) => {
      const mine = ++token.current
      setState({ status: "loading", error: null, playing: text })
      try {
        const url = await audioUrl(text, lang, { voiceId: voice, model, speed })
        if (mine !== token.current) return
        setState({ status: "playing", error: null, playing: text })
        await playUrl(url)
        if (mine === token.current) setState({ status: "idle", error: null, playing: null })
      } catch (e) {
        if (mine === token.current) {
          setState({ status: "error", error: e instanceof Error ? e.message : String(e), playing: null })
        }
      }
    },
    [lang, voice, model, speed],
  )

  const stop = useCallback(() => {
    token.current++
    stopAudio()
    setState({ status: "idle", error: null, playing: null })
  }, [])

  return { ...state, say, stop }
}

// ---- recording ---------------------------------------------------------------------------------

export const MAX_RECORDING_MS = 15_000

export type Recording = { bytes: Uint8Array; ms: number }
export type RecorderState = "idle" | "recording"

const RECORDING_TYPES = ["audio/webm;codecs=opus", "audio/webm", "audio/ogg;codecs=opus"]

/** One-shot microphone recording, stopped by the caller or after 15 seconds. */
export function useRecorder() {
  const [state, setState] = useState<RecorderState>("idle")
  const [error, setError] = useState<string | null>(null)
  const session = useRef<{ recorder: MediaRecorder; stream: MediaStream; chunks: Blob[]; started: number; timer: number } | null>(null)
  const finish = useRef<((r: Recording | null) => void) | null>(null)

  const release = useCallback(() => {
    const s = session.current
    if (!s) return
    window.clearTimeout(s.timer)
    s.stream.getTracks().forEach((t) => t.stop())
    session.current = null
  }, [])

  useEffect(
    () => () => {
      finish.current = null
      const s = session.current
      if (s && s.recorder.state !== "inactive") s.recorder.stop()
      release()
    },
    [release],
  )

  const start = useCallback(async () => {
    setError(null)
    try {
      if (typeof MediaRecorder === "undefined" || !navigator.mediaDevices?.getUserMedia) {
        throw new Error("This window cannot record audio.")
      }
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const type = RECORDING_TYPES.find((t) => MediaRecorder.isTypeSupported(t))
      const recorder = new MediaRecorder(stream, type ? { mimeType: type } : undefined)
      const chunks: Blob[] = []
      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunks.push(e.data)
      }
      recorder.onstop = async () => {
        const started = session.current?.started ?? performance.now()
        release()
        setState("idle")
        const done = finish.current
        finish.current = null
        if (!done) return
        try {
          const blob = new Blob(chunks, { type: recorder.mimeType || "audio/webm" })
          done({ bytes: new Uint8Array(await blob.arrayBuffer()), ms: Math.round(performance.now() - started) })
        } catch {
          done(null)
        }
      }
      const timer = window.setTimeout(() => {
        if (recorder.state !== "inactive") recorder.stop()
      }, MAX_RECORDING_MS)
      session.current = { recorder, stream, chunks, started: performance.now(), timer }
      recorder.start()
      setState("recording")
    } catch (e) {
      release()
      setState("idle")
      setError(micError(e))
    }
  }, [release])

  /** Stops and resolves with the recording, or null when nothing usable was captured. */
  const stop = useCallback(
    () =>
      new Promise<Recording | null>((resolve) => {
        const s = session.current
        if (!s || s.recorder.state === "inactive") return resolve(null)
        finish.current = resolve
        s.recorder.stop()
      }),
    [],
  )

  return { state, error, start, stop }
}

function micError(e: unknown): string {
  if (e instanceof DOMException && (e.name === "NotAllowedError" || e.name === "SecurityError")) {
    return "The microphone is blocked. Allow microphone access for this app and try again."
  }
  if (e instanceof DOMException && e.name === "NotFoundError") return "No microphone was found."
  return e instanceof Error ? e.message : String(e)
}
