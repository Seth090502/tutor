// Lookahead scheduling on the audio clock: a coarse timer calls `tick()`, and every onset that
// falls inside the lookahead window is handed to the audio graph with its exact start time.
// Visuals are drawn on the animation frame nearest the scheduled time.

export type AudioClock = { readonly currentTime: number } // seconds

export class LookaheadScheduler {
  private onsets: number[] = []
  private next = 0
  constructor(
    private clock: AudioClock,
    private lookahead: number, // seconds
    private schedule: (index: number, when: number, late: boolean) => void,
  ) {}

  setOnsets(onsets: number[]) {
    this.onsets = onsets
    this.next = 0
  }

  /** Schedule every pending onset before `now + lookahead`. An onset already in the past (the
   * timer was throttled) is still scheduled once, flagged late, so the log shows the slip. */
  tick() {
    const now = this.clock.currentTime
    while (this.next < this.onsets.length && this.onsets[this.next] < now + this.lookahead) {
      const when = this.onsets[this.next]
      this.schedule(this.next, when, when < now)
      this.next++
    }
  }

  cancel() {
    this.next = this.onsets.length
  }

  get done() {
    return this.next >= this.onsets.length
  }
}

/** Onsets (audio-clock seconds) of `count` trials starting at `start`. */
export const blockOnsets = (start: number, count: number, trialMs: number) =>
  Array.from({ length: count }, (_, i) => start + (i * trialMs) / 1000)

/** Whether a frame at `frameTime` is the one nearest `onset` (same units), given the frame
 * interval. Called on successive frames, the first true is the nearest frame. */
export const isNearestFrame = (frameTime: number, onset: number, frameInterval: number) =>
  frameTime >= onset - frameInterval / 2

/** Map a performance.now() time (ms) to audio-clock seconds using an output timestamp. */
export const contextTimeAt = (perfMs: number, stamp: { contextTime: number; performanceTime: number }) =>
  stamp.contextTime + (perfMs - stamp.performanceTime) / 1000

/** Index of the trial whose response window [onset, onset + trialMs) contains `t` (seconds), or -1. */
export function trialAt(onsets: number[], t: number, trialMs: number): number {
  for (let i = onsets.length - 1; i >= 0; i--) {
    if (t >= onsets[i]) return t < onsets[i] + trialMs / 1000 ? i : -1
  }
  return -1
}
