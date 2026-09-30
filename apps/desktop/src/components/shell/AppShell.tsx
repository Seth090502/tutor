// The app frame (contract C5): ambient layer, sidebar, per-section header and the section
// transition. Screens are supplied by App.tsx (contract C4).

import { useEffect, useLayoutEffect, useRef, useState } from "react"

import { useWorld } from "@/lib/world"

import { Ambient } from "./Ambient"
import { Sidebar } from "./Sidebar"
import { SectionHeader } from "./SectionHeader"
import type { Section } from "./types"
import { useSectionPhase } from "./useSectionPhase"

export function AppShell({ sections, settings }: { sections: Section[]; settings: Section }) {
  const { world } = useWorld()
  const [activeId, setActiveId] = useState(sections[0].id)
  const { shown, phase } = useSectionPhase(activeId)
  const section = [...sections, settings].find((s) => s.id === shown) ?? sections[0]

  // Alt+1 .. Alt+9 jump straight to a section, in sidebar order.
  useEffect(() => {
    const all = [...sections, settings]
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return
      const n = /^Digit([1-9])$/.exec(e.code)?.[1]
      const target = n ? all[Number(n) - 1] : undefined
      if (!target) return
      e.preventDefault()
      setActiveId(target.id)
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [sections, settings])

  const scroller = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => {
    scroller.current?.scrollTo({ top: 0 })
  }, [shown])

  return (
    <div className="bg-background text-foreground relative isolate flex h-screen overflow-hidden">
      <Ambient />
      <Sidebar sections={sections} settings={settings} activeId={activeId} onSelect={setActiveId} />
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden" aria-labelledby="section-title">
        <SectionHeader section={section} world={world} phase={phase} />
        {/* The rise animation moves this wrapper, so it sits outside the scroller and is clipped by <main>. */}
        <div className="shell-phase min-h-0 flex-1" data-phase={phase}>
          <div ref={scroller} className="h-full overflow-auto">
            <section.Screen />
          </div>
        </div>
      </main>
    </div>
  )
}
