// Guided Reading for one section (spec: docs/tutor/CORPUS.md, Stage 1): the frame, the stepper and the step
// that the flow state machine is on. Paper mode is the same flow with three steps reworded.
import { useLayoutEffect, useRef } from "react"
import { ArrowLeft, Check } from "lucide-react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

import type { ReadingApi } from "./api"
import { STEP_LABEL, visibleSteps, type FlowState, type Step } from "./flow"
import { PredictStep } from "./PredictStep"
import { Reader, scrollParent } from "./ReaderPane"
import { sectionTitle } from "./reader"
import { RecallStep, SummaryStep } from "./RecallSummarySteps"
import { AtomsStep, DoneView, ExplanationsStep, MissedStep } from "./ReviewSteps"
import { FlowFailed, Working } from "./States"
import type { Chunk, DocDetail, OutlineEntry } from "./types"
import { useAbstract, useGuidedReading, useSectionText } from "./useReadingSession"
import "./reading.css"

function Stepper({ state }: { state: FlowState }) {
  const steps = visibleSteps(state)
  const at = state.phase === "done" ? steps.length : state.phase === "step" ? steps.indexOf(state.step) : -1
  return (
    <ol className="flex flex-wrap items-center gap-x-3.5 gap-y-2" aria-label="Steps">
      {steps.map((step: Step, i) => {
        const done = i < at
        const current = i === at
        return (
          <li
            key={step}
            aria-current={current ? "step" : undefined}
            className={cn(
              "flex items-center gap-1.5 border-b-2 pb-1.5 font-mono text-[11px] tracking-[0.08em] uppercase",
              current ? "border-world text-foreground" : done ? "text-ink-2 border-transparent" : "text-ink-3 border-transparent",
            )}
          >
            <span
              aria-hidden
              className={cn("grid size-5 place-items-center rounded-full border text-[11px] tabular-nums", current ? "border-world bg-world-soft" : "border-hairline-strong")}
            >
              {done ? <Check className="size-3" strokeWidth={2.5} /> : i + 1}
            </span>
            {STEP_LABEL[state.mode][step]}
            {done ? <span className="sr-only"> (done)</span> : null}
          </li>
        )
      })}
    </ol>
  )
}

export function ReadingFlow({
  api,
  doc,
  entry,
  next,
  onOutline,
  onNext,
}: {
  api: ReadingApi
  doc: DocDetail
  entry: OutlineEntry
  next: OutlineEntry | null
  onOutline: () => void
  onNext: () => void
}) {
  const g = useGuidedReading(api, doc, entry)
  const { state } = g
  const paper = state.mode === "paper"
  const text = useSectionText(api, doc.doc_id, state.section)
  const abstract = useAbstract(api, doc, paper)
  const root = useRef<HTMLDivElement>(null)
  const chunks: Chunk[] | null = text.phase === "ready" ? text.value : null
  const src = { api, docId: doc.doc_id, docTitle: doc.title, chunks }

  // Each step starts at the top of the pane.
  useLayoutEffect(() => {
    if (root.current) scrollParent(root.current)?.scrollTo({ top: 0 })
  }, [state.step, state.phase])

  const title = state.section ? sectionTitle(state.section.heading_path) : sectionTitle(entry.heading_path)

  return (
    <div ref={root} className="mx-auto flex max-w-3xl flex-col gap-8 px-8 pt-6 pb-12">
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <Button variant="ghost" size="sm" onClick={onOutline} className="-ml-2">
            <ArrowLeft />
            Outline
          </Button>
          <p className="text-ink-3 min-w-0 truncate font-mono text-[11px] tracking-[0.04em]">
            {doc.title} <span aria-hidden>·</span> {title}
          </p>
        </div>
        <Stepper state={state} />
      </div>

      {state.phase === "opening" ? <Working label="Opening the section" /> : null}
      {state.phase === "preparing" ? (
        <Working label="Preparing this section" detail="The section is sent to the lesson model once, to draft questions, idea units and candidate cards." />
      ) : null}
      {state.phase === "failed" && state.failure ? <FlowFailed failure={state.failure} onRetry={g.retry} onBack={onOutline} /> : null}
      {state.phase === "done" ? <DoneView state={state} next={next} onOutline={onOutline} onNext={onNext} /> : null}

      {state.phase === "step" && state.step === "prequestions" ? (
        <PredictStep state={state} paper={paper} abstract={abstract} onAnswer={g.answer} onContinue={g.advance} />
      ) : null}
      {state.phase === "step" && state.step === "reading" && state.section ? (
        <Reader docId={doc.doc_id} docTitle={doc.title} section={state.section} text={text} onRetry={text.retry} onDone={g.advance} />
      ) : null}
      {state.phase === "step" && state.step === "recall" && state.section ? (
        <RecallStep paper={paper} section={state.section} busy={state.busy} error={state.error} onSubmit={g.recall} onSkip={g.skipRecall} />
      ) : null}
      {state.phase === "step" && state.step === "summary" ? <SummaryStep paper={paper} busy={state.busy} error={state.error} onSubmit={g.summarize} /> : null}
      {state.phase === "step" && state.step === "missed" ? <MissedStep state={state} src={src} onContinue={g.advance} /> : null}
      {state.phase === "step" && state.step === "atoms" ? <AtomsStep state={state} src={src} onAccept={g.acceptAtoms} onContinue={g.advance} /> : null}
      {state.phase === "step" && state.step === "explanations" ? <ExplanationsStep state={state} paper={paper} onFinish={g.advance} /> : null}
    </div>
  )
}
