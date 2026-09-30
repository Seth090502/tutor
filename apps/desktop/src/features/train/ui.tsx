// Small form controls for the Train screen: a segmented control, a stepper, a switch row, a key picker.
// Styled from the C5 tokens; the kit has no form controls yet (reported as a gap).
import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react"
import { Minus, Plus } from "lucide-react"

import { DeltaChip, Eyebrow, Kbd } from "@/components/kit"
import { cn } from "@/lib/utils"
import { keyLabel } from "./parts"
import { commitInt } from "./view"

const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"
const SPRING = "cubic-bezier(.3,1.35,.4,1)"

/** A labelled control cell: mono eyebrow above the control. */
export function Field({ label, hint, children, className }: { label: string; hint?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <div className={cn("flex min-w-0 flex-col gap-2.5", className)}>
      <Eyebrow>{label}</Eyebrow>
      {children}
      {hint ? <p className="text-ink-3 text-xs leading-snug">{hint}</p> : null}
    </div>
  )
}

export type SegOption<T> = { value: T; label: ReactNode; ariaLabel?: string }

/**
 * Equal-width radio group with a sliding thumb. Arrow keys, Home and End move and choose. With
 * `onClear`, choosing the chosen option again clears it (for optional answers).
 */
export function Segmented<T extends string | number | boolean>({
  value,
  options,
  onChange,
  onClear,
  label,
  className,
}: {
  value: T | null
  options: SegOption<T>[]
  onChange: (v: T) => void
  onClear?: () => void
  label: string
  className?: string
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([])
  const at = options.findIndex((o) => o.value === value)
  const count = options.length

  const onKey = (e: KeyboardEvent) => {
    const cur = at < 0 ? 0 : at
    let next: number | null = null
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (cur + 1) % count
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = (cur - 1 + count) % count
    else if (e.key === "Home") next = 0
    else if (e.key === "End") next = count - 1
    if (next === null) return
    e.preventDefault()
    onChange(options[next].value)
    refs.current[next]?.focus()
  }

  return (
    <div
      role="radiogroup"
      aria-label={label}
      onKeyDown={onKey}
      className={cn("bg-muted border-hairline relative inline-grid auto-cols-fr grid-flow-col rounded-full border p-0.5", className)}
    >
      <span
        aria-hidden
        className="bg-card border-hairline-strong absolute inset-y-0.5 left-0.5 rounded-full border shadow-[0_1px_0_color-mix(in_oklab,var(--ink)_8%,transparent)]"
        style={{
          width: `calc((100% - 4px) / ${count})`,
          transform: `translateX(${Math.max(at, 0) * 100}%)`,
          opacity: at < 0 ? 0 : 1,
          transition: `transform var(--dur-slow) ${SPRING}, opacity var(--dur-base) ease`,
        }}
      />
      {options.map((o, i) => (
        <button
          key={String(o.value)}
          ref={(el) => {
            refs.current[i] = el
          }}
          type="button"
          role="radio"
          aria-checked={at === i}
          aria-label={o.ariaLabel}
          tabIndex={at === i || (at < 0 && i === 0) ? 0 : -1}
          onClick={() => (at === i && onClear ? onClear() : onChange(o.value))}
          className={cn(
            "relative rounded-full px-4 py-1.5 text-sm whitespace-nowrap transition-colors",
            FOCUS,
            at === i ? "text-foreground font-medium" : "text-ink-2 hover:text-foreground",
          )}
          style={{ transitionDuration: "var(--dur-base)" }}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}

/** Whole-number stepper: minus, a typeable value, plus. Typed values apply when valid and clamp on blur. */
export function Stepper({
  value,
  min,
  max,
  onChange,
  label,
}: {
  value: number
  min: number
  max: number
  onChange: (v: number) => void
  label: string
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const step = (d: number) => onChange(Math.min(max, Math.max(min, value + d)))
  const btn = cn(
    "text-ink-2 hover:text-foreground hover:bg-accent grid size-8 place-items-center rounded-lg transition-colors disabled:opacity-40 disabled:hover:bg-transparent",
    FOCUS,
  )
  return (
    <div className="border-hairline-strong bg-card inline-flex h-10 w-fit items-center rounded-xl border px-0.5" role="group" aria-label={label}>
      <button type="button" className={btn} aria-label={`Decrease ${label}`} disabled={value <= min} onClick={() => step(-1)}>
        <Minus className="size-4" aria-hidden />
      </button>
      <input
        type="number"
        inputMode="numeric"
        aria-label={label}
        min={min}
        max={max}
        value={draft ?? String(value)}
        onChange={(e) => {
          setDraft(e.target.value)
          const v = commitInt(e.target.value, min, max)
          if (v !== null && String(v) === e.target.value.trim()) onChange(v)
        }}
        onBlur={() => {
          const v = draft === null ? null : commitInt(draft, min, max)
          if (v !== null) onChange(v)
          setDraft(null)
        }}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        className={cn(
          "w-11 bg-transparent text-center font-mono text-[15px] tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none",
          "rounded-md outline-none focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-world",
        )}
      />
      <button type="button" className={btn} aria-label={`Increase ${label}`} disabled={value >= max} onClick={() => step(1)}>
        <Plus className="size-4" aria-hidden />
      </button>
    </div>
  )
}

/** A setting row: title, one line of detail, an on/off switch, and the non-comparable mark when it is on. */
export function SwitchRow({
  title,
  detail,
  checked,
  onChange,
  breaksComparable,
}: {
  title: string
  detail: string
  checked: boolean
  onChange: (v: boolean) => void
  breaksComparable?: boolean
}) {
  const id = useId()
  return (
    <div className="flex items-center gap-4 py-3.5">
      <div className="min-w-0 flex-1">
        <label htmlFor={id} className="text-[15px] font-medium">
          {title}
        </label>
        <p className="text-ink-2 mt-0.5 text-[13px] leading-snug">{detail}</p>
      </div>
      {checked && breaksComparable ? <DeltaChip tone="warn" text="Not comparable" className="max-[560px]:hidden" /> : null}
      <span className="text-ink-3 w-7 text-right font-mono text-[11px] tracking-[0.1em] uppercase" aria-hidden>
        {checked ? "On" : "Off"}
      </span>
      <button
        id={id}
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative h-6 w-10 shrink-0 rounded-full border transition-colors",
          FOCUS,
          checked ? "border-world bg-world" : "border-hairline-strong bg-muted",
        )}
        style={{ transitionDuration: "var(--dur-base)" }}
      >
        <span
          aria-hidden
          className={cn("absolute top-[2px] left-[2px] size-[18px] rounded-full shadow-sm", checked ? "bg-world-ink" : "bg-ink-3")}
          style={{
            transform: checked ? "translateX(16px)" : "none",
            transition: `transform var(--dur-slow) ${SPRING}, background-color var(--dur-base) ease`,
          }}
        />
      </button>
    </div>
  )
}

/** Click, then press a key to remap. Escape cancels. */
export function KeyPicker({ label, code, onPick }: { label: string; code: string; onPick: (code: string) => void }) {
  const [listening, setListening] = useState(false)
  useEffect(() => {
    if (!listening) return
    const onKey = (e: globalThis.KeyboardEvent) => {
      e.preventDefault()
      setListening(false)
      if (e.code !== "Escape") onPick(e.code)
    }
    window.addEventListener("keydown", onKey, { once: true })
    return () => window.removeEventListener("keydown", onKey)
  }, [listening, onPick])
  return (
    <button
      type="button"
      onClick={() => setListening(true)}
      aria-label={`${label} key: ${keyLabel(code)}. Activate, then press a new key. Escape cancels.`}
      className={cn(
        "border-hairline-strong bg-card flex h-10 items-center gap-3 rounded-xl border pr-1.5 pl-3.5 text-sm transition-colors",
        FOCUS,
        listening && "border-world",
      )}
    >
      <span className="text-ink-2">{label}</span>
      <Kbd className="m-0 min-w-9 px-2 py-1 text-[13px]">{listening ? "press a key" : keyLabel(code)}</Kbd>
    </button>
  )
}
