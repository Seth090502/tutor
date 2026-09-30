// Practice: a typed drill over the learner's language cards, production first, run as a C12 study session.
// The server owns each attempt: open, the learner's answer sealed, then confidence, then the check, then a
// repair or a self-assessment, then closure with the rating and where it came from. This file draws the
// session's accepted snapshot and sends the learner's moves (practice.ts); it never shows answer text that
// did not arrive in `attempt.released`.
import { useCallback, useEffect, useRef, useState } from "react"
import { ArrowRight, BookOpenCheck, Lightbulb, Loader2, RefreshCw, ServerOff } from "lucide-react"

import { Eyebrow, KbdHint, Kbd, KeycapGroup, Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import type { TargetLanguage } from "@/lib/language"
import { languageMeta } from "@/lib/language"
import { StudyError, type ConfidenceLevel, type Snapshot, type StudyAttempt, type StudyQueueItem, type StudySession } from "@/lib/study"
import { errorText, starterDeckCommand } from "./api"
import {
  assistanceNote,
  canHint,
  cardMeta,
  diffChars,
  feedbackLine,
  formatDue,
  hintBlocker,
  hintsGiven,
  latestCheck,
  MAX_RUNG,
  nextEntry,
  outcomeOf,
  ratingLine,
  targetSentence,
  verdictWord,
  type CardKind,
} from "./drill"
import type { AudioEnv } from "./hooks"
import { CopyCommand, FIELD, Notice, PlayButton, StateBlock, Verdict } from "./parts"
import { cardActions, openNext, openPractice, type CardActions } from "./practice"
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

type Load =
  | { status: "loading" }
  | { status: "offline" }
  | { status: "error"; message: string }
  | { status: "empty" }
  | { status: "ready" }

type Tally = { reviewed: number; assisted: number }

const failure = (e: unknown): Load => (e instanceof StudyError && e.code === "network" ? { status: "offline" } : { status: "error", message: errorText(e) })

export function PracticeTab({ language, audio }: { language: TargetLanguage; audio: AudioEnv }) {
  const meta = languageMeta(language)
  const [load, setLoad] = useState<Load>({ status: "loading" })
  const [queue, setQueue] = useState<StudyQueueItem[]>([])
  const [snap, setSnap] = useState<Snapshot | null>(null)
  const [done, setDone] = useState<number[]>([])
  const [finished, setFinished] = useState(false)
  const [tally, setTally] = useState<Tally>({ reviewed: 0, assisted: 0 })
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const { say, stop, status: sayStatus, error: sayError } = useSay(language)
  const session = useRef<StudySession | null>(null)
  const unsubscribe = useRef<(() => void) | null>(null)
  const counted = useRef(new Set<string>())
  const busy = useRef(false)
  const generation = useRef(0)

  /** Counts a closed attempt once: it is done for this visit, and it is reviewed if the server recorded a rating. */
  const account = useCallback((s: Snapshot) => {
    const a = s.attempt
    if (a?.status !== "closed" || counted.current.has(a.attempt_id)) return
    counted.current.add(a.attempt_id)
    setDone((d) => (d.includes(a.entry_id) ? d : [...d, a.entry_id]))
    const d = a.closure?.scheduler_disposition
    if (d?.rating != null) setTally((t) => ({ reviewed: t.reviewed + 1, assisted: t.assisted + (d.provenance === "assisted" ? 1 : 0) }))
  }, [])

  const start = useCallback(async () => {
    const mine = ++generation.current
    unsubscribe.current?.()
    unsubscribe.current = null
    session.current = null
    counted.current = new Set()
    setLoad({ status: "loading" })
    setSnap(null)
    setDone([])
    setFinished(false)
    setTally({ reviewed: 0, assisted: 0 })
    setError(null)
    stop()
    try {
      const opened = await openPractice(language)
      if (mine !== generation.current) return
      setQueue(opened.queue)
      const s = opened.session
      if (!s) return setLoad({ status: "empty" })
      session.current = s
      unsubscribe.current = s.subscribe((next) => {
        setSnap(next)
        account(next)
      })
      account(s.state)
      const live = s.state.attempt !== null && s.state.attempt.status !== "closed"
      if (live) setSnap(s.state)
      setLoad({ status: "ready" })
      if (live) return
      const opening = openNext(s, opened.queue, [])
      if (opening) await opening
      else if (opened.queue.length === 0) setLoad({ status: "empty" })
      else setFinished(true)
    } catch (e) {
      if (mine === generation.current) setLoad(failure(e))
    }
  }, [language, account, stop])

  useEffect(() => {
    void start()
    return () => {
      generation.current++
      unsubscribe.current?.()
      unsubscribe.current = null
    }
  }, [start])

  const attempt = snap?.attempt ?? null
  const kindOf = (a: StudyAttempt): CardKind => {
    const item = queue.find((q) => q.entry_id === a.entry_id)
    return item ? cardMeta(item).kind : "unknown"
  }

  // The reference is spoken once when the server releases it.
  const releasedId = attempt?.released ? attempt.attempt_id : null
  useEffect(() => {
    if (!attempt?.released || audio.blocker) return
    void say(targetSentence(kindOf(attempt), attempt.item.prompt, attempt.released.correct_answer))
  }, [releasedId])

  /** Runs one of the learner's moves: one at a time, with a readable error when the server refuses it. */
  const run = useCallback(async (move: () => Promise<unknown> | null) => {
    if (busy.current) return
    busy.current = true
    setPending(true)
    setError(null)
    try {
      await move()
    } catch (e) {
      setError(errorText(e))
    } finally {
      busy.current = false
      setPending(false)
    }
  }, [])

  const onNext = () =>
    void run(async () => {
      stop()
      const s = session.current
      if (!s) return
      const opening = openNext(s, queue, done)
      if (opening) await opening
      else setFinished(true)
    })

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

  if (finished) {
    return (
      <StateBlock
        icon={BookOpenCheck}
        title="That is every card due right now"
        actions={<RetryButton onClick={() => void start()} label="Check again" />}
      >
        You reviewed {tally.reviewed} {tally.reviewed === 1 ? "card" : "cards"}
        {tally.assisted ? `, ${tally.assisted} with help before answering` : ", none with help before answering"}. New cards come due on the
        schedule, so there is nothing to gain from drilling these again today.
      </StateBlock>
    )
  }

  if (!attempt) {
    return (
      <div role="status" className="text-ink-2 flex items-center justify-center gap-3 py-24 text-sm">
        <Loader2 aria-hidden className="size-4 animate-spin" /> Opening your next card
      </div>
    )
  }

  const remaining = queue.filter((q) => !done.includes(q.entry_id))
  const live = attempt.status !== "closed"
  const position = `Card ${live ? done.length + 1 : done.length} of ${done.length + remaining.length + (live && !remaining.some((q) => q.entry_id === attempt.entry_id) ? 1 : 0)}`

  return (
    <CardView
      key={attempt.attempt_id}
      attempt={attempt}
      kind={kindOf(attempt)}
      language={language}
      position={position}
      tally={tally}
      pending={pending}
      error={error}
      actions={cardActions(session.current!)}
      run={(move) => void run(move)}
      onNext={onNext}
      isLast={!nextEntry(queue, done, attempt)}
      audio={audio}
      say={say}
      stop={stop}
      sayStatus={sayStatus}
      sayError={sayError}
    />
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

export type CardViewProps = {
  attempt: StudyAttempt
  kind: CardKind
  language: TargetLanguage
  position: string
  tally: Tally
  pending: boolean
  error: string | null
  actions: CardActions
  /** Runs a move (one of `actions`) with the pending state and error handling; the tab supplies it. */
  run: (move: () => Promise<unknown> | null) => void
  onNext: () => void
  isLast: boolean
  audio: AudioEnv
  say: (text: string) => Promise<void>
  stop: () => void
  sayStatus: "idle" | "loading" | "playing" | "error"
  sayError: string | null
}

/** One card, drawn from the attempt the server last accepted. */
export function CardView(props: CardViewProps) {
  return (
    <div className="@container">
      <div className="grid items-start gap-5 @3xl:grid-cols-[minmax(0,1fr)_17rem]">
        <CardPane {...props} />
        <aside className="flex flex-col gap-5">
          <ConfidencePanel attempt={props.attempt} pending={props.pending} actions={props.actions} run={props.run} />
          <Ladder attempt={props.attempt} />
        </aside>
      </div>
    </div>
  )
}

/** Confidence comes after the answer is sealed: before that there is nothing to press. */
function ConfidencePanel({ attempt, pending, actions, run }: Pick<CardViewProps, "attempt" | "pending" | "actions" | "run">) {
  const c = attempt.confidence
  return (
    <Panel eyebrow="After you answer" title="How sure are you?" className="lg-enter p-5" style={{ "--i": 1 } as React.CSSProperties}>
      {attempt.status === "open" ? (
        <p className="text-ink-2 text-sm leading-relaxed">You say how sure you are once your answer is submitted, so it cannot change the answer.</p>
      ) : attempt.status === "committed" ? (
        <>
          <KeycapGroup
            aria-label="Confidence, 1 guess to 4 certain"
            choices={CONFIDENCE.map((x) => ({ id: x.id, legend: x.id, label: x.label }))}
            value={null}
            sendOnPress
            onSubmit={(id) => run(() => actions.confidence(Number(id) as ConfidenceLevel))}
            disabled={pending}
          />
          <KbdHint className="mt-4 text-[14px]">
            Press <Kbd>1</Kbd> to <Kbd>4</Kbd>.
          </KbdHint>
          <Button variant="ghost" size="sm" className="mt-3" disabled={pending} onClick={() => run(() => actions.skipConfidence())}>
            Skip
          </Button>
        </>
      ) : (
        <p className="text-sm">
          {c?.level ? `${CONFIDENCE[c.level - 1].label} (${c.level} of 4)` : "Skipped"}
        </p>
      )}
    </Panel>
  )
}

function Ladder({ attempt }: { attempt: StudyAttempt }) {
  const cap = Math.min(MAX_RUNG, attempt.item.rungs_available)
  const used = hintsGiven(attempt).length
  return (
    <Panel eyebrow="Hint ladder" className="lg-enter p-5" style={{ "--i": 2 } as React.CSSProperties}>
      <div className="flex gap-1.5" aria-hidden>
        {Array.from({ length: MAX_RUNG }, (_, i) => (
          <span key={i} className="lg-rung" data-used={i < used ? "" : undefined} style={i >= cap ? { opacity: 0.35 } : undefined} />
        ))}
      </div>
      <p className="text-ink-2 mt-3 text-sm leading-relaxed">
        {attempt.item.rungs_available === 0
          ? "This card has no written hints."
          : `${used} of ${cap} used. Hints open if your answer is missed. Your first answer is already sealed, so they do not change its rating.`}
      </p>
    </Panel>
  )
}

function CardPane({ attempt, kind, language, position, tally, pending, error, actions, run, onNext, isLast, audio, say, stop, sayStatus, sayError }: CardViewProps) {
  const meta = languageMeta(language)
  const copy = KIND_COPY[kind]
  const targetPrompt = kind !== "production"
  const status = attempt.status
  const [draft, setDraft] = useState("")
  const [repair, setRepair] = useState("")
  const inputRef = useRef<HTMLInputElement>(null)
  const repairRef = useRef<HTMLInputElement>(null)
  const checkRef = useRef<HTMLButtonElement>(null)
  const nextRef = useRef<HTMLButtonElement>(null)
  const repairs = attempt.repairs.length

  useEffect(() => {
    if (status === "open") inputRef.current?.focus()
    if (status === "ready") checkRef.current?.focus()
    if (status === "closed") nextRef.current?.focus()
  }, [status])
  useEffect(() => {
    if (status === "remediating") repairRef.current?.focus()
    setRepair("")
  }, [status, repairs])

  const hidePrompt = kind === "listening" && (status === "open" || status === "committed" || status === "ready")
  const first = attempt.first_response
  const hints = hintsGiven(attempt)
  const blocker = hintBlocker(attempt)

  return (
    <Panel
      eyebrow={`${position} · ${copy.label}`}
      title={copy.ask(meta.english)}
      className="lg-enter p-6"
      actions={<span className="text-ink-3 font-mono text-[11px] tracking-[0.08em]">{tally.reviewed} reviewed</span>}
    >
      {hidePrompt ? (
        <div className="flex items-center gap-3 py-2">
          <PlayButton label="Play the sentence" status={sayStatus} onPlay={() => void say(attempt.item.prompt)} onStop={stop} disabledReason={audio.blocker} variant="secondary" size="default" />
          <span className="text-ink-2 text-sm">{audio.blocker ?? "The text stays hidden until you check."}</span>
        </div>
      ) : (
        <p lang={targetPrompt ? language : "en"} className="font-serif text-[26px] leading-snug tracking-[-0.01em] text-balance">
          {attempt.item.prompt}
        </p>
      )}

      {status === "open" ? (
        <div className="mt-5">
          <label htmlFor="lg-answer" className="mb-1.5 block">
            <Eyebrow>Your answer</Eyebrow>
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
            value={draft}
            aria-describedby="lg-answer-help"
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter" || e.nativeEvent.isComposing) return
              e.preventDefault()
              if (!pending) run(() => actions.answer(draft))
            }}
          />
          <p id="lg-answer-help" className="text-ink-3 mt-1.5 text-[13px]">
            Your first answer is sealed when you submit it. Then you say how sure you were, then it is checked.
          </p>
        </div>
      ) : first ? (
        <dl className="mt-5">
          <dt>
            <Eyebrow>Your answer</Eyebrow>
          </dt>
          <dd lang={targetPrompt ? "en" : language} className="mt-1.5 font-serif text-[20px] leading-snug">
            {first.input_mode === "dont_know" ? <span className="text-ink-2">You said you did not know.</span> : first.response}
          </dd>
        </dl>
      ) : null}

      {hints.length > 0 ? (
        <ol className="mt-4 flex flex-col gap-2" aria-label="Hints used">
          {hints.map((h) => (
            <li key={h.rung} className="border-hairline bg-secondary/60 flex gap-3 rounded-lg border px-3.5 py-2.5 text-[15px] leading-relaxed">
              <span className="text-ink-3 mt-0.5 shrink-0 font-mono text-[11px] tracking-[0.14em] uppercase">Hint {h.rung}</span>
              <span lang={language}>{h.text ?? "No written text for this rung. Try once more from what you know."}</span>
            </li>
          ))}
        </ol>
      ) : null}

      {error ? (
        <Notice tone="warn" className="mt-4">
          {error}
        </Notice>
      ) : null}

      {status === "open" ? (
        <div className="mt-5 flex flex-wrap items-center gap-3">
          <Button onClick={() => run(() => actions.answer(draft))} disabled={pending || draft.trim() === ""}>
            {pending ? <Loader2 aria-hidden className="animate-spin" /> : null}
            Submit answer
          </Button>
          <Button variant="outline" onClick={() => run(() => actions.dontKnow())} disabled={pending}>
            I don't know
          </Button>
        </div>
      ) : null}

      {status === "committed" ? <p className="text-ink-2 mt-5 text-sm">Answer sealed. Say how sure you are to go on.</p> : null}

      {status === "ready" ? (
        <div className="mt-5">
          <Button ref={checkRef} onClick={() => run(() => actions.check())} disabled={pending}>
            {pending ? <Loader2 aria-hidden className="animate-spin" /> : null}
            Check
          </Button>
        </div>
      ) : null}

      {status === "remediating" || status === "awaiting_self_assessment" || status === "closed" ? (
        <Feedback attempt={attempt} kind={kind} language={language} audio={audio} say={say} stop={stop} sayStatus={sayStatus} sayError={sayError} />
      ) : null}

      {status === "remediating" ? (
        <div className="mt-5">
          <label htmlFor="lg-repair" className="mb-1.5 block">
            <Eyebrow>Try again</Eyebrow>
          </label>
          <input
            id="lg-repair"
            ref={repairRef}
            type="text"
            lang={targetPrompt ? "en" : language}
            autoComplete="off"
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            className={FIELD}
            placeholder="Correct your answer"
            value={repair}
            onChange={(e) => setRepair(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter" || e.nativeEvent.isComposing) return
              e.preventDefault()
              if (!pending) run(() => actions.repair(repair))
            }}
          />
          <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2">
            <Button onClick={() => run(() => actions.repair(repair))} disabled={pending || repair.trim() === ""}>
              {pending ? <Loader2 aria-hidden className="animate-spin" /> : null}
              Submit repair
            </Button>
            <Button variant="outline" onClick={() => run(() => actions.hint())} disabled={pending || !canHint(attempt)} title={blocker ?? undefined}>
              <Lightbulb aria-hidden />
              Hint
            </Button>
            <Button variant="outline" onClick={() => run(() => actions.giveUp())} disabled={pending}>
              Give up
            </Button>
            <span className="text-ink-3 text-[13px]" aria-live="polite">
              {blocker ?? ""}
            </span>
          </div>
        </div>
      ) : null}

      {status === "awaiting_self_assessment" ? (
        <div className="border-hairline-strong mt-5 flex flex-wrap items-center gap-3 rounded-xl border p-4">
          <span className="text-[15px]">Compare yours with the reference. Did you have it?</span>
          <Button variant="outline" disabled={pending} onClick={() => run(() => actions.selfAssess("correct"))}>
            I got it
          </Button>
          <Button variant="outline" disabled={pending} onClick={() => run(() => actions.selfAssess("incorrect"))}>
            I missed it
          </Button>
        </div>
      ) : null}

      {status === "closed" ? (
        <div className="mt-5">
          <Button ref={nextRef} onClick={onNext} disabled={pending}>
            {isLast ? "Finish" : "Next card"}
            <ArrowRight aria-hidden />
          </Button>
        </div>
      ) : null}
    </Panel>
  )
}

function Feedback({
  attempt,
  kind,
  language,
  audio,
  say,
  stop,
  sayStatus,
  sayError,
}: Pick<CardViewProps, "attempt" | "kind" | "language" | "audio" | "say" | "stop" | "sayStatus" | "sayError">) {
  const check = latestCheck(attempt)
  const released = attempt.released
  const closed = attempt.status === "closed"
  const production = kind === "production"
  const chip = closed
    ? outcomeOf(attempt)
    : { word: check ? verdictWord(check) : "", ok: check ? (check.verdict === "correct" ? true : check.verdict === "incorrect" ? false : null) : null }
  const slip = attempt.checks[0]?.diagnostic === "accent" && attempt.first_response !== null
  const heard = released ? targetSentence(kind, attempt.item.prompt, released.correct_answer) : ""
  const confidentWrong = attempt.checks[0]?.verdict === "incorrect" && (attempt.confidence?.level ?? 0) >= 3
  const rating = closed ? ratingLine(attempt) : null
  const due = formatDue(attempt.closure?.scheduler_disposition.applied?.due_at)

  return (
    <div className="lg-enter mt-6 flex flex-col gap-5" aria-live="polite">
      <div className="flex flex-wrap items-center gap-3">
        <Verdict ok={chip.ok} word={chip.word} />
        {!closed && check ? <span className="text-ink-2 text-[15px]">{feedbackLine(check)}</span> : null}
      </div>

      {attempt.repairs.length > 0 ? (
        <ol className="flex flex-col gap-1.5" aria-label="Your repairs">
          {attempt.repairs.map((r) => (
            <li key={r.response_id} lang={production ? language : "en"} className="font-serif text-[18px] leading-snug">
              {r.response}
            </li>
          ))}
        </ol>
      ) : null}

      {released ? (
        <dl className="grid gap-x-6 gap-y-4 @xl:grid-cols-2">
          {slip && production && attempt.first_response ? (
            <div>
              <dt>
                <Eyebrow>Accents in your first answer</Eyebrow>
              </dt>
              <dd lang={language} className="mt-1.5 font-serif text-[20px] leading-snug">
                {diffChars(attempt.first_response.response, released.correct_answer).map((seg, i) =>
                  seg.differs ? (
                    <span key={i} className="lg-diff">
                      {seg.text}
                    </span>
                  ) : (
                    <span key={i}>{seg.text}</span>
                  ),
                )}
              </dd>
            </div>
          ) : null}
          <div>
            <dt>
              <Eyebrow>Reference</Eyebrow>
            </dt>
            <dd lang={production ? language : "en"} className="mt-1.5 font-serif text-[20px] leading-snug">
              {released.correct_answer}
            </dd>
          </div>
        </dl>
      ) : null}

      {released ? (
        <div className="flex flex-wrap items-center gap-3">
          <PlayButton label={audio.blocker ? "Hear it" : "Hear it again"} status={sayStatus} onPlay={() => void say(heard)} onStop={stop} disabledReason={audio.blocker} />
          <span lang={language} className="text-ink-2 min-w-0 text-sm">{audio.blocker ? audio.blocker : sayError ?? heard}</span>
        </div>
      ) : null}

      {released?.explanation ? <p className="lg-reading text-[16px] leading-relaxed">{released.explanation}</p> : null}
      {released?.misconception ? (
        <p className="border-hairline-strong rounded-lg border-l-2 py-1 pl-4 font-serif text-[16px] leading-relaxed">
          <span className="text-ink-3 mr-2 font-mono text-[11px] tracking-[0.14em] uppercase">Watch for</span>
          {released.misconception}
        </p>
      ) : null}

      {confidentWrong ? (
        <Notice>You were {attempt.confidence?.level === 4 ? "certain" : "fairly sure"} and it was wrong. Confident misses are worth slowing down for: this card comes back soon.</Notice>
      ) : null}

      {attempt.offer && !closed ? <Notice>Two repairs have missed. Give up to see the answer and its explanation.</Notice> : null}

      {rating ? (
        <div className="border-hairline flex flex-wrap items-center gap-x-5 gap-y-2 border-t pt-4 text-sm">
          {rating.rating ? (
            <span>
              <Eyebrow className="mr-2">Rated</Eyebrow>
              <strong className="font-medium">{rating.rating}</strong>
            </span>
          ) : null}
          <span className="text-ink-2">{rating.why}</span>
          {rating.rating ? <span className="text-ink-2">{due ? `Next review ${due}.` : "No next review date."}</span> : null}
          <span className="text-ink-3">{assistanceNote(hintsGiven(attempt).length)}</span>
        </div>
      ) : null}
    </div>
  )
}
