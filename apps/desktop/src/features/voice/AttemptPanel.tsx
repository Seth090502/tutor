import { useEffect, useRef, useState, type CSSProperties, type FormEvent, type ReactNode } from "react"
import { Check, Minus, X } from "lucide-react"

import { Eyebrow, Kbd, KbdHint, KeycapGroup, Panel, QuestionCard, QuietNote, TextField, type KeycapStatus } from "@/components/kit"
import { isEditableTarget } from "@/components/kit/keycap-logic"
import { Button } from "@/components/ui/button"
import type { ConfidenceLevel, StudyAttempt } from "@/lib/study"
import { useReducedMotion } from "@/lib/motion"
import { cn } from "@/lib/utils"
import {
  CONFIDENCE_WORDS,
  HINT_RUNGS,
  SKIP_REASONS,
  attemptKey,
  confidenceWord,
  correctOptionId,
  hintsGiven,
  lastCheck,
  offerWords,
  optionList,
  reviewWords,
} from "./attempt"
import { type PanelActions, studyErrorText } from "./practice"
import "./tutor.css"

type AttemptPanelProps = {
  /** The attempt as the server last accepted it (C12 public state). */
  attempt: StudyAttempt | null
  /** Typed controls (source ui). Undefined while there is no study session to send them to. */
  actions?: PanelActions
  /** A problem reaching the practice service, shown while there is no attempt. */
  notice?: string | null
  /** Starts typed practice without a call (source ui); absent while it cannot. */
  onStart?: () => void
  className?: string
}

/**
 * The attempt the tutor and the learner are working on, shown neutrally: the question card, the
 * controls for the attempt's status, the hint ladder and, once the server releases it, the flipped
 * card with the answer. No subject colour here; green and red appear only as the verdict, always
 * with a word and a mark.
 */
export function AttemptPanel({ attempt, actions, notice, onStart, className }: AttemptPanelProps) {
  return (
    <section aria-label="Practice question" className={cn("flex min-w-0 flex-col gap-4", className)}>
      {attempt ? <AttemptView key={attempt.attempt_id} attempt={attempt} actions={actions} /> : <EmptyAttempt notice={notice} onStart={onStart} />}
    </section>
  )
}

function EmptyAttempt({ notice, onStart }: { notice?: string | null; onStart?: () => void }) {
  return (
    <div className="tv-fade border-hairline-strong flex min-h-44 flex-col items-center justify-center gap-3 rounded-2xl border border-dashed px-8 py-10 text-center @max-4xl:hidden">
      <Eyebrow>Practice</Eyebrow>
      <p className="font-serif text-xl">No question yet</p>
      <KbdHint className="max-w-64 text-[15px]">
        When the tutor asks one it appears here. Press <Kbd>A</Kbd>–<Kbd>D</Kbd> to answer and <Kbd>1</Kbd>–<Kbd>4</Kbd> for how sure
        you are.
      </KbdHint>
      {onStart ? (
        <Button variant="outline" size="sm" onClick={onStart}>
          Practise without a call
        </Button>
      ) : null}
      {notice ? <QuietNote tone="warn">{notice}</QuietNote> : null}
    </div>
  )
}

const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1)

type Verdict = { tone: "ok" | "bad" | "neutral"; word: string }

/** The result so far in a word: the closure when closed, else the newest check. */
function verdictOf(a: StudyAttempt): Verdict | null {
  const { closure } = a
  if (closure) {
    if (closure.reason === "correct") return { tone: "ok", word: a.repairs.length ? "Correct on a retry" : "Correct" }
    if (closure.reason === "self_assessed")
      return a.self_assessment?.judged === "correct" ? { tone: "ok", word: "You got it" } : { tone: "bad", word: "You missed it" }
    if (closure.reason === "revealed") return { tone: "neutral", word: "Answer shown" }
    if (closure.reason === "invalidated") return { tone: "neutral", word: "Set aside: the answer key could not be read" }
    return { tone: "neutral", word: "Left without a check" }
  }
  const verdict = lastCheck(a)?.verdict
  if (verdict === "incorrect") return { tone: "bad", word: "Not quite" }
  if (verdict === "unverifiable") return { tone: "neutral", word: "Not graded automatically" }
  return null
}

function announcement(a: StudyAttempt): string {
  const verdict = verdictOf(a)
  if (verdict) return `${verdict.word}.`
  if (a.pending_proposal) return "The tutor heard an answer. Confirm or correct it."
  if (a.status === "committed") return "Answer confirmed. How sure are you?"
  if (a.status === "ready") return "Ready to check."
  const rung = hintsGiven(a).length
  if (rung) return `Hint ${rung} of ${HINT_RUNGS.length}, ${HINT_RUNGS[rung - 1]}.`
  return "New question."
}

function AttemptView({ attempt, actions }: { attempt: StudyAttempt; actions?: PanelActions }) {
  const { status, released } = attempt
  const options = optionList(attempt.item.options)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // The learner can flip back to the question once the answer is out.
  const [backToFront, setBackToFront] = useState(false)
  const flipped = Boolean(released) && !backToFront

  const busyRef = useRef(false)

  /** Sends one typed action; a refusal shows as a sentence and changes nothing on screen. */
  const run = (send: (a: PanelActions) => Promise<unknown>) => {
    if (!actions || busyRef.current) return
    busyRef.current = true
    setBusy(true)
    setError(null)
    send(actions)
      .catch((e: unknown) => setError(`${capitalize(studyErrorText(e))}.`))
      .finally(() => {
        busyRef.current = false
        setBusy(false)
      })
  }

  const choosing = status === "open" || status === "remediating"
  const enabled = Boolean(actions) && !busy
  /** A-D: the first answer while open, a new try while remediating. */
  const choose = (id: string) => run((a) => (status === "remediating" ? a.repair(id) : a.answer(id)))
  const rate = (level: ConfidenceLevel) => run((a) => a.confidence(level))

  const latest = useRef({ attempt, choose, rate })
  latest.current = { attempt, choose, rate }

  // A-D answer, 1-4 rate. One listener for the whole panel, so the two key sets cannot collide.
  const keys = Boolean(actions) && ((choosing && options.length > 0) || status === "committed")
  useEffect(() => {
    if (!keys) return
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.repeat || e.ctrlKey || e.metaKey || e.altKey || isEditableTarget(e.target)) return
      const s = latest.current
      const key = attemptKey(e.key, optionList(s.attempt.item.options).length)
      if (!key) return
      const st = s.attempt.status
      if (key.kind === "answer" && (st === "open" || st === "remediating")) {
        e.preventDefault()
        s.choose(optionList(s.attempt.item.options)[key.index].id)
      } else if (key.kind === "confidence" && st === "committed") {
        e.preventDefault()
        s.rate(key.level)
      }
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [keys])

  // Marks only once the answer is released: before that the keys stay pressable for a new try, and
  // the verdict shows below the card.
  const correctId = released ? correctOptionId(options, released.correct_answer) : null
  const correctOption = options.find((o) => o.id === correctId)
  const responses = [attempt.first_response, ...attempt.repairs].filter((r) => r !== null)
  const newest = responses[responses.length - 1]?.response ?? null
  const newestWrong = lastCheck(attempt)?.verdict === "incorrect"
  const statusOf = (id: string): KeycapStatus | undefined => {
    if (!released) return undefined
    if (id === correctId) return "correct"
    if (id === newest && newestWrong) return "wrong"
    return undefined
  }
  const verdict = verdictOf(attempt)
  const helped = attempt.assistance.some((a) => a.before_commit)

  return (
    <div className="tv-enter flex flex-col gap-4">
      <QuestionCard
        className="tv-card"
        eyebrow="Practice question"
        mastery={null}
        flipped={flipped}
        onFlippedChange={(f) => setBackToFront(!f)}
        verdict={released && verdict ? <VerdictPill verdict={verdict} /> : undefined}
        explanation={
          released ? (
            <div className="flex flex-col gap-4">
              {released.correct_answer ? (
                <div>
                  <Eyebrow as="div" className="mb-1.5">
                    {status === "awaiting_self_assessment" ? "Reference answer" : "Correct answer"}
                  </Eyebrow>
                  <p className="text-[19px] leading-snug whitespace-pre-wrap">
                    {correctOption ? `${correctOption.id}. ${correctOption.text}` : released.correct_answer}
                  </p>
                </div>
              ) : null}
              {released.explanation ? <p className="whitespace-pre-wrap">{released.explanation}</p> : null}
            </div>
          ) : undefined
        }
        misconception={released?.misconception ? <p className="whitespace-pre-wrap">{released.misconception}</p> : undefined}
      >
        <div className="flex flex-col gap-5">
          <p className="font-serif text-[21px] leading-[1.45] [overflow-wrap:anywhere] whitespace-pre-wrap text-pretty">
            {attempt.item.prompt}
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
              value={newest}
              sendOnPress
              onSubmit={choose}
              hotkeys="none"
              disabled={!(choosing && enabled)}
            />
          ) : null}
          {!options.length && responses.length ? (
            <p className="text-ink-2 text-[15px]">
              {responses.length > 1 ? "Your latest try: " : "Your answer: "}
              <span className="text-foreground [overflow-wrap:anywhere] whitespace-pre-wrap">
                {newest || "I don't know"}
              </span>
            </p>
          ) : null}
          {status === "open" && !options.length ? (
            <TextAnswer label="Your answer" submit="Answer" disabled={!enabled} onSubmit={(t) => run((a) => a.answer(t))} />
          ) : null}
          {status === "open" ? (
            <div>
              <Button variant="ghost" size="sm" disabled={!enabled} onClick={() => run((a) => a.dontKnow())}>
                I don't know
              </Button>
            </div>
          ) : null}
        </div>
      </QuestionCard>

      {attempt.pending_proposal && choosing ? (
        <Proposal
          key={attempt.pending_proposal.proposed_at}
          proposal={attempt.pending_proposal}
          remediating={status === "remediating"}
          disabled={!enabled}
          onConfirm={(raw, confirmed) => run((a) => a.confirm(raw, confirmed))}
          onDontKnow={() => run((a) => a.dontKnow())}
        />
      ) : null}

      <Panel className="flex flex-col gap-5 p-4">
        {status === "open" ? (
          <div className="flex flex-col gap-2">
            <div>
              <Button variant="outline" size="sm" disabled={!enabled || hintsGiven(attempt).length >= HINT_RUNGS.length} onClick={() => run((a) => a.hint())}>
                Hint
              </Button>
            </div>
            <p className="text-ink-2 text-[13px] leading-snug">
              {helped
                ? "You had help before answering, so this question will not count as unaided."
                : "A hint before you answer means this question will not count as unaided."}
            </p>
          </div>
        ) : null}

        {status === "committed" ? (
          <div className="flex flex-col gap-3">
            <div className="flex items-baseline justify-between gap-3">
              <Eyebrow>How sure are you?</Eyebrow>
              {actions ? (
                <span className="text-ink-3 text-xs">
                  Press <Kbd>1</Kbd>–<Kbd>4</Kbd>
                </span>
              ) : null}
            </div>
            <KeycapGroup
              aria-label="How sure are you?"
              className="tv-conf items-stretch"
              choices={CONFIDENCE_WORDS.map((word, i) => ({ id: String(i + 1), legend: String(i + 1), label: word }))}
              value={null}
              sendOnPress
              onSubmit={(id) => rate(Number(id) as ConfidenceLevel)}
              hotkeys="none"
              disabled={!enabled}
            />
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-ink-2 text-[13px]">Skip, because:</span>
              {SKIP_REASONS.map((reason) => (
                <Button key={reason} variant="ghost" size="xs" disabled={!enabled} onClick={() => run((a) => a.skipConfidence(reason))}>
                  {reason}
                </Button>
              ))}
            </div>
          </div>
        ) : null}

        {status === "ready" ? (
          <div className="flex items-center justify-between gap-3">
            <ConfidenceNote attempt={attempt} />
            <Button disabled={!enabled} onClick={() => run((a) => a.check())}>
              Check
            </Button>
          </div>
        ) : null}

        {status === "remediating" ? (
          <div className="flex flex-col gap-3">
            {verdict ? <VerdictPill verdict={verdict} /> : null}
            {lastCheck(attempt)?.diagnostic ? <p className="text-[15px]">{lastCheck(attempt)?.diagnostic}</p> : null}
            {attempt.offer ? <p className="text-ink-2 text-[15px]">{offerWords(attempt.offer)}</p> : null}
            <TextAnswer label="Try again" submit="Try again" disabled={!enabled} onSubmit={(t) => run((a) => a.repair(t))} />
            <div className="flex flex-wrap gap-2">
              <Button variant="outline" size="sm" disabled={!enabled || hintsGiven(attempt).length >= HINT_RUNGS.length} onClick={() => run((a) => a.hint())}>
                Hint
              </Button>
              <Button variant="ghost" size="sm" disabled={!enabled} onClick={() => run((a) => a.giveUp())}>
                Give up
              </Button>
            </div>
          </div>
        ) : null}

        {status === "awaiting_self_assessment" ? (
          <div className="flex flex-col gap-3">
            <p className="text-[15px]">
              This answer has no automatic check. Compare yours with the reference answer on the card, then say how it went.
            </p>
            <div className="flex flex-wrap gap-2">
              <Button disabled={!enabled} onClick={() => run((a) => a.selfAssess("correct"))}>
                I got it
              </Button>
              <Button variant="outline" disabled={!enabled} onClick={() => run((a) => a.selfAssess("incorrect"))}>
                I missed it
              </Button>
            </div>
          </div>
        ) : null}

        {status === "closed" ? (
          <div className="flex flex-col gap-3">
            {verdict ? <VerdictPill verdict={verdict} /> : null}
            <ClosedNote attempt={attempt} />
            <div>
              <Button
                disabled={!enabled}
                onClick={() =>
                  run(async (a) => {
                    if (!(await a.next())) setError("No practice questions are due right now.")
                  })
                }
              >
                Next question
              </Button>
            </div>
          </div>
        ) : null}

        <HintLadder attempt={attempt} />
      </Panel>

      {error ? <QuietNote tone="warn">{error}</QuietNote> : null}

      <p aria-live="polite" className="sr-only">
        {announcement(attempt)}
      </p>
    </div>
  )
}

function TextAnswer({
  label,
  submit,
  disabled,
  onSubmit,
  initial = "",
}: {
  label: string
  submit: string
  disabled: boolean
  onSubmit: (text: string) => void
  initial?: string
}) {
  const [value, setValue] = useState(initial)
  const send = (e: FormEvent) => {
    e.preventDefault()
    if (value.trim()) onSubmit(value.trim())
  }
  return (
    <form className="flex gap-2" onSubmit={send}>
      <TextField aria-label={label} className="flex-1" value={value} onChange={(e) => setValue(e.target.value)} disabled={disabled} />
      <Button type="submit" variant="outline" disabled={disabled || !value.trim()}>
        {submit}
      </Button>
    </form>
  )
}

/** What the tutor heard, for the learner to confirm as is or after editing. */
function Proposal({
  proposal,
  remediating,
  disabled,
  onConfirm,
  onDontKnow,
}: {
  proposal: NonNullable<StudyAttempt["pending_proposal"]>
  remediating: boolean
  disabled: boolean
  onConfirm: (raw: string, confirmed: string) => void
  onDontKnow: () => void
}) {
  const raw = proposal.raw_transcript
  const [edited, setEdited] = useState(raw)
  if (proposal.input_mode === "dont_know") {
    return (
      <Panel className="flex flex-col gap-3 p-4">
        <p className="text-[15px]">Tutor heard: I don't know</p>
        {remediating ? (
          <p className="text-ink-2 text-[13px]">To stop here, press Give up.</p>
        ) : (
          <div>
            <Button disabled={disabled} onClick={onDontKnow}>
              Confirm
            </Button>
          </div>
        )}
      </Panel>
    )
  }
  const confirmed = edited.trim()
  return (
    <Panel className="flex flex-col gap-3 p-4">
      <p className="text-[15px]">
        Tutor heard: <span className="font-serif text-[17px] [overflow-wrap:anywhere]">{raw}</span>
      </p>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault()
          if (confirmed) onConfirm(raw, confirmed)
        }}
      >
        <TextField
          aria-label={remediating ? "Your new try, as it should read" : "Your answer, as it should read"}
          className="flex-1"
          value={edited}
          onChange={(e) => setEdited(e.target.value)}
          disabled={disabled}
        />
        <Button type="submit" disabled={disabled || !confirmed}>
          Confirm
        </Button>
      </form>
    </Panel>
  )
}

function ConfidenceNote({ attempt }: { attempt: StudyAttempt }) {
  const c = attempt.confidence
  if (!c) return <span />
  return (
    <span className="text-ink-3 font-mono text-[11px] tracking-[0.14em] uppercase">
      {c.level ? `Confidence · ${confidenceWord(c.level)}` : `Confidence skipped · ${c.skipped_reason ?? ""}`}
    </span>
  )
}

/** After closure: how the question was scheduled and why, when it comes back, and any offer. */
function ClosedNote({ attempt }: { attempt: StudyAttempt }) {
  const review = reviewWords(attempt)
  const due = attempt.closure?.scheduler_disposition.applied?.due_at
  const lines: ReactNode[] = []
  if (review) lines.push(<span key="review">{review}</span>)
  if (due != null) {
    const when = new Date(due * 1000).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" })
    lines.push(<span key="due">Next review {when}</span>)
  }
  if (attempt.confidence?.level) lines.push(<span key="conf">Confidence · {confidenceWord(attempt.confidence.level)}</span>)
  return (
    <div className="flex flex-col gap-2">
      <p className="text-ink-2 flex flex-wrap gap-x-5 gap-y-1 text-[13px]">{lines}</p>
      {attempt.offer ? <p className="text-ink-2 text-[15px]">{offerWords(attempt.offer)}</p> : null}
    </div>
  )
}

function VerdictPill({ verdict }: { verdict: Verdict }) {
  const Icon = verdict.tone === "ok" ? Check : verdict.tone === "bad" ? X : Minus
  return (
    <p
      className={cn(
        "inline-flex w-fit items-center gap-2 rounded-full border py-1 pr-3.5 pl-3 font-sans text-[13px] font-medium",
        verdict.tone === "ok" && "border-ok/40 bg-ok/10 text-[color-mix(in_oklab,var(--ok)_78%,var(--ink))]",
        verdict.tone === "bad" && "border-bad/40 bg-bad/10 text-[color-mix(in_oklab,var(--bad)_78%,var(--ink))]",
        verdict.tone === "neutral" && "border-hairline-strong text-ink-2",
      )}
    >
      <Icon aria-hidden className="size-3.5" strokeWidth={2.5} />
      {verdict.word}
    </p>
  )
}

function HintLadder({ attempt }: { attempt: StudyAttempt }) {
  const hints = hintsGiven(attempt)
  const rung = hints.length
  const text = hints[rung - 1]?.text ?? null
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
      <ol aria-label="Hint rungs" className="grid grid-cols-4 gap-1.5">
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
          <p className={cn("font-serif text-[17px] leading-[1.6] text-pretty", text === null && "text-ink-2 italic")}>
            {text ?? "The tutor is giving this one aloud."}
          </p>
        </div>
      ) : null}
    </div>
  )
}
