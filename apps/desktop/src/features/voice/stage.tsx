import { useRef, type CSSProperties, type KeyboardEvent, type ReactNode } from "react"

import { Scrim, StateChip } from "@/components/kit"
import { cn } from "@/lib/utils"
import { STAGE_WORD, type StageState } from "./session"
import "./tutor.css"

// Color presets from ElevenLabs' own Orb demo (github.com/elevenlabs/ui, MIT).
export const VOICES: { name: string; colors: [string, string] }[] = [
  { name: "Sky", colors: ["#CADCFC", "#A0B9D1"] },
  { name: "Sand", colors: ["#F6E7D8", "#E0CFC2"] },
  { name: "Slate", colors: ["#E5E7EB", "#9CA3AF"] },
]

const STATES: StageState[] = ["idle", "listening", "thinking", "speaking"]

/** Four static washes behind the screen; the one for the current orb state fades in. */
export function StateTint({ state }: { state: StageState }) {
  return (
    <div aria-hidden className="tv-tint">
      {STATES.map((s) => (
        <i key={s} data-state={s} data-on={s === state ? "" : undefined} />
      ))}
    </div>
  )
}

export function OrbFrame({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <div className={cn("tv-orb-frame", className)}>
      <div>{children}</div>
    </div>
  )
}

const CHIP_TONE: Record<StageState, "world" | "ok" | "warn"> = {
  idle: "world",
  listening: "ok",
  thinking: "warn",
  speaking: "world",
}

/** Two hairline rings around the orb, drawn once. Decoration at the edge, never behind text. */
function Rings() {
  return (
    <svg
      aria-hidden
      viewBox="-200 -200 400 400"
      className="text-hairline-strong tv-fade pointer-events-none absolute top-1/2 left-1/2 size-[26rem] -translate-x-1/2 -translate-y-[46%] fill-none opacity-70"
      stroke="currentColor"
      strokeWidth="0.6"
    >
      <circle r="112" />
      <circle r="158" strokeDasharray="1 5" strokeLinecap="round" strokeWidth="1.2" />
      <circle r="196" opacity="0.5" />
    </svg>
  )
}

type OrbStageProps = {
  state: StageState
  /** The Orb itself, sized by the frame. */
  orb: ReactNode
  status: ReactNode
  /** An error is shown in the destructive colour, with no shimmer. */
  error?: boolean
  /** Orb diameter classes. The default is the compact live size. */
  orbClass?: string
  className?: string
}

/**
 * The quiet stage: the orb centred on two hairline rings, a dark-glass chip with the state word
 * under it, and the status line on a local scrim. The chip and scrim carry the text, so nothing
 * readable sits on the orb or the rings.
 */
export function OrbStage({ state, orb, status, error = false, orbClass = "size-32 @4xl:size-36", className }: OrbStageProps) {
  return (
    <div className={cn("relative isolate flex flex-col items-center overflow-hidden py-4", className)}>
      <Rings />
      <div className="tv-fade relative">
        <OrbFrame className={orbClass}>{orb}</OrbFrame>
        <StateChip
          state={STAGE_WORD[state]}
          tone={CHIP_TONE[state]}
          dot={state !== "idle"}
          className="absolute bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2"
        />
      </div>
      {status ? (
        <Scrim strength={78} className="tv-enter mt-7 max-w-md text-center" style={{ "--i": 2 } as CSSProperties}>
          <p
            aria-live="polite"
            className={cn("min-h-6 px-2 text-[15px] leading-6 text-balance", error ? "text-destructive" : "text-ink-2")}
          >
            {status}
          </p>
        </Scrim>
      ) : (
        <p aria-live="polite" className="mt-7 min-h-6" />
      )}
    </div>
  )
}

const sphere = ([a, b]: [string, string]) =>
  `radial-gradient(circle at 34% 30%, #fff 0, ${a} 34%, ${b} 100%)`

/** Voice style picker. The swatches are CSS spheres, so only the stage orb uses WebGL. */
export function VoiceSwatches({ value, onChange }: { value: number; onChange: (index: number) => void }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([])

  const onKeyDown = (e: KeyboardEvent) => {
    const delta = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0
    if (!delta) return
    e.preventDefault()
    const next = (value + delta + VOICES.length) % VOICES.length
    onChange(next)
    refs.current[next]?.focus()
  }

  return (
    <div role="radiogroup" aria-label="Voice style" onKeyDown={onKeyDown} className="flex items-center gap-1">
      {VOICES.map((v, i) => (
        <button
          key={v.name}
          ref={(el) => {
            refs.current[i] = el
          }}
          type="button"
          role="radio"
          aria-checked={value === i}
          tabIndex={value === i ? 0 : -1}
          onClick={() => onChange(i)}
          className={cn(
            "focus-visible:outline-world flex items-center gap-2 rounded-full p-1 text-xs outline-offset-2 transition-colors duration-(--dur-quick) focus-visible:outline-2",
            value === i ? "bg-accent text-foreground pr-3" : "text-ink-2 hover:text-foreground",
          )}
        >
          <span
            aria-hidden
            className="border-hairline-strong size-5 rounded-full border shadow-[inset_0_-2px_4px_rgb(0_0_0/0.18)]"
            style={{ background: sphere(v.colors) }}
          />
          {value === i ? v.name : <span className="sr-only">{v.name}</span>}
        </button>
      ))}
    </div>
  )
}
