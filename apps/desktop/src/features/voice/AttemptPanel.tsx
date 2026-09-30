import { useEffect, useRef, useState, type CSSProperties } from "react"
import { Check, Minus, X } from "lucide-react"

import { Eyebrow, Kbd, KbdHint, KeycapGroup, Panel, QuestionCard, type KeycapStatus } from "@/components/kit"
import { isEditableTarget } from "@/components/kit/keycap-logic"
import { useReducedMotion } from "@/lib/motion"
import { cn } from "@/lib/utils"
import {
  CONFIDENCE_WORDS,
  HINT_RUNGS,
  attemptKey,
  confidenceWord,
  correctOptionId,
  ratingWord,
  type Attempt,
  type Confidence,
} from "./attempt"
import "./tutor.css"

type AttemptPanelProps = {
  attempt: Attempt | null
  /** Send the chosen option to the agent. Undefined while there is no live call. */
  onAnswer?: (choice: string) => void
  /** Send a confidence level to the agent. Undefined while there is no live call. */
  onRate?: (level: Confidence) => void
  className?: string
}

/**
 * The attempt the tutor is running, shown neutrally: the question card, the confidence keys, the
 * hint ladder and, after the check, the flipped card with the verdict. No subject colour here;
 * green and red appear only as the verdict, always with a word and a mark.
 */
export function AttemptPanel({ attempt, onAnswer, onRate, className }: AttemptPanelProps) {
  return (
    <section aria-label="Practice question" className={cn("flex min-w-0 flex-col gap-4", className)}>
      {attempt ? (
        <AttemptView key={attempt.attemptId} attempt={attempt} onAnswer={onAnswer} onRate={onRate} />
      ) : (
        <EmptyAttempt />
      )}
    </section>
  )
}

function EmptyAttempt() {
  return (
    <div className="tv-fade border-hairline-strong flex min-h-44 flex-col items-center justify-center gap-3 rounded-2xl border border-dashed px-8 py-10 text-center @max-4xl:hidden">
      <Eyebrow>Practice</Eyebrow>
      <p className="font-serif text-xl">No question yet</p>
      <KbdHint className="max-w-64 text-[15px]">
        When the tutor asks one it appears here. Press <Kbd>A</Kbd>–<Kbd>D</Kbd> to answer and <Kbd>1</Kbd>–<Kbd>4</Kbd> for how sure
        you are.
      </KbdHint>
    </div>
  )
}

function announcement(a: Attempt): string {
  if (a.checked) return a.checked.correct === true ? "Correct." : a.checked.correct === false ? "Not correct." : "Answer checked."
  if (a.rung) return `Hint ${a.rung} of ${HINT_RUNGS.length}, ${HINT_RUNGS[a.rung - 1]}.`
  return "New question."
}

function AttemptView({ attempt, onAnswer, onRate }: { attempt: Attempt } & Pick<AttemptPanelProps, "onAnswer" | "onRate">) {
  const { checked, options } = attempt
  // The learner can flip back to the question after the check; a new check flips forward again.
  const [backToFront, setBackToFront] = useState(false)
  const flipped = Boolean(checked) && !backToFront
  const live = !checked && Boolean(onAnswer && onRate)

  const correctId = checked ? correctOptionId(options, checked.correctAnswer) : null
  const correctOption = options.find((o) => o.id === correctId)

  const latest = useRef({ attempt, onAnswer, onRate })
  latest.current = { attempt, onAnswer, onRate }

  const answer = (id: string) => {
    const s = latest.current
    if (s.attempt.answer === null) s.onAnswer?.(id)
  }

  // A-D answer, 1-4 rate. One listener for the whole panel, so the two key sets cannot collide.
  useEffect(() => {
    if (!live) return
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.repeat || e.ctrlKey || e.metaKey || e.altKey || isEditableTarget(e.target)) return
      const s = latest.current
      const key = attemptKey(e.key, s.attempt.options.length)
      if (!key) return
      e.preventDefault()
      if (key.kind === "answer") answer(s.attempt.options[key.index].id)
      else s.onRate?.(key.level)
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [live])

  const statusOf = (id: string): KeycapStatus | undefined => {
    if (!checked) return undefined
    if (id === correctId) return "correct"
    if (id === attempt.answer && checked.correct !== null) return checked.correct ? "correct" : "wrong"
    return undefined
  }

  return (
    <div className="tv-enter flex flex-col gap-4">
      <QuestionCard
        className="tv-card"
        eyebrow="Practice question"
        mastery={null}
        flipped={flipped}
        onFlippedChange={(f) => setBackToFront(!f)}
        verdict={checked ? <Verdict correct={checked.correct} chosen={attempt.answer} /> : undefined}
        explanation={
          checked ? (
            <div className="flex flex-col gap-4">
              {checked.correctAnswer ? (
                <div>
                  <Eyebrow as="div" className="mb-1.5">
                    Correct answer
                  </Eyebrow>
                  <p className="text-[19px] leading-snug">
                    {correctOption ? `${correctOption.id}. ${correctOption.text}` : checked.correctAnswer}
                  </p>
                </div>
              ) : null}
              {checked.explanation ? <p className="whitespace-pre-wrap">{checked.explanation}</p> : null}
            </div>
          ) : undefined
        }
        misconception={checked?.misconception ? <p className="whitespace-pre-wrap">{checked.misconception}</p> : undefined}
      >
        <div className="flex flex-col gap-5">
          <p className="font-serif text-[21px] leading-[1.45] [overflow-wrap:anywhere] whitespace-pre-wrap text-pretty">
            {attempt.question}
          </p>
          {options.length ? (
            <KeycapGroup
              aria-label="Answer choices"
              className="items-stretch"
              choices={options.map((o, i) => ({
                id: o.id,
                legend: o.id.length === 1 ? o.id.toUpperCase() : String(i + 1),
                label: o.text,
                status: statusOf(o.id),
              }))}
              value={attempt.answer}
              sendOnPress
              onSubmit={answer}
              hotkeys="none"
              disabled={!checked && !onAnswer}
            />
          ) : (
            <KbdHint>Answer aloud, or type it in the message box.</KbdHint>
          )}
        </div>
      </QuestionCard>

      {checked ? <Footer attempt={attempt} /> : null}

      <Panel className="flex flex-col gap-5 p-4">
        {!checked ? (
          <div>
            <div className="mb-2 flex items-baseline justify-between gap-3">
              <Eyebrow>How sure are you?</Eyebrow>
              {live ? (
                <span className="text-ink-3 text-xs">
                  Press <Kbd>1</Kbd>–<Kbd>4</Kbd>
                </span>
              ) : null}
            </div>
            <KeycapGroup
              aria-label="How sure are you?"
              className="tv-conf items-stretch"
              choices={CONFIDENCE_WORDS.map((word, i) => ({ id: String(i + 1), legend: String(i + 1), label: word }))}
              value={attempt.confidence ? String(attempt.confidence) : null}
              sendOnPress
              onSubmit={(id) => onRate?.(Number(id) as Confidence)}
              hotkeys="none"
              disabled={!onRate}
            />
          </div>
        ) : null}
        <HintLadder rung={attempt.rung} text={attempt.hintText} isAnswer={attempt.hintIsAnswer} />
      </Panel>

      <p aria-live="polite" className="sr-only">
        {announcement(attempt)}
      </p>
    </div>
  )
}

function Verdict({ correct, chosen }: { correct: boolean | null; chosen: string | null }) {
  const Icon = correct === true ? Check : correct === false ? X : Minus
  const word = correct === true ? "Correct" : correct === false ? "Not quite" : "Not graded automatically"
  return (
    <p
      className={cn(
        "inline-flex w-fit items-center gap-2 rounded-full border py-1 pr-3.5 pl-3 font-sans text-[13px] font-medium",
        correct === true && "border-ok/40 bg-ok/10 text-[color-mix(in_oklab,var(--ok)_78%,var(--ink))]",
        correct === false && "border-bad/40 bg-bad/10 text-[color-mix(in_oklab,var(--bad)_78%,var(--ink))]",
        correct === null && "border-hairline-strong text-ink-2",
      )}
    >
      <Icon aria-hidden className="size-3.5" strokeWidth={2.5} />
      {word}
      {chosen && correct === false ? <span className="text-ink-2 font-normal">· you chose {chosen}</span> : null}
    </p>
  )
}

function HintLadder({ rung, text, isAnswer }: { rung: number; text: string | null; isAnswer: boolean }) {
  // The ladder can sit below the fold of the column; bring a new hint into view, gently.
  const hintRef = useRef<HTMLDivElement>(null)
  const reduced = useReducedMotion()
  const shown = useRef(rung)
  useEffect(() => {
    // Only a hint that arrives while the panel is open, not one already there at first render.
    if (rung === shown.current) return
    shown.current = rung
    hintRef.current?.scrollIntoView({ block: "nearest", behavior: reduced ? "auto" : "smooth" })
  }, [rung, reduced])

  return (
    <div>
      <div className="mb-2.5 flex items-baseline justify-between gap-3">
        <Eyebrow>Hint ladder</Eyebrow>
        <span className="text-ink-2 text-xs">
          {rung ? `Rung ${rung} of ${HINT_RUNGS.length}, ${HINT_RUNGS[rung - 1]}` : "No hints used"}
        </span>
      </div>
      <ol aria-label="Hint rungs" className="grid grid-cols-5 gap-1.5">
        {HINT_RUNGS.map((word, i) => (
          <li key={word} aria-current={i + 1 === rung ? "step" : undefined} className="flex flex-col gap-1.5">
            <span aria-hidden className="tv-rung" data-on={i < rung ? "" : undefined} style={{ "--n": i } as CSSProperties}>
              <i />
            </span>
            <span
              className={cn(
                "font-mono text-[11px] leading-tight text-balance",
                i + 1 === rung ? "text-foreground" : i < rung ? "text-ink-2" : "text-ink-3",
              )}
            >
              {i + 1} {word}
              <span className="sr-only">{i < rung ? ", given" : ", not used"}</span>
            </span>
          </li>
        ))}
      </ol>
      {rung ? (
        <div key={rung} ref={hintRef} className="tv-fade border-hairline-strong mt-3.5 scroll-mb-4 border-l-2 pl-3.5">
          {isAnswer ? (
            <Eyebrow as="div" className="mb-1.5">
              Answer given
            </Eyebrow>
          ) : null}
          <p className={cn("font-serif text-[17px] leading-[1.6] text-pretty", text === null && "text-ink-2 italic")}>
            {text ?? "The tutor is giving this one aloud."}
          </p>
        </div>
      ) : null}
    </div>
  )
}

/** After the check: what the learner said, and when the card comes back. Quiet, in the margin. */
function Footer({ attempt }: { attempt: Attempt }) {
  const { confidence, reviewed } = attempt
  if (!confidence && !reviewed) return null
  return (
    <p className="text-ink-3 flex flex-wrap gap-x-5 gap-y-1 font-mono text-[11px] tracking-[0.14em] uppercase">
      {confidence ? <span>Confidence · {confidenceWord(confidence)}</span> : null}
      {reviewed ? (
        <span className="tv-fade">
          {ratingWord(reviewed.rating)} · next review {reviewed.nextDue ?? "scheduled"}
        </span>
      ) : null}
    </p>
  )
}
