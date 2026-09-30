import { useEffect, useRef, type ComponentProps, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react"
import { Check, X } from "lucide-react"

import { cn } from "@/lib/utils"
import { hotkeyIndex, isEditableTarget, stepIndex, type HotkeyMode } from "./keycap-logic"
import "./kit.css"
import "./keycap.css"

export type { HotkeyMode }
export type KeycapStatus = "correct" | "wrong"

type KeycapProps = Omit<ComponentProps<"button">, "children"> & {
  /** The legend printed on the key, e.g. "A". */
  legend: string
  /** Chosen: the key stays down and its LED is lit. */
  latched?: boolean
  /** Result: face tint plus a check or cross mark, never tint alone. */
  status?: KeycapStatus
  /** Text shown beside the key; the whole row is the button. */
  children?: ReactNode
}

/**
 * A layered keycap: contact shadow, base plate, cap with skirt, top face with bevels and a
 * letterpress legend. About 4px of travel with a slight shrink; the spring feel comes from
 * linear() easings in keycap.css. Instant under reduced motion (durations use --dur-*).
 */
export function Keycap({ legend, latched = false, status, children, className, ...props }: KeycapProps) {
  return (
    <button
      type="button"
      data-slot="kit-keycap"
      data-latched={latched ? "" : undefined}
      data-status={status}
      aria-pressed={props.role === "radio" ? undefined : latched}
      className={cn("kc", className)}
      {...props}
    >
      <span className="kc-key">
        <span aria-hidden className="kc-contact" />
        <span aria-hidden className="kc-base" />
        <span aria-hidden className="kc-cap">
          <span className="kc-skirt" />
          <span className="kc-face">
            <span className="kc-legend">{legend}</span>
            <span className="kit-led kc-led" data-tone="world" />
            {status === "correct" ? <Check className="kc-mark" strokeWidth={3} /> : null}
            {status === "wrong" ? <X className="kc-mark" strokeWidth={3} /> : null}
          </span>
        </span>
      </span>
      {children ? <span className="kc-label">{children}</span> : null}
      {status ? <span className="sr-only">{status === "correct" ? "Correct" : "Incorrect"}</span> : null}
    </button>
  )
}

export type KeycapChoice = {
  id: string
  /** Legend on the key; defaults to A, B, C, D by position. */
  legend?: string
  label: ReactNode
  status?: KeycapStatus
}

type KeycapGroupProps = {
  choices: KeycapChoice[]
  /** The chosen id, or null. */
  value: string | null
  /** Called with the chosen id. Optional with `sendOnPress`, where a press goes to `onSubmit` and the parent
   *  shows the choice by setting `value` itself. */
  onChange?: (id: string) => void
  /** Called with the chosen id on Enter, or at once on a press when `sendOnPress` is set. */
  onSubmit?: (id: string) => void
  /**
   * Choosing sends: a click or hotkey calls `onChange` and then `onSubmit`, and the arrow keys only move
   * focus (they do not choose, so moving never sends). For answers that go straight to a tutor.
   */
  sendOnPress?: boolean
  /** Which keys choose: A-D and 1-4 ("all", the default), A-D only ("letters"), or none, for a caller that
   *  owns the keyboard. Enter still submits unless `sendOnPress` is set. */
  hotkeys?: HotkeyMode
  /** Hotkeys (per `hotkeys`, and Enter) listen on the window while true. */
  active?: boolean
  /** Dimmed and inert. A graded group (any choice has a status) is read-only but not dimmed. */
  disabled?: boolean
  "aria-label"?: string
  className?: string
}

const DEFAULT_LEGENDS = ["A", "B", "C", "D"]

/**
 * Multiple choice as a radio group of keycaps. Hotkeys A-D and 1-4 choose, Enter submits, arrow
 * keys move and choose, Tab enters on the chosen key (roving tabindex).
 */
export function KeycapGroup({
  choices,
  value,
  onChange,
  onSubmit,
  sendOnPress = false,
  hotkeys = "all",
  active = true,
  disabled = false,
  className,
  "aria-label": ariaLabel = "Answer choices",
}: KeycapGroupProps) {
  const rootRef = useRef<HTMLDivElement>(null)
  const keyRefs = useRef<(HTMLButtonElement | null)[]>([])
  const locked = disabled || choices.some((c) => c.status)

  // Latest props for the window listener, so it subscribes once per `active` change.
  const latest = useRef({ choices, value, onChange, onSubmit, locked, sendOnPress })
  latest.current = { choices, value, onChange, onSubmit, locked, sendOnPress }

  /** `press`: a click or hotkey, which sends when `sendOnPress`. Arrow keys only choose. */
  const choose = (i: number, press = false) => {
    const s = latest.current
    const c = s.choices[i]
    if (!c) return
    s.onChange?.(c.id)
    if (press && s.sendOnPress) s.onSubmit?.(c.id)
    keyRefs.current[i]?.focus()
  }

  useEffect(() => {
    if (!active) return
    const onKeyDown = (e: KeyboardEvent) => {
      const s = latest.current
      if (s.locked || e.defaultPrevented || e.repeat || e.ctrlKey || e.metaKey || e.altKey) return
      if (isEditableTarget(e.target)) return

      if (e.key === "Enter") {
        if (s.sendOnPress) return
        const target = e.target as Element | null
        const elsewhere = target?.closest?.("button, a, [role=button], [role=link]")
        if (s.value === null || !s.onSubmit || (elsewhere && !rootRef.current?.contains(elsewhere))) return
        e.preventDefault()
        s.onSubmit(s.value)
        return
      }
      const i = hotkeyIndex(e.key, s.choices.length, hotkeys)
      if (i === null) return
      e.preventDefault()
      choose(i, true)
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [active, hotkeys, sendOnPress])

  const onGroupKeyDown = (e: ReactKeyboardEvent) => {
    if (locked) return
    const delta = e.key === "ArrowDown" || e.key === "ArrowRight" ? 1 : e.key === "ArrowUp" || e.key === "ArrowLeft" ? -1 : 0
    if (!delta) return
    e.preventDefault()
    const at = keyRefs.current.findIndex((k) => k === document.activeElement)
    const next = stepIndex(at < 0 ? 0 : at, delta, choices.length)
    if (sendOnPress) keyRefs.current[next]?.focus()
    else choose(next)
  }

  const chosen = choices.findIndex((c) => c.id === value)

  return (
    <div
      ref={rootRef}
      role="radiogroup"
      aria-label={ariaLabel}
      aria-readonly={locked && !disabled ? true : undefined}
      onKeyDown={onGroupKeyDown}
      className={cn("flex flex-col items-start gap-1.5", className)}
    >
      {choices.map((c, i) => (
        <Keycap
          key={c.id}
          ref={(el: HTMLButtonElement | null) => {
            keyRefs.current[i] = el
          }}
          role="radio"
          aria-checked={value === c.id}
          tabIndex={chosen === i || (chosen < 0 && i === 0) ? 0 : -1}
          legend={c.legend ?? DEFAULT_LEGENDS[i] ?? String(i + 1)}
          latched={value === c.id}
          status={c.status}
          disabled={disabled}
          aria-disabled={locked && !disabled ? true : undefined}
          onClick={() => {
            if (!locked) choose(i, true)
          }}
        >
          {c.label}
        </Keycap>
      ))}
    </div>
  )
}
