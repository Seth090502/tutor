import { cn } from "@/lib/utils"
import { MarkerGlyph } from "../charts/LineChart"
import type { MarkerShape } from "../charts/geometry"

type DecisionBadgeProps = {
  /** Same shape the chart draws for this state, so the badge and the marker match. */
  shape: MarkerShape
  /** The word: the shape and the word carry the meaning, colour is secondary. */
  label: string
  /** Any CSS colour for the shape. */
  color: string
  className?: string
}

/** Outlined pill with a marker shape and a word, e.g. KEEP or REVERT. Never rests on colour alone. */
export function DecisionBadge({ shape, label, color, className }: DecisionBadgeProps) {
  return (
    <span
      data-slot="kit-decision-badge"
      className={cn(
        "border-hairline-strong text-foreground inline-flex shrink-0 items-center gap-2 rounded-full border px-2.5 py-1.5 font-mono text-[11px] leading-none tracking-[0.12em] uppercase",
        className,
      )}
    >
      <svg width="12" height="12" viewBox="-6 -6 12 12" aria-hidden="true">
        <MarkerGlyph shape={shape} color={color} size={3.6} filled />
      </svg>
      {label}
    </span>
  )
}
