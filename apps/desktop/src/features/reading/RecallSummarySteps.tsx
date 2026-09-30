// Steps 3 and 4: closed-book recall, then the learner's own summary with a 1-4 judgement. In paper mode the
// recall is an evidence table and the summary is a critique; both travel on the same routes (C11).
import { useState } from "react"
import { Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Panel } from "@/components/kit"

import { blankRow, evidenceText, hasEvidence, type EvidenceRow } from "./evidence"
import { COVERAGE_LABELS, LongText, NumberKeys, StepError } from "./inputs"
import { sectionTitle } from "./reader"
import { StepShell, rise } from "./StepShell"
import type { Confidence, SectionRef } from "./types"

type Busy = { busy: boolean; error: string | null }

export function RecallStep({
  paper,
  section,
  busy,
  error,
  onSubmit,
  onSkip,
}: Busy & { paper: boolean; section: SectionRef; onSubmit: (text: string) => void; onSkip: () => void }) {
  const [text, setText] = useState("")
  const [rows, setRows] = useState<EvidenceRow[]>(() => [blankRow(), blankRow(), blankRow()])
  const title = sectionTitle(section.heading_path)
  const ready = paper ? hasEvidence(rows) : text.trim() !== ""
  const submit = () => {
    if (ready && !busy) onSubmit(paper ? evidenceText(rows) : text.trim())
  }
  const edit = (i: number, patch: Partial<EvidenceRow>) => setRows((rs) => rs.map((r, j) => (j === i ? { ...r, ...patch } : r)))

  return (
    <StepShell
      eyebrow="Book closed"
      title={paper ? "Fill the evidence table from memory" : "Write what you remember"}
      lead={
        paper
          ? "The paper is hidden. For each main claim, note the evidence behind it and where it appeared. Leave a cell empty if you cannot recall it."
          : `The text of “${title}” is hidden. Write everything you can recall, in any order. Do not look it up: pulling it from memory is what makes it last.`
      }
    >
      <Panel className="rd-rise flex flex-col gap-6" style={rise(1)}>
        {paper ? (
          <div className="flex flex-col gap-4">
            <ol className="flex flex-col gap-4" aria-label="Evidence rows">
              {rows.map((r, i) => (
                <li key={i} className="grid gap-3 md:grid-cols-[minmax(0,1.3fr)_minmax(0,1.3fr)_minmax(0,0.6fr)]">
                  <Cell label={`Claim ${i + 1}`} value={r.claim} onChange={(v) => edit(i, { claim: v })} disabled={busy} />
                  <Cell label="Evidence" value={r.evidence} onChange={(v) => edit(i, { evidence: v })} disabled={busy} />
                  <Cell label="Where" value={r.where} onChange={(v) => edit(i, { where: v })} disabled={busy} placeholder="Page" />
                </li>
              ))}
            </ol>
            {rows.length < 12 ? (
              <Button variant="ghost" size="sm" className="self-start" onClick={() => setRows((rs) => [...rs, blankRow()])}>
                <Plus />
                Add a row
              </Button>
            ) : null}
          </div>
        ) : (
          <LongText label="What do you remember?" value={text} onChange={setText} onSubmit={submit} disabled={busy} rows={9} />
        )}
        <StepError message={error} />
        <div className="flex flex-wrap items-center gap-3">
          <Button size="lg" disabled={!ready || busy} onClick={submit}>
            {busy ? "Sending" : paper ? "Submit the table" : "Submit my recall"}
          </Button>
          <Button variant="ghost" disabled={busy} onClick={onSkip}>
            Skip this step
          </Button>
        </div>
      </Panel>
    </StepShell>
  )
}

function Cell({ label, value, onChange, disabled, placeholder }: { label: string; value: string; onChange: (v: string) => void; disabled: boolean; placeholder?: string }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-ink-3 font-mono text-[11px] tracking-[0.14em] uppercase">{label}</span>
      <input
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="border-input bg-card placeholder:text-ink-3 focus-visible:border-ring focus-visible:ring-ring/50 h-10 w-full rounded-lg border px-2.5 font-serif text-[16px] transition-colors outline-none focus-visible:ring-3 disabled:opacity-50"
      />
    </label>
  )
}

export function SummaryStep({
  paper,
  busy,
  error,
  onSubmit,
}: Busy & { paper: boolean; onSubmit: (text: string, judgement: Confidence) => void }) {
  const [text, setText] = useState("")
  const [judgement, setJudgement] = useState<Confidence | null>(null)
  const ready = text.trim() !== "" && judgement !== null && !busy
  const submit = () => {
    if (ready) onSubmit(text.trim(), judgement)
  }
  return (
    <StepShell
      eyebrow={paper ? "Your critique" : "Your summary"}
      title={paper ? "Write your critique" : "Say it in your own words"}
      lead={
        paper
          ? "What do the authors claim, how strong is the evidence, and what would you challenge? Write it without the paper open."
          : "Summarise the section as if explaining it to someone who has not read it. Then judge how much of it you covered before anything is checked."
      }
    >
      <Panel className="rd-rise flex flex-col gap-6" style={rise(1)}>
        <LongText label={paper ? "Your critique" : "Your summary"} value={text} onChange={setText} onSubmit={submit} disabled={busy} rows={8} />
        <div className="flex flex-col gap-2">
          <p className="text-[14px] font-medium">{paper ? "How complete is your critique?" : "How much of the section does your summary cover?"}</p>
          <NumberKeys label="How much does it cover?" labels={COVERAGE_LABELS} value={judgement} onChange={setJudgement} disabled={busy} />
        </div>
        <StepError message={error} />
        <div className="flex flex-wrap items-center gap-4">
          <Button size="lg" disabled={!ready} onClick={submit}>
            {busy ? "Checking" : paper ? "Check my critique" : "Check my summary"}
          </Button>
          <p className="text-ink-3 font-mono text-[11px] tracking-[0.04em]">Keys 1 to 4 set your judgement. Ctrl + Enter sends.</p>
        </div>
      </Panel>
    </StepShell>
  )
}
