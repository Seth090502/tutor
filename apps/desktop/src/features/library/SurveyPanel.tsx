// Latest survey: measured size, duplicate bytes, and the per-GB extrapolation with its projections.
import type { CSSProperties } from "react"

import { DataTable } from "@/components/charts"
import { Eyebrow, Panel } from "@/components/kit"
import { fmtBytes, fmtCompact, fmtInt } from "./format"
import type { Survey } from "./logic"
import "./library.css"

function Fact({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="border-hairline flex flex-col gap-2 border-b py-4 first:pt-0 last:border-b-0 last:pb-0">
      <dt>
        <Eyebrow>{label}</Eyebrow>
      </dt>
      <dd className="m-0 flex flex-wrap items-baseline gap-x-3">
        <span className="text-[28px] leading-none font-extralight tracking-[-0.03em] tabular-nums">{value}</span>
        {hint ? <span className="text-ink-2 font-mono text-[12px]">{hint}</span> : null}
      </dd>
    </div>
  )
}

export function SurveyPanel({
  survey,
  error,
  className,
  style,
}: {
  survey: Survey | null | undefined
  error: string | null
  className?: string
  style?: CSSProperties
}) {
  const ex = survey?.extrapolation
  const rows: (string | number)[][] = ex
    ? [
        ["Per GB", fmtCompact(ex.tokens_est_per_gb), fmtCompact(ex.pages_per_gb)],
        ...(ex.projections ?? []).map((p) => [`${fmtInt(p.corpus_gb)} GB`, fmtCompact(p.tokens_est), fmtCompact(p.pages)]),
      ]
    : []
  const copies = survey?.totals?.duplicate_files

  return (
    <Panel eyebrow="Survey" title="Corpus size and projections" className={className} style={style}>
      {error ? (
        <p className="text-ink-2 text-[13px] leading-relaxed break-words">Could not load the latest survey: {error}</p>
      ) : survey === undefined ? (
        <p className="text-ink-3 font-mono text-[12px]">Reading the latest survey…</p>
      ) : survey === null ? (
        <p className="text-ink-2 max-w-[52ch] text-[13px] leading-relaxed">
          No survey yet. Run Survey to measure the corpus and project how large it could grow.
        </p>
      ) : (
        <div className="flex flex-col gap-6">
          <div className="grid gap-x-10 gap-y-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
            <dl className="m-0 flex flex-col">
              <Fact label="Measured" value={fmtBytes(ex?.measured_bytes)} />
              <Fact
                label="Duplicate bytes"
                value={fmtBytes(survey.totals?.duplicate_bytes)}
                hint={copies !== undefined ? `${fmtInt(copies)} extra ${copies === 1 ? "copy" : "copies"}` : undefined}
              />
            </dl>
            {rows.length > 0 ? (
              <DataTable
                label="Survey projections"
                table={{ caption: "Tokens and pages per GB, and projected totals", columns: ["Corpus size", "Tokens (est.)", "Pages"], rows }}
              />
            ) : (
              <p className="text-ink-2 text-[13px] leading-relaxed">This survey has no extrapolation.</p>
            )}
          </div>
          {survey.root ? (
            <p className="border-hairline text-ink-3 flex min-w-0 gap-3 border-t pt-4 font-mono text-[11px] tracking-[0.04em]">
              <span className="shrink-0 tracking-[0.18em] uppercase">Surveyed</span>
              <span className="min-w-0 truncate" title={survey.root}>
                {survey.root}
              </span>
            </p>
          ) : null}
        </div>
      )}
    </Panel>
  )
}
