// Small pieces shared by the four tabs: a segmented choice, notices, state blocks, a play button, the
// coverage meter and the copyable command. Learning panes stay neutral; the world accent appears only on
// chrome (the active tab, focus rings) and the functional signals noted below.
import { useEffect, useId, useRef, useState, type ComponentType, type ReactNode } from "react"
import { Check, Copy, Info, Loader2, Square, TriangleAlert, Volume2, X, type LucideIcon } from "lucide-react"

import { Eyebrow, Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { COVERAGE_HIGH, COVERAGE_LOW, coverageBand, coverageWords } from "./reading"
import "./languages.css"

export const FIELD =
  "border-hairline-strong bg-background text-foreground placeholder:text-ink-3 w-full rounded-lg border px-3 py-2 text-[16px] leading-normal outline-offset-2 focus-visible:outline-2 focus-visible:outline-world disabled:opacity-60"

export const PILL =
  "border-hairline-strong text-ink-2 hover:bg-world-soft hover:text-foreground focus-visible:outline-world aria-checked:bg-foreground aria-checked:text-background aria-checked:border-foreground rounded-full border px-3.5 py-1.5 text-sm outline-offset-2 transition-colors duration-(--dur-quick) focus-visible:outline-2 disabled:opacity-50"

type Option<T extends string> = { id: T; label: ReactNode; hint?: string; lang?: string }

/** A radio group drawn as pills, with arrow-key movement (roving tabindex). */
export function Segmented<T extends string>({
  value,
  onChange,
  options,
  label,
  className,
  disabled,
  compact,
}: {
  value: T
  onChange: (v: T) => void
  options: readonly Option<T>[]
  label: string
  className?: string
  disabled?: boolean
  compact?: boolean
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([])
  const move = (e: React.KeyboardEvent, i: number) => {
    const delta = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0
    if (!delta || disabled) return
    e.preventDefault()
    const next = (i + delta + options.length) % options.length
    onChange(options[next].id)
    refs.current[next]?.focus()
  }
  return (
    <div role="radiogroup" aria-label={label} className={cn("flex flex-wrap", compact ? "gap-1.5" : "gap-2", className)}>
      {options.map((o, i) => (
        <button
          key={o.id}
          ref={(el) => {
            refs.current[i] = el
          }}
          type="button"
          role="radio"
          lang={o.lang}
          aria-checked={value === o.id}
          tabIndex={value === o.id ? 0 : -1}
          disabled={disabled}
          title={o.hint}
          className={cn(PILL, compact && "px-3 py-1 text-[13px]")}
          onClick={() => onChange(o.id)}
          onKeyDown={(e) => move(e, i)}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}

/** A quiet call-out. The glyph and the words carry the meaning; the tint is secondary. */
export function Notice({
  tone = "info",
  children,
  className,
}: {
  tone?: "info" | "warn"
  children: ReactNode
  className?: string
}) {
  const Icon = tone === "warn" ? TriangleAlert : Info
  return (
    <div
      role={tone === "warn" ? "alert" : "note"}
      className={cn(
        "flex items-start gap-3 rounded-xl border px-4 py-3 text-sm leading-relaxed",
        tone === "warn" ? "border-warn/45 bg-warn/10" : "border-hairline-strong bg-card",
        className,
      )}
    >
      <Icon aria-hidden className={cn("mt-0.5 size-4 shrink-0", tone === "warn" ? "text-warn" : "text-ink-2")} />
      <div className="min-w-0 break-words">{children}</div>
    </div>
  )
}

/** A full-width state (offline, empty, blocked): a glyph, a title, a line and optional actions. */
export function StateBlock({
  icon: Icon,
  title,
  children,
  actions,
  className,
}: {
  icon: LucideIcon | ComponentType<{ className?: string; strokeWidth?: number }>
  title: string
  children?: ReactNode
  actions?: ReactNode
  className?: string
}) {
  return (
    <Panel role="status" className={cn("lg-enter mx-auto flex max-w-xl flex-col items-center gap-3 py-10 text-center", className)}>
      <Icon aria-hidden className="text-ink-3 size-7" strokeWidth={1.5} />
      <p className="font-serif text-xl tracking-[-0.01em]">{title}</p>
      {children ? <div className="text-ink-2 max-w-[46ch] text-sm leading-relaxed text-pretty">{children}</div> : null}
      {actions ? <div className="mt-1 flex flex-wrap justify-center gap-2">{actions}</div> : null}
    </Panel>
  )
}

/** A shell command to copy. The screen shows it; it never runs it. */
export function CopyCommand({ command }: { command: string }) {
  const [copied, setCopied] = useState(false)
  useEffect(() => {
    if (!copied) return
    const t = window.setTimeout(() => setCopied(false), 1800)
    return () => window.clearTimeout(t)
  }, [copied])
  return (
    <div className="border-hairline-strong bg-background flex w-full max-w-full items-center gap-2 rounded-lg border py-1.5 pr-1.5 pl-3 text-left">
      <code className="text-foreground min-w-0 flex-1 font-mono text-[12px] leading-relaxed break-all">{command}</code>
      <Button
        variant="outline"
        size="sm"
        onClick={() => {
          navigator.clipboard?.writeText(command).then(
            () => setCopied(true),
            () => {},
          )
        }}
      >
        {copied ? <Check aria-hidden /> : <Copy aria-hidden />}
        {copied ? "Copied" : "Copy"}
      </Button>
    </div>
  )
}

/** Plays spoken text. Disabled with a reason when audio is off; shows loading, playing and errors. */
export function PlayButton({
  label,
  status,
  onPlay,
  onStop,
  disabledReason,
  size = "sm",
  variant = "outline",
  className,
}: {
  label: string
  status: "idle" | "loading" | "playing" | "error"
  onPlay: () => void
  onStop?: () => void
  disabledReason?: string | null
  size?: "sm" | "default" | "icon"
  variant?: "outline" | "ghost" | "secondary"
  className?: string
}) {
  const off = Boolean(disabledReason)
  const busy = status === "loading"
  const playing = status === "playing"
  return (
    <Button
      variant={variant}
      size={size}
      className={className}
      title={disabledReason ?? undefined}
      aria-label={size === "icon" ? label : undefined}
      disabled={off || busy}
      onClick={playing && onStop ? onStop : onPlay}
    >
      {busy ? <Loader2 aria-hidden className="animate-spin" /> : playing ? <Square aria-hidden /> : <Volume2 aria-hidden />}
      {size === "icon" ? null : playing ? "Stop" : label}
    </Button>
  )
}

/** Correct / not quite, as a glyph plus a word. */
export function Verdict({ ok, word }: { ok: boolean | null; word: string }) {
  const Icon = ok === true ? Check : ok === false ? X : Info
  const tone = ok === true ? "border-ok/40 bg-ok/10 [&>svg]:text-ok" : ok === false ? "border-bad/40 bg-bad/10 [&>svg]:text-bad" : "border-hairline-strong bg-muted [&>svg]:text-ink-2"
  return (
    <span className={cn("inline-flex items-center gap-2 rounded-full border px-3 py-1 text-sm font-medium", tone)}>
      <Icon aria-hidden className="size-4" strokeWidth={2.5} />
      {word}
    </span>
  )
}

/**
 * Known-word coverage with the 95 to 98% target band marked. The fill is neutral; the target band is the
 * functional signal, marked by a labelled bracket as well as by tint.
 */
export function CoverageMeter({ coverage, unknown }: { coverage: number; unknown: number }) {
  const id = useId()
  const pct = Math.round(coverage * 1000) / 10
  const band = coverageBand(coverage)
  // Show 80 to 100%: below 80 the text is not graded for this learner anyway.
  const lo = 0.8
  const pos = (v: number) => `${Math.max(0, Math.min(1, (v - lo) / (1 - lo))) * 100}%`
  return (
    <div>
      <div className="flex items-baseline justify-between gap-4">
        <Eyebrow id={id}>Known-word coverage</Eyebrow>
        <span className="text-[28px] leading-none font-light tracking-[-0.03em] tabular-nums">{pct.toFixed(1)}%</span>
      </div>
      <div
        role="meter"
        aria-labelledby={id}
        aria-valuemin={80}
        aria-valuemax={100}
        aria-valuenow={pct}
        aria-valuetext={`${pct.toFixed(1)} percent. ${coverageWords(coverage, unknown)}`}
        className="lg-meter mt-3"
      >
        <span className="lg-meter-target" style={{ left: pos(COVERAGE_LOW), width: `calc(${pos(COVERAGE_HIGH)} - ${pos(COVERAGE_LOW)})` }} aria-hidden />
        <span className="lg-meter-fill" data-band={band} style={{ width: pos(coverage) }} aria-hidden />
      </div>
      <div className="text-ink-3 relative mt-1.5 h-4 font-mono text-[11px]" aria-hidden>
        <span className="absolute left-0">80%</span>
        <span className="absolute -translate-x-1/2" style={{ left: pos(COVERAGE_LOW) }}>95</span>
        <span className="absolute -translate-x-1/2" style={{ left: pos(COVERAGE_HIGH) }}>98</span>
        <span className="absolute right-0">100%</span>
      </div>
      <p className="text-ink-2 mt-2 text-sm leading-relaxed">{coverageWords(coverage, unknown)}</p>
    </div>
  )
}
