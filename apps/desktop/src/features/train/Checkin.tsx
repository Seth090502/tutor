// After the session: the check-in (strategy, expectation, sleep, exercise, fatigue). Every question is
// optional; answers are logged with the session and never shown as a score.
import { useId, useState, type CSSProperties, type ReactNode } from "react"

import { Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { Segmented } from "./ui"
import type { Checkin } from "./types"
import "./train.css"

const FIELD =
  "border-hairline-strong bg-card placeholder:text-ink-3 rounded-xl border text-[15px] outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

const SCALE = [1, 2, 3, 4, 5].map((v) => ({ value: v, label: <span className="font-mono tabular-nums">{v}</span> }))

export function CheckinForm({ onSubmit, saving }: { onSubmit: (c: Checkin) => void; saving: boolean }) {
  const [strategy, setStrategy] = useState("")
  const [expectation, setExpectation] = useState<number | null>(null)
  const [sleep, setSleep] = useState("")
  const [exercise, setExercise] = useState<boolean | null>(null)
  const [fatigue, setFatigue] = useState<number | null>(null)
  const strategyId = useId()
  const sleepId = useId()

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
    <Panel eyebrow="Check-in" title="Before you go" className="train-rise" style={{ "--i": 1 } as CSSProperties}>
      <p className="text-ink-2 -mt-2 mb-6 max-w-[60ch] font-serif text-[15px] leading-relaxed italic">
        Five optional questions. They are logged with the session so a later comparison has something to go on.
      </p>
      <div className="flex flex-col gap-6">
        <Q label="Which strategy did you use?" htmlFor={strategyId}>
          <textarea
            id={strategyId}
            className={cn(FIELD, "w-full resize-y px-4 py-3 leading-relaxed")}
            rows={2}
            value={strategy}
            onChange={(e) => setStrategy(e.target.value)}
            placeholder="For example: rehearsed the last few letters, tracked positions as a path…"
          />
        </Q>

        <Q label="How much do you expect this training to help you outside the task?">
          <Scale label="Expectation" value={expectation} onChange={setExpectation} low="Not at all" high="A lot" />
        </Q>

        <div className="grid grid-cols-1 gap-6 min-[640px]:grid-cols-[auto_auto_1fr] min-[640px]:gap-x-10">
          <Q label="Sleep last night" htmlFor={sleepId}>
            <div className="flex items-center gap-2">
              <input
                id={sleepId}
                type="number"
                min={0}
                max={24}
                step={0.5}
                className={cn(FIELD, "h-10 w-24 px-3 text-center font-mono tabular-nums")}
                value={sleep}
                onChange={(e) => setSleep(e.target.value)}
              />
              <span className="text-ink-3 font-mono text-xs">hours</span>
            </div>
          </Q>
          <Q label="Exercised today?">
            <Segmented
              label="Exercised today"
              value={exercise}
              onChange={setExercise}
              onClear={() => setExercise(null)}
              options={[
                { value: true, label: "Yes" },
                { value: false, label: "No" },
              ]}
            />
          </Q>
          <Q label="Fatigue now">
            <Scale label="Fatigue" value={fatigue} onChange={setFatigue} low="Fresh" high="Exhausted" />
          </Q>
        </div>

        <div className="border-hairline flex items-center gap-4 border-t pt-5">
          <Button size="lg" onClick={submit} disabled={saving}>
            {saving ? "Saving…" : "Save check-in"}
          </Button>
          <span className="text-ink-3 text-xs">Skipping a question leaves it blank.</span>
        </div>
      </div>
    </Panel>
  )
}

function Q({ label, htmlFor, children }: { label: string; htmlFor?: string; children: ReactNode }) {
  return (
    <div className="flex min-w-0 flex-col gap-2.5">
      {htmlFor ? (
        <label htmlFor={htmlFor} className="text-[15px] font-medium">
          {label}
        </label>
      ) : (
        <span className="text-[15px] font-medium">{label}</span>
      )}
      {children}
    </div>
  )
}

/** A 1 to 5 scale with its two ends named; choosing the chosen value again clears it. */
function Scale({
  label,
  value,
  onChange,
  low,
  high,
}: {
  label: string
  value: number | null
  onChange: (v: number | null) => void
  low: string
  high: string
}) {
  return (
    <div className="flex w-full max-w-[17rem] flex-col gap-1.5">
      <Segmented label={label} value={value} options={SCALE} onChange={onChange} onClear={() => onChange(null)} className="w-full" />
      <div className="text-ink-3 flex justify-between px-1 text-xs">
        <span>1 {low}</span>
        <span>5 {high}</span>
      </div>
    </div>
  )
}
