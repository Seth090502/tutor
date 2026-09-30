// Step 1: guess before reading. Each prequestion is answered in the learner's own words, then rated 1-4 for
// confidence. Nothing is marked here: the answers come after the section has been read.
import { useState } from "react"
import { Check } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow, Panel } from "@/components/kit"

import { CONFIDENCE_LABELS, LongText, NumberKeys, StepError } from "./inputs"
import { nextQuestion, type FlowState } from "./flow"
import { paragraphs, pageLabel } from "./reader"
import { StepShell, rise } from "./StepShell"
import { Working } from "./States"
import type { Chunk, Confidence } from "./types"
import type { Loaded } from "./useReadingSession"

function QuestionForm({
  n,
  of,
  question,
  busy,
  error,
  paper,
  onSubmit,
}: {
  n: number
  of: number
  question: string
  busy: boolean
  error: string | null
  paper: boolean
  onSubmit: (text: string, confidence: Confidence) => void
}) {
  const [text, setText] = useState("")
  const [confidence, setConfidence] = useState<Confidence | null>(null)
  const ready = text.trim() !== "" && confidence !== null && !busy
  const submit = () => {
    if (ready) onSubmit(text.trim(), confidence)
  }
  return (
    <Panel className="rd-rise flex flex-col gap-6" style={rise(1)}>
      <Eyebrow rule>{`${paper ? "Prediction" : "Question"} ${n} of ${of}`}</Eyebrow>
      <LongText
        label={<span className="font-serif text-[20px] leading-snug font-normal text-pretty">{question}</span>}
        hint={paper ? "Commit to a prediction. Being wrong is useful." : "Your best guess, in a sentence or two. A guess is fine."}
        value={text}
        onChange={setText}
        onSubmit={submit}
        disabled={busy}
        rows={3}
      />
      <div className="flex flex-col gap-2">
        <p className="text-[14px] font-medium">
          How sure are you?
        </p>
        <NumberKeys label="How sure are you?" labels={CONFIDENCE_LABELS} value={confidence} onChange={setConfidence} disabled={busy} />
      </div>
      <StepError message={error} />
      <div className="flex flex-wrap items-center gap-4">
        <Button size="lg" disabled={!ready} onClick={submit}>
          {busy ? "Recording" : "Record guess"}
        </Button>
        <p className="text-ink-3 font-mono text-[11px] tracking-[0.04em]">Keys 1 to 4 set your confidence. Ctrl + Enter records.</p>
      </div>
    </Panel>
  )
}

export function PredictStep({
  state,
  paper,
  abstract,
  onAnswer,
  onContinue,
}: {
  state: FlowState
  paper: boolean
  abstract: Loaded<Chunk[] | null>
  onAnswer: (id: string | number, text: string, confidence: Confidence) => void
  onContinue: () => void
}) {
  const questions = state.prep?.prequestions ?? []
  const current = nextQuestion(state)
  const answered = questions.filter((q) => String(q.id) in state.answers)

  return (
    <StepShell
      eyebrow={paper ? "Before the paper" : "Before you read"}
      title={paper ? "Predict from the abstract" : "Guess before you read"}
      lead={
        paper
          ? "Read the abstract, then commit to a prediction for each question. The paper itself stays closed until you have."
          : "You have not seen this section yet. Answer each question from what you already know, then say how sure you are. A wrong guess still helps what you read next."
      }
    >
      {paper ? <Abstract abstract={abstract} /> : null}

      {answered.length > 0 ? (
        <ol className="rd-rise flex flex-col gap-2" aria-label="Recorded guesses">
          {answered.map((q, i) => (
            <li key={String(q.id)} className="border-hairline flex items-start gap-3 rounded-lg border px-4 py-3">
              <Check aria-hidden className="text-ink-2 mt-1 size-4 shrink-0" strokeWidth={2.5} />
              <div className="min-w-0 text-[14px] leading-relaxed">
                <p className="text-ink-2">
                  <span className="text-ink-3 font-mono text-[11px] tracking-[0.04em]">{paper ? "Prediction" : "Question"} {i + 1} </span>
                  {q.q}
                </p>
                <p className="font-serif text-[16px] text-pretty">{state.answers[String(q.id)].answer}</p>
                <p className="text-ink-3 font-mono text-[11px] tracking-[0.04em]">
                  Confidence {state.answers[String(q.id)].confidence} of 4 · {CONFIDENCE_LABELS[state.answers[String(q.id)].confidence - 1]}
                </p>
              </div>
            </li>
          ))}
        </ol>
      ) : null}

      {current ? (
        <QuestionForm
          key={String(current.id)}
          n={questions.indexOf(current) + 1}
          of={questions.length}
          question={current.q}
          busy={state.busy}
          error={state.error}
          paper={paper}
          onSubmit={(text, confidence) => onAnswer(current.id, text, confidence)}
        />
      ) : (
        <Panel className="rd-rise flex flex-col items-start gap-4" style={rise(1)} eyebrow="Recorded">
          <p className="text-[15px] leading-relaxed">
            {paper ? "Your predictions are recorded." : "Your guesses are recorded."} You will see how they did once you have read the {paper ? "paper" : "section"}.
          </p>
          <Button size="lg" onClick={onContinue}>
            {paper ? "Open the paper" : "Start reading"}
          </Button>
        </Panel>
      )}
    </StepShell>
  )
}

function Abstract({ abstract }: { abstract: Loaded<Chunk[] | null> }) {
  if (abstract.phase === "loading") return <Working label="Loading the abstract" />
  if (abstract.phase === "error")
    return (
      <p role="alert" className="text-bad font-mono text-[12px]">
        The abstract could not be loaded: {abstract.message}
      </p>
    )
  const chunks = abstract.value
  if (!chunks || chunks.length === 0)
    return <p className="text-ink-2 rounded-lg border border-dashed px-4 py-3 text-[14px]">This paper has no heading called Abstract. Predict from its title instead.</p>
  const first = chunks[0]
  return (
    <Panel className="rd-rise" style={rise(1)} eyebrow={`Abstract${first.page_start != null ? ` · ${pageLabel(first.page_start, first.page_end)}` : ""}`}>
      <div className="rd-prose">
        {chunks.flatMap((c) => paragraphs(c.text)).map((p, i) => (
          <p key={i}>{p}</p>
        ))}
      </div>
    </Panel>
  )
}
