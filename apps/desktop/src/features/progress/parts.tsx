// Small pieces local to the Progress screen: notices, state blocks, an error boundary and a width hook.
import { Component, useEffect, useRef, useState, type ErrorInfo, type ReactNode } from "react"
import { RefreshCw, TriangleAlert, type LucideIcon } from "lucide-react"

import { Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"

/** A failed or partial load. Glyph and words carry the meaning; the tint is secondary. */
export function Notice({ children }: { children: ReactNode }) {
  return (
    <div role="alert" className="border-warn/45 bg-warn/10 flex items-start gap-3 rounded-xl border px-4 py-3 text-sm">
      <TriangleAlert aria-hidden className="text-warn mt-0.5 size-4 shrink-0" />
      <div className="min-w-0 break-words">{children}</div>
    </div>
  )
}

/** Muted line inside a panel whose data did not load or is empty. */
export function Quiet({ children }: { children: ReactNode }) {
  return <p className="text-ink-2 text-sm leading-relaxed text-pretty">{children}</p>
}

/** Full-width state (offline, empty): a centred glyph, one line and an optional action. */
export function StateBlock({
  icon: Icon,
  title,
  detail,
  onRetry,
  busy,
  retryLabel = "Try again",
}: {
  icon: LucideIcon
  title: string
  detail?: string
  onRetry?: () => void
  busy?: boolean
  retryLabel?: string
}) {
  return (
    <Panel className="mx-auto flex max-w-xl flex-col items-center gap-3 py-12 text-center" role="status">
      <Icon aria-hidden className="text-ink-3 size-7" strokeWidth={1.5} />
      <p className="font-serif text-xl tracking-[-0.01em]">{title}</p>
      {detail ? <p className="text-ink-2 max-w-[44ch] text-sm leading-relaxed text-pretty">{detail}</p> : null}
      {onRetry ? (
        <Button variant="outline" size="sm" onClick={onRetry} disabled={busy} className="mt-2">
          <RefreshCw className={busy ? "opacity-60" : undefined} />
          {retryLabel}
        </Button>
      ) : null}
    </Panel>
  )
}

/** Keeps a render bug in this screen from blanking the whole app. */
export class ScreenBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }
  static getDerivedStateFromError(error: Error) {
    return { error }
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Progress screen crashed", error, info.componentStack)
  }
  render() {
    if (this.state.error) {
      return (
        <div className="mx-auto max-w-5xl px-6 py-8 lg:px-8">
          <StateBlock icon={TriangleAlert} title="Progress hit a display error" detail={this.state.error.message} />
        </div>
      )
    }
    return this.props.children
  }
}

/** Width in px of the element the ref is attached to; `fallback` until it is measured. */
export function useWidth<T extends HTMLElement>(fallback: number) {
  const ref = useRef<T | null>(null)
  const [width, setWidth] = useState(fallback)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const read = () => {
      const w = Math.round(el.getBoundingClientRect().width)
      if (w > 0) setWidth(w)
    }
    read()
    const ro = new ResizeObserver(read)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])
  return [ref, width] as const
}
