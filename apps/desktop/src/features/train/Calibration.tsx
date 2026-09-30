// Audio/visual sync check: 8 flashes with beeps, scheduled exactly like trials. Shows the measured
// frame offsets and the audio latencies, then asks whether the two lined up.
import { useEffect, useRef, useState } from "react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { audioContext, latencies, loadClips, toBuffer } from "./audio"
import { DEFAULT_SETTINGS } from "./engine/session"
import { BlockRunner } from "./runner"
import { storeCalibration, type Calibration } from "./storage"
import { Section } from "./parts"

const FLASHES = 8
const ms = (s: number | null) => (s === null ? "unknown" : `${(s * 1000).toFixed(1)} ms`)

export function CalibrationPanel({
  calibration,
  onChange,
}: {
  calibration: Calibration | null
  onChange: (c: Calibration) => void
}) {
  const [state, setState] = useState<"idle" | "running" | "asking">("idle")
  const [flash, setFlash] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const result = useRef<Omit<Calibration, "in_sync" | "at"> | null>(null)
  const runner = useRef<BlockRunner | null>(null)

  useEffect(() => () => void runner.current?.abort(), [])

  const run = async () => {
    setError(null)
    const ctx = audioContext() // created inside the click
    try {
      const clips = await loadClips()
      const beep = toBuffer(ctx, clips.beep)
      setState("running")
      const r = new BlockRunner({
        ctx,
        mode: "letter",
        settings: { ...DEFAULT_SETTINGS, trial_ms: 1000, stimulus_ms: 120 },
        stimuli: Array.from({ length: FLASHES }, () => ({
          position: null,
          letter: 0,
          pos_match: false,
          let_match: false,
          lure: false,
        })),
        sounds: [beep],
        sessionZero: ctx.currentTime,
        onShow: (i) => setFlash(i !== null),
        onDone: (trials) => {
          runner.current = null
          const offsets = trials.map((t) => t.actual_onset - t.intended_onset)
          result.current = {
            ...latencies(ctx),
            mean_offset_ms: offsets.reduce((a, b) => a + b, 0) / offsets.length,
            max_offset_ms: Math.max(...offsets.map(Math.abs)),
          }
          setState("asking")
        },
      })
      runner.current = r
      r.start()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      setState("idle")
    }
  }

  const answer = (in_sync: boolean) => {
    if (!result.current) return
    const c = { ...result.current, in_sync, at: Date.now() / 1000 }
    storeCalibration(c)
    onChange(c)
    setState("idle")
  }

  return (
    <Section title="Timing check">
      <div className="flex flex-wrap items-center gap-5">
        <div
          aria-hidden
          className={cn(
            "border-hairline size-20 shrink-0 rounded-lg border",
            flash ? "bg-world" : "bg-muted/40",
          )}
        />
        <div className="min-w-0 flex-1 text-sm">
          {calibration ? (
            <p className="text-ink-2">
              Checked {new Date(calibration.at * 1000).toLocaleDateString()}:{" "}
              {calibration.in_sync ? "in sync" : <span className="text-warn">reported out of sync</span>}. Output
              latency {ms(calibration.output_latency)}, base latency {ms(calibration.base_latency)}, frame offset mean{" "}
              {calibration.mean_offset_ms.toFixed(1)} ms (max {calibration.max_offset_ms.toFixed(1)} ms).
            </p>
          ) : (
            <p className="text-ink-2">
              Timing: <span className="text-warn">unmeasured</span>. The check flashes this square with a beep 8 times;
              watch and listen, then say whether they lined up.
            </p>
          )}
          {state === "asking" && result.current && (
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <span className="text-ink-2">
                Output latency {ms(result.current.output_latency)}, frame offset mean{" "}
                {result.current.mean_offset_ms.toFixed(1)} ms. Did the flash and beep line up?
              </span>
              <Button size="sm" onClick={() => answer(true)}>
                Yes, in sync
              </Button>
              <Button size="sm" variant="outline" onClick={() => answer(false)}>
                No
              </Button>
            </div>
          )}
          {error && <p className="text-bad mt-2">{error}</p>}
        </div>
        {state !== "asking" && (
          <Button variant="outline" size="sm" disabled={state === "running"} onClick={run}>
            {state === "running" ? "Checking…" : calibration ? "Check again" : "Run sync check"}
          </Button>
        )}
      </div>
    </Section>
  )
}
