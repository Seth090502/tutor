// Practice: a typed drill over the learner's language cards, production first. It drives the C7 attempt
// routes directly. Attempt first, confidence before the check, the hint ladder, a check that names an
// accent slip, then the server-rated review. The state machine is drill.ts; this file draws it.
import { useCallback, useEffect, useReducer, useRef, useState } from "react"
import { ArrowRight, BookOpenCheck, Lightbulb, Loader2, RefreshCw, ServerOff } from "lucide-react"

import { Eyebrow, KbdHint, Kbd, KeycapGroup, Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { TargetLanguage } from "./_pending/language"
import { languageMeta } from "./_pending/language"
import { checkAnswer, errorText, isOffline, loadQueue, openCard, requestHint, sendConfidence, starterDeckCommand, submitReview, type QueueItem } from "./api"
import {
  assistanceNote,
  assisted,
  canCheck,
  canHint,
  canReview,
  diffChars,
  drillReducer,
  feedbackLine,
  hintBlocker,
  MAX_RUNG,
  needsJudgement,
  newDrill,
  targetSentence,
  type Confidence,
  type DrillState,
  type CardKind,
} from "./drill"
import type { AudioEnv } from "./hooks"
import { CopyCommand, FIELD, Notice, PlayButton, StateBlock, Verdict } from "./parts"
import { useSay } from "./speech"

const CONFIDENCE = [
  { id: "1", label: "Guess" },
  { id: "2", label: "Unsure" },
  { id: "3", label: "Fairly sure" },
  { id: "4", label: "Certain" },
] as const

const KIND_COPY: Record<CardKind, { label: string; ask: (lang: string) => string }> = {
  production: { label: "Production", ask: (l) => `Write this in ${l}` },
  recognition: { label: "Recognition", ask: () => "What does this mean?" },
  listening: { label: "Listening", ask: () => "Listen, then type what it means" },
  unknown: { label: "Card", ask: () => "Answer" },
}

const RATING_WORD: Record<string, string> = { good: "Good", hard: "Hard", again: "Again" }

type Load =
  | { status: "loading" }
  | { status: "offline" }
  | { status: "error"; message: string }
  | { status: "empty" }
  | { status: "ready" }

type Tally = { reviewed: number; assisted: number }

export function PracticeTab({ language, audio }: { language: TargetLanguage; audio: AudioEnv }) {
  const meta = languageMeta(language)
  const [load, setLoad] = useState<Load>({ status: "loading" })
  const [queue, setQueue] = useState<QueueItem[]>([])
  const [index, setIndex] = useState(0)
  const [drill, dispatchRaw] = useReducer(
    (s: DrillState | null, a: Parameters<typeof drillReducer>[1] | { type: "open"; drill: DrillState | null }) =>
      a.type === "open" ? a.drill : s && drillReducer(s, a),
    null,
  )
  const [tally, setTally] = useState<Tally>({ reviewed: 0, assisted: 0 })
  const { say, stop, status: sayStatus, error: sayError } = useSay(language)
  const attempted = useRef(new Set<string>())
  const generation = useRef(0)

  const open = useCallback(async (items: QueueItem[], at: number) => {
    const mine = generation.current
    try {
      const card = await openCard(items[at])
      if (mine === generation.current) dispatchRaw({ type: "open", drill: newDrill(card) })
    } catch (e) {
      if (mine !== generation.current) return
      setLoad(isOffline(e) ? { status: "offline" } : { status: "error", message: errorText(e) })
    }
  }, [])

  const start = useCallback(async () => {
    const mine = ++generation.current
    setLoad({ status: "loading" })
    dispatchRaw({ type: "open", drill: null })
    stop()
    try {
      const items = await loadQueue(language)
      if (mine !== generation.current) return
      setQueue(items)
      setIndex(0)
      setTally({ reviewed: 0, assisted: 0 })
      if (items.length === 0) return setLoad({ status: "empty" })
      setLoad({ status: "ready" })
      await open(items, 0)
    } catch (e) {
      if (mine === generation.current) setLoad(isOffline(e) ? { status: "offline" } : { status: "error", message: errorText(e) })
    }
  }, [language, open, stop])

  useEffect(() => {
    void start()
    return () => {
      generation.current++
    }
  }, [start])

  const dispatch = dispatchRaw

  const onHint = async () => {
    if (!drill || !canHint(drill)) return
    dispatch({ type: "hint:start" })
    try {
      dispatch({ type: "hint:done", hint: await requestHint(drill.card.attemptId) })
    } catch (e) {
      dispatch({ type: "failed", message: errorText(e) })
    }
  }

  const onCheck = async () => {
    if (!drill || !canCheck(drill) || drill.confidence === null) return
    dispatch({ type: "check:start" })
    try {
      await sendConfidence(drill.card.attemptId, drill.confidence)
      const result = await checkAnswer(drill.card.attemptId, drill.draft.trim())
      dispatch({ type: "check:done", result })
      if (!audio.blocker) void say(targetSentence(drill.card, result))
    } catch (e) {
      dispatch({ type: "failed", message: errorText(e) })
    }
  }

  const runReview = useCallback(async (s: DrillState) => {
    dispatch({ type: "review:start" })
    try {
      const result = await submitReview(s.card, s.draft.trim(), s.judged)
      dispatch({ type: "review:done", result })
      setTally((t) => ({ reviewed: t.reviewed + 1, assisted: t.assisted + (assisted(s) ? 1 : 0) }))
    } catch (e) {
      dispatch({ type: "failed", message: errorText(e) })
    }
  }, [])

  // The review is sent once per attempt as soon as the check (or the learner's verdict) allows it.
  // A failed review is retried with the button, not by this effect.
  useEffect(() => {
    if (!drill || !canReview(drill) || attempted.current.has(drill.card.attemptId)) return
    attempted.current.add(drill.card.attemptId)
    void runReview(drill)
  }, [drill, runReview])

  const next = async () => {
    stop()
    const at = index + 1
    if (at >= queue.length) return dispatchRaw({ type: "open", drill: null })
    setIndex(at)
    await open(queue, at)
  }

  if (load.status === "loading") {
    return (
      <div role="status" className="text-ink-2 flex items-center justify-center gap-3 py-24 text-sm">
        <Loader2 aria-hidden className="size-4 animate-spin" /> Loading your {meta.english} cards
      </div>
    )
  }
  if (load.status === "offline") {
    return (
      <StateBlock icon={ServerOff} title="The backend is not running" actions={<RetryButton onClick={() => void start()} />}>
        Cards and their schedule live in the DeepTutor backend on this machine. Start it from Settings, then try again.
      </StateBlock>
    )
  }
  if (load.status === "error") {
    return (
      <StateBlock icon={ServerOff} title="Could not load your cards" actions={<RetryButton onClick={() => void start()} />}>
        {load.message}
      </StateBlock>
    )
  }
  if (load.status === "empty") return <NoCards language={language} onRetry={() => void start()} />

  if (!drill) {
    return (
      <StateBlock
        icon={BookOpenCheck}
        title="That is every card due right now"
        actions={<RetryButton onClick={() => void start()} label="Check again" />}
      >
        You reviewed {tally.reviewed} {tally.reviewed === 1 ? "card" : "cards"}
        {tally.assisted ? `, ${tally.assisted} with hints` : ", none with hints"}. New cards come due on the schedule, so there is nothing to
        gain from drilling these again today.
      </StateBlock>
    )
  }

  return (
    <div className="@container">
      <div className="grid items-start gap-5 @3xl:grid-cols-[minmax(0,1fr)_17rem]">
        <CardPane
          key={drill.card.attemptId}
          drill={drill}
          language={language}
          position={`Card ${index + 1} of ${queue.length}`}
          tally={tally}
          dispatch={dispatch}
          onHint={onHint}
          onCheck={onCheck}
          onRetryReview={() => void runReview(drill)}
          onNext={() => void next()}
          isLast={index + 1 >= queue.length}
          audio={audio}
          say={say}
          stop={stop}
          sayStatus={sayStatus}
          sayError={sayError}
        />
        <aside className="flex flex-col gap-5">
          <Panel eyebrow="Before you check" title="How sure are you?" className="lg-enter p-5" style={{ "--i": 1 } as React.CSSProperties}>
            <KeycapGroup
              aria-label="Confidence, 1 guess to 4 certain"
              choices={CONFIDENCE.map((c) => ({ id: c.id, legend: c.id, label: c.label }))}
              value={drill.confidence ? String(drill.confidence) : null}
              onChange={(id) => dispatch({ type: "confidence", level: Number(id) as Confidence })}
              onSubmit={() => void onCheck()}
              active={drill.phase === "attempt"}
              disabled={drill.pending !== null && drill.phase === "attempt"}
            />
            <KbdHint className="mt-4 text-[14px]">
              Press <Kbd>1</Kbd> to <Kbd>4</Kbd>, then <Kbd>Enter</Kbd> to check.
            </KbdHint>
          </Panel>
          <Ladder drill={drill} />
        </aside>
      </div>
    </div>
  )
}

function RetryButton({ onClick, label = "Try again" }: { onClick: () => void; label?: string }) {
  return (
    <Button variant="outline" size="sm" onClick={onClick}>
      <RefreshCw aria-hidden />
      {label}
    </Button>
  )
}

function NoCards({ language, onRetry }: { language: TargetLanguage; onRetry: () => void }) {
  const meta = languageMeta(language)
  return (
    <StateBlock
      icon={BookOpenCheck}
      title={`No ${meta.english} cards are due`}
      actions={<RetryButton onClick={onRetry} label="Check again" />}
    >
      <p>Either you are caught up, or there are no {meta.english} cards yet. To start with a deck built from open sources (Tatoeba sentence pairs and a frequency list), run this in a terminal in the project folder, then check again:</p>
      <div className="mt-3">
        <CopyCommand command={starterDeckCommand(language)} />
      </div>
    </StateBlock>
  )
}

function Ladder({ drill }: { drill: DrillState }) {
  const cap = Math.min(MAX_RUNG, drill.card.rungsAvailable)
  return (
    <Panel eyebrow="Hint ladder" className="lg-enter p-5" style={{ "--i": 2 } as React.CSSProperties}>
      <div className="flex gap-1.5" aria-hidden>
        {Array.from({ length: MAX_RUNG }, (_, i) => (
          <span key={i} className="lg-rung" data-used={i < drill.hints.length ? "" : undefined} style={i >= cap ? { opacity: 0.35 } : undefined} />
        ))}
      </div>
      <p className="text-ink-2 mt-3 text-sm leading-relaxed">
        {drill.card.rungsAvailable === 0
          ? "This card has no written hints."
          : `${drill.hints.length} of ${cap} used. Each hint needs a fresh attempt first, and any hint lowers the rating.`}
      </p>
    </Panel>
  )
}

type PaneProps = {
  drill: DrillState
  language: TargetLanguage
  position: string
  tally: Tally
  dispatch: (a: Parameters<typeof drillReducer>[1]) => void
  onHint: () => void
  onCheck: () => void
  onRetryReview: () => void
  onNext: () => void
  isLast: boolean
  audio: AudioEnv
  say: (text: string) => Promise<void>
  stop: () => void
  sayStatus: "idle" | "loading" | "playing" | "error"
  sayError: string | null
}

function CardPane({ drill, language, position, tally, dispatch, onHint, onCheck, onRetryReview, onNext, isLast, audio, say, stop, sayStatus, sayError }: PaneProps) {
  const meta = languageMeta(language)
  const { card } = drill
  const kind = KIND_COPY[card.kind]
  const targetPrompt = card.kind !== "production"
  const listening = card.kind === "listening"
  const checked = drill.phase !== "attempt"
  const inputRef = useRef<HTMLInputElement>(null)
  const nextRef = useRef<HTMLButtonElement>(null)
  const blocker = hintBlocker(drill)
  const check = drill.check

  useEffect(() => {
    if (drill.phase === "attempt") inputRef.current?.focus()
  }, [drill.phase])
  useEffect(() => {
    if (drill.phase === "done") nextRef.current?.focus()
  }, [drill.phase])

  const heard = check ? targetSentence(card, check) : card.prompt

  return (
    <Panel
      eyebrow={`${position} · ${kind.label}`}
      title={kind.ask(meta.english)}
      className="lg-enter p-6"
      actions={<span className="text-ink-3 font-mono text-[11px] tracking-[0.08em]">{tally.reviewed} reviewed</span>}
    >
      {listening && !checked ? (
        <div className="flex items-center gap-3 py-2">
          <PlayButton label="Play the sentence" status={sayStatus} onPlay={() => void say(card.prompt)} onStop={stop} disabledReason={audio.blocker} variant="secondary" size="default" />
          <span className="text-ink-2 text-sm">{audio.blocker ?? "The text stays hidden until you check."}</span>
        </div>
      ) : (
        <p lang={targetPrompt ? language : "en"} className="font-serif text-[26px] leading-snug tracking-[-0.01em] text-balance">
          {card.prompt}
        </p>
      )}

      <div className="mt-5">
        <label htmlFor="lg-answer" className="mb-1.5 block">
          <Eyebrow>Your attempt</Eyebrow>
        </label>
        <input
          id="lg-answer"
          ref={inputRef}
          type="text"
          lang={targetPrompt ? "en" : language}
          autoComplete="off"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          className={FIELD}
          placeholder={targetPrompt ? "Type the meaning in English" : `Type it in ${meta.english}`}
          value={drill.draft}
          readOnly={checked}
          aria-describedby="lg-answer-help"
          onChange={(e) => dispatch({ type: "draft", text: e.target.value })}
          onKeyDown={(e) => {
            if (e.key !== "Enter" || e.nativeEvent.isComposing) return
            e.preventDefault()
            if (canCheck(drill)) onCheck()
            else if (!checked) document.querySelector<HTMLElement>('[data-slot="kit-keycap"]')?.focus()
          }}
        />
        <p id="lg-answer-help" className="text-ink-3 mt-1.5 text-[13px]">
          Attempt first: hints open once you have typed something.
        </p>
      </div>

      {drill.hints.length > 0 ? (
        <ol className="mt-4 flex flex-col gap-2" aria-label="Hints used">
          {drill.hints.map((h) => (
            <li key={h.rung} className="border-hairline bg-secondary/60 flex gap-3 rounded-lg border px-3.5 py-2.5 text-[15px] leading-relaxed">
              <span className="text-ink-3 mt-0.5 shrink-0 font-mono text-[11px] tracking-[0.14em] uppercase">Hint {h.rung}</span>
              <span lang={language}>{h.text ?? "No written text for this rung. Try once more from what you know."}</span>
            </li>
          ))}
        </ol>
      ) : null}

      {drill.error ? (
        <Notice tone="warn" className="mt-4">
          {drill.error}
        </Notice>
      ) : null}

      {!checked ? (
        <div className="mt-5 flex flex-wrap items-center gap-x-3 gap-y-2">
          <Button variant="outline" onClick={onHint} disabled={!canHint(drill)} title={blocker ?? undefined}>
            {drill.pending === "hint" ? <Loader2 aria-hidden className="animate-spin" /> : <Lightbulb aria-hidden />}
            Hint
          </Button>
          <Button onClick={onCheck} disabled={!canCheck(drill)}>
            {drill.pending === "check" ? <Loader2 aria-hidden className="animate-spin" /> : null}
            Check
          </Button>
          <span className="text-ink-3 text-[13px]" aria-live="polite">
            {drill.confidence === null && drill.draft.trim() ? "Choose how sure you are to check." : blocker ?? ""}
          </span>
        </div>
      ) : check ? (
        <Feedback
          drill={drill}
          language={language}
          heard={heard}
          audio={audio}
          say={say}
          stop={stop}
          sayStatus={sayStatus}
          sayError={sayError}
          onRetryReview={onRetryReview}
          onJudge={(correct) => dispatch({ type: "judge", correct })}
        />
      ) : null}

      {drill.phase === "done" ? (
        <div className="mt-5">
          <Button ref={nextRef} onClick={onNext}>
            {isLast ? "Finish" : "Next card"}
            <ArrowRight aria-hidden />
          </Button>
        </div>
      ) : null}
    </Panel>
  )
}

function Feedback({
  drill,
  language,
  heard,
  audio,
  say,
  stop,
  sayStatus,
  sayError,
  onRetryReview,
  onJudge,
}: {
  drill: DrillState
  language: TargetLanguage
  heard: string
  audio: AudioEnv
  say: (text: string) => Promise<void>
  stop: () => void
  sayStatus: "idle" | "loading" | "playing" | "error"
  sayError: string | null
  onRetryReview: () => void
  onJudge: (correct: boolean) => void
}) {
  const check = drill.check!
  const word = check.correct === true ? "Correct" : check.correct === false ? (check.diagnostic === "accent" ? "Accent slip" : "Not quite") : "Compare"
  const production = drill.card.kind === "production"
  const confidentWrong = check.correct === false && (drill.confidence ?? 0) >= 3
  const accent = check.correct === false && check.diagnostic === "accent" && production

  return (
    <div className="lg-enter mt-6 flex flex-col gap-5" aria-live="polite">
      <div className="flex flex-wrap items-center gap-3">
        <Verdict ok={check.correct} word={word} />
        <span className="text-ink-2 text-[15px]">{feedbackLine(check)}</span>
      </div>

      <dl className="grid gap-x-6 gap-y-4 @xl:grid-cols-2">
        <div>
          <dt>
            <Eyebrow>You wrote</Eyebrow>
          </dt>
          <dd lang={production ? language : "en"} className="mt-1.5 font-serif text-[20px] leading-snug">
            {accent ? diffChars(drill.draft.trim(), check.correctAnswer).map((seg, i) => (seg.differs ? <span key={i} className="lg-diff">{seg.text}</span> : <span key={i}>{seg.text}</span>)) : drill.draft.trim()}
          </dd>
        </div>
        <div>
          <dt>
            <Eyebrow>Reference</Eyebrow>
          </dt>
          <dd lang={production ? language : "en"} className="mt-1.5 font-serif text-[20px] leading-snug">
            {check.correctAnswer}
          </dd>
        </div>
      </dl>

      <div className="flex flex-wrap items-center gap-3">
        <PlayButton label={audio.blocker ? "Hear it" : "Hear it again"} status={sayStatus} onPlay={() => void say(heard)} onStop={stop} disabledReason={audio.blocker} />
        <span lang={language} className="text-ink-2 min-w-0 text-sm">{audio.blocker ? audio.blocker : sayError ?? heard}</span>
      </div>

      {check.explanation ? (
        <p className="lg-reading text-[16px] leading-relaxed">{check.explanation}</p>
      ) : null}
      {check.misconception ? (
        <p className="border-hairline-strong rounded-lg border-l-2 py-1 pl-4 font-serif text-[16px] leading-relaxed">
          <span className="text-ink-3 mr-2 font-mono text-[11px] tracking-[0.14em] uppercase">Watch for</span>
          {check.misconception}
        </p>
      ) : null}
      {confidentWrong ? (
        <Notice>You were {drill.confidence === 4 ? "certain" : "fairly sure"} and it was wrong. Read the reference once more before moving on: this card comes back soon.</Notice>
      ) : null}

      {needsJudgement(drill) ? (
        <div className="border-hairline-strong flex flex-wrap items-center gap-3 rounded-xl border p-4">
          <span className="text-[15px]">Compare yours with the reference. Did you have it?</span>
          <Button variant="outline" onClick={() => onJudge(true)}>
            I had it
          </Button>
          <Button variant="outline" onClick={() => onJudge(false)}>
            I missed it
          </Button>
        </div>
      ) : null}

      <Outcome drill={drill} onRetryReview={onRetryReview} />
    </div>
  )
}

function Outcome({ drill, onRetryReview }: { drill: DrillState; onRetryReview: () => void }) {
  if (drill.phase === "done" && drill.review) {
    return (
      <div className="border-hairline flex flex-wrap items-center gap-x-5 gap-y-2 border-t pt-4 text-sm">
        <span>
          <Eyebrow className="mr-2">Rated</Eyebrow>
          <strong className="font-medium">{RATING_WORD[drill.review.rating] ?? drill.review.rating}</strong>
        </span>
        <span className="text-ink-2">{drill.review.nextDue ? `Next review ${drill.review.nextDue}.` : "No next review date."}</span>
        <span className="text-ink-3">{assistanceNote(drill.hints.length)}</span>
      </div>
    )
  }
  if (drill.pending === "review") {
    return (
      <p className="text-ink-2 flex items-center gap-2 text-sm">
        <Loader2 aria-hidden className={cn("size-4 animate-spin")} /> Scheduling the next review
      </p>
    )
  }
  if (canReview(drill) && drill.error) {
    return (
      <div className="flex items-center gap-3">
        <Button variant="outline" size="sm" onClick={onRetryReview}>
          <RefreshCw aria-hidden /> Retry the review
        </Button>
      </div>
    )
  }
  return null
}
