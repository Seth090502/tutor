// After the session: what happened (raw counts next to d' and c), then the check-in questions.
import { useState, type ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { streamsOf } from "./engine/session"
import { MetricsTable, Section, streamLabels } from "./parts"
import type { Checkin, Metrics, NBackSession } from "./types"

/** Sum counts over scored blocks; d' and c are the mean of the per-block values. */
function pooled(ms: Metrics[]): Metrics | null {
  if (ms.length === 0) return null
  const sum = (k: keyof Metrics) => ms.reduce((a, m) => a + m[k], 0)
  return {
    hits: sum("hits"),
    misses: sum("misses"),
    false_alarms: sum("false_alarms"),
    correct_rejections: sum("correct_rejections"),
    d_prime: sum("d_prime") / ms.length,
    c: sum("c") / ms.length,
  }
}

export function SessionSummary({ record }: { record: NBackSession }) {
  const scored = record.blocks.filter((b) => !b.void)
  const labels = streamLabels(record.mode === "transfer")
  const streams = streamsOf(record.mode)
  const first = record.blocks[0]?.n
  const last = record.blocks[record.blocks.length - 1]?.next_n
  const minutes = Math.round((record.ended_at - record.started_at) / 60)
  return (
    <Section title={record.mode === "transfer" ? "Transfer check (measurement)" : "Session"}>
      <p className="text-ink-2 mb-3 text-sm">
        {record.blocks.length} blocks ({record.blocks.length - scored.length} void), {minutes} min.{" "}
        {record.mode === "transfer" ? `Fixed n = ${first}.` : `Level ${first} to ${last}.`}{" "}
        {record.comparable ? "Comparable." : "Not comparable."} Timing{" "}
        {record.timing.calibrated ? "checked on this device." : "unmeasured (run the timing check)."}
      </p>
      <MetricsTable
        rows={[
          { label: labels.position, m: streams.position ? pooled(scored.flatMap((b) => b.metrics.position ?? [])) : null },
          { label: labels.letter, m: streams.letter ? pooled(scored.flatMap((b) => b.metrics.letter ?? [])) : null },
        ]}
      />
      <p className="text-ink-3 mt-2 text-xs">
        Counts are summed over scored blocks; d′ and c are block means. n-back scores are noisy and do not measure
        intelligence.
      </p>
    </Section>
  )
}

export function CheckinForm({ onSubmit, saving }: { onSubmit: (c: Checkin) => void; saving: boolean }) {
  const [strategy, setStrategy] = useState("")
  const [expectation, setExpectation] = useState<number | null>(null)
  const [sleep, setSleep] = useState("")
  const [exercise, setExercise] = useState<boolean | null>(null)
  const [fatigue, setFatigue] = useState<number | null>(null)

  const submit = () => {
    const hours = sleep.trim() === "" ? null : Number(sleep)
    onSubmit({
      strategy: strategy.trim() || null,
      expectation,
      sleep_hours: hours !== null && Number.isFinite(hours) && hours >= 0 && hours <= 24 ? hours : null,
      exercise,
      fatigue,
    })
  }

  return (
    <Section title="Check-in">
      <div className="flex flex-col gap-4 text-sm">
        <Q label="Which strategy did you use?">
          <textarea
            className="border-hairline bg-background w-full rounded-md border px-3 py-2"
            rows={2}
            value={strategy}
            onChange={(e) => setStrategy(e.target.value)}
            placeholder="For example: rehearsed the last few letters, tracked positions as a path…"
          />
        </Q>
        <Q label="How much do you expect this training to help you outside the task? (1 not at all, 5 a lot)">
          <Scale value={expectation} onChange={setExpectation} />
        </Q>
        <div className="flex flex-wrap gap-6">
          <Q label="Sleep last night (hours)">
            <input
              type="number"
              min={0}
              max={24}
              step={0.5}
              className="border-hairline bg-background w-24 rounded-md border px-2 py-1 font-mono"
              value={sleep}
              onChange={(e) => setSleep(e.target.value)}
            />
          </Q>
          <Q label="Exercised today?">
            <div className="flex gap-1">
              {[true, false].map((v) => (
                <Choice key={String(v)} active={exercise === v} onClick={() => setExercise(exercise === v ? null : v)}>
                  {v ? "Yes" : "No"}
                </Choice>
              ))}
            </div>
          </Q>
          <Q label="Fatigue now (1 fresh, 5 exhausted)">
            <Scale value={fatigue} onChange={setFatigue} />
          </Q>
        </div>
        <div>
          <Button onClick={submit} disabled={saving}>
            {saving ? "Saving…" : "Save check-in"}
          </Button>
        </div>
      </div>
    </Section>
  )
}

function Q({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-ink-2">{label}</span>
      {children}
    </div>
  )
}

function Scale({ value, onChange }: { value: number | null; onChange: (v: number | null) => void }) {
  return (
    <div className="flex gap-1">
      {[1, 2, 3, 4, 5].map((v) => (
        <Choice key={v} active={value === v} onClick={() => onChange(value === v ? null : v)}>
          {v}
        </Choice>
      ))}
    </div>
  )
}

function Choice({ active, onClick, children }: { active: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={cn(
        "border-hairline min-w-9 rounded-md border px-2.5 py-1 font-mono",
        active && "border-world bg-world-soft",
      )}
    >
      {children}
    </button>
  )
}
