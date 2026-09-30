// Session setup: mode, starting level, block count, keys, the adaptation rule, lures, in-trial feedback,
// the comparability mark, the length estimate and the weekly dose. The transfer check has its own panel.
import { DeltaChip, Eyebrow, LedStatus, Panel, Segmented, Stat, StatRow, Stepper, SwitchRow, riseStyle } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { ruleText } from "./engine/adapt"
import { WEEK_SESSIONS, WEEK_SOFT_CAP_MIN } from "./engine/history"
import { TRANSFER_SETTINGS, isComparable, transferLevel } from "./engine/session"
import { Field, KeyPicker } from "./ui"
import type { Mode, Settings } from "./types"
import { doseFraction, nonComparableReasons } from "./view"
import "./train.css"

export type TrainMode = Exclude<Mode, "transfer">

const MODES: { id: TrainMode; label: string; hint: string }[] = [
  { id: "dual", label: "Dual", hint: "Grid position and spoken letter: two streams at once." },
  { id: "position", label: "Position only", hint: "Visual only: the grid position stream." },
  { id: "letter", label: "Letter only", hint: "Audio only: the spoken letter stream." },
]

/** Rough session length in minutes: trials plus rests. */
export const estimateMinutes = (s: Settings, n: number) =>
  Math.round((s.blocks * (20 + n) * s.trial_ms + (s.blocks - 1) * 10_000) / 60_000)

/** Shown beside a switch that, turned on, makes the session not comparable with earlier ones. */
const NOT_COMPARABLE = <DeltaChip tone="warn" text="Not comparable" className="max-[560px]:hidden" />

export type Week = { minutes: number; sessions: number }

/** This week's minutes against the soft cap. The cap is a reminder, never a block. */
function DoseCell({ week }: { week: Week | null }) {
  const past = week !== null && week.minutes >= WEEK_SOFT_CAP_MIN
  return (
    <div data-slot="kit-stat" className="bg-card flex min-w-0 flex-col gap-3.5 px-6 py-5">
      <dt>
        <Eyebrow>This week</Eyebrow>
      </dt>
      {week ? (
        <dd className="m-0 flex flex-col gap-3">
          <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-2">
            <span className="text-[40px] leading-none font-light tracking-[-0.04em] tabular-nums">{week.minutes}</span>
            <span className="text-ink-2 font-mono text-xs tracking-[0.06em]">
              of {WEEK_SOFT_CAP_MIN} min · {week.sessions} of up to {WEEK_SESSIONS} sessions
            </span>
            {past && <DeltaChip tone="warn" text="Past soft limit" className="self-center" />}
          </div>
          <div
            role="meter"
            aria-label="Minutes trained in the last 7 days"
            aria-valuemin={0}
            aria-valuemax={WEEK_SOFT_CAP_MIN}
            aria-valuenow={Math.min(week.minutes, WEEK_SOFT_CAP_MIN)}
            aria-valuetext={`${week.minutes} of ${WEEK_SOFT_CAP_MIN} minutes`}
            className="bg-hairline h-[3px] w-full overflow-hidden rounded-full"
          >
            <div
              className={cn("h-full origin-left", past ? "bg-warn" : "bg-world")}
              style={{ transform: `scaleX(${doseFraction(week.minutes, WEEK_SOFT_CAP_MIN)})` }}
            />
          </div>
          {past && (
            <p className="text-ink-2 text-xs leading-snug">
              You are past the weekly soft limit. More time has no documented benefit and takes time from review.
            </p>
          )}
        </dd>
      ) : (
        <dd className="text-ink-2 m-0 text-sm leading-snug">Weekly minutes are tracked in the desktop app.</dd>
      )}
    </div>
  )
}

export function SetupPanel({
  mode,
  setMode,
  settings,
  setSettings,
  startN,
  setStartN,
  lastLevel,
  week,
  starting,
  error,
  onStart,
}: {
  mode: TrainMode
  setMode: (m: TrainMode) => void
  settings: Settings
  setSettings: (s: Settings) => void
  startN: number
  setStartN: (n: number) => void
  /** The level this mode ended at last time, when there is a history. */
  lastLevel: number | null
  week: Week | null
  starting: boolean
  error: string | null
  onStart: () => void
}) {
  const comparable = isComparable(mode, settings)
  const reasons = nonComparableReasons(settings)
  const set = (patch: Partial<Settings>) => setSettings({ ...settings, ...patch })
  const hint = MODES.find((m) => m.id === mode)?.hint

  return (
    <Panel eyebrow="Session" title="Set up a session" className="kit-rise @container" style={riseStyle(0)}>
      <p className="text-ink-2 -mt-2 mb-6 max-w-[62ch] font-serif text-[16px] leading-relaxed italic">
        This trains the n-back task. Transfer to other skills is small or unproven; the Experiment tab tests it on you.
      </p>

      <div className="flex flex-col gap-6">
        <Field label="Mode" hint={hint}>
          <Segmented
            label="Mode"
            value={mode}
            onChange={setMode}
            options={MODES.map((m) => ({ value: m.id, label: m.label }))}
            className="w-full max-w-md"
          />
        </Field>

        <div className="grid grid-cols-2 gap-x-8 gap-y-6 @2xl:grid-cols-[auto_auto_1fr]">
          <Field label="Starting n" hint={lastLevel === null ? "Start at 1." : `Last ended at ${lastLevel}.`}>
            <Stepper label="starting n" value={startN} min={1} max={9} onChange={setStartN} />
          </Field>
          <Field label="Blocks" hint="12 to 20, 10 s rests.">
            <Stepper label="blocks" value={settings.blocks} min={12} max={20} onChange={(blocks) => set({ blocks })} />
          </Field>
          <Field label="Keys" hint="Activate one, then press a new key. Escape cancels." className="col-span-2 @2xl:col-span-1">
            <div className="flex flex-wrap gap-2">
              <KeyPicker
                label="Position"
                code={settings.keys.position}
                onPick={(position) => position !== settings.keys.letter && set({ keys: { ...settings.keys, position } })}
              />
              <KeyPicker
                label="Letter"
                code={settings.keys.letter}
                onPick={(letter) => letter !== settings.keys.position && set({ keys: { ...settings.keys, letter } })}
              />
            </div>
          </Field>
        </div>

        <div className="border-hairline border-t pt-5">
          <Eyebrow as="h3" rule>
            Adaptation rule
          </Eyebrow>
          <div className="mt-4 flex flex-wrap items-start gap-x-8 gap-y-4">
            <Field label="Up at most" hint="errors per stream">
              <Stepper
                label="up at most errors"
                value={settings.up_max_errors}
                min={0}
                max={10}
                onChange={(up_max_errors) => set({ up_max_errors, down_min_errors: Math.max(settings.down_min_errors, up_max_errors + 1) })}
              />
            </Field>
            <Field label="Down at" hint="errors in either stream">
              <Stepper
                label="down at errors"
                value={settings.down_min_errors}
                min={settings.up_max_errors + 1}
                max={20}
                onChange={(down_min_errors) => set({ down_min_errors })}
              />
            </Field>
            <p className="text-ink-2 min-w-[15rem] flex-1 self-center font-serif text-[15px] leading-relaxed italic">
              {ruleText(settings)}
            </p>
          </div>
        </div>

        <div className="border-hairline divide-hairline divide-y border-t">
          <SwitchRow
            title="Lures"
            detail="n±1 repeats, tagged in the log."
            checked={settings.lures}
            onChange={(lures) => set({ lures })}
            note={settings.lures ? NOT_COMPARABLE : undefined}
          />
          <SwitchRow
            title="In-trial feedback"
            detail="Marks each response correct or wrong as you go. The default is a summary at the end of each block."
            checked={settings.in_trial_feedback}
            onChange={(in_trial_feedback) => set({ in_trial_feedback })}
            note={settings.in_trial_feedback ? NOT_COMPARABLE : undefined}
          />
        </div>

        <StatRow>
          <Stat label="Estimated length" value={estimateMinutes(settings, startN)} unit="min, trials plus rests" size="md" animate={false} />
          <DoseCell week={week} />
        </StatRow>

        <div className="flex flex-col gap-2">
          <LedStatus tone={comparable ? "ok" : "warn"} state={comparable ? "Comparable" : "Not comparable"} />
          <p className="text-ink-2 max-w-[68ch] text-[13px] leading-relaxed">
            {comparable
              ? "Default task settings, so this session goes on the training curve."
              : `${reasons.join(", ").replace(/^./, (c) => c.toUpperCase())}: logged, but shown apart from the training curve.`}{" "}
            Suggested dose: 15-20 minutes, up to {WEEK_SESSIONS} sessions a week, after your due reviews.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-x-5 gap-y-3">
          <Button size="lg" className="h-10 px-5 text-[15px]" onClick={onStart} disabled={starting}>
            {starting ? "Loading sounds…" : "Start session"}
          </Button>
          <span className="text-ink-3 text-xs">Keep this window focused: leaving it voids the current block.</span>
        </div>
        {error && (
          <p className="text-bad text-sm" role="alert">
            {error}
          </p>
        )}
      </div>
    </Panel>
  )
}

/** The transfer check's description and start control: a measurement with different stimuli at a fixed
 * level, kept apart from training. Shown inside the Transfer panel (History.tsx). */
export function TransferStart({ level, detail, starting, onStart }: { level: number; detail: string; starting: boolean; onStart: () => void }) {
  const n = transferLevel(level)
  return (
    <>
      <p className="text-ink-2 max-w-[66ch] text-sm leading-relaxed">
        Measurement, not training: a dual n-back with shapes instead of grid positions and tones instead of letters, at a
        fixed n = {n} (your level {level} minus 1), {TRANSFER_SETTINGS.blocks} blocks, fixed settings, no adaptation and no
        feedback. Take it about every 4 weeks and don't practise it in between.
      </p>
      <div className="mt-5 flex flex-wrap items-center gap-x-5 gap-y-3">
        <Button variant="outline" size="lg" className="h-10 px-5 text-[15px]" onClick={onStart} disabled={starting}>
          {starting ? "Loading sounds…" : "Start transfer check"}
        </Button>
        <span className="text-ink-3 text-xs">{detail}</span>
      </div>
    </>
  )
}
