// One document: its details and outline (GET /documents/{id}, C9), with "Read this section" on each entry.
import { useCallback, useEffect, useState, type CSSProperties } from "react"
import { ArrowLeft, BookOpenText } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow } from "@/components/kit"
import { cn } from "@/lib/utils"

import { toFault, type ReadingApi, type ServiceFault } from "./api"
import { ShelfTag } from "./DocumentBrowser"
import { isPaper, outlineDepth, relativeAdded, sizeLabel } from "./docs"
import { pageLabel } from "./reader"
import { EngineError, EngineOffline, Working } from "./States"
import type { DocDetail, OutlineEntry } from "./types"
import "./reading.css"

const rise = (i: number) => ({ "--i": i }) as CSSProperties

type Load = { phase: "loading" } | { phase: "ready"; doc: DocDetail } | { phase: "error"; fault: ServiceFault }

export function DocumentView({
  api,
  docId,
  onBack,
  onRead,
}: {
  api: ReadingApi
  docId: string
  onBack: () => void
  onRead: (doc: DocDetail, entry: OutlineEntry) => void
}) {
  const [load, setLoad] = useState<Load>({ phase: "loading" })
  const [nonce, setNonce] = useState(0)

  useEffect(() => {
    let live = true
    setLoad({ phase: "loading" })
    api
      .document(docId)
      .then((doc) => live && setLoad({ phase: "ready", doc }))
      .catch((e) => live && setLoad({ phase: "error", fault: toFault("rag-engine", e) }))
    return () => {
      live = false
    }
  }, [api, docId, nonce])

  const retry = useCallback(() => setNonce((n) => n + 1), [])

  if (load.phase === "loading") return <Working label="Opening the document" />
  if (load.phase === "error") {
    const f = load.fault
    if (f.offline) return <EngineOffline onRetry={retry} />
    return f.status === 404 ? (
      <EngineError title="That document is no longer in the library" message={f.message} onBack={onBack} />
    ) : (
      <EngineError title="rag-engine answered with an error" message={f.message} onRetry={retry} onBack={onBack} />
    )
  }
  return <Loaded api={api} doc={load.doc} onBack={onBack} onRead={onRead} />
}

function Loaded({ api, doc, onBack, onRead }: { api: ReadingApi; doc: DocDetail; onBack: () => void; onRead: (doc: DocDetail, entry: OutlineEntry) => void }) {
  const [filter, setFilter] = useState("")
  const [starting, setStarting] = useState(false)
  const [startError, setStartError] = useState<string | null>(null)
  const needle = filter.trim().toLowerCase()
  const rows = needle ? doc.outline.filter((e) => e.heading_path.join(" ").toLowerCase().includes(needle)) : doc.outline
  const paper = isPaper(doc)

  // No headings were found: the first chunk is where reading can start.
  const readFromStart = async () => {
    setStarting(true)
    setStartError(null)
    try {
      const first = (await api.chunks(doc.doc_id, 0, 1)).chunks[0]
      if (!first) throw new Error("This document has no text chunks.")
      onRead(doc, { heading_path: first.heading_path, page_start: first.page_start, page_end: first.page_end, chunk_id: first.chunk_id })
    } catch (e) {
      setStartError(toFault("rag-engine", e).message)
    } finally {
      setStarting(false)
    }
  }

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6 px-8 pt-6 pb-10">
      <div className="rd-rise flex flex-col gap-4">
        <Button variant="ghost" size="sm" onClick={onBack} className="-ml-2 self-start">
          <ArrowLeft />
          Library
        </Button>
        <div className="flex flex-col gap-3">
          <Eyebrow rule>{paper ? "Paper" : doc.source_type === "note" ? "Note" : "Book"}</Eyebrow>
          <h2 className="max-w-[30ch] font-serif text-[30px] leading-[1.15] font-normal tracking-[-0.015em] text-balance">{doc.title}</h2>
          <p className="text-ink-3 flex flex-wrap items-center gap-x-2.5 gap-y-1 font-mono text-[11px] tracking-[0.04em]">
            <ShelfTag domain={doc.domain} />
            <span aria-hidden>·</span>
            <span>{sizeLabel(doc)}</span>
            {doc.added_at != null ? (
              <>
                <span aria-hidden>·</span>
                <span>added {relativeAdded(doc.added_at)}</span>
              </>
            ) : null}
          </p>
          {paper ? (
            <p className="text-ink-2 max-w-[60ch] text-[14px] leading-relaxed">
              Papers are read in paper mode: predict from the abstract, read, fill an evidence table from memory, then write a critique.
            </p>
          ) : null}
        </div>
      </div>

      {doc.ocr_pending > 0 ? (
        <p role="status" className="border-warn/45 bg-warn/10 text-foreground rounded-lg border px-4 py-3 text-[13px] leading-relaxed">
          {doc.ocr_pending} of {doc.pages} pages are waiting for OCR, so their text is not in the outline yet.
        </p>
      ) : null}

      {doc.chunks === 0 ? (
        <p className="text-ink-2 text-[14px] leading-relaxed">No text has been extracted from this document yet, so there is nothing to read.</p>
      ) : doc.outline.length === 0 ? (
        <div className="rd-rise flex flex-col items-start gap-3" style={rise(1)}>
          <p className="text-ink-2 max-w-[56ch] text-[14px] leading-relaxed">No headings were found in this document, so there is no outline. You can read from the beginning.</p>
          <Button variant="outline" disabled={starting} onClick={() => void readFromStart()}>
            <BookOpenText />
            {starting ? "Opening" : "Read from the beginning"}
          </Button>
          {startError ? (
            <p role="alert" className="text-bad font-mono text-[12px]">
              {startError}
            </p>
          ) : null}
        </div>
      ) : (
        <section className="rd-rise flex flex-col gap-3" style={rise(1)} aria-labelledby="outline-title">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <Eyebrow as="h3" id="outline-title" rule>
              Outline · {doc.outline.length} sections
            </Eyebrow>
            {doc.outline.length >= 12 ? (
              <label className="border-input bg-card focus-within:border-ring focus-within:ring-ring/50 flex h-9 w-64 max-w-full items-center rounded-lg border px-3 focus-within:ring-3">
                <span className="sr-only">Filter the outline</span>
                <input
                  type="search"
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  placeholder="Filter headings"
                  className="placeholder:text-ink-3 min-w-0 flex-1 bg-transparent text-[13px] outline-none"
                />
              </label>
            ) : null}
          </div>
          {rows.length === 0 ? (
            <p className="text-ink-2 py-6 text-[14px]">No heading matches “{filter.trim()}”.</p>
          ) : (
            <ul className="border-hairline bg-card divide-hairline divide-y overflow-hidden rounded-2xl border">
              {rows.map((e, i) => {
                const title = e.heading_path[e.heading_path.length - 1]
                const depth = outlineDepth(e)
                return (
                  <li key={`${e.chunk_id}-${i}`} className="flex items-center gap-4 py-2.5 pr-4 pl-5">
                    <span className="min-w-0 flex-1" style={{ paddingLeft: `${Math.min(depth, 4) * 1.25}rem` }}>
                      <span className={cn("block text-pretty", depth === 0 ? "font-serif text-[17px]" : "text-[14px]")}>{title}</span>
                      {depth > 0 && needle ? <span className="text-ink-3 block truncate font-mono text-[11px]">{e.heading_path.slice(0, -1).join(" › ")}</span> : null}
                    </span>
                    <span className="text-ink-3 shrink-0 font-mono text-[11px] tracking-[0.04em]">{pageLabel(e.page_start, e.page_end)}</span>
                    <Button variant="outline" size="sm" aria-label={`Read this section: ${title}`} onClick={() => onRead(doc, e)}>
                      Read this section
                    </Button>
                  </li>
                )
              })}
            </ul>
          )}
        </section>
      )}
    </div>
  )
}
