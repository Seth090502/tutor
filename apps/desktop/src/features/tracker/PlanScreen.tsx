// Plan: docs/tutor/PLAN.md as an editorial reading surface. Parsing is in features/plan/markdown.ts,
// rendering in features/plan/MarkdownView.tsx.
import { Component, useCallback, useEffect, useMemo, useRef, useState, type ErrorInfo, type MouseEvent, type ReactNode } from "react"
import { ChevronRight, RefreshCw, TriangleAlert } from "lucide-react"

import { Eyebrow, riseStyle } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { DesktopRequired } from "@/features/plan/DesktopRequired"
import { headingsOf, parseBlocks, readingMinutes, type Heading } from "@/features/plan/markdown"
import { MarkdownView, headingDomId, sectionNumbers } from "@/features/plan/MarkdownView"
import { useScrollSpy } from "@/features/plan/useScrollSpy"
import { inTauri, shell } from "@/lib/backend"
import { cn } from "@/lib/utils"

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))

export function PlanScreen() {
  return (
    <PlanBoundary>
      {inTauri ? (
        <PlanLoader />
      ) : (
        <DesktopRequired title="Open in the desktop app">
          The plan is read from disk by the desktop shell, so it shows only inside the app.
        </DesktopRequired>
      )}
    </PlanBoundary>
  )
}

/** Keeps a render bug in the document from blanking the whole app. */
class PlanBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }
  static getDerivedStateFromError(error: Error) {
    return { error }
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("plan screen failed to render", error, info.componentStack)
  }
  render() {
    const { error } = this.state
    if (!error) return this.props.children
    return (
      <DesktopRequired title="The plan failed to render">
        <span className="font-mono text-[13px] break-words">{String(error.message ?? error)}</span>
      </DesktopRequired>
    )
  }
}

function PlanLoader() {
  const [markdown, setMarkdown] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setMarkdown((await shell.planMarkdown()).markdown)
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const blocks = useMemo(() => (markdown === null ? null : parseBlocks(markdown)), [markdown])
  const headings = useMemo(() => (blocks ? headingsOf(blocks) : []), [blocks])

  return <PlanReader blocks={blocks} headings={headings} error={error} loading={loading} onRefresh={() => void load()} />
}

type ReaderProps = {
  blocks: ReturnType<typeof parseBlocks> | null
  headings: Heading[]
  error: string | null
  loading: boolean
  onRefresh: () => void
}

function PlanReader({ blocks, headings, error, loading, onRefresh }: ReaderProps) {
  const root = useRef<HTMLDivElement>(null)
  const bar = useRef<HTMLDivElement>(null)
  const outline = useMemo(() => headings.filter((h) => h.level === 2 || h.level === 3), [headings])
  const active = useScrollSpy(root, bar, outline.map(headingDomId))

  const hasDoc = blocks !== null && blocks.length > 0
  const minutes = blocks && hasDoc ? readingMinutes(blocks) : null

  return (
    <div ref={root} className="relative min-h-full">
      {/* Reading progress: a 2px hairline pinned to the top of the pane, scaled by scroll position. */}
      <div aria-hidden className="pointer-events-none sticky top-0 z-10 h-0.5">
        <div ref={bar} className="bg-world h-full origin-left" style={{ transform: "scaleX(0)" }} />
      </div>

      <div className="@container">
        <div className="mx-auto grid max-w-[76rem] justify-center gap-x-14 px-8 pt-8 pb-28 @[60rem]:grid-cols-[minmax(0,68ch)_13rem]">
          <div className="min-w-0">
            <div className="kit-rise mb-10 flex items-center justify-between gap-4">
              <Eyebrow rule className="text-muted-foreground">
                <span className="tracking-[0.04em] normal-case">docs/tutor/PLAN.md</span>
                {minutes !== null && <span>{minutes} min read</span>}
              </Eyebrow>
              <Button variant="ghost" size="sm" onClick={onRefresh} disabled={loading} aria-label="Refresh the plan">
                <RefreshCw className={cn(loading && "motion-safe:animate-spin")} />
                Refresh
              </Button>
            </div>

            {error && <ErrorNotice error={error} />}

            {outline.length > 1 && (
              <details className="kit-rise border-hairline group/toc mb-8 border-y py-3 @[60rem]:hidden" style={riseStyle(1)}>
                <summary className="text-ink-2 flex cursor-pointer list-none items-center gap-2 font-mono text-[11px] tracking-[0.16em] uppercase [&::-webkit-details-marker]:hidden">
                  <ChevronRight aria-hidden className="size-3.5 transition-transform group-open/toc:rotate-90 motion-reduce:transition-none" />
                  Contents
                </summary>
                <Outline headings={outline} active={active} className="mt-3" />
              </details>
            )}

            {blocks === null ? (
              !error && <Loading />
            ) : hasDoc ? (
              <article className="font-serif text-[17.5px] leading-[1.6] text-pretty [font-optical-sizing:auto]">
                <MarkdownView blocks={blocks} headings={headings} />
              </article>
            ) : (
              <Empty />
            )}
          </div>

          {outline.length > 1 && (
            <aside className="kit-rise hidden @[60rem]:block" style={riseStyle(3)}>
              <div className="sticky top-8 max-h-[calc(100vh-14rem)] overflow-y-auto pt-0.5">
                <Eyebrow className="text-muted-foreground mb-4">On this page</Eyebrow>
                <Outline headings={outline} active={active} />
              </div>
            </aside>
          )}
        </div>
      </div>
    </div>
  )
}

function Outline({ headings, active, className }: { headings: Heading[]; active: string | null; className?: string }) {
  const numbers = sectionNumbers(headings)
  const jump = (e: MouseEvent, id: string) => {
    const el = document.getElementById(id)
    if (!el) return
    e.preventDefault()
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches
    el.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "start" })
    el.focus({ preventScroll: true })
  }
  return (
    <nav aria-label="Plan outline" className={className}>
      <ol className="border-hairline relative border-l">
        {headings.map((h) => {
          const id = headingDomId(h)
          const current = id === active
          return (
            <li key={id}>
              <a
                href={`#${id}`}
                onClick={(e) => jump(e, id)}
                aria-current={current ? "location" : undefined}
                className={cn(
                  "focus-visible:ring-ring/70 relative flex gap-2.5 rounded-sm py-1.5 pr-2 text-[13px] leading-snug outline-none focus-visible:ring-2 focus-visible:ring-inset",
                  h.level === 3 ? "pl-8" : "pl-4",
                  current ? "text-foreground" : "text-ink-2 hover:text-foreground",
                )}
              >
                <span
                  aria-hidden
                  className={cn(
                    "bg-world absolute top-[0.85rem] -left-[3px] size-[5px] rounded-full transition-[transform,opacity] duration-300 motion-reduce:transition-none",
                    current ? "scale-100 opacity-100" : "scale-[0.3] opacity-0",
                  )}
                />
                {numbers.has(h.id) && <span className="text-muted-foreground font-mono text-[11px] leading-[1.45rem] tabular-nums">{numbers.get(h.id)}</span>}
                <span className="min-w-0 text-balance">{h.text}</span>
              </a>
            </li>
          )
        })}
      </ol>
    </nav>
  )
}

function ErrorNotice({ error }: { error: string }) {
  return (
    <div role="alert" className="kit-rise border-bad/60 mb-8 flex items-start gap-3 border-l-2 py-1 pl-4">
      <TriangleAlert aria-hidden className="text-bad mt-0.5 size-4 shrink-0" />
      <div className="min-w-0">
        <p className="text-[14px] font-medium">Could not read the plan</p>
        <p className="text-ink-2 mt-1 font-mono text-[12px] leading-relaxed break-words">{error}</p>
      </div>
    </div>
  )
}

function Empty() {
  return (
    <div className="kit-rise border-hairline flex flex-col gap-2 border-t pt-8 font-sans">
      <p className="text-[15px] font-medium">The plan is empty</p>
      <p className="text-ink-2 text-[14px]">
        <code className="font-mono text-[13px]">docs/tutor/PLAN.md</code> has no content yet. Refresh after editing it.
      </p>
    </div>
  )
}

/** Static placeholder lines (no shimmer): the real document rises in when it arrives. */
function Loading() {
  const widths = ["w-2/3", "w-full", "w-[92%]", "w-[78%]", "w-full", "w-1/2"]
  return (
    <div role="status" aria-busy="true" className="kit-rise flex flex-col gap-4">
      <span className="sr-only">Reading the plan</span>
      <Eyebrow className="text-muted-foreground">Reading the plan</Eyebrow>
      <div aria-hidden className="mt-2 flex flex-col gap-3.5">
        {widths.map((w, i) => (
          <div key={i} className={cn("h-3 rounded-full bg-[color-mix(in_oklab,var(--ink)_7%,transparent)]", w, i === 0 && "h-6")} />
        ))}
      </div>
    </div>
  )
}
