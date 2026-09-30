// Small input pieces shared by the flow steps: a long-text field and the 1-4 keys.
import { useEffect, useId, type ReactNode } from "react"

import { KeycapGroup } from "@/components/kit"
import { Textarea } from "@/components/ui/textarea"
import { cn } from "@/lib/utils"

import type { Confidence } from "./types"
import "./reading.css"

export const CONFIDENCE_LABELS = ["Guess", "Unsure", "Fairly sure", "Certain"] as const
export const COVERAGE_LABELS = ["Little", "Some", "Most", "Nearly all"] as const

const isTyping = (t: EventTarget | null) => {
  const el = t as { tagName?: string; isContentEditable?: boolean } | null
  return !!el && (el.isContentEditable === true || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName ?? ""))
}

/**
 * Four keycaps, 1 to 4. The keys 1-4 choose while focus is not in a text field. The kit group's A-D hotkeys
 * are switched off: a letter must never pick a confidence here.
 */
export function NumberKeys({
  label,
  labels,
  value,
  onChange,
  disabled,
}: {
  label: string
  labels: readonly [string, string, string, string]
  value: Confidence | null
  onChange: (v: Confidence) => void
  disabled?: boolean
}) {
  useEffect(() => {
    if (disabled) return
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.repeat || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return
      const n = "1234".indexOf(e.key)
      if (n >= 0) onChange((n + 1) as Confidence)
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [disabled, onChange])

  return (
    <KeycapGroup
      aria-label={label}
      className="rd-keys"
      active={false}
      disabled={disabled}
      value={value === null ? null : String(value)}
      onChange={(id) => onChange(Number(id) as Confidence)}
      choices={labels.map((l, i) => ({ id: String(i + 1), legend: String(i + 1), label: l }))}
    />
  )
}

/** A labelled long-text field. Ctrl or Cmd with Enter submits. */
export function LongText({
  label,
  hint,
  value,
  onChange,
  onSubmit,
  disabled,
  placeholder,
  className,
  rows = 5,
}: {
  label: ReactNode
  hint?: ReactNode
  value: string
  onChange: (v: string) => void
  onSubmit?: () => void
  disabled?: boolean
  placeholder?: string
  className?: string
  rows?: number
}) {
  const id = useId()
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <label htmlFor={id} className="text-[14px] font-medium">
        {label}
      </label>
      {hint ? (
        <p id={`${id}-hint`} className="text-ink-2 text-[13px] leading-relaxed">
          {hint}
        </p>
      ) : null}
      <Textarea
        id={id}
        rows={rows}
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        aria-describedby={hint ? `${id}-hint` : undefined}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (onSubmit && e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault()
            onSubmit()
          }
        }}
        className="bg-card dark:bg-card min-h-28 font-serif text-[17px] leading-relaxed md:text-[17px]"
      />
    </div>
  )
}

export function StepError({ message }: { message: string | null }) {
  return message ? (
    <p role="alert" className="text-bad font-mono text-[12px] leading-relaxed break-words">
      {message} Your text is still here; send it again.
    </p>
  ) : null
}
