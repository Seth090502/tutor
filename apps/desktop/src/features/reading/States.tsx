// Everything the Reading screens show when there is nothing to read yet: loading, empty library, no
// matches, rag-engine offline or failing, backend offline, a section that could not be prepared.
import type { CSSProperties, ReactNode } from "react"
import { ArrowLeft, LibraryBig, RotateCw, SearchX, ServerOff, TriangleAlert } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow, KbdHint, Led } from "@/components/kit"

import type { Failure } from "./flow"
import "./reading.css"

const at = (i: number) => ({ "--i": i }) as CSSProperties

function Calm({
  mark,
  eyebrow,
  title,
  detail,
  children,
  className = "min-h-full",
}: {
  mark: ReactNode
  eyebrow: ReactNode
  title: string
  detail?: ReactNode
  children?: ReactNode
  className?: string
}) {
  return (
    <div className={`mx-auto flex ${className} max-w-lg flex-col items-center justify-center gap-5 px-8 py-16 text-center`}>
      <span
        aria-hidden
        className="rd-rise border-hairline-strong text-ink-2 bg-card grid size-14 place-items-center rounded-full border [box-shadow:inset_0_1px_0_var(--kit-highlight),var(--kit-shadow)]"
      >
        {mark}
      </span>
      <div className="rd-rise flex flex-col items-center gap-3" style={at(1)}>
        <Eyebrow>{eyebrow}</Eyebrow>
        <h2 className="font-serif text-[24px] leading-snug font-normal tracking-[-0.01em] text-balance">{title}</h2>
        {detail ? <p className="text-ink-2 max-w-[46ch] text-[14px] leading-relaxed text-pretty">{detail}</p> : null}
      </div>
      {children ? (
        <div className="rd-rise flex flex-col items-center gap-4" style={at(2)}>
          {children}
        </div>
      ) : null}
    </div>
  )
}

export function Working({ label, detail }: { label: string; detail?: string }) {
  return (
    <div className="mx-auto flex min-h-[50vh] flex-col items-center justify-center gap-3 px-8 py-16 text-center" role="status">
      <span className="text-ink-3 inline-flex items-center gap-3 font-mono text-[11px] tracking-[0.18em] uppercase">
        <Led tone="off" />
        {label}
      </span>
      {detail ? <p className="text-ink-3 max-w-[44ch] text-[13px] leading-relaxed">{detail}</p> : null}
    </div>
  )
}

const SETTINGS_HINT = <KbdHint>Settings is at the foot of the sidebar.</KbdHint>

export function EngineOffline({ onRetry }: { onRetry: () => void }) {
  return (
    <Calm
      mark={<ServerOff className="size-6" strokeWidth={1.5} />}
      eyebrow="rag-engine · offline"
      title="rag-engine is not running; start it from Settings"
      detail="The document list and the page text come from rag-engine on 127.0.0.1:8811."
    >
      <Button variant="outline" onClick={onRetry}>
        <RotateCw />
        Check again
      </Button>
      {SETTINGS_HINT}
    </Calm>
  )
}

export function EngineError({ title, message, onRetry, onBack }: { title: string; message: string; onRetry?: () => void; onBack?: () => void }) {
  return (
    <Calm
      mark={<TriangleAlert className="size-6" strokeWidth={1.5} />}
      eyebrow="rag-engine · error"
      title={title}
      detail={<span className="font-mono text-[12px] break-words">{message}</span>}
    >
      <div className="flex flex-wrap justify-center gap-2">
        {onBack ? (
          <Button variant="outline" onClick={onBack}>
            <ArrowLeft />
            Back to the library
          </Button>
        ) : null}
        {onRetry ? (
          <Button variant="outline" onClick={onRetry}>
            <RotateCw />
            Try again
          </Button>
        ) : null}
      </div>
    </Calm>
  )
}

export function EmptyLibrary() {
  return (
    <Calm
      mark={<LibraryBig className="size-6" strokeWidth={1.5} />}
      eyebrow="Library · empty"
      title="There is nothing to read yet"
      detail="Put books and papers in a subject shelf under your corpus folder, then run Ingest from the Library screen. Indexed documents appear here."
    />
  )
}

export function NoMatches({ onClear }: { onClear: () => void }) {
  return (
    <Calm
      mark={<SearchX className="size-6" strokeWidth={1.5} />}
      eyebrow="Library · no match"
      title="No document matches"
      detail="Search looks at titles and file paths. Try fewer words, or clear the filters."
      className=""
    >
      <Button variant="outline" onClick={onClear}>
        Clear search and filters
      </Button>
    </Calm>
  )
}

/** The flow could not get as far as the first step: the backend is down, or it answered with an error. */
export function FlowFailed({ failure, onRetry, onBack }: { failure: Failure; onRetry: () => void; onBack: () => void }) {
  const backendDown = failure.offline
  const preparing = failure.stage === "prepare"
  return (
    <Calm
      mark={backendDown ? <ServerOff className="size-6" strokeWidth={1.5} /> : <TriangleAlert className="size-6" strokeWidth={1.5} />}
      eyebrow={backendDown ? "backend · offline" : preparing ? "preparation failed" : "section not opened"}
      title={
        backendDown
          ? "The backend is not running; start it from Settings"
          : preparing
            ? "This section could not be prepared"
            : "This section could not be opened"
      }
      detail={
        backendDown ? (
          "Guided Reading prepares each section through the DeepTutor backend on 127.0.0.1:8001."
        ) : (
          <span className="font-mono text-[12px] break-words">{failure.message}</span>
        )
      }
    >
      <div className="flex flex-wrap justify-center gap-2">
        <Button variant="outline" onClick={onBack}>
          <ArrowLeft />
          Back to the outline
        </Button>
        <Button variant="outline" onClick={onRetry}>
          <RotateCw />
          {backendDown ? "Check again" : "Try again"}
        </Button>
      </div>
      {backendDown ? SETTINGS_HINT : null}
    </Calm>
  )
}
