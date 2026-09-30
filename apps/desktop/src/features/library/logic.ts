// Types and pure helpers for the Library screen (contract C1). No browser globals, so they test under node.

export type JobState = "queued" | "running" | "done" | "failed"

export type Job = {
  job_id: string
  kind: "survey" | "ingest"
  state: JobState
  progress: { done: number; total: number }
  started_at: number | null
  finished_at: number | null
  error: string | null
}

export type Status = {
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
export type Survey = {
  root?: string
  totals?: { duplicate_bytes?: number; duplicate_files?: number }
  extrapolation?: {
    measured_bytes?: number
    tokens_est_per_gb: number | null
    pages_per_gb: number | null
    projections?: { corpus_gb: number; tokens_est: number | null; pages: number | null }[]
  }
}

export const isActive = (j: Job | null | undefined): j is Job => j?.state === "queued" || j?.state === "running"

export type Row = { label: string; value: number }

export const sortedRows = (counts: Record<string, number> | undefined): Row[] =>
  Object.entries(counts ?? {})
    .map(([label, value]) => ({ label, value }))
    .sort((a, b) => b.value - a.value || a.label.localeCompare(b.label))

const SUBJECT_ACCENT: Record<string, string> = {
  math: "var(--acc-math)",
  english: "var(--acc-english)",
  science: "var(--acc-science)",
  chemistry: "var(--acc-chemistry)",
  languages: "var(--acc-languages)",
}

/** A subject's accent for its documents; every other shelf (unsorted, unknown) stays neutral. */
export const domainColor = (domain: string): string | undefined => SUBJECT_ACCENT[domain.toLowerCase()]

/** Share of `whole` in [0, 1]; 0 when there is no whole. */
export const ratio = (part: number, whole: number): number => (whole > 0 ? Math.min(1, Math.max(0, part / whole)) : 0)

// ---------- job log ----------

export type LogLine = { id: string; at: number; state: JobState; detail: string }
export type JobLog = { jobId: string; lines: LogLine[] }

const shortId = (id: string) => id.replace(/-/g, "").slice(0, 8)

function detailFor(job: Job): string {
  if (job.state === "failed") return job.error ?? "no error text"
  if (job.state === "done" && job.started_at !== null && job.finished_at !== null) {
    const secs = job.finished_at - job.started_at
    // started_at and finished_at are not pinned to a unit in C1; show elapsed time only when it is plausible seconds.
    if (secs >= 0 && secs < 86_400) return `finished in ${secs < 10 ? secs.toFixed(1) : Math.round(secs)}s`
  }
  return `${job.kind} ${shortId(job.job_id)}`
}

/** Appends a line when the job's state differs from the last logged one; a different job starts a fresh log. */
export function appendLog(log: JobLog | null, job: Job, now: number): JobLog {
  const base = log && log.jobId === job.job_id ? log : { jobId: job.job_id, lines: [] }
  const last = base.lines[base.lines.length - 1]
  if (last && last.state === job.state) return base
  const line: LogLine = { id: `${job.job_id}:${job.state}`, at: now, state: job.state, detail: detailFor(job) }
  return { jobId: base.jobId, lines: [...base.lines, line] }
}
