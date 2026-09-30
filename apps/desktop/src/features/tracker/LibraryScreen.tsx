// Library: rag-engine status, survey projections and job control (contract C1).
import { useCallback, useEffect, useState } from "react"
import { Play, ScanSearch, ServerOff } from "lucide-react"

import { Button } from "@/components/ui/button"
import { HttpError, RAG_ENGINE_URL, fetchJson } from "@/lib/backend"
import {
  BarList,
  LegendDot,
  Notice,
  Page,
  Panel,
  ScreenBoundary,
  StackedBar,
  Stat,
  StateMessage,
  errorText,
  fmtBytes,
  fmtCompact,
  fmtInt,
  fmtPct,
  fmtUsd,
  type Segment,
} from "./ui"

type Job = {
  job_id: string
  kind: "survey" | "ingest"
  state: "queued" | "running" | "done" | "failed"
  progress: { done: number; total: number }
  started_at: number | null
  finished_at: number | null
  error: string | null
}

type Status = {
  corpus_root: string
  data_dir: string
  documents: { total: number; by_domain: Record<string, number>; by_source_type: Record<string, number> }
  pages: { total: number; by_class: Record<string, number> }
  chunks: { total: number; embedded: number }
  ocr: { pending_pages: number; done_pages: number }
  errors: number
  embedding: { provider: string | null; model: string | null; dims: number | null }
  active_job: Job | null
  spend_usd: { embedding: number; ocr: number }
}

// GET /survey/latest returns the `rag-engine survey` report (crates/rag-engine/src/report.rs).
type Survey = {
  root?: string
  totals?: { duplicate_bytes?: number; duplicate_files?: number }
  extrapolation?: {
    measured_bytes?: number
    tokens_est_per_gb: number | null
    pages_per_gb: number | null
    projections?: { corpus_gb: number; tokens_est: number | null; pages: number | null }[]
  }
}

const POLL_STATUS_MS = 5000
const POLL_JOB_MS = 1000

const isActive = (j: Job | null | undefined): j is Job => j?.state === "queued" || j?.state === "running"

const TRIAGE: { key: string; label: string; color: string }[] = [
  { key: "born_digital", label: "Born digital", color: "var(--viz-1)" },
  { key: "scanned", label: "Scanned", color: "var(--viz-2)" },
  { key: "math_heavy", label: "Math heavy", color: "var(--viz-3)" },
  { key: "empty", label: "Empty", color: "var(--viz-neutral)" },
]

const sortedRows = (counts: Record<string, number> | undefined) =>
  Object.entries(counts ?? {})
    .map(([label, value]) => ({ label, value }))
    .sort((a, b) => b.value - a.value || a.label.localeCompare(b.label))

export function LibraryScreen() {
  return (
    <ScreenBoundary>
      <LibraryView />
    </ScreenBoundary>
  )
}

function LibraryView() {
  const [status, setStatus] = useState<Status | null>(null)
  const [fault, setFault] = useState<{ offline: boolean; message: string } | null>(null)
  const [survey, setSurvey] = useState<Survey | null | undefined>(undefined) // null: none yet
  const [surveyError, setSurveyError] = useState<string | null>(null)
  const [job, setJob] = useState<Job | null>(null) // the job this window started
  const [starting, setStarting] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)

  const loadStatus = useCallback(async () => {
    try {
      setStatus(await fetchJson<Status>(RAG_ENGINE_URL, "/status"))
      setFault(null)
    } catch (e) {
      setFault({ offline: !(e instanceof HttpError), message: errorText(e) })
    }
  }, [])

  const loadSurvey = useCallback(async () => {
    try {
      setSurvey(await fetchJson<Survey>(RAG_ENGINE_URL, "/survey/latest"))
      setSurveyError(null)
    } catch (e) {
      if (e instanceof HttpError && e.status === 404) {
        setSurvey(null)
        setSurveyError(null)
      } else {
        setSurveyError(errorText(e))
      }
    }
  }, [])

  useEffect(() => {
    void loadStatus()
    void loadSurvey()
    const timer = setInterval(() => void loadStatus(), POLL_STATUS_MS)
    return () => clearInterval(timer)
  }, [loadStatus, loadSurvey])

  const jobId = job?.job_id
  const polling = isActive(job)
  useEffect(() => {
    if (!jobId || !polling) return
    let alive = true
    const timer = setInterval(async () => {
      try {
        const next = await fetchJson<Job>(RAG_ENGINE_URL, `/jobs/${jobId}`)
        if (!alive) return
        setJob(next)
        setNotice(null)
      } catch (e) {
        if (alive) setNotice(`Could not read job ${jobId}: ${errorText(e)}`)
      }
    }, POLL_JOB_MS)
    return () => {
      alive = false
      clearInterval(timer)
    }
  }, [jobId, polling])

  const finishedKind = job?.state === "done" ? job.kind : null
  useEffect(() => {
    if (!finishedKind) return
    void loadStatus()
    if (finishedKind === "survey") void loadSurvey()
  }, [finishedKind, loadStatus, loadSurvey])

  async function start(kind: "survey" | "ingest") {
    setStarting(true)
    setNotice(null)
    try {
      const { job_id } = await fetchJson<{ job_id: string }>(RAG_ENGINE_URL, `/${kind}`, {
        method: "POST",
        body: JSON.stringify(kind === "ingest" ? { rebuild: false } : {}),
      })
      setJob({ job_id, kind, state: "queued", progress: { done: 0, total: 0 }, started_at: null, finished_at: null, error: null })
    } catch (e) {
      setNotice(e instanceof HttpError && e.status === 409 ? `A job is already running: ${errorText(e)}` : errorText(e))
    } finally {
      setStarting(false)
    }
  }

  if (fault?.offline) return <StateMessage icon={ServerOff} title="rag-engine is not running; start it from Settings" />
  if (!status) {
    return fault ? (
      <StateMessage icon={ServerOff} title="rag-engine returned an error" detail={fault.message} />
    ) : (
      <StateMessage title="Loading…" />
    )
  }

  const remote = status.active_job
  const shown = job && (!remote || remote.job_id === job.job_id) ? job : (remote ?? job)
  const busy = starting || isActive(job)

  const triage: Segment[] = TRIAGE.map((t) => ({ ...t, value: status.pages?.by_class?.[t.key] ?? 0 }))
  const pagesTotal = status.pages?.total ?? 0
  const { provider, model, dims } = status.embedding ?? { provider: null, model: null, dims: null }
  const spend = (status.spend_usd?.embedding ?? 0) + (status.spend_usd?.ocr ?? 0)

  return (
    <Page>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <span className="bg-muted inline-block rounded-full px-2.5 py-1 text-xs font-medium">
            {provider ? `${provider} · ${model ?? "default model"}${dims ? ` · ${dims} dims` : ""}` : "BM25 only"}
          </span>
          <p className="text-muted-foreground mt-2 truncate text-xs">{status.corpus_root}</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => void start("survey")} disabled={busy}>
            <ScanSearch />
            Survey
          </Button>
          <Button onClick={() => void start("ingest")} disabled={busy}>
            <Play />
            Ingest
          </Button>
        </div>
      </div>

      {fault && <Notice>rag-engine returned an error: {fault.message}</Notice>}
      {notice && <Notice>{notice}</Notice>}
      {shown && <JobPanel job={shown} />}

      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-6">
        <Stat label="Documents" value={fmtInt(status.documents?.total)} />
        <Stat label="Pages" value={fmtInt(pagesTotal)} />
        <Stat
          label="Chunks embedded"
          value={fmtPct(status.chunks?.embedded ?? 0, status.chunks?.total ?? 0)}
          hint={`${fmtInt(status.chunks?.embedded)} of ${fmtInt(status.chunks?.total)}`}
        />
        <Stat label="OCR pending" value={fmtInt(status.ocr?.pending_pages)} hint={`${fmtInt(status.ocr?.done_pages)} pages done`} />
        <Stat label="Errors" value={fmtInt(status.errors)} />
        <Stat
          label="Spend"
          value={fmtUsd(spend)}
          hint={`embedding ${fmtUsd(status.spend_usd?.embedding)} · OCR ${fmtUsd(status.spend_usd?.ocr)}`}
        />
      </div>

      <Panel title="Pages by triage class">
        <StackedBar segments={triage} label="Pages by triage class" height={18} />
        <ul className="mt-4 grid gap-2 sm:grid-cols-2">
          {triage.map((t) => (
            <LegendDot key={t.key} color={t.color} label={t.label} value={pagesTotal > 0 ? `${fmtInt(t.value)} · ${fmtPct(t.value, pagesTotal)}` : fmtInt(t.value)} />
          ))}
        </ul>
      </Panel>

      <div className="grid gap-6 md:grid-cols-2">
        <Panel title="Documents by domain">
          <BarList rows={sortedRows(status.documents?.by_domain)} color="var(--viz-1)" />
        </Panel>
        <Panel title="Documents by source type">
          <BarList rows={sortedRows(status.documents?.by_source_type)} color="var(--viz-2)" />
        </Panel>
      </div>

      <SurveyPanel survey={survey} error={surveyError} />

      <p className="text-muted-foreground text-xs">Data: {status.data_dir}</p>
    </Page>
  )
}

function JobPanel({ job }: { job: Job }) {
  const { done, total } = job.progress ?? { done: 0, total: 0 }
  const active = isActive(job)
  const known = total > 0
  const ratio = job.state === "done" ? 1 : known ? Math.min(1, done / total) : 0
  return (
    <Panel>
      <div className="mb-2 flex items-baseline justify-between gap-3 text-sm">
        <span className="font-medium">
          {job.kind === "survey" ? "Survey" : "Ingest"} <span className="text-muted-foreground font-normal">· {job.state}</span>
        </span>
        <span className="text-muted-foreground tabular-nums">
          {known ? `${fmtInt(done)} / ${fmtInt(total)} (${fmtPct(done, total)})` : active ? "starting…" : ""}
        </span>
      </div>
      <div
        role="progressbar"
        aria-label={`${job.kind} progress`}
        aria-valuemin={0}
        aria-valuemax={known ? total : undefined}
        aria-valuenow={known ? done : undefined}
        className="bg-muted h-2 overflow-hidden rounded-full"
      >
        <div
          className={`h-full rounded-full ${job.state === "failed" ? "bg-destructive" : "bg-primary"} ${active && !known ? "w-1/3 animate-pulse" : ""}`}
          style={active && !known ? undefined : { width: `${ratio * 100}%` }}
        />
      </div>
      {job.error && <p className="text-destructive mt-2 text-sm break-words">{job.error}</p>}
    </Panel>
  )
}

function SurveyPanel({ survey, error }: { survey: Survey | null | undefined; error: string | null }) {
  const ex = survey?.extrapolation
  return (
    <Panel title="Latest survey">
      {error ? (
        <p className="text-destructive text-sm break-words">Could not load the survey: {error}</p>
      ) : survey === undefined ? (
        <p className="text-muted-foreground text-sm">Loading…</p>
      ) : survey === null ? (
        <p className="text-muted-foreground text-sm">No survey yet. Run Survey to measure the corpus.</p>
      ) : (
        <div className="space-y-5">
          <div className="grid grid-cols-2 gap-x-6 gap-y-4 md:grid-cols-4">
            <Fact label="Tokens per GB" value={fmtCompact(ex?.tokens_est_per_gb)} />
            <Fact label="Pages per GB" value={fmtCompact(ex?.pages_per_gb)} />
            <Fact label="Measured" value={fmtBytes(ex?.measured_bytes)} />
            <Fact
              label="Duplicate bytes"
              value={fmtBytes(survey.totals?.duplicate_bytes)}
              hint={survey.totals?.duplicate_files !== undefined ? `${fmtInt(survey.totals.duplicate_files)} extra copies` : undefined}
            />
          </div>
          {ex?.projections && ex.projections.length > 0 && (
            <table className="w-full text-sm">
              <thead>
                <tr className="text-muted-foreground border-b text-left text-xs">
                  <th className="py-2 font-medium">Projected corpus</th>
                  <th className="py-2 text-right font-medium">Tokens (est.)</th>
                  <th className="py-2 text-right font-medium">Pages</th>
                </tr>
              </thead>
              <tbody>
                {ex.projections.map((p) => (
                  <tr key={p.corpus_gb} className="border-b last:border-0">
                    <td className="py-2">{fmtInt(p.corpus_gb)} GB</td>
                    <td className="py-2 text-right tabular-nums">{fmtCompact(p.tokens_est)}</td>
                    <td className="py-2 text-right tabular-nums">{fmtCompact(p.pages)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {survey.root && <p className="text-muted-foreground truncate text-xs">Surveyed: {survey.root}</p>}
        </div>
      )}
    </Panel>
  )
}

function Fact({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div>
      <div className="text-muted-foreground text-xs">{label}</div>
      <div className="mt-0.5 text-lg font-semibold tabular-nums">{value}</div>
      {hint && <div className="text-muted-foreground text-xs">{hint}</div>}
    </div>
  )
}
