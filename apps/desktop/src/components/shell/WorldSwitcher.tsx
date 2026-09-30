// Subject world switcher: a compact segmented radio group. The selected segment is a solid
// world-colour fill that glides between segments on the shared spring.

import { useRef, type ComponentType, type KeyboardEvent } from "react"
import { Atom, BookOpen, CircleDot, Hexagon, Pi } from "lucide-react"

import { cn } from "@/lib/utils"
import { WORLDS, useWorld, worldMeta, type World } from "@/lib/world"

import { useGlide } from "./useGlide"

const GLYPHS: Record<World, ComponentType<{ className?: string }>> = {
  neutral: CircleDot,
  math: Pi,
  english: BookOpen,
  science: Atom,
  chemistry: Hexagon,
}

export function WorldSwitcher({ className }: { className?: string }) {
  const { world, setWorld } = useWorld()
  const meta = worldMeta(world)
  const activeIndex = WORLDS.findIndex((w) => w.id === world)

  const group = useRef<HTMLDivElement>(null)
  const items = useRef<(HTMLButtonElement | null)[]>([])
  const pill = useRef<HTMLSpanElement>(null)
  const ready = useGlide("x", activeIndex, items, pill, group)

  const choose = (index: number) => {
    const next = (index + WORLDS.length) % WORLDS.length
    setWorld(WORLDS[next].id)
    items.current[next]?.focus()
  }

  const onKeyDown = (e: KeyboardEvent) => {
    const keys: Record<string, number> = {
      ArrowRight: activeIndex + 1,
      ArrowDown: activeIndex + 1,
      ArrowLeft: activeIndex - 1,
      ArrowUp: activeIndex - 1,
      Home: 0,
      End: WORLDS.length - 1,
    }
    if (!(e.key in keys)) return
    e.preventDefault()
    choose(keys[e.key])
  }

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="text-ink-3 flex items-center gap-2 font-mono text-[11px] tracking-[0.18em] uppercase">
        <span className="bg-hairline-strong h-px w-4" aria-hidden="true" />
        <span id="world-switcher-label">Subject</span>
      </div>

      <div
        ref={group}
        role="radiogroup"
        aria-labelledby="world-switcher-label"
        onKeyDown={onKeyDown}
        className="border-hairline bg-sidebar-accent relative grid grid-cols-5 rounded-xl border p-[3px]"
      >
        <span
          ref={pill}
          aria-hidden="true"
          className={cn(
            "bg-world absolute top-[3px] left-[3px] rounded-[9px] opacity-0 shadow-[0_1px_0_0_rgb(255_255_255/0.22)_inset,0_2px_6px_-2px_color-mix(in_oklab,var(--world)_60%,transparent)]",
            ready && "opacity-100",
          )}
        />
        {WORLDS.map((w, i) => {
          const Glyph = GLYPHS[w.id]
          const selected = w.id === world
          return (
            <button
              key={w.id}
              ref={(el) => {
                items.current[i] = el
              }}
              type="button"
              role="radio"
              aria-checked={selected}
              aria-label={w.label}
              title={w.label}
              tabIndex={selected ? 0 : -1}
              onClick={() => setWorld(w.id)}
              className={cn(
                "ease-spring focus-visible:outline-world relative z-10 flex h-8 items-center justify-center rounded-[9px] outline-offset-2 transition-[color,transform] duration-(--dur-base) focus-visible:outline-2 active:scale-95",
                selected ? "text-world-ink" : "text-ink-2 hover:text-foreground",
              )}
            >
              <Glyph className="size-4" />
            </button>
          )
        })}
      </div>

      <p className="text-ink-2 min-h-8 text-xs leading-4" aria-live="polite">
        <span className="text-foreground font-medium">{meta.label}</span>
        <span className="block">{meta.description}</span>
      </p>
    </div>
  )
}
