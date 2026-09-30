// The run view's presentation: a neutral stage with the 3x3 grid (or shapes, or an audio-only stage),
// large response keycaps, the level and rule shown quietly, and a thin block progress rule. Nothing here
// animates except the stimulus onset (train.css) and the key press (kit Keycap).
import type { CSSProperties, ReactNode } from "react"
import { Volume2 } from "lucide-react"

import { Eyebrow, Keycap } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { streamsOf } from "./engine/session"
import { cellOf, keyLabel, streamLabels } from "./parts"
import type { Stream } from "./runner"
import type { Mode } from "./types"
import "./train.css"

const SOFT_CELL = "bg-[color-mix(in_oklab,var(--ink)_4%,var(--card))]"
const STAGE_SIZE = "w-[min(22rem,calc(100vh-25rem))] min-w-48 max-w-full"

/** The lit cell's shape: a diamond, so a lit cell is never told apart by colour alone. */
function CellMark() {
  return (
    <svg viewBox="0 0 100 100" className="size-[46%]" aria-hidden>
      <path d="M50 12 88 50 50 88 12 50Z" fill="currentColor" />
    </svg>
  )
}

export function Grid({ active }: { active: number | null }) {
  const lit = active === null ? -1 : cellOf(active)
  return (
    <div
      className={cn("grid aspect-square grid-cols-3 grid-rows-3 gap-2.5", STAGE_SIZE)}
      role="img"
      aria-label={active === null ? "Grid, no square lit. The centre square is never used." : `Grid, square ${active + 1} of 8 lit`}
    >
      {Array.from({ length: 9 }, (_, g) => (
        <div
          key={g}
          data-lit={g === lit ? "" : undefined}
          data-unused={g === 4 ? "" : undefined}
          className={cn(
            "relative grid place-items-center rounded-2xl border",
            g === 4 ? "border-hairline-strong border-dashed opacity-60" : cn("border-hairline", SOFT_CELL),
          )}
        >
          {g === 4 && (
            <svg viewBox="0 0 16 16" className="text-ink-3 size-4" aria-hidden>
              <path d="M8 2v12M2 8h12" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" />
            </svg>
          )}
          {g === lit && (
            <span className="train-onset bg-world text-world-ink absolute -inset-px grid place-items-center rounded-2xl shadow-[inset_0_1px_0_rgb(255_255_255/0.28),inset_0_0_0_3px_color-mix(in_oklab,var(--world-ink)_22%,transparent),0_10px_24px_-12px_color-mix(in_oklab,var(--world)_80%,transparent)]">
              <CellMark />
            </span>
          )}
        </div>
      ))}
    </div>
  )
}

const SHAPES: { name: string; el: ReactNode }[] = [
  { name: "circle", el: <circle cx="50" cy="50" r="38" /> },
  { name: "square", el: <rect x="14" y="14" width="72" height="72" /> },
  { name: "triangle", el: <polygon points="50,10 90,86 10,86" /> },
  { name: "diamond", el: <polygon points="50,6 94,50 50,94 6,50" /> },
  { name: "star", el: <polygon points="50,6 61,38 95,38 67,58 78,92 50,71 22,92 33,58 5,38 39,38" /> },
  { name: "hexagon", el: <polygon points="28,10 72,10 94,50 72,90 28,90 6,50" /> },
  { name: "plus", el: <polygon points="36,8 64,8 64,36 92,36 92,64 64,64 64,92 36,92 36,64 8,64 8,36 36,36" /> },
  { name: "chevron", el: <polygon points="10,20 50,50 90,20 90,52 50,82 10,52" /> },
]

/** Transfer-mode visual stimulus: one of 8 shapes in a single frame. The shape is the signal. */
export function ShapeStage({ shape }: { shape: number | null }) {
  return (
    <div
      data-lit={shape === null ? undefined : ""}
      className={cn("border-hairline grid aspect-square place-items-center rounded-2xl border", SOFT_CELL, STAGE_SIZE)}
      role="img"
      aria-label={shape === null ? "No shape" : SHAPES[shape].name}
    >
      {shape === null ? (
        <span className="bg-hairline-strong size-1.5 rounded-full" aria-hidden />
      ) : (
        <svg viewBox="0 0 100 100" className="train-onset text-world size-1/2" aria-hidden>
          <g fill="currentColor">{SHAPES[shape].el}</g>
        </svg>
      )}
    </div>
  )
}

/** Letter-only mode has nothing to see: the stage says so and stays still. */
export function ListenStage({ transfer }: { transfer?: boolean }) {
  return (
    <div
      className={cn("border-hairline grid aspect-square place-items-center rounded-2xl border border-dashed", STAGE_SIZE)}
      role="img"
      aria-label="Audio only: listen for the letters"
    >
      <div className="text-ink-2 flex flex-col items-center gap-3 text-center text-sm">
        <Volume2 className="text-ink-3 size-7" strokeWidth={1.5} aria-hidden />
        {transfer ? "Listen for the tones" : "Listen for the letters"}
      </div>
    </div>
  )
}

export type Marks = Record<Stream, boolean | null>

/** A large response keycap: presses on the keyboard key, on click and on touch. */
function ResponseKey({
  code,
  label,
  pressed,
  mark,
  onPress,
}: {
  code: string
  label: string
  pressed: boolean
  mark: boolean | null
  onPress: (timeStamp: number) => void
}) {
  return (
    <Keycap
      legend={keyLabel(code)}
      latched={pressed}
      aria-pressed={undefined}
      status={mark === null ? undefined : mark ? "correct" : "wrong"}
      aria-label={`${label}, key ${keyLabel(code)}`}
      onPointerDown={(e) => {
        e.preventDefault()
        onPress(e.timeStamp)
      }}
      // Keyboard activation of a focused key (Enter or Space) arrives as a click with no pointer.
      onClick={(e) => e.detail === 0 && onPress(e.timeStamp)}
      className="justify-center text-center"
      style={{ "--u": "4.5rem", flexDirection: "column", gap: "0.7rem", padding: "6px" } as CSSProperties}
    >
      <span className="text-[15px]">{label}</span>
    </Keycap>
  )
}

export type RunStageProps = {
  mode: Mode
  n: number
  blockNo: number
  blocks: number
  phase: "block" | "rest"
  /** Trials started in this block, and the block's length. */
  trial: number
  trials: number
  /** Position or shape index while a stimulus is showing. */
  lit: number | null
  keys: { position: string; letter: string }
  pressed: Record<Stream, boolean>
  marks: Marks
  onPress: (stream: Stream, timeStamp: number) => void
  onStop: () => void
  /** Seconds of rest left, and the rest's length. */
  restLeft: number
  restTotal: number
  /** The adaptation rule, as logged. */
  rule: string
  /** Shown in place of the stimulus during a rest (the block summary). */
  summary?: ReactNode
}

export function RunStage(p: RunStageProps) {
  const transfer = p.mode === "transfer"
  const streams = streamsOf(p.mode)
  const labels = streamLabels(transfer)
  const resting = p.phase === "rest"
  const now = resting ? p.restTotal - p.restLeft : p.trial
  const max = resting ? p.restTotal : p.trials
  const done = max > 0 ? Math.min(1, now / max) : 0
  const mode = transfer ? "Transfer check" : p.mode === "dual" ? "Dual n-back" : p.mode === "position" ? "Position only" : "Letter only"
  const cue = [streams.position && labels.position.toLowerCase(), streams.letter && labels.letter.toLowerCase()].filter(Boolean).join(" or ")

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 px-6 py-5">
      <header className="flex items-end justify-between gap-4">
        <div className="flex items-end gap-8">
          <div className="flex flex-col gap-2">
            <Eyebrow>{transfer ? "Fixed level" : "Level"}</Eyebrow>
            <div className="flex items-baseline gap-2.5 leading-none">
              <span className="text-ink-3 font-mono text-sm">n =</span>
              <span className="text-[52px] leading-none font-extralight tracking-[-0.04em] tabular-nums">{p.n}</span>
            </div>
          </div>
          <div className="flex flex-col gap-2 pb-0.5">
            <Eyebrow>{resting ? "Rest before block" : "Block"}</Eyebrow>
            <span className="font-mono text-[15px] tabular-nums">
              {Math.min(p.blockNo, p.blocks)} <span className="text-ink-3">of {p.blocks}</span>
            </span>
          </div>
          <div className="flex flex-col gap-2 pb-0.5 max-[700px]:hidden">
            <Eyebrow>{mode}</Eyebrow>
            <span className="text-ink-2 font-mono text-[15px]">{transfer ? "Measurement" : "Training"}</span>
          </div>
        </div>
        <Button variant="outline" size="sm" onClick={p.onStop}>
          Stop session
        </Button>
      </header>

      {/* Thin progress rule: steps once per trial (or per rest second), never eases. */}
      <div
        role="progressbar"
        aria-label={resting ? "Rest" : "Block progress"}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuenow={now}
        className="bg-hairline h-[2px] w-full overflow-hidden rounded-full"
      >
        <div className="bg-world h-full origin-left" style={{ transform: `scaleX(${done})` }} />
      </div>

      {/* Keys flank the stage, left hand A and right hand L, so the whole task fits one screen. */}
      <div className="flex min-h-56 items-center justify-center gap-6 py-2">
        {resting && p.summary ? (
          p.summary
        ) : (
          <>
            <div className="flex w-36 shrink-0 justify-end max-[760px]:w-28">
              {streams.position && (
                <ResponseKey
                  code={p.keys.position}
                  label={`${labels.position} match`}
                  pressed={p.pressed.position}
                  mark={p.marks.position}
                  onPress={(t) => p.onPress("position", t)}
                />
              )}
            </div>
            {transfer ? <ShapeStage shape={p.lit} /> : streams.position ? <Grid active={p.lit} /> : <ListenStage />}
            <div className="flex w-36 shrink-0 justify-start max-[760px]:w-28">
              {streams.letter && (
                <ResponseKey
                  code={p.keys.letter}
                  label={`${labels.letter} match`}
                  pressed={p.pressed.letter}
                  mark={p.marks.letter}
                  onPress={(t) => p.onPress("letter", t)}
                />
              )}
            </div>
          </>
        )}
      </div>

      <p className="text-ink-3 mx-auto max-w-[62ch] text-center text-xs leading-relaxed text-pretty">
        {transfer ? `Fixed level: n stays at ${p.n} for this check. No adaptation.` : `Rule: ${p.rule}`} Press when the {cue} matches
        the one {p.n} back.
      </p>
    </div>
  )
}
