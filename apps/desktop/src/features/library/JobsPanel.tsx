// Jobs: Survey and Ingest buttons, progress as a thin rule with a tabular percent, and a typed mono log of
// state changes. Everything animates once when it appears.
import { useEffect, useState, type CSSProperties } from "react"
import { Play, ScanSearch, TriangleAlert } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow, LedStatus, LogLine, LogList, Panel, type LedTone } from "@/components/kit"
import { fmtInt } from "./format"
import { appendLog, isActive, ratio, type Job, type JobLog, type JobState, type LogLine as LogEntry } from "./logic"

const TONE: Record<JobState, LedTone> = { queued: "warn", running: "ok", done: "ok", failed: "bad" }

const NO_TOTAL: Record<JobState, string> = { queued: "waiting to start", running: "progress not reported", done: "complete", failed: "stopped" }

const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour12: false })

function LogRow({ line, latest, active }: { line: LogEntry; latest: boolean; active: boolean }) {
  return (
    <LogLine
      time={clock(line.at)}
      label={line.state}
      detail={line.detail}
      tone={line.state === "failed" ? "bad" : "default"}
      mark={line.state === "done" ? "check" : line.state === "failed" ? "cross" : undefined}
      caret={latest && active}
    />
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
            <LogList label={`${job.kind} log`}>
              {log.lines.map((line, i) => (
                <LogRow key={line.id} line={line} latest={i === log.lines.length - 1} active={active} />
              ))}
            </LogList>
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
