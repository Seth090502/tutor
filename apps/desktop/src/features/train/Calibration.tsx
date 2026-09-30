// Audio/visual sync check: 8 flashes with beeps, scheduled exactly like trials. Shows the measured
// frame offsets and the audio latencies, then asks whether the two lined up.
import { useEffect, useRef, useState } from "react"

import { LedStatus, Panel, riseStyle } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { audioContext, latencies, loadClips, toBuffer } from "./audio"
import { DEFAULT_SETTINGS } from "./engine/session"
import { BlockRunner } from "./runner"
import { storeCalibration, type Calibration } from "./storage"
import "./train.css"

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
    <Panel
      eyebrow="Timing"
      title="Timing check"
      className="kit-rise"
      style={riseStyle(3)}
      actions={
        calibration ? (
          <LedStatus tone={calibration.in_sync ? "ok" : "bad"} state={calibration.in_sync ? "In sync" : "Reported out of sync"} />
        ) : (
          <LedStatus tone="warn" state="Unmeasured" />
        )
      }
    >
      <div className="flex flex-wrap items-center gap-x-6 gap-y-4">
        <div
          aria-hidden
          className={cn(
            "border-hairline grid size-20 shrink-0 place-items-center rounded-2xl border",
            flash ? "bg-world" : "bg-[color-mix(in_oklab,var(--ink)_4%,var(--card))]",
          )}
        >
          <svg viewBox="0 0 100 100" className={cn("size-8", flash ? "text-world-ink" : "text-hairline-strong")}>
            <path d="M50 12 88 50 50 88 12 50Z" fill="currentColor" />
          </svg>
        </div>
        <div className="min-w-0 flex-1 basis-64 text-sm">
          {calibration ? (
            <p className="text-ink-2 leading-relaxed">
              Checked {new Date(calibration.at * 1000).toLocaleDateString()}. Output latency {ms(calibration.output_latency)},
              base latency {ms(calibration.base_latency)}, frame offset mean {calibration.mean_offset_ms.toFixed(1)} ms (max{" "}
              {calibration.max_offset_ms.toFixed(1)} ms).
            </p>
          ) : (
            <p className="text-ink-2 leading-relaxed">
              Timing is unmeasured on this device. The check flashes this square with a beep 8 times; watch and listen, then
              say whether they lined up.
            </p>
          )}
          {state === "asking" && result.current && (
            <div className="mt-3 flex flex-wrap items-center gap-2" role="group" aria-label="Did the flash and beep line up?">
              <span className="text-ink-2 mr-1">
                Output latency {ms(result.current.output_latency)}, frame offset mean {result.current.mean_offset_ms.toFixed(1)} ms.
                Did the flash and beep line up?
              </span>
              <Button size="sm" onClick={() => answer(true)}>
                Yes, in sync
              </Button>
              <Button size="sm" variant="outline" onClick={() => answer(false)}>
                No
              </Button>
            </div>
          )}
          {error && (
            <p className="text-bad mt-2" role="alert">
              {error}
            </p>
          )}
        </div>
        {state !== "asking" && (
          <Button variant="outline" size="lg" className="h-10 px-4" disabled={state === "running"} onClick={run}>
            {state === "running" ? "Checking…" : calibration ? "Check again" : "Run sync check"}
          </Button>
        )}
      </div>
    </Panel>
  )
}
