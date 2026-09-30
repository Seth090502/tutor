// The Library when there is nothing to show yet: loading, rag-engine offline, rag-engine error.
import type { ReactNode } from "react"
import { RotateCw, ServerOff, TriangleAlert } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Eyebrow, KbdHint, Led, riseStyle } from "@/components/kit"

function CalmState({
  mark,
  eyebrow,
  title,
  detail,
  children,
}: {
  mark: ReactNode
  eyebrow: ReactNode
  title: string
  detail?: ReactNode
  children?: ReactNode
}) {
  return (
    <div className="mx-auto flex min-h-full max-w-lg flex-col items-center justify-center gap-5 px-8 py-16 text-center">
      <span
        aria-hidden
        className="kit-rise border-hairline-strong text-ink-2 bg-card grid size-14 place-items-center rounded-full border [box-shadow:inset_0_1px_0_var(--kit-highlight),var(--kit-shadow)]"
      >
        {mark}
      </span>
      <div className="kit-rise flex flex-col items-center gap-3" style={riseStyle(1)}>
        <Eyebrow>{eyebrow}</Eyebrow>
        <h2 className="font-serif text-[24px] leading-snug font-normal tracking-[-0.01em] text-balance">{title}</h2>
        {detail ? <p className="text-ink-2 max-w-[44ch] text-[14px] leading-relaxed text-pretty">{detail}</p> : null}
      </div>
      {children ? (
        <div className="kit-rise flex flex-col items-center gap-4" style={riseStyle(2)}>
          {children}
        </div>
      ) : null}
    </div>
  )
}

export function LoadingState() {
  return (
    <div className="mx-auto flex min-h-full items-center justify-center px-8 py-16" role="status">
      <span className="text-ink-3 inline-flex items-center gap-3 font-mono text-[11px] tracking-[0.18em] uppercase">
        <Led tone="off" />
        Reading the library
      </span>
    </div>
  )
}

export function OfflineState({ onRetry }: { onRetry: () => void }) {
  return (
    <CalmState
      mark={<ServerOff className="size-6" strokeWidth={1.5} />}
      eyebrow="rag-engine · offline"
      title="rag-engine is not running; start it from Settings"
      detail="The Library reads your corpus from rag-engine on 127.0.0.1:8811 and looks again every few seconds."
    >
      <Button variant="outline" onClick={onRetry}>
        <RotateCw />
        Check again
      </Button>
      <KbdHint>Settings is at the foot of the sidebar</KbdHint>
    </CalmState>
  )
}

export function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <CalmState
      mark={<TriangleAlert className="size-6" strokeWidth={1.5} />}
      eyebrow="rag-engine · error"
      title="rag-engine answered with an error"
      detail={<span className="font-mono text-[12px] break-words">{message}</span>}
    >
      <Button variant="outline" onClick={onRetry}>
        <RotateCw />
        Try again
      </Button>
    </CalmState>
  )
}
