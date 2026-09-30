// Small shared pieces of the Evals screen: the state block and the command chip.
import type { ComponentType, ReactNode } from "react"

/** Centred message for the states that have no data: browser, empty and error. */
export function StateBlock({ icon: Icon, title, children, action }: { icon: ComponentType<{ className?: string }>; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div role="status" className="mx-auto flex min-h-[46vh] max-w-md flex-col items-center justify-center gap-4 px-6 py-12 text-center">
      <span className="border-hairline-strong text-ink-2 grid size-12 place-items-center rounded-full border">
        <Icon className="size-5" />
      </span>
      <h2 className="font-serif text-[26px] leading-tight tracking-[-0.01em] text-balance">{title}</h2>
      {children ? <div className="text-ink-2 text-sm leading-relaxed text-pretty">{children}</div> : null}
      {action}
    </div>
  )
}

export function Code({ children }: { children: ReactNode }) {
  return <code className="bg-muted border-hairline text-foreground inline-block rounded-md border px-2.5 py-1.5 font-mono text-[12.5px] break-words">{children}</code>
}
