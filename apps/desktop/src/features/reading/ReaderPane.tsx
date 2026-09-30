// The clean reader: the original text of one section, exactly as the document has it, with page markers and
// a thin progress rule. No summary, no highlighting, no chat: the learner reads, then closes the book.
import { useEffect, useMemo, useRef, type RefObject } from "react"
import { RotateCw } from "lucide-react"

import { Button } from "@/components/ui/button"

import { pageLabel, readerBlocks, readingProgress, sectionTitle } from "./reader"
import { SourceChip } from "./SourceChip"
import { Working } from "./States"
import type { Chunk, SectionRef } from "./types"
import type { Loaded } from "./useReadingSession"
import "./reading.css"

/** The nearest ancestor that scrolls (the shell's content pane), or null when the window does. */
export function scrollParent(el: HTMLElement): HTMLElement | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const overflow = getComputedStyle(p).overflowY
    if (overflow === "auto" || overflow === "scroll") return p
  }
  return null
}

/** Writes the scrolled share to the rule (scaleX) and its percent label, one frame at a time, on scroll only. */
function useProgressRule(root: RefObject<HTMLElement | null>, bar: RefObject<HTMLElement | null>, label: RefObject<HTMLElement | null>, active: boolean) {
  useEffect(() => {
    const el = root.current
    if (!el || !active) return
    const scroller = scrollParent(el)
    const source: HTMLElement | Window = scroller ?? window
    let frame = 0
    const update = () => {
      frame = 0
      const top = scroller ? scroller.scrollTop : window.scrollY
      const height = scroller ? scroller.scrollHeight : document.documentElement.scrollHeight
      const view = scroller ? scroller.clientHeight : window.innerHeight
      const p = readingProgress(top, height, view)
      if (bar.current) bar.current.style.transform = `scaleX(${p})`
      if (label.current) label.current.textContent = `${Math.round(p * 100)}%`
    }
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }
    update()
    source.addEventListener("scroll", onScroll, { passive: true })
    window.addEventListener("resize", onScroll)
    return () => {
      source.removeEventListener("scroll", onScroll)
      window.removeEventListener("resize", onScroll)
      if (frame) cancelAnimationFrame(frame)
    }
  }, [root, bar, label, active])
}

export function Reader({
  docId,
  docTitle,
  section,
  text,
  onRetry,
  onDone,
}: {
  docId: string
  docTitle: string
  section: SectionRef
  text: Loaded<Chunk[]>
  onRetry: () => void
  onDone: () => void
}) {
  const root = useRef<HTMLDivElement>(null)
  const bar = useRef<HTMLSpanElement>(null)
  const label = useRef<HTMLSpanElement>(null)
  const chunks = text.phase === "ready" ? text.value : null
  const blocks = useMemo(() => (chunks ? readerBlocks(chunks) : []), [chunks])
  useProgressRule(root, bar, label, chunks !== null)
  const pages = pageLabel(section.page_start, section.page_end)

  return (
    <div ref={root} className="rd-measure mx-auto w-full">
      <div className="bg-card sticky top-0 z-10 -mx-3 rounded-b-lg px-3 pt-3">
        <div className="flex items-baseline justify-between gap-4 pb-2">
          <p className="text-ink-2 min-w-0 truncate font-mono text-[11px] tracking-[0.12em] uppercase">
            {sectionTitle(section.heading_path)}
            {pages ? ` · ${pages}` : ""}
          </p>
          <span ref={label} className="text-ink-3 shrink-0 font-mono text-[11px] tabular-nums" aria-hidden>
            0%
          </span>
        </div>
        <div className="rd-progress" role="progressbar" aria-label="Reading progress" aria-valuemin={0} aria-valuemax={100}>
          <span ref={bar} />
        </div>
      </div>

      {text.phase === "loading" ? <Working label="Loading the section text" /> : null}
      {text.phase === "error" ? (
        <div role="alert" className="flex flex-col items-start gap-3 py-10">
          <p className="text-ink-2 text-[14px] leading-relaxed">The section text could not be loaded from rag-engine.</p>
          <p className="text-bad font-mono text-[12px] break-words">{text.message}</p>
          <Button variant="outline" onClick={onRetry}>
            <RotateCw />
            Try again
          </Button>
        </div>
      ) : null}

      {chunks ? (
        <>
          <h3 className="mt-8 mb-6 font-serif text-[28px] leading-[1.15] font-normal tracking-[-0.015em] text-balance">{sectionTitle(section.heading_path)}</h3>
          <article className="rd-prose" aria-label={sectionTitle(section.heading_path)}>
            {blocks.map((b, i) =>
              b.kind === "page" ? (
                <div key={i} role="separator" aria-label={`Page ${b.page}`} className="rd-page">
                  <span>{b.label}</span>
                </div>
              ) : (
                <p key={i}>{b.text}</p>
              ),
            )}
          </article>
          {chunks.length < section.chunk_ids.length ? (
            <p role="status" className="text-warn mt-6 font-mono text-[12px]">
              Showing {chunks.length} of {section.chunk_ids.length} chunks of this section.
            </p>
          ) : null}
          <div className="border-hairline mt-12 flex flex-col gap-5 border-t pt-6 pb-4">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              <span className="text-ink-3 font-mono text-[11px] tracking-[0.18em] uppercase">Source</span>
              <SourceChip docId={docId} title={docTitle} page={section.page_start ?? chunks[0]?.page_start ?? null} pageEnd={section.page_end} status="verified" />
            </div>
            <div className="flex flex-wrap items-center gap-x-5 gap-y-3">
              <Button size="lg" onClick={onDone}>
                I have finished reading
              </Button>
              <p className="text-ink-2 font-serif text-[15px] italic">Next the text is hidden and you write what you remember.</p>
            </div>
          </div>
        </>
      ) : null}
    </div>
  )
}
