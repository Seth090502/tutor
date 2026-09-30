import { useEffect, useState, type ReactNode } from "react"
import { Coffee } from "lucide-react"

import { Eyebrow, idleDimClass } from "@/components/kit"
import { usePageVisible } from "@/lib/motion"
import { cn } from "@/lib/utils"
import { SESSION_MINUTES, sessionClock, type SessionClock } from "./session"
import "./tutor.css"

/**
 * Elapsed time of the current call. It counts from when the call connects, ticks once a second
 * only while the page is visible, and stays on the last value after the call ends. The time is
 * derived from timestamps, so a hidden page never drifts.
 */
export function useSessionClock(active: boolean): SessionClock & { started: boolean } {
  const [span, setSpan] = useState<{ start: number; now: number } | null>(null)
  const visible = usePageVisible()

  useEffect(() => {
    if (!active) return
    const t = Date.now()
    setSpan({ start: t, now: t })
  }, [active])

  useEffect(() => {
    if (!active || !visible) return
    const tick = () => setSpan((s) => (s ? { start: s.start, now: Date.now() } : s))
    tick()
    const id = window.setInterval(tick, 1000)
    return () => window.clearInterval(id)
  }, [active, visible])

  return { ...sessionClock(span ? span.now - span.start : 0), started: span !== null }
}

/** The timer and its 45-minute hairline. It dims with the rest of the chrome when idle. */
export function SessionTimer({ clock, live }: { clock: SessionClock & { started: boolean }; live: boolean }) {
  return (
    <div className={cn("flex min-w-0 flex-wrap items-baseline gap-x-4 gap-y-1", idleDimClass)}>
      <Eyebrow rule>{live ? "Session" : clock.started ? "Session ended" : "Session"}</Eyebrow>
      <span className="flex items-baseline gap-2">
        <span
          role="timer"
          aria-label={`Session time ${clock.label}`}
          className={cn(
            "text-[28px] leading-none font-extralight tracking-[-0.03em] tabular-nums",
            !live && "text-ink-3",
          )}
        >
          {clock.label}
        </span>
        <span className="text-ink-3 text-xs">of {SESSION_MINUTES} min</span>
      </span>
      <span aria-hidden className="tv-progress self-center">
        <i style={{ transform: `scaleX(${clock.progress})` }} />
      </span>
    </div>
  )
}

/**
 * The session row: timer on the left, `children` (the voice picker) on the right, and the break
 * note under it once 45 minutes are up. The clock state lives here, so its once-a-second tick
 * re-renders only this row and never the orb or the conversation.
 */
export function SessionChrome({ live, children }: { live: boolean; children?: ReactNode }) {
  const clock = useSessionClock(live)
  const [dismissed, setDismissed] = useState(false)

  // A new call is a new session, so its break suggestion can appear again.
  useEffect(() => {
    if (live) setDismissed(false)
  }, [live])

  return (
    <>
      <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-3">
        <SessionTimer clock={clock} live={live} />
        {children}
      </div>
      {live && clock.breakDue && !dismissed ? <BreakNote onDismiss={() => setDismissed(true)} /> : null}
    </>
  )
}

/** A gentle suggestion at 45 minutes. It is a status note, not a blocker, and can be dismissed. */
export function BreakNote({ onDismiss }: { onDismiss: () => void }) {
  return (
    <div
      role="status"
      className="tv-enter border-hairline-strong bg-card text-ink-2 flex items-start gap-3 rounded-xl border px-4 py-3 text-sm"
    >
      <Coffee aria-hidden className="text-ink-3 mt-0.5 size-4 shrink-0" />
      <p className="flex-1 leading-relaxed">
        That is {SESSION_MINUTES} minutes. A short break lets what you practised settle. Stretch, drink some water, and come back when
        you are ready.
      </p>
      <button
        type="button"
        onClick={onDismiss}
        className="text-foreground border-hairline-strong hover:bg-world-soft focus-visible:outline-world shrink-0 rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.14em] uppercase outline-offset-2 transition-colors duration-(--dur-quick) focus-visible:outline-2"
      >
        Keep going
      </button>
    </div>
  )
}
