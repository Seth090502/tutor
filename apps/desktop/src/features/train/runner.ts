// Runs one block in real time. Audio is scheduled ahead on the audio clock; the visual stimulus is
// drawn on the animation frame nearest its scheduled time (mapped through the output timestamp, so
// "scheduled" means "heard"). Each trial logs intended and actual (visual) onsets on the session clock.
import type { Mode, Settings, Trial } from "./types"
import { contextTimeAt, isNearestFrame, LookaheadScheduler, blockOnsets, trialAt } from "./engine/scheduler"
import { streamsOf, toTrial, type Responses } from "./engine/session"
import type { Stim } from "./engine/generate"
import { outputStamp, playAt } from "./audio"

const LOOKAHEAD_S = 0.12
const TIMER_MS = 25
const LEAD_IN_S = 0.6

export type Stream = "position" | "letter"
export type Feedback = { stream: Stream; correct: boolean; trial: number }

export type RunnerOptions = {
  ctx: AudioContext
  mode: Mode
  settings: Settings
  stimuli: Stim[]
  /** Letter or tone buffers, indexed like the stimulus values. */
  sounds: AudioBuffer[]
  /** Audio-clock seconds at session start: the zero of the session clock. */
  sessionZero: number
  /** The visual stimulus to show now (trial index), or null to blank. */
  onShow: (trial: number | null) => void
  onFeedback?: (f: Feedback) => void
  onDone: (trials: Trial[]) => void
}

export class BlockRunner {
  private onsets: number[]
  private actual: (number | null)[]
  private responses: Responses[]
  private scheduler: LookaheadScheduler
  private timer = 0
  private frame = 0
  private shown = -1 // last trial made visible
  private visible = false
  private lastFrame = 0
  private frameInterval = 1000 / 60
  private sources: AudioBufferSourceNode[] = []
  private finished = false

  constructor(private o: RunnerOptions) {
    const start = o.ctx.currentTime + LEAD_IN_S
    this.onsets = blockOnsets(start, o.stimuli.length, o.settings.trial_ms)
    this.actual = o.stimuli.map(() => null)
    this.responses = o.stimuli.map(() => ({ position: null, letter: null }))
    const hasLetters = streamsOf(o.mode).letter
    this.scheduler = new LookaheadScheduler(o.ctx, LOOKAHEAD_S, (i, when) => {
      const v = o.stimuli[i].letter
      if (hasLetters && v !== null) this.sources.push(playAt(o.ctx, o.sounds[v], when))
    })
  }

  start() {
    this.scheduler.setOnsets(this.onsets)
    this.scheduler.tick()
    this.timer = window.setInterval(() => this.scheduler.tick(), TIMER_MS)
    this.frame = requestAnimationFrame(this.onFrame)
  }

  private now(perfMs: number) {
    return contextTimeAt(perfMs, outputStamp(this.o.ctx))
  }

  private onFrame = (t: number) => {
    if (this.finished) return
    if (this.lastFrame) {
      // Track the display's frame interval with a slow average; ignore stalls.
      const d = t - this.lastFrame
      if (d > 4 && d < 50) this.frameInterval += (d - this.frameInterval) * 0.1
    }
    this.lastFrame = t
    const nowMs = this.now(t) * 1000
    const next = this.shown + 1
    if (next < this.onsets.length && isNearestFrame(nowMs, this.onsets[next] * 1000, this.frameInterval)) {
      // Close out misses of the trial that just ended, for in-trial feedback.
      if (this.shown >= 0) this.closeTrial(this.shown)
      this.shown = next
      this.actual[next] = nowMs / 1000
      this.visible = true
      this.o.onShow(next)
    } else if (
      this.visible &&
      isNearestFrame(nowMs, this.onsets[this.shown] * 1000 + this.o.settings.stimulus_ms, this.frameInterval)
    ) {
      this.visible = false
      this.o.onShow(null)
    }
    const last = this.onsets.length - 1
    if (this.shown === last && nowMs >= (this.onsets[last] * 1000 + this.o.settings.trial_ms)) {
      this.closeTrial(last)
      this.finish()
      this.o.onDone(this.trials(this.onsets.length))
      return
    }
    this.frame = requestAnimationFrame(this.onFrame)
  }

  private closeTrial(i: number) {
    const s = this.o.stimuli[i]
    const streams = streamsOf(this.o.mode)
    if (streams.position && s.pos_match && this.responses[i].position === null)
      this.o.onFeedback?.({ stream: "position", correct: false, trial: i })
    if (streams.letter && s.let_match && this.responses[i].letter === null)
      this.o.onFeedback?.({ stream: "letter", correct: false, trial: i })
  }

  /** A response at `eventTimeMs` (event.timeStamp, performance clock). The first press per stream
   * per trial counts; RT is from the visual onset (position) or the scheduled audio onset (letter). */
  respond(stream: Stream, eventTimeMs: number) {
    if (this.finished) return
    const t = this.now(eventTimeMs)
    const i = trialAt(this.onsets, t, this.o.settings.trial_ms)
    if (i < 0 || this.responses[i][stream] !== null) return
    const onset = stream === "position" ? (this.actual[i] ?? this.onsets[i]) : this.onsets[i]
    this.responses[i][stream] = Math.max(0, Math.round((t - onset) * 10000) / 10)
    const s = this.o.stimuli[i]
    this.o.onFeedback?.({ stream, correct: stream === "position" ? s.pos_match : s.let_match, trial: i })
  }

  /** Stop now (void or quit) and return the trials that had started. */
  abort(): Trial[] {
    this.finish()
    for (const src of this.sources) {
      try {
        src.stop()
      } catch {
        // already stopped
      }
    }
    return this.trials(this.shown + 1)
  }

  private finish() {
    this.finished = true
    window.clearInterval(this.timer)
    cancelAnimationFrame(this.frame)
    this.scheduler.cancel()
    this.o.onShow(null)
  }

  private trials(count: number): Trial[] {
    const ms = (s: number) => Math.round((s - this.o.sessionZero) * 10000) / 10
    return this.o.stimuli.slice(0, count).map((s, i) =>
      toTrial(this.o.mode, s, i, this.responses[i], ms(this.onsets[i]), ms(this.actual[i] ?? this.onsets[i])),
    )
  }
}
