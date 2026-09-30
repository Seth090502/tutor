// Session setup: mode, starting level, block count, keys, lures, in-trial feedback, the rule.
import { useEffect, useState, type ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { ruleText } from "./engine/adapt"
import { DEFAULT_SETTINGS, TRANSFER_SETTINGS, isComparable, transferLevel } from "./engine/session"
import { Section, keyLabel } from "./parts"
import type { Mode, Settings } from "./types"

const MODES: { id: Mode; label: string; hint: string }[] = [
  { id: "dual", label: "Dual", hint: "Grid position and spoken letter" },
  { id: "position", label: "Position only", hint: "Visual only" },
  { id: "letter", label: "Letter only", hint: "Audio only" },
  { id: "transfer", label: "Transfer check", hint: "Measurement, every 4 weeks" },
]

export const TRANSFER_EVERY_DAYS = 28

/** Rough session length in minutes: trials plus rests. */
export const estimateMinutes = (s: Settings, n: number) =>
  Math.round((s.blocks * (20 + n) * s.trial_ms + (s.blocks - 1) * 10_000) / 60_000)

export function SetupPanel({
  mode,
  setMode,
  settings,
  setSettings,
  startN,
  setStartN,
  level,
  lastTransferAt,
  starting,
  error,
  onStart,
}: {
  mode: Mode
  setMode: (m: Mode) => void
  settings: Settings
  setSettings: (s: Settings) => void
  startN: number
  setStartN: (n: number) => void
  level: number
  lastTransferAt: number | null
  starting: boolean
  error: string | null
  onStart: () => void
}) {
  const transfer = mode === "transfer"
  const effective = transfer ? TRANSFER_SETTINGS : settings
  const n = transfer ? transferLevel(level) : startN
  const comparable = isComparable(mode, effective)
  const reasons = [
    effective.in_trial_feedback && "in-trial feedback is on",
    effective.lures && "lures are on",
    (effective.trial_ms !== DEFAULT_SETTINGS.trial_ms || effective.stimulus_ms !== DEFAULT_SETTINGS.stimulus_ms) &&
      "timing differs from the default",
    (effective.up_max_errors !== DEFAULT_SETTINGS.up_max_errors ||
      effective.down_min_errors !== DEFAULT_SETTINGS.down_min_errors) &&
      "the adaptation rule differs from the default",
  ].filter(Boolean)
  const set = (patch: Partial<Settings>) => setSettings({ ...settings, ...patch })
  const daysSince = lastTransferAt === null ? null : Math.floor((Date.now() / 1000 - lastTransferAt) / 86_400)

  return (
    <Section title="Session">
      <p className="text-ink-2 mb-4 text-sm">
        This trains the n-back task. Transfer to other skills is small or unproven; the Experiment tab tests it on you.
      </p>

      <fieldset className="mb-4">
        <legend className="text-ink-3 mb-2 text-xs font-medium">Mode</legend>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {MODES.map((m) => (
            <label
              key={m.id}
              className={cn(
                "border-hairline cursor-pointer rounded-lg border px-3 py-2 text-sm",
                mode === m.id && "border-world bg-world-soft",
              )}
            >
              <input
                type="radio"
                name="nback-mode"
                className="sr-only"
                checked={mode === m.id}
                onChange={() => setMode(m.id)}
              />
              <span className="block font-medium">{m.label}</span>
              <span className="text-ink-3 block text-xs">{m.hint}</span>
            </label>
          ))}
        </div>
      </fieldset>

      {transfer ? (
        <p className="text-ink-2 mb-4 text-sm">
          Measurement, not training: a dual n-back with shapes instead of grid positions and tones instead of letters, at
          a fixed n = {n} (your level {level} minus 1), {TRANSFER_SETTINGS.blocks} blocks, fixed settings, no adaptation
          and no feedback. Take it about every 4 weeks and don't practise it in between.{" "}
          {daysSince === null
            ? "No check yet."
            : daysSince >= TRANSFER_EVERY_DAYS
              ? `Last check ${daysSince} days ago: due.`
              : `Last check ${daysSince} days ago: next due in ${TRANSFER_EVERY_DAYS - daysSince} days.`}
        </p>
      ) : (
        <div className="mb-4 grid grid-cols-2 gap-x-6 gap-y-3 text-sm sm:grid-cols-3">
          <Field label="Starting n">
            <NumberInput value={startN} min={1} max={9} onChange={setStartN} />
          </Field>
          <Field label="Blocks">
            <NumberInput value={settings.blocks} min={12} max={20} onChange={(blocks) => set({ blocks })} />
          </Field>
          <Field label="Keys">
            <div className="flex gap-2">
              <KeyPicker label="Position" code={settings.keys.position} onPick={(position) => position !== settings.keys.letter && set({ keys: { ...settings.keys, position } })} />
              <KeyPicker label="Letter" code={settings.keys.letter} onPick={(letter) => letter !== settings.keys.position && set({ keys: { ...settings.keys, letter } })} />
            </div>
          </Field>
          <Field label="Up at most errors">
            <NumberInput value={settings.up_max_errors} min={0} max={10} onChange={(up_max_errors) => set({ up_max_errors })} />
          </Field>
          <Field label="Down at errors">
            <NumberInput
              value={settings.down_min_errors}
              min={settings.up_max_errors + 1}
              max={20}
              onChange={(down_min_errors) => set({ down_min_errors })}
            />
          </Field>
          <div className="flex flex-col gap-1.5">
            <Check checked={settings.lures} onChange={(lures) => set({ lures })}>
              Lures (n±1 repeats, tagged)
            </Check>
            <Check checked={settings.in_trial_feedback} onChange={(in_trial_feedback) => set({ in_trial_feedback })}>
              In-trial feedback
            </Check>
          </div>
        </div>
      )}

      {!transfer && <p className="text-ink-3 mb-2 text-xs">Rule: {ruleText(settings)}</p>}
      <p className={cn("mb-4 text-xs", comparable ? "text-ink-3" : "text-warn")}>
        {comparable
          ? "Comparable: default task settings, so this session goes on the training curve."
          : `Not comparable (${reasons.join(", ")}): logged, but shown apart from the training curve.`}{" "}
        About {estimateMinutes(effective, n)} min. Suggested dose: 15-20 minutes, up to 3 sessions a week, after your due
        reviews.
      </p>

      <div className="flex items-center gap-3">
        <Button onClick={onStart} disabled={starting}>
          {starting ? "Loading sounds…" : transfer ? "Start transfer check" : "Start session"}
        </Button>
        <span className="text-ink-3 text-xs">Keep this window focused: leaving it voids the current block.</span>
      </div>
      {error && <p className="text-bad mt-2 text-sm">{error}</p>}
    </Section>
  )
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-ink-3 text-xs font-medium">{label}</span>
      {children}
    </label>
  )
}

function NumberInput({ value, min, max, onChange }: { value: number; min: number; max: number; onChange: (v: number) => void }) {
  return (
    <input
      type="number"
      className="border-hairline bg-background w-20 rounded-md border px-2 py-1 font-mono"
      value={value}
      min={min}
      max={max}
      onChange={(e) => {
        const v = Math.round(Number(e.target.value))
        if (Number.isFinite(v)) onChange(Math.min(max, Math.max(min, v)))
      }}
    />
  )
}

function Check({ checked, onChange, children }: { checked: boolean; onChange: (v: boolean) => void; children: ReactNode }) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      {children}
      {checked && <span className="text-warn text-xs">(not comparable)</span>}
    </label>
  )
}

/** Click, then press a key to remap. Escape cancels. */
function KeyPicker({ label, code, onPick }: { label: string; code: string; onPick: (code: string) => void }) {
  const [listening, setListening] = useState(false)
  useEffect(() => {
    if (!listening) return
    const onKey = (e: KeyboardEvent) => {
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
      className={cn(
        "border-hairline-strong bg-card rounded-md border border-b-2 px-2 py-1 text-xs",
        listening && "border-world",
      )}
      title={`${label} key: click, then press a key`}
    >
      {label} <span className="font-mono font-semibold">{listening ? "…" : keyLabel(code)}</span>
    </button>
  )
}
