import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react"
import { RotateCcw } from "lucide-react"

import { cn } from "@/lib/utils"
import { clamp01, prefersReducedMotion } from "./_motion"
import { Eyebrow } from "./Eyebrow"
import { RESTING_POSE, isSettled, layerOffsets, layerOpacities, stepPose, tiltDegrees, type HoloPose } from "./holo"
import "./kit.css"
import "./question-card.css"

export type Mastery = "new" | "learning" | "mastered"

const MASTERY_WORD: Record<Mastery, string> = { new: "New", learning: "Learning", mastered: "Mastered" }

/** Shape plus colour: hollow, half and filled-with-check, so mastery is never tint alone. */
function MasteryGlyph({ mastery }: { mastery: Mastery }) {
  return (
    <svg aria-hidden viewBox="0 0 12 12" className="size-3 shrink-0" style={{ color: "var(--qc-glyph)" }}>
      <circle cx="6" cy="6" r="4.75" fill={mastery === "mastered" ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.5" />
      {mastery === "learning" ? <path d="M6 1.25a4.75 4.75 0 0 1 0 9.5Z" fill="currentColor" /> : null}
      {mastery === "mastered" ? (
        <path d="m3.9 6.2 1.4 1.4 2.8-3" fill="none" stroke="var(--card)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
      ) : null}
    </svg>
  )
}

/**
 * Drives the tilt and holo layers for one card. A requestAnimationFrame loop runs only while the
 * pointer is over the card or the pose is still settling, and writes transforms and opacity
 * straight to the DOM. Never attaches for touch pointers or under reduced motion.
 */
function useHoloTilt(rootRef: RefObject<HTMLDivElement | null>) {
  useEffect(() => {
    const root = rootRef.current
    const tilt = root?.querySelector<HTMLElement>(".qc-tilt")
    const holo = root?.querySelector<HTMLElement>(".qc-holo")
    if (!root || !tilt || !holo) return
    const layer = (name: string) => holo.querySelector<HTMLElement>(`[data-layer="${name}"]`)
    const layers = {
      rainbow: layer("rainbow"),
      diffraction: layer("diffraction"),
      sparkle: layer("sparkle"),
      sheen: layer("sheen"),
      glare: layer("glare"),
    }

    let pose: HoloPose = RESTING_POSE
    let target: HoloPose = RESTING_POSE
    let raf = 0
    let last = 0

    const rest = () => {
      tilt.style.transform = ""
      for (const el of Object.values(layers)) if (el) { el.style.transform = ""; el.style.opacity = "" }
      delete root.dataset.holo
      pose = RESTING_POSE
    }

    const paint = () => {
      const { rx, ry } = tiltDegrees(pose.nx, pose.ny)
      tilt.style.transform = `perspective(1200px) rotateX(${rx.toFixed(3)}deg) rotateY(${ry.toFixed(3)}deg)`
      const off = layerOffsets(pose.nx, pose.ny)
      const op = layerOpacities(pose.nx, pose.ny, pose.hm)
      for (const name of Object.keys(layers) as (keyof typeof layers)[]) {
        const el = layers[name]
        if (!el) continue
        el.style.transform = `translate3d(${off[name][0].toFixed(3)}%, ${off[name][1].toFixed(3)}%, 0)`
        el.style.opacity = op[name].toFixed(4)
      }
    }

    const frame = (ts: number) => {
      const dt = last ? Math.min(0.05, (ts - last) / 1000) : 1 / 60
      last = ts
      pose = stepPose(pose, target, dt)
      if (isSettled(pose, target)) {
        raf = 0
        if (target.hm === 0) rest()
        else { pose = target; paint() }
        return
      }
      paint()
      raf = requestAnimationFrame(frame)
    }
    const kick = () => {
      if (raf) return
      last = 0
      raf = requestAnimationFrame(frame)
    }

    // Backstop for a missed pointerleave (e.g. the card scrolled or re-rendered under the
    // pointer): any pointer movement outside the card ends the hover. Attached only while hovered.
    const onWindowMove = (e: PointerEvent) => {
      if (!root.contains(e.target as Node)) onLeave()
    }

    const onMove = (e: PointerEvent) => {
      if (e.pointerType === "touch" || prefersReducedMotion()) return
      if (!("holo" in root.dataset)) window.addEventListener("pointermove", onWindowMove, { passive: true })
      const r = root.getBoundingClientRect()
      if (!r.width || !r.height) return
      root.dataset.holo = ""
      target = { nx: clamp01((e.clientX - r.left) / r.width), ny: clamp01((e.clientY - r.top) / r.height), hm: 1 }
      kick()
    }
    const onLeave = () => {
      if (!("holo" in root.dataset)) return
      window.removeEventListener("pointermove", onWindowMove)
      target = RESTING_POSE
      kick()
    }

    root.addEventListener("pointerenter", onMove)
    root.addEventListener("pointermove", onMove)
    root.addEventListener("pointerleave", onLeave)
    return () => {
      root.removeEventListener("pointerenter", onMove)
      root.removeEventListener("pointermove", onMove)
      root.removeEventListener("pointerleave", onLeave)
      window.removeEventListener("pointermove", onWindowMove)
      cancelAnimationFrame(raf)
      rest()
    }
  }, [rootRef])
}

type QuestionCardProps = {
  /** Small caption above the question, e.g. "Question 3 of 10 · Limits". */
  eyebrow?: ReactNode
  /** Tints the frame ring and prints the word. "new" by default; null when mastery is not known, which
   *  prints no word and leaves the ring neutral. */
  mastery?: Mastery | null
  /** The front face: the question and its answer keys. */
  children: ReactNode
  /** Back face content. With none of the three, the card does not flip. */
  explanation?: ReactNode
  misconception?: ReactNode
  /** The result of the attempt, shown first on the back face, e.g. a Correct or Not quite pill. */
  verdict?: ReactNode
  flipped?: boolean
  defaultFlipped?: boolean
  onFlippedChange?: (flipped: boolean) => void
  className?: string
}

const FLIP_BTN =
  "text-ink-2 hover:bg-world-soft hover:text-foreground border-hairline-strong inline-flex items-center gap-2 rounded-full border px-3 py-1.5 font-mono text-[11px] tracking-[0.14em] uppercase transition-colors duration-(--dur-quick) outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

/**
 * Study card. Hover (mouse or pen, not touch) tilts it 7 degrees and plays a holo sheen and glare
 * behind the text; there is no idle animation. Flips to explanation and misconception over about
 * 0.9 s with a slight overshoot. Static under reduced motion.
 */
export function QuestionCard({
  eyebrow,
  mastery = "new",
  children,
  explanation,
  misconception,
  verdict,
  flipped: flippedProp,
  defaultFlipped = false,
  onFlippedChange,
  className,
}: QuestionCardProps) {
  const rootRef = useRef<HTMLDivElement>(null)
  const frontBtn = useRef<HTMLButtonElement>(null)
  const backBtn = useRef<HTMLButtonElement>(null)
  const [inner, setInner] = useState(defaultFlipped)
  const flipped = flippedProp ?? inner
  const canFlip = Boolean(explanation || misconception || verdict)
  const moveFocus = useRef(false)

  useHoloTilt(rootRef)

  const setFlipped = (next: boolean) => {
    moveFocus.current = Boolean(rootRef.current?.contains(document.activeElement))
    if (flippedProp === undefined) setInner(next)
    onFlippedChange?.(next)
  }

  // Keep keyboard focus on the visible face's button after a flip.
  useLayoutEffect(() => {
    if (!moveFocus.current) return
    moveFocus.current = false
    ;(flipped ? backBtn : frontBtn).current?.focus()
  }, [flipped])

  return (
    <div
      ref={rootRef}
      data-slot="kit-question-card"
      data-mastery={mastery ?? undefined}
      className={cn("qc", className)}
    >
      <div className="qc-tilt">
        <div className="qc-flip" data-flipped={flipped && canFlip ? "" : undefined}>
          <article className="qc-face qc-front" inert={flipped && canFlip} aria-label="Question">
            <div aria-hidden className="qc-holo">
              <i data-layer="rainbow" className="qc-layer qc-rainbow" />
              <i data-layer="diffraction" className="qc-layer qc-diffraction" />
              <i data-layer="sparkle" className="qc-layer qc-sparkle" />
              <i data-layer="sheen" className="qc-layer qc-sheen" />
              <i data-layer="glare" className="qc-layer qc-glare" />
            </div>
            <div className="qc-body">
              <div className="flex items-center justify-between gap-4">
                {eyebrow ? <Eyebrow rule>{eyebrow}</Eyebrow> : <span />}
                {mastery ? (
                  <span className="text-ink-2 inline-flex items-center gap-2 font-mono text-[11px] tracking-[0.14em] uppercase">
                    <MasteryGlyph mastery={mastery} />
                    {MASTERY_WORD[mastery]}
                  </span>
                ) : null}
              </div>
              <div className="flex-1">{children}</div>
              {canFlip ? (
                <div className="flex justify-end">
                  <button ref={frontBtn} type="button" className={FLIP_BTN} onClick={() => setFlipped(true)}>
                    <RotateCcw aria-hidden className="size-3" />
                    Explanation
                  </button>
                </div>
              ) : null}
            </div>
          </article>

          {canFlip ? (
            <article className="qc-face qc-back" inert={!flipped} aria-label="Explanation">
              <div className="qc-body">
                <div className="flex items-center justify-between gap-4">
                  <Eyebrow rule>Explanation</Eyebrow>
                  {mastery ? (
                    <span className="text-ink-2 inline-flex items-center gap-2 font-mono text-[11px] tracking-[0.14em] uppercase">
                      <MasteryGlyph mastery={mastery} />
                      {MASTERY_WORD[mastery]}
                    </span>
                  ) : null}
                </div>
                <div className="flex flex-1 flex-col gap-5 font-serif text-[17px] leading-[1.6] text-pretty">
                  {verdict || explanation ? (
                    <div className="flex flex-col gap-4">
                      {verdict ? <div>{verdict}</div> : null}
                      {explanation}
                    </div>
                  ) : null}
                  {misconception ? (
                    <div className="border-warn/50 border-l-2 pl-4">
                      <Eyebrow as="div" className="mb-2">
                        Common misconception
                      </Eyebrow>
                      {misconception}
                    </div>
                  ) : null}
                </div>
                <div className="flex justify-end">
                  <button ref={backBtn} type="button" className={FLIP_BTN} onClick={() => setFlipped(false)}>
                    <RotateCcw aria-hidden className="size-3 -scale-x-100" />
                    Back to question
                  </button>
                </div>
              </div>
            </article>
          ) : null}
        </div>
      </div>
    </div>
  )
}
