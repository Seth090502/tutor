// Session chrome maths: the 45-minute block from PEDAGOGY.md (Sessions) and the orb states the
// stage shows. Pure, so the timer and the state words are testable without a browser.

export const SESSION_MINUTES = 45

const pad = (n: number) => String(n).padStart(2, "0")

/** m:ss under an hour, h:mm:ss after; negative input counts as zero. */
export function formatClock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`
}

export type SessionClock = {
  label: string
  /** 0 to 1 across the 45 minutes, for the quiet progress hairline. */
  progress: number
  /** True from 45:00 on, when the gentle break suggestion appears. */
  breakDue: boolean
}

export function sessionClock(elapsedMs: number, minutes = SESSION_MINUTES): SessionClock {
  const limit = minutes * 60_000
  const elapsed = Math.max(0, elapsedMs)
  return { label: formatClock(elapsed), progress: Math.min(1, elapsed / limit), breakDue: elapsed >= limit }
}

export type StageState = "idle" | "listening" | "thinking" | "speaking"

/** The Orb's own state names map to the four words shown on the state chip. */
export function stageState(agent: null | "listening" | "thinking" | "talking"): StageState {
  return agent === "talking" ? "speaking" : (agent ?? "idle")
}

export const STAGE_WORD: Record<StageState, string> = {
  idle: "Idle",
  listening: "Listening",
  thinking: "Thinking",
  speaking: "Speaking",
}
