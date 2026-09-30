// Stimulus audio. Letter clips are pre-rendered WAVs (scripts/render-letters.ps1), decoded at
// startup with an OfflineAudioContext, trimmed under 450 ms and RMS-normalised. Transfer tones and
// the calibration beep are synthesised. The one live AudioContext is created on the Start click.
import { normalizeRms, synthTone, trimSilence } from "./engine/dsp"
import { LETTERS, TONES } from "./engine/session"

const WAVS = import.meta.glob("./audio/*.wav", { eager: true, query: "?url", import: "default" }) as Record<
  string,
  string
>
const RATE = 48_000
const TARGET_RMS = 0.1

export type Clip = { samples: Float32Array; sampleRate: number }
export type Clips = { letters: Clip[]; tones: Clip[]; beep: Clip }

let clips: Promise<Clips> | null = null

/** Decode and prepare every clip once. No AudioContext is started. */
export function loadClips(): Promise<Clips> {
  clips ??= (async () => {
    const decoder = new OfflineAudioContext(1, 1, RATE)
    const letters = await Promise.all(
      LETTERS.map(async (letter) => {
        const url = WAVS[`./audio/${letter.toLowerCase()}.wav`]
        if (!url) throw new Error(`missing letter clip ${letter}`)
        const res = await fetch(url)
        if (!res.ok) throw new Error(`letter clip ${letter}: HTTP ${res.status}`)
        const buf = await decoder.decodeAudioData(await res.arrayBuffer())
        const trimmed = trimSilence(buf.getChannelData(0), buf.sampleRate)
        return { samples: normalizeRms(trimmed, TARGET_RMS), sampleRate: buf.sampleRate }
      }),
    )
    const tones = TONES.map((hz) => ({ samples: normalizeRms(synthTone(hz, RATE, 400), TARGET_RMS), sampleRate: RATE }))
    const beep = { samples: synthTone(1000, RATE, 50), sampleRate: RATE }
    return { letters, tones, beep }
  })()
  clips.catch(() => (clips = null)) // allow a retry after a failed load
  return clips
}

let context: AudioContext | null = null

/** The app's single AudioContext. Call only from a user gesture (the Start or Check click). */
export function audioContext(): AudioContext {
  if (!context || context.state === "closed") context = new AudioContext({ latencyHint: "interactive" })
  if (context.state === "suspended") void context.resume()
  return context
}

export function toBuffer(ctx: BaseAudioContext, clip: Clip): AudioBuffer {
  const buf = ctx.createBuffer(1, Math.max(1, clip.samples.length), clip.sampleRate)
  buf.copyToChannel(clip.samples as Float32Array<ArrayBuffer>, 0)
  return buf
}

/** Start `buffer` at `when` on the audio clock. */
export function playAt(ctx: AudioContext, buffer: AudioBuffer, when: number): AudioBufferSourceNode {
  const src = ctx.createBufferSource()
  src.buffer = buffer
  src.connect(ctx.destination)
  src.start(when)
  return src
}

/** Map performance.now() milliseconds to the audio clock as heard (output timestamp). Before the
 * output clock is running, fall back to currentTime. */
export function outputStamp(ctx: AudioContext): { contextTime: number; performanceTime: number } {
  const s = ctx.getOutputTimestamp?.()
  if (s && s.performanceTime && s.contextTime !== undefined) {
    return { contextTime: s.contextTime, performanceTime: s.performanceTime }
  }
  return { contextTime: ctx.currentTime, performanceTime: performance.now() }
}

export const latencies = (ctx: AudioContext) => ({
  output_latency: Number.isFinite(ctx.outputLatency) ? ctx.outputLatency : null,
  base_latency: Number.isFinite(ctx.baseLatency) ? ctx.baseLatency : null,
})
