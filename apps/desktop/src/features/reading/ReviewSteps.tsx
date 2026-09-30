// Steps 5 to 7 and the end: the idea units the summary missed (with their source), candidate atoms the learner
// may accept as cards, the prequestion answers, and the finish. The tutor never writes a summary here.
import { useState } from "react"
import { ArrowRight, Check, CircleHelp, TriangleAlert } from "lucide-react"

import { Button } from "@/components/ui/button"
import { DeltaChip, Panel, Stat, StatRow } from "@/components/kit"
import { cn } from "@/lib/utils"

import type { ReadingApi } from "./api"
import { CONFIDENCE_LABELS, COVERAGE_LABELS, StepError } from "./inputs"
import { highConfidenceMiss, type FlowState, type PreAnswer } from "./flow"
import { pageLabel, paragraphs } from "./reader"
import { calibration, shortText } from "./review"
import { SourceChip } from "./SourceChip"
import { StepShell, rise } from "./StepShell"
import type { Chunk, OutlineEntry } from "./types"
import { useChunk } from "./useReadingSession"

type Source = { api: ReadingApi; docId: string; docTitle: string; chunks: Chunk[] | null }

/** The chip for a chunk id, optionally with the passage itself. The chunk is looked up, so the page is the document's own. */
function ChunkSource({ src, chunkId, passage }: { src: Source; chunkId: string; passage?: boolean }) {
  const chunk = useChunk(src.api, src.docId, chunkId, src.chunks)
  const [open, setOpen] = useState(false)
  const found = chunk.phase === "ready" ? chunk.value : null
  const text = found ? paragraphs(found.text).join("\n\n") : ""
  const { text: short, cut } = shortText(text, 420)

  return (
    <div className="flex flex-col gap-3">
      {passage ? (
        chunk.phase === "loading" ? (
          <p role="status" className="text-ink-3 font-mono text-[11px] tracking-[0.18em] uppercase">
            Loading the passage
          </p>
        ) : found ? (
          <blockquote className="border-hairline-strong border-l-2 pl-4">
            <div className="rd-prose text-[16px] leading-[1.55]">
              {(open ? text : short).split("\n\n").map((p, i) => (
                <p key={i}>{p}</p>
              ))}
            </div>
            {cut ? (
              <button type="button" onClick={() => setOpen(!open)} className="text-ink-2 mt-2 text-[13px] underline underline-offset-[3px]">
                {open ? "Show less" : "Show the whole passage"}
              </button>
            ) : null}
          </blockquote>
        ) : (
          <p role="alert" className="text-ink-2 text-[13px]">
            The passage could not be loaded{chunk.phase === "error" ? `: ${chunk.message}` : "."}
          </p>
        )
      ) : null}
      <SourceChip
        docId={src.docId}
        title={src.docTitle}
        page={found?.page_start ?? null}
        pageEnd={found?.page_end}
        status={found ? "verified" : chunk.phase === "loading" ? "checking" : "unverified"}
        className="self-start"
      />
    </div>
  )
}

export function MissedStep({ state, src, onContinue }: { state: FlowState; src: Source; onContinue: () => void }) {
  const summary = state.summary
  if (!summary) return null
  const { covered, missed } = summary.result
  const total = covered + missed.length
  const cal = calibration(summary.judgement, covered, total)
  const word = summary.judgement - 1
  return (
    <StepShell
      eyebrow="Checked"
      title={missed.length === 0 ? "Your summary covered every idea" : missed.length === 1 ? "One idea to fill in" : `${missed.length} ideas to fill in`}
      lead={
        missed.length === 0
          ? "Nothing is added. Move on to the cards."
          : "Only what your summary missed is shown, each with the original passage it comes from."
      }
    >
      <StatRow className="rd-rise grid-cols-3" style={rise(1)}>
        <Stat size="md" label="Covered" value={covered} unit={`of ${total} ideas`} />
        <Stat size="md" label="Missed" value={missed.length} unit={missed.length === 1 ? "idea" : "ideas"} />
        <Stat
          size="md"
          label="You judged"
          value={summary.judgement}
          unit={`of 4 · ${COVERAGE_LABELS[word].toLowerCase()}`}
          delta={
            cal.verdict === "none"
              ? undefined
              : cal.verdict === "close"
                ? { text: "about right", tone: "ok" }
                : { text: cal.verdict === "over" ? "overestimated" : "underestimated", tone: "warn" }
          }
        />
      </StatRow>

      {missed.length > 0 ? (
        <ol className="flex flex-col gap-4" aria-label="Missed idea units">
          {missed.map((m, i) => (
            <li key={`${m.chunk_id}-${i}`} className="rd-rise" style={rise(2 + Math.min(i, 5))}>
              <Panel className="flex flex-col gap-4">
                <p className="font-serif text-[20px] leading-snug text-pretty">{m.unit}</p>
                <ChunkSource src={src} chunkId={m.chunk_id} passage />
              </Panel>
            </li>
          ))}
        </ol>
      ) : null}

      <div>
        <Button size="lg" onClick={onContinue}>
          Continue
          <ArrowRight />
        </Button>
      </div>
    </StepShell>
  )
}

export function AtomsStep({
  state,
  src,
  onAccept,
  onContinue,
}: {
  state: FlowState
  src: Source
  onAccept: (ids: (string | number)[]) => void
  onContinue: () => void
}) {
  const atoms = state.prep?.atoms ?? []
  const [picked, setPicked] = useState<Set<string>>(new Set())
  const decided = state.atoms
  const toggle = (id: string) =>
    setPicked((p) => {
      const next = new Set(p)
      if (!next.delete(id)) next.add(id)
      return next
    })
  const chosen = atoms.filter((a) => picked.has(String(a.id))).map((a) => a.id)

  return (
    <StepShell
      eyebrow="Candidate cards"
      title={decided ? "Cards decided" : "Keep what is worth remembering"}
      lead={
        decided
          ? `${decided.accepted.length === 0 ? "No cards were added." : `${decided.accepted.length} ${decided.accepted.length === 1 ? "card was" : "cards were"} added to your practice queue.`}`
          : "These are suggestions from the text. Nothing is added unless you accept it, and each card keeps its source."
      }
    >
      <ul className="flex flex-col gap-3" aria-label="Candidate atoms">
        {atoms.map((a, i) => {
          const id = String(a.id)
          const on = decided ? decided.accepted.map(String).includes(id) : picked.has(id)
          return (
            <li key={id} className="rd-rise" style={rise(1 + Math.min(i, 5))}>
              <div className={cn("bg-card flex gap-4 rounded-2xl border p-5 transition-colors", on ? "border-hairline-strong" : "border-hairline")}>
                <input
                  id={`atom-${id}`}
                  type="checkbox"
                  checked={on}
                  disabled={!!decided || state.busy}
                  onChange={() => toggle(id)}
                  className="accent-foreground mt-1.5 size-4 shrink-0"
                />
                <div className="flex min-w-0 flex-1 flex-col gap-3">
                  <label htmlFor={`atom-${id}`} className="flex cursor-pointer flex-col gap-1.5">
                    <span className="font-serif text-[19px] leading-snug text-pretty">{a.front}</span>
                    <span className="text-ink-2 text-[14px] leading-relaxed text-pretty">{a.back}</span>
                  </label>
                  <ChunkSource src={src} chunkId={a.chunk_id} />
                </div>
              </div>
            </li>
          )
        })}
      </ul>
      <StepError message={state.error} />
      {decided ? (
        <div>
          <Button size="lg" onClick={onContinue}>
            Continue
            <ArrowRight />
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-center gap-3">
          <Button size="lg" disabled={chosen.length === 0 || state.busy} onClick={() => onAccept(chosen)}>
            {state.busy ? "Adding" : chosen.length === 0 ? "Choose cards to add" : `Add ${chosen.length} ${chosen.length === 1 ? "card" : "cards"}`}
          </Button>
          <Button variant="ghost" disabled={state.busy} onClick={() => onAccept([])}>
            Add none
          </Button>
        </div>
      )}
    </StepShell>
  )
}

function Result({ a }: { a: PreAnswer }) {
  if (a.correct === true) return <DeltaChip tone="ok" text="Marked correct" />
  if (a.correct === false) return <DeltaChip tone="bad" text="Marked incorrect" />
  return (
    <span className="border-hairline-strong text-foreground inline-flex items-center gap-1.5 rounded-full border px-2 py-[3px] font-mono text-[11px] leading-none tracking-[0.04em]">
      <CircleHelp aria-hidden className="size-3" strokeWidth={2.5} />
      Not auto-checked
    </span>
  )
}

export function ExplanationsStep({ state, paper, onFinish }: { state: FlowState; paper: boolean; onFinish: () => void }) {
  const questions = state.prep?.prequestions ?? []
  return (
    <StepShell
      eyebrow="Now you have read it"
      title={paper ? "How your predictions did" : "How your guesses did"}
      lead="Compare each guess and how sure you were with what the text says. A confident miss is worth going back to."
    >
      <ol className="flex flex-col gap-4" aria-label="Prequestion answers">
        {questions.map((q, i) => {
          const a = state.answers[String(q.id)]
          if (!a) return null
          const miss = highConfidenceMiss(a)
          return (
            <li key={String(q.id)} className="rd-rise" style={rise(1 + Math.min(i, 5))}>
              <Panel className="flex flex-col gap-4" eyebrow={`${paper ? "Prediction" : "Question"} ${i + 1}`}>
                <p className="font-serif text-[20px] leading-snug text-pretty">{q.q}</p>
                <div className="flex flex-wrap items-center gap-2">
                  <Result a={a} />
                  <span className="text-ink-2 font-mono text-[11px] tracking-[0.04em]">
                    Confidence {a.confidence} of 4 · {CONFIDENCE_LABELS[a.confidence - 1]}
                  </span>
                  {miss ? (
                    <span className="border-warn/45 bg-warn/10 text-foreground inline-flex items-center gap-1.5 rounded-full border px-2 py-[3px] font-mono text-[11px] leading-none tracking-[0.04em]">
                      <TriangleAlert aria-hidden className="text-warn size-3" strokeWidth={2.5} />
                      Confident miss
                    </span>
                  ) : null}
                </div>
                <div className="border-hairline-strong border-l-2 pl-4">
                  <p className="text-ink-3 mb-1 font-mono text-[11px] tracking-[0.14em] uppercase">Your guess</p>
                  <p className="text-[15px] leading-relaxed text-pretty">{a.answer}</p>
                </div>
                {a.explanation ? (
                  <div className="rd-prose text-[17px]">
                    <p>{a.explanation}</p>
                  </div>
                ) : (
                  <p className="text-ink-2 text-[14px] leading-relaxed">
                    No explanation came back for this question. Check your guess against the section text.
                  </p>
                )}
                {miss ? <p className="text-[14px] leading-relaxed">You were sure and it was marked wrong. Reread that part of the section before moving on.</p> : null}
              </Panel>
            </li>
          )
        })}
      </ol>
      <div>
        <Button size="lg" onClick={onFinish}>
          Finish
          <Check />
        </Button>
      </div>
    </StepShell>
  )
}

export function DoneView({
  state,
  next,
  onOutline,
  onNext,
}: {
  state: FlowState
  next: OutlineEntry | null
  onOutline: () => void
  onNext: () => void
}) {
  const asked = state.prep?.prequestions.length ?? 0
  const right = Object.values(state.answers).filter((a) => a.correct === true).length
  const s = state.summary
  const total = s ? s.result.covered + s.result.missed.length : 0
  return (
    <StepShell eyebrow="Section complete" title="That section is done" lead="Come back to it after a day or two and try the recall step again from cold.">
      <StatRow className="rd-rise grid-cols-3" style={rise(1)}>
        <Stat size="md" label="Ideas covered" value={s ? s.result.covered : 0} unit={total ? `of ${total}` : undefined} />
        <Stat size="md" label="Guesses right" value={right} unit={`of ${asked}`} />
        <Stat size="md" label="Cards added" value={state.atoms?.accepted.length ?? 0} />
      </StatRow>
      <div className="flex flex-wrap items-center gap-3">
        {next ? (
          <Button size="lg" onClick={onNext}>
            Read the next section
            <ArrowRight />
          </Button>
        ) : null}
        <Button size="lg" variant="outline" onClick={onOutline}>
          Back to the outline
        </Button>
      </div>
      {next ? (
        <p className="text-ink-3 font-mono text-[11px] tracking-[0.04em]">
          Next: {next.heading_path[next.heading_path.length - 1]}
          {next.page_start != null ? ` · ${pageLabel(next.page_start, next.page_end)}` : ""}
        </p>
      ) : null}
    </StepShell>
  )
}
