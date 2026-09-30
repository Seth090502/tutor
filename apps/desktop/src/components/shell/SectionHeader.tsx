import { worldMeta, type World } from "@/lib/world"

import { WorldMotif } from "./motifs"
import type { Section } from "./types"
import type { Phase } from "./useSectionPhase"

/**
 * Mono eyebrow (the world, with a 24px hairline), the section title, the subtitle and the world
 * motif. The text block follows the section phase; the motif redraws only when the world changes.
 */
export function SectionHeader({ section, world, phase }: { section: Section; world: World; phase: Phase }) {
  return (
    <header className="border-hairline flex items-end justify-between gap-6 border-b px-8 pt-6 pb-5">
      <div className="shell-phase min-w-0" data-phase={phase}>
        <p className="mb-2.5 flex items-center gap-3 font-mono text-[11px] tracking-[0.18em] text-[color-mix(in_oklab,var(--world)_72%,var(--ink))] uppercase">
          <span className="bg-hairline-strong h-px w-6 shrink-0" aria-hidden="true" />
          <span key={world} className="shell-fade">
            {worldMeta(world).label}
          </span>
        </p>
        <h1 id="section-title" className="font-serif text-[2rem] leading-[1.1] font-normal tracking-[-0.015em]">
          {section.label}
        </h1>
        <p className="text-ink-2 mt-1.5 text-sm">{section.subtitle}</p>
      </div>
      <WorldMotif key={world} world={world} className="h-auto w-56 shrink-0 max-[820px]:hidden" />
    </header>
  )
}
