import type { ReactNode } from "react"

import { Eyebrow } from "@/components/kit"

import "./rise.css"

/** Shown instead of a screen that only works inside the Tauri shell (plain browser, `npm run dev`). */
export function DesktopRequired({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mx-auto flex min-h-full max-w-lg flex-col justify-center gap-4 px-8 py-16">
      <div className="screen-rise flex flex-col gap-4">
        <Eyebrow rule className="text-muted-foreground">Desktop shell</Eyebrow>
        <h2 className="font-serif text-[1.75rem] leading-tight tracking-[-0.015em] text-balance">{title}</h2>
        <p className="text-ink-2 text-[15px] leading-relaxed text-pretty">{children}</p>
      </div>
    </div>
  )
}
