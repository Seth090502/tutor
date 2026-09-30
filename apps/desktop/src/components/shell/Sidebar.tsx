// The app rail: brand, subject switcher, the sections and Settings at the bottom. The
// active-item pill glides on the shared spring. Alt+1..9 jump to a section (see AppShell); the
// arrow keys, Home and End move focus through the items.

import { useRef, useState, type FocusEvent, type KeyboardEvent } from "react"

import { cn } from "@/lib/utils"

import { cardioidPath } from "./motifs"
import type { Section } from "./types"
import { useGlide } from "./useGlide"
import { WorldSwitcher } from "./WorldSwitcher"

function BrandMark() {
  return (
    <span className="bg-world text-world-ink grid size-8 place-items-center rounded-[10px] shadow-[0_1px_0_0_rgb(255_255_255/0.22)_inset,0_3px_10px_-4px_color-mix(in_oklab,var(--world)_70%,transparent)]">
      <svg viewBox="-12 -12 24 24" className="size-[22px]" fill="none" stroke="currentColor" strokeWidth="0.7" aria-hidden="true">
        <circle r="10.5" strokeWidth="1" opacity="0.55" />
        <path d={cardioidPath(0, 0, 10.5, 36, 2)} />
      </svg>
    </span>
  )
}

export function Sidebar({
  sections,
  settings,
  activeId,
  onSelect,
}: {
  sections: Section[]
  settings: Section
  activeId: string
  onSelect: (id: string) => void
}) {
  const all = [...sections, settings]
  const activeIndex = Math.max(0, all.findIndex((s) => s.id === activeId))

  const list = useRef<HTMLDivElement>(null)
  const items = useRef<(HTMLButtonElement | null)[]>([])
  const pill = useRef<HTMLSpanElement>(null)
  const ready = useGlide("y", activeIndex, items, pill, list)

  // Roving tab stop: the active item, or whichever one the arrow keys last reached.
  const [roving, setRoving] = useState<number | null>(null)
  const stop = roving ?? activeIndex

  const onKeyDown = (e: KeyboardEvent) => {
    const at = items.current.findIndex((el) => el === document.activeElement)
    const moves: Record<string, number> = {
      ArrowDown: at + 1,
      ArrowUp: at - 1,
      Home: 0,
      End: all.length - 1,
    }
    if (at < 0 || !(e.key in moves)) return
    e.preventDefault()
    const next = (moves[e.key] + all.length) % all.length
    setRoving(next)
    items.current[next]?.focus()
  }

  const onBlur = (e: FocusEvent) => {
    if (!e.currentTarget.contains(e.relatedTarget)) setRoving(null)
  }

  const renderItem = (s: Section, i: number) => {
    const active = s.id === activeId
    const Icon = s.icon
    return (
      <button
        key={s.id}
        ref={(el) => {
          items.current[i] = el
        }}
        type="button"
        onClick={() => onSelect(s.id)}
        aria-current={active ? "page" : undefined}
        aria-keyshortcuts={`Alt+${i + 1}`}
        tabIndex={i === stop ? 0 : -1}
        className={cn(
          "group focus-visible:outline-world relative z-10 flex h-10 w-full items-center gap-3 rounded-[10px] px-3 text-left text-sm outline-offset-2 transition-colors duration-(--dur-base) focus-visible:outline-2",
          active ? "text-foreground" : "text-ink-2 hover:text-foreground hover:bg-sidebar-accent",
        )}
      >
        <Icon
          className={cn(
            "size-[18px] shrink-0 transition-colors duration-(--dur-base)",
            active ? "text-world" : "text-ink-3 group-hover:text-foreground",
          )}
        />
        <span className={cn("flex-1", active && "font-medium")}>{s.label}</span>
        <kbd className={cn("font-mono text-[11px] tracking-wide", active ? "text-ink-2" : "text-ink-3")}>{`Alt ${i + 1}`}</kbd>
      </button>
    )
  }

  return (
    <aside className="bg-sidebar text-sidebar-foreground border-hairline flex w-[15.5rem] shrink-0 flex-col gap-5 border-r px-3 pt-5 pb-4">
      <div className="flex items-center gap-3 px-2">
        <BrandMark />
        <span className="font-serif text-[1.375rem] leading-none font-medium tracking-[-0.01em]">Tutor</span>
      </div>

      <WorldSwitcher className="px-1" />

      <nav aria-label="Sections" className="flex min-h-0 flex-1 flex-col">
        <div
          ref={list}
          onKeyDown={onKeyDown}
          onBlur={onBlur}
          className="relative flex min-h-0 flex-1 flex-col gap-0.5"
        >
          <span
            ref={pill}
            aria-hidden="true"
            className={cn(
              "bg-world-soft ring-hairline absolute top-0 left-0 rounded-[10px] opacity-0 ring-1 ring-inset",
              "before:bg-world before:absolute before:top-2.5 before:bottom-2.5 before:left-0 before:w-[3px] before:rounded-full",
              ready && "opacity-100",
            )}
          />
          {sections.map((s, i) => renderItem(s, i))}
          <div className="mt-auto pt-2">{renderItem(settings, sections.length)}</div>
        </div>
      </nav>
    </aside>
  )
}
