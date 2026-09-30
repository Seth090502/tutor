// Jobs: Survey and Ingest buttons, progress as a thin rule with a tabular percent, and a typed mono log of
// state changes. Everything animates once when it appears.
import { useEffect, useState, type CSSProperties } from "react"
import { Check, Play, ScanSearch, TriangleAlert, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow, LedStatus, Panel, type LedTone } from "@/components/kit"
import { fmtInt } from "./format"
import { appendLog, isActive, ratio, type Job, type JobLog, type JobState, type LogLine } from "./logic"
import "./library.css"

const TONE: Record<JobState, LedTone> = { queued: "warn", running: "ok", done: "ok", failed: "bad" }

const NO_TOTAL: Record<JobState, string> = { queued: "waiting to start", running: "progress not reported", done: "complete", failed: "stopped" }

const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour12: false })

function LogRow({ line, latest, active }: { line: LogLine; latest: boolean; active: boolean }) {
  const text = `${clock(line.at)}  ${line.state.padEnd(7)}  ${line.detail}`
  const ch = Math.min(text.length, 64)
  return (
    <li className="lib-type grid grid-cols-[8ch_8ch_minmax(0,1fr)_auto] items-start gap-x-3" style={{ "--ch": ch } as CSSProperties}>
      <span className="text-ink-3">{clock(line.at)}</span>
      <span className={line.state === "failed" ? "text-bad" : "text-foreground"}>{line.state}</span>
      <span className="text-ink-2 min-w-0 break-words">
        {line.detail}
        {latest && active ? (
          <span aria-hidden className="text-world ml-1.5">
            ▍
          </span>
        ) : null}
      </span>
      {line.state === "done" ? (
        <span
          aria-hidden
          className="lib-stamp border-ok/60 bg-ok/10 text-ok mt-px grid size-[18px] place-items-center rounded-full border"
        >
          <Check className="size-3" strokeWidth={3} />
        </span>
      ) : null}
      {line.state === "failed" ? (
        <span aria-hidden className="border-bad/60 bg-bad/10 text-bad mt-px grid size-[18px] place-items-center rounded-full border">
          <X className="size-3" strokeWidth={3} />
        </span>
      ) : null}
    </li>
  )
}

function Progress({ job }: { job: Job }) {
  const { done, total } = job.progress ?? { done: 0, total: 0 }
  const known = total > 0 || job.state === "done"
  const r = job.state === "done" ? 1 : ratio(done, total)
  const percent = Math.round(r * 100)
  return (
    <div>
      <div className="flex items-end justify-between gap-4">
        <span className="leading-none tracking-[-0.04em] tabular-nums" aria-hidden>
          <span className="text-[44px] font-extralight">{known ? percent : "—"}</span>
          {known ? <span className="text-ink-3 ml-1 font-mono text-[13px] tracking-normal">%</span> : null}
        </span>
        <span className="text-ink-2 font-mono text-[12px] tabular-nums">
          {total > 0 ? `${fmtInt(done)} / ${fmtInt(total)}` : NO_TOTAL[job.state]}
        </span>
      </div>
      <div
        role="progressbar"
        aria-label={`${job.kind} progress`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={known ? percent : undefined}
        aria-valuetext={known ? `${percent} percent` : NO_TOTAL[job.state]}
        className="bg-hairline-strong relative mt-3.5 h-[2px] overflow-hidden rounded-full"
      >
        <div
          className={job.state === "failed" ? "bg-bad absolute inset-0 origin-left" : "bg-world absolute inset-0 origin-left"}
          style={{ transform: `scaleX(${r})`, transition: "transform 500ms cubic-bezier(.2,.8,.2,1)" }}
        />
      </div>
    </div>
  )
}

export function JobsPanel({
  job,
  busy,
  notice,
  onStart,
  className,
  style,
}: {
  job: Job | null
  busy: boolean
  notice: string | null
  onStart: (kind: "survey" | "ingest") => void
  className?: string
  style?: CSSProperties
}) {
  const [log, setLog] = useState<JobLog | null>(null)
  const jobId = job?.job_id
  const state = job?.state
  const error = job?.error
  useEffect(() => {
    if (job) setLog((prev) => appendLog(prev, job, Date.now()))
    // A new log line is wanted on a state change only; progress ticks must not re-run this.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [jobId, state, error])

  const active = isActive(job)
  return (
    <Panel
      eyebrow="Jobs"
      title="Survey and ingest"
      className={className}
      style={style}
      actions={
        <>
          <Button variant="outline" size="sm" onClick={() => onStart("survey")} disabled={busy}>
            <ScanSearch />
            Survey
          </Button>
          <Button size="sm" onClick={() => onStart("ingest")} disabled={busy}>
            <Play />
            Ingest
          </Button>
        </>
      }
    >
      {job ? (
        <div className="flex flex-col gap-5">
          <div className="flex items-center justify-between gap-3">
            <LedStatus tone={TONE[job.state]} state={job.state} />
            <Eyebrow>{job.kind}</Eyebrow>
          </div>
          <Progress job={job} />
          {log && log.jobId === job.job_id ? (
            <ol
              role="log"
              aria-label={`${job.kind} log`}
              className="border-hairline m-0 flex list-none flex-col gap-1.5 border-t p-0 pt-4 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap"
            >
              {log.lines.map((line, i) => (
                <LogRow key={line.id} line={line} latest={i === log.lines.length - 1} active={active} />
              ))}
            </ol>
          ) : null}
        </div>
      ) : (
        <div className="flex flex-col gap-5">
          <LedStatus tone="off" state="Idle" />
          <dl className="border-hairline m-0 flex flex-col border-t">
            {(
              [
                ["Survey", "Measures the corpus and projects how large it could grow."],
                ["Ingest", "Reads new files into the index."],
              ] as const
            ).map(([term, text]) => (
              <div key={term} className="border-hairline flex flex-col gap-1.5 border-b py-3.5 last:border-b-0 last:pb-0">
                <dt>
                  <Eyebrow>{term}</Eyebrow>
                </dt>
                <dd className="text-ink-2 m-0 text-[13px] leading-snug">{text}</dd>
              </div>
            ))}
          </dl>
        </div>
      )}
      {notice ? (
        <p role="status" className="text-ink-2 mt-4 flex items-start gap-2 text-[13px] leading-snug">
          <TriangleAlert aria-hidden className="text-warn mt-0.5 size-4 shrink-0" />
          <span className="min-w-0 break-words">{notice}</span>
        </p>
      ) : null}
    </Panel>
  )
}
