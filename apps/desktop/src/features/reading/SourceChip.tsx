// Source chip: where a passage came from. Title, page, a verified badge (a word and a mark, never a number
// or a colour alone) and, on a click, the page itself rendered by rag-engine with the quote marked.
// Used by Guided Reading now and meant for the tutor's citations (CORPUS.md, Stage 0 step 6).
import { useEffect, useRef, useState } from "react"
import { BookOpen, Check, LoaderCircle, TriangleAlert, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

import { pageImageUrl } from "./api"
import { pageLabel } from "./reader"
import "./reading.css"

/**
 * verified: the chunk and the pages come from rag-engine, and a quote, if any, was found in it.
 * unverified: a citation that failed the check (the tutor says it cannot verify it).
 */
export type ChipStatus = "verified" | "unverified" | "checking"

const BADGE: Record<ChipStatus, { word: string; Icon: typeof Check; cls: string; why: string }> = {
  verified: { word: "Verified", Icon: Check, cls: "border-ok/40 bg-ok/10 [&>svg]:text-ok", why: "The passage exists in this document on these pages." },
  unverified: { word: "Unverified", Icon: TriangleAlert, cls: "border-warn/45 bg-warn/10 [&>svg]:text-warn", why: "This citation could not be checked against the document." },
  checking: { word: "Checking", Icon: LoaderCircle, cls: "border-hairline-strong bg-transparent [&>svg]:text-ink-3", why: "Checking the citation." },
}

export function VerifiedBadge({ status, className }: { status: ChipStatus; className?: string }) {
  const { word, Icon, cls, why } = BADGE[status]
  return (
    <span
      data-status={status}
      title={why}
      className={cn("text-foreground inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2 py-[3px] font-mono text-[11px] leading-none tracking-[0.04em]", cls, className)}
    >
      <Icon aria-hidden className="size-3" strokeWidth={2.5} />
      {word}
    </span>
  )
}

export type SourceChipProps = {
  docId: string
  title: string
  page: number | null
  pageEnd?: number | null
  /** Marked on the page crop when it is found in the page's text. */
  quote?: string
  status: ChipStatus
  className?: string
}

export function SourceChip({ docId, title, page, pageEnd, quote, status, className }: SourceChipProps) {
  const [open, setOpen] = useState(false)
  const pages = pageLabel(page, pageEnd ?? null)
  const body = (
    <>
      <BookOpen aria-hidden className="text-ink-3 size-3.5 shrink-0" />
      <span className="min-w-0 truncate text-[13px]">{title}</span>
      {pages ? <span className="text-ink-2 shrink-0 font-mono text-[11px] tracking-[0.04em]">{pages}</span> : null}
      <VerifiedBadge status={status} />
    </>
  )
  const frame = "border-hairline-strong bg-card inline-flex max-w-full min-w-0 items-center gap-2.5 rounded-full border py-1 pr-1.5 pl-3"
  if (page == null) return <span className={cn(frame, className)}>{body}</span>
  return (
    <>
      <button
        type="button"
        aria-haspopup="dialog"
        aria-label={`${title}, ${pages}, ${BADGE[status].word}. Show the page`}
        onClick={() => setOpen(true)}
        className={cn(frame, "hover:bg-muted focus-visible:ring-ring/50 cursor-pointer text-left transition-colors outline-none focus-visible:ring-3", className)}
      >
        {body}
      </button>
      {open ? <PageModal docId={docId} title={title} page={page} pages={pages} quote={quote} status={status} onClose={() => setOpen(false)} /> : null}
    </>
  )
}

function PageModal({
  docId,
  title,
  page,
  pages,
  quote,
  status,
  onClose,
}: {
  docId: string
  title: string
  page: number
  pages: string
  quote?: string
  status: ChipStatus
  onClose: () => void
}) {
  const ref = useRef<HTMLDialogElement>(null)
  const [image, setImage] = useState<"loading" | "ready" | "error">("loading")
  const src = pageImageUrl(docId, page, { highlight: quote })

  useEffect(() => {
    const d = ref.current
    if (d && !d.open) d.showModal()
  }, [])

  return (
    <dialog
      ref={ref}
      className="rd-dialog"
      aria-label={`${title}, ${pages}`}
      onClose={onClose}
      onClick={(e) => {
        if (e.target === e.currentTarget) ref.current?.close()
      }}
    >
      <header className="border-hairline flex items-start justify-between gap-4 border-b px-5 py-4">
        <div className="flex min-w-0 flex-col gap-2">
          <p className="truncate text-[15px] font-medium">{title}</p>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
            <span className="text-ink-2 font-mono text-[11px] tracking-[0.14em] uppercase">
              p. {page}
              {pages !== `p. ${page}` ? ` · section ${pages}` : ""}
            </span>
            <VerifiedBadge status={status} />
          </div>
        </div>
        <Button variant="ghost" size="icon-sm" aria-label="Close" onClick={() => ref.current?.close()}>
          <X />
        </Button>
      </header>
      <div className="bg-muted relative min-h-40 flex-1 overflow-auto p-4">
        {image === "loading" ? (
          <p role="status" className="text-ink-3 absolute inset-0 grid place-items-center font-mono text-[11px] tracking-[0.18em] uppercase">
            Rendering the page
          </p>
        ) : null}
        {image === "error" ? (
          <p role="alert" className="text-ink-2 mx-auto max-w-[44ch] py-10 text-center text-[14px] leading-relaxed">
            The page image is not available. The document may not be a PDF, or rag-engine did not answer.
          </p>
        ) : (
          <img
            src={src}
            alt={`${title}, ${pages}${quote ? `, with “${quote}” marked` : ""}`}
            onLoad={() => setImage("ready")}
            onError={() => setImage("error")}
            className={cn("mx-auto block h-auto max-w-full rounded-md bg-white shadow-sm", image === "loading" && "invisible h-40")}
          />
        )}
      </div>
      <footer className="border-hairline text-ink-2 border-t px-5 py-3 text-[13px] leading-relaxed">
        {quote ? (
          <>
            Marked on the page: <span className="font-serif text-[15px] italic">“{quote.length > 160 ? `${quote.slice(0, 160)}…` : quote}”</span>
          </>
        ) : (
          "Rendered from your file by rag-engine."
        )}
      </footer>
    </dialog>
  )
}
