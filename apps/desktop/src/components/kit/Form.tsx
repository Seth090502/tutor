import { useId, useRef, useState, type ComponentProps, type KeyboardEvent, type ReactNode } from "react"
import { Minus, Plus } from "lucide-react"

import { cn } from "@/lib/utils"
import { commitInt, radioTarget } from "./form-logic"

const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

export type SegmentedOption<T> = { value: T; label: ReactNode; ariaLabel?: string }

type SegmentedProps<T> = {
  value: T | null
  options: SegmentedOption<T>[]
  onChange: (v: T) => void
  /** Choosing the chosen option again clears it (for optional answers). */
  onClear?: () => void
  /** Accessible name of the group. */
  label: string
  className?: string
}

/**
 * Equal-width radio group with a thumb that slides to the chosen option. Arrow keys, Home and End move
 * and choose. The thumb hides while nothing is chosen.
 */
export function Segmented<T extends string | number | boolean>({ value, options, onChange, onClear, label, className }: SegmentedProps<T>) {
  const refs = useRef<(HTMLButtonElement | null)[]>([])
  const at = options.findIndex((o) => o.value === value)
  const count = options.length

  const onKey = (e: KeyboardEvent) => {
    const next = radioTarget(e.key, at, count)
    if (next === null) return
    e.preventDefault()
    onChange(options[next].value)
    refs.current[next]?.focus()
  }

  return (
    <div
      data-slot="kit-segmented"
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
          transition: `transform var(--dur-slow) var(--ease-spring), opacity var(--dur-base) ease`,
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

type StepperProps = {
  value: number
  min: number
  max: number
  onChange: (v: number) => void
  /** Accessible name; the buttons read "Decrease {label}" and "Increase {label}". */
  label: string
}

/** Whole-number stepper: minus, a typeable value, plus. A typed value applies once it is valid and clamps on blur. */
export function Stepper({ value, min, max, onChange, label }: StepperProps) {
  const [draft, setDraft] = useState<string | null>(null)
  const step = (d: number) => onChange(Math.min(max, Math.max(min, value + d)))
  const btn = cn(
    "text-ink-2 hover:text-foreground hover:bg-accent grid size-8 place-items-center rounded-lg transition-colors disabled:opacity-40 disabled:hover:bg-transparent",
    FOCUS,
  )
  return (
    <div
      data-slot="kit-stepper"
      className="border-hairline-strong bg-card inline-flex h-10 w-fit items-center rounded-xl border px-0.5"
      role="group"
      aria-label={label}
    >
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
          "focus-visible:outline-world rounded-md outline-none focus-visible:outline-2 focus-visible:outline-offset-1",
        )}
      />
      <button type="button" className={btn} aria-label={`Increase ${label}`} disabled={value >= max} onClick={() => step(1)}>
        <Plus className="size-4" aria-hidden />
      </button>
    </div>
  )
}

type SwitchRowProps = {
  title: string
  detail: string
  checked: boolean
  onChange: (v: boolean) => void
  /** Shown before the On/Off word, e.g. a chip that says what turning this on costs. */
  note?: ReactNode
}

/** A setting row: title, one line of detail, and an on/off switch with its word. */
export function SwitchRow({ title, detail, checked, onChange, note }: SwitchRowProps) {
  const id = useId()
  return (
    <div data-slot="kit-switch-row" className="flex items-center gap-4 py-3.5">
      <div className="min-w-0 flex-1">
        <label htmlFor={id} className="text-[15px] font-medium">
          {title}
        </label>
        <p className="text-ink-2 mt-0.5 text-[13px] leading-snug">{detail}</p>
      </div>
      {note}
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
            transition: `transform var(--dur-slow) var(--ease-spring), background-color var(--dur-base) ease`,
          }}
        />
      </button>
    </div>
  )
}

/** Single-line text input in mono: ids, paths and other things that are not prose. */
export function TextField({ className, type = "text", ...props }: ComponentProps<"input">) {
  return (
    <input
      data-slot="kit-text-field"
      type={type}
      className={cn(
        "border-hairline-strong placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 h-9 min-w-0 rounded-lg border bg-transparent px-3 font-mono text-[13px] tracking-[0.02em] outline-none focus-visible:ring-3",
        className,
      )}
      {...props}
    />
  )
}

/**
 * A field for a secret. Never autofilled, never spell-checked, and the WebView's own show-password eye is
 * hidden: a typed key is not displayed either.
 */
export function PasswordField({ className, ...props }: Omit<ComponentProps<"input">, "type">) {
  return <TextField type="password" autoComplete="off" spellCheck={false} className={cn("[&::-ms-reveal]:hidden", className)} {...props} />
}
