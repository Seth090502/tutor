// Layout shared by the flow steps: an eyebrow, a serif title, one line of lead, then the step's content.
import type { CSSProperties, ReactNode } from "react"

import { Eyebrow } from "@/components/kit"

import "./reading.css"

export const rise = (i: number) => ({ "--i": i }) as CSSProperties

export function StepShell({ eyebrow, title, lead, children }: { eyebrow: string; title: string; lead?: ReactNode; children: ReactNode }) {
  return (
    <section className="mx-auto flex w-full max-w-3xl flex-col gap-7" aria-label={title}>
      <header className="rd-rise flex flex-col gap-3">
        <Eyebrow rule>{eyebrow}</Eyebrow>
        <h3 className="font-serif text-[28px] leading-[1.15] font-normal tracking-[-0.015em] text-balance">{title}</h3>
        {lead ? <p className="text-ink-2 max-w-[60ch] text-[15px] leading-relaxed text-pretty">{lead}</p> : null}
      </header>
      {children}
    </section>
  )
}
