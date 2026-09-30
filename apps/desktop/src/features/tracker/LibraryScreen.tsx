// Library: rag-engine status, survey projections and job control (contract C1).
import { Component, type ErrorInfo, type ReactNode } from "react"

import { ChartFrame, StackedBar, stackTable, type StackSegment } from "@/components/charts"
import { BarList, Led, NEUTRAL_BAR, Panel, Stat, StatRow, riseStyle } from "@/components/kit"
import { fmtInt, fmtPct } from "@/features/library/format"
import { JobsPanel } from "@/features/library/JobsPanel"
import { ErrorState, LoadingState, OfflineState } from "@/features/library/LibraryStates"
import { domainColor, sortedRows, type Status } from "@/features/library/logic"
import { SurveyPanel } from "@/features/library/SurveyPanel"
import { useLibrary } from "@/features/library/useLibrary"

// Pages by triage class. The first three take the chart palette in order; an empty page is neutral.
const TRIAGE: { id: string; label: string; color?: string }[] = [
  { id: "born_digital", label: "Born digital" },
  { id: "scanned", label: "Scanned" },
  { id: "math_heavy", label: "Math heavy" },
  { id: "empty", label: "Empty", color: NEUTRAL_BAR },
]

/** Entrance stagger: each block rises 8px and fades, 70 ms after the one before. */
const rise = riseStyle

class ScreenBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }
  static getDerivedStateFromError(error: Error) {
    return { error }
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("library screen failed to render", error, info.componentStack)
  }
  render() {
    const { error } = this.state
    if (error) return <ErrorState message={String(error.message ?? error)} onRetry={() => this.setState({ error: null })} />
    return this.props.children
  }
}

export function LibraryScreen() {
  return (
    <ScreenBoundary>
      <LibraryView />
    </ScreenBoundary>
  )
}

function LibraryView() {
  const { status, fault, survey, surveyError, job, busy, notice, start, reload } = useLibrary()

  if (fault?.offline && !status) return <OfflineState onRetry={reload} />
  if (!status) return fault ? <ErrorState message={fault.message} onRetry={reload} /> : <LoadingState />
  return <Loaded status={status} stale={fault} {...{ survey, surveyError, job, busy, notice, start }} />
}

function Loaded({
  status,
  stale,
  survey,
  surveyError,
  job,
  busy,
  notice,
  start,
}: { status: Status; stale: { offline: boolean; message: string } | null } & Omit<ReturnType<typeof useLibrary>, "status" | "fault" | "reload">) {
  const pagesTotal = status.pages?.total ?? 0
  const triage: StackSegment[] = TRIAGE.map((t) => ({ id: t.id, label: t.label, color: t.color, value: status.pages?.by_class?.[t.id] ?? 0 }))
  const chunks = status.chunks ?? { total: 0, embedded: 0 }
  const { provider, model, dims } = status.embedding ?? { provider: null, model: null, dims: null }
  const spendEmbedding = status.spend_usd?.embedding ?? 0
  const spendOcr = status.spend_usd?.ocr ?? 0
  const spend = spendEmbedding + spendOcr
  const errors = status.errors ?? 0
  const usd = (n: number) => `$${n.toFixed(n > 0 && n < 1 ? 4 : 2)}`

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6 px-8 pt-6 pb-10">
      <div className="kit-rise flex flex-wrap items-center justify-between gap-x-6 gap-y-3">
        <span className="border-hairline bg-card inline-flex max-w-full items-center gap-3 rounded-full border py-1.5 pr-4 pl-3">
          <Led tone={provider ? "ok" : "off"} />
          <span className="text-ink-3 font-mono text-[11px] tracking-[0.18em] uppercase">Embeddings</span>
          <span className="text-foreground min-w-0 truncate font-mono text-[12px]">
            {provider ? `${provider} · ${model ?? "default model"}${dims ? ` · ${dims} dims` : ""}` : "BM25 only"}
          </span>
        </span>
        <p className="text-ink-3 flex min-w-0 max-w-full gap-3 font-mono text-[11px] tracking-[0.04em]">
          <span className="shrink-0 tracking-[0.18em] uppercase">Corpus</span>
          <span className="min-w-0 truncate" title={status.corpus_root}>
            {status.corpus_root}
          </span>
        </p>
      </div>

      {stale ? (
        <p role="status" className="text-ink-2 -mt-2 font-mono text-[12px]">
          {stale.offline ? "rag-engine stopped answering; showing the last reading." : `rag-engine returned an error; showing the last reading. ${stale.message}`}
        </p>
      ) : null}

      <StatRow className="kit-rise grid-cols-3" style={rise(1)}>
        <Stat label="Documents" value={status.documents?.total ?? 0} />
        <Stat label="Pages" value={pagesTotal} />
        <Stat
          label="Chunks embedded"
          value={chunks.embedded}
          caption={chunks.total > 0 ? `of ${fmtInt(chunks.total)} · ${fmtPct(chunks.embedded, chunks.total)}` : "of 0"}
        />
        <Stat
          size="md"
          label="OCR pending"
          value={status.ocr?.pending_pages ?? 0}
          unit={`pages · ${fmtInt(status.ocr?.done_pages)} done`}
        />
        <Stat
          size="md"
          label="Errors"
          value={errors}
          delta={errors > 0 ? { text: "needs a look", tone: "bad" } : { text: "none", tone: "ok" }}
        />
        <Stat size="md" label="Spend" value={spend} decimals={spend > 0 && spend < 1 ? 4 : 2} unit="USD" />
      </StatRow>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,5fr)_minmax(0,6fr)]">
        <JobsPanel className="kit-rise h-full" style={rise(2)} job={job} busy={busy} notice={notice} onStart={(k) => void start(k)} />
        <div className="kit-rise min-w-0 [&>section]:h-full" style={rise(3)}>
          <ChartFrame
            title="Pages by triage class"
            subtitle="Scanned and math-heavy pages wait for OCR."
            table={stackTable(triage, "pages", fmtInt, "Pages by triage class")}
          >
            {pagesTotal > 0 ? (
              <StackedBar segments={triage} name="Pages by triage class" unit="pages" height={28} />
            ) : (
              <p className="text-ink-2 py-6 text-[13px] leading-relaxed">No pages yet. Run Ingest once there are files in the corpus.</p>
            )}
          </ChartFrame>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Panel className="kit-rise" style={rise(4)} eyebrow="Corpus" title="Documents by domain">
          <BarList
            label="Documents by domain"
            rows={sortedRows(status.documents?.by_domain).map((r) => ({ ...r, color: domainColor(r.label) }))}
            capitalize
            empty="No documents yet. Put files in a subject shelf, then run Ingest."
          />
        </Panel>
        <Panel className="kit-rise" style={rise(5)} eyebrow="Corpus" title="Documents by source type">
          <BarList label="Documents by source type" rows={sortedRows(status.documents?.by_source_type)} capitalize empty="No documents yet." />
        </Panel>
      </div>

      <SurveyPanel className="kit-rise" style={rise(6)} survey={survey} error={surveyError} />

      <footer className="kit-rise border-hairline text-ink-3 flex flex-col gap-1.5 border-t pt-4 font-mono text-[11px] tracking-[0.04em]" style={rise(7)}>
        <p className="min-w-0 truncate" title={status.data_dir}>
          Data {status.data_dir}
        </p>
        <p>
          Spend: embedding {usd(spendEmbedding)} · OCR {usd(spendOcr)}
        </p>
      </footer>
    </div>
  )
}
