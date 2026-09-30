// The document browser: search, subject and type chips with counts, sort, and the list (GET /documents, C9).
import { useState, type CSSProperties } from "react"
import { Check, ChevronRight, Search } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow } from "@/components/kit"
import { domainColor } from "@/features/library/logic"
import { cn } from "@/lib/utils"

import type { ReadingApi } from "./api"
import {
  DEFAULT_FILTERS,
  KNOWN_DOMAINS,
  KNOWN_SOURCE_TYPES,
  SORTS,
  chipOptions,
  hasFilters,
  relativeAdded,
  shelfLabel,
  sizeLabel,
  type Filters,
} from "./docs"
import { EmptyLibrary, EngineError, EngineOffline, NoMatches, Working } from "./States"
import type { Doc, SortKey } from "./types"
import { useDocuments } from "./useDocuments"
import "./reading.css"

const TYPE_WORD: Record<string, string> = { book: "Book", paper: "Paper", note: "Note", unknown: "Unknown type" }
const TYPE_PLURAL: Record<string, string> = { book: "Books", paper: "Papers", note: "Notes", unknown: "Unknown" }
const rise = (i: number) => ({ "--i": i }) as CSSProperties

export function ShelfTag({ domain }: { domain: string }) {
  const color = domainColor(domain)
  return (
    <span className="text-ink-2 inline-flex items-center gap-1.5">
      <span aria-hidden className="size-2 rounded-full border" style={{ background: color ?? "transparent", borderColor: color ?? "var(--hairline-strong)" }} />
      {shelfLabel(domain)}
    </span>
  )
}

function Chip({ pressed, onClick, children, count }: { pressed: boolean; onClick: () => void; children: string; count?: number | null }) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={cn(
        "focus-visible:ring-ring/50 inline-flex h-8 items-center gap-1.5 rounded-full border px-3 text-[13px] transition-colors outline-none focus-visible:ring-3",
        pressed ? "border-world bg-world-soft text-foreground font-medium" : "border-hairline-strong text-ink-2 hover:bg-muted",
      )}
    >
      {pressed ? <Check aria-hidden className="size-3.5" strokeWidth={2.5} /> : null}
      {children}
      {count != null ? <span className="text-ink-3 font-mono text-[11px] tabular-nums">{count}</span> : null}
    </button>
  )
}

export function DocumentBrowser({ api, onOpen }: { api: ReadingApi; onOpen: (doc: Doc) => void }) {
  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS)
  const d = useDocuments(api, filters)
  const set = (patch: Partial<Filters>) => setFilters((f) => ({ ...f, ...patch }))

  if (d.phase === "error" && d.fault) {
    return d.fault.offline ? (
      <EngineOffline onRetry={d.reload} />
    ) : (
      <EngineError title="rag-engine answered with an error" message={d.fault.message} onRetry={d.reload} />
    )
  }
  if (d.phase === "loading") return <Working label="Reading the library" />
  if (d.total === 0 && !hasFilters(filters)) return <EmptyLibrary />

  const domains = chipOptions(KNOWN_DOMAINS, d.counts?.by_domain)
  const types = chipOptions(KNOWN_SOURCE_TYPES, d.counts?.by_source_type)
  const all = d.counts?.total ?? null

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6 px-8 pt-6 pb-10">
      <div className="rd-rise flex flex-wrap items-center gap-3">
        <label className="border-input bg-card focus-within:border-ring focus-within:ring-ring/50 relative flex h-10 min-w-[16rem] flex-1 items-center gap-2.5 rounded-lg border px-3 focus-within:ring-3">
          <Search aria-hidden className="text-ink-3 size-4 shrink-0" />
          <span className="sr-only">Search titles and paths</span>
          <input
            type="search"
            value={filters.q}
            onChange={(e) => set({ q: e.target.value })}
            placeholder="Search titles and paths"
            className="placeholder:text-ink-3 min-w-0 flex-1 bg-transparent text-[14px] outline-none"
          />
        </label>
        <label className="text-ink-3 flex items-center gap-2.5 font-mono text-[11px] tracking-[0.18em] uppercase">
          Sort
          <select
            value={filters.sort}
            onChange={(e) => set({ sort: e.target.value as SortKey })}
            className="border-input bg-card text-foreground focus-visible:border-ring focus-visible:ring-ring/50 h-10 rounded-lg border px-3 font-sans text-[13px] tracking-normal normal-case outline-none focus-visible:ring-3"
          >
            {SORTS.map((s) => (
              <option key={s.id} value={s.id}>
                {s.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="rd-rise flex flex-col gap-3" style={rise(1)}>
        <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Subject">
          <Eyebrow className="mr-1 w-14">Subject</Eyebrow>
          <Chip pressed={filters.domain === null} onClick={() => set({ domain: null })} count={all}>
            All
          </Chip>
          {domains.map((c) => (
            <Chip key={c.id} pressed={filters.domain === c.id} onClick={() => set({ domain: filters.domain === c.id ? null : c.id })} count={c.count}>
              {shelfLabel(c.id)}
            </Chip>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Type">
          <Eyebrow className="mr-1 w-14">Type</Eyebrow>
          <Chip pressed={filters.sourceType === null} onClick={() => set({ sourceType: null })} count={all}>
            All
          </Chip>
          {types.map((c) => (
            <Chip key={c.id} pressed={filters.sourceType === c.id} onClick={() => set({ sourceType: filters.sourceType === c.id ? null : c.id })} count={c.count}>
              {TYPE_PLURAL[c.id] ?? shelfLabel(c.id)}
            </Chip>
          ))}
        </div>
      </div>

      {d.docs.length === 0 ? (
        <NoMatches onClear={() => setFilters(DEFAULT_FILTERS)} />
      ) : (
        <div className="rd-rise flex flex-col gap-3" style={rise(2)}>
          <p role="status" className="text-ink-3 font-mono text-[11px] tracking-[0.04em]">
            {d.docs.length < d.total ? `Showing ${d.docs.length} of ${d.total}` : `${d.total} ${d.total === 1 ? "document" : "documents"}`}
          </p>
          <ul className={cn("border-hairline bg-card divide-hairline divide-y overflow-hidden rounded-2xl border transition-opacity", d.refreshing && "opacity-60")}>
            {d.docs.map((doc) => (
              <li key={doc.doc_id}>
                <button
                  type="button"
                  onClick={() => onOpen(doc)}
                  className="hover:bg-muted focus-visible:ring-ring/50 flex w-full items-center gap-4 px-5 py-4 text-left transition-colors outline-none focus-visible:ring-3 focus-visible:ring-inset"
                >
                  <span className="flex min-w-0 flex-1 flex-col gap-1.5">
                    <span className="font-serif text-[18px] leading-snug font-normal tracking-[-0.005em] text-pretty">{doc.title}</span>
                    <span className="text-ink-3 flex flex-wrap items-center gap-x-2.5 gap-y-1 font-mono text-[11px] tracking-[0.04em]">
                      <ShelfTag domain={doc.domain} />
                      <span aria-hidden>·</span>
                      <span>{TYPE_WORD[doc.source_type] ?? shelfLabel(doc.source_type)}</span>
                      <span aria-hidden>·</span>
                      <span>{sizeLabel(doc)}</span>
                      {doc.added_at != null ? (
                        <>
                          <span aria-hidden>·</span>
                          <span>added {relativeAdded(doc.added_at)}</span>
                        </>
                      ) : null}
                      {doc.ocr_pending > 0 ? (
                        <>
                          <span aria-hidden>·</span>
                          <span className="text-warn">{doc.ocr_pending} pages waiting for OCR</span>
                        </>
                      ) : null}
                    </span>
                  </span>
                  <ChevronRight aria-hidden className="text-ink-3 size-4 shrink-0" />
                </button>
              </li>
            ))}
          </ul>
          {d.docs.length < d.total ? (
            <div className="flex items-center gap-4">
              <Button variant="outline" disabled={d.loadingMore} onClick={d.more}>
                {d.loadingMore ? "Loading" : `Show ${Math.min(50, d.total - d.docs.length)} more`}
              </Button>
              {d.moreError ? (
                <span role="alert" className="text-bad font-mono text-[12px]">
                  {d.moreError}
                </span>
              ) : null}
            </div>
          ) : null}
        </div>
      )}
    </div>
  )
}
