// The running session: blocks with lookahead-scheduled stimuli, voids on focus loss, window hiding
// or an audio device change, 10 s rests with a block-end summary, then the finished record.
import { useEffect, useMemo, useRef, useState } from "react"

import { Button } from "@/components/ui/button"
import { latencies, toBuffer, type Clips } from "./audio"
import { ruleText } from "./engine/adapt"
import {
  REST_MS,
  finishBlock,
  nextBlock,
  startSession,
  stopSession,
  streamsOf,
  toRecord,
  voidBlock,
  type SessionConfig,
  type SessionMachine,
} from "./engine/session"
import { errors } from "./engine/scoring"
import { Grid, Keycap, MetricsTable, ShapeStage, streamLabels } from "./parts"
import { BlockRunner, type Feedback, type Stream } from "./runner"
import type { Block, NBackSession } from "./types"

const EMPTY_CHECKIN = { expectation: null, sleep_hours: null, exercise: null, fatigue: null, strategy: null }

export function RunView({
  config,
  ctx,
  clips,
  calibrated,
  onEnd,
}: {
  config: SessionConfig
  ctx: AudioContext
  clips: Clips
  calibrated: boolean
  onEnd: (record: NBackSession) => void
}) {
  const [machine, setMachine] = useState<SessionMachine>(() => startSession(config))
  const [shown, setShown] = useState<number | null>(null)
  const [pressed, setPressed] = useState<Record<Stream, boolean>>({ position: false, letter: false })
  const [marks, setMarks] = useState<Record<Stream, boolean | null>>({ position: null, letter: null })
  const [restLeft, setRestLeft] = useState(0)
  const runner = useRef<BlockRunner | null>(null)
  const meta = useRef({ id: crypto.randomUUID(), started_at: Date.now() / 1000, zero: ctx.currentTime })

  const transfer = config.mode === "transfer"
  const streams = streamsOf(config.mode)
  const labels = streamLabels(transfer)
  const { settings } = config
  const sounds = useMemo(
    () => (transfer ? clips.tones : clips.letters).map((c) => toBuffer(ctx, c)),
    [ctx, clips, transfer],
  )

  // Run the current block.
  const current = machine.current
  useEffect(() => {
    if (machine.phase !== "block" || !current) return
    const feedback = (f: Feedback) => {
      if (!settings.in_trial_feedback) return
      setMarks((m) => ({ ...m, [f.stream]: f.correct }))
      window.setTimeout(() => setMarks((m) => ({ ...m, [f.stream]: null })), 450)
    }
    const r = new BlockRunner({
      ctx,
      mode: config.mode,
      settings,
      stimuli: current.stimuli,
      sounds,
      sessionZero: meta.current.zero,
      onShow: setShown,
      onFeedback: feedback,
      onDone: (trials) => {
        runner.current = null
        setMachine((m) => finishBlock(m, trials))
      },
    })
    runner.current = r
    r.start()

    const voidWith = (reason: string) => {
      if (runner.current !== r) return
      runner.current = null
      const trials = r.abort()
      setMachine((m) => voidBlock(m, reason, trials))
    }
    const onBlur = () => voidWith("focus lost")
    const onVisibility = () => document.visibilityState === "hidden" && voidWith("window hidden")
    const onDevice = () => voidWith("audio device changed")
    const onState = () => ctx.state !== "running" && voidWith(`audio ${ctx.state}`)
    window.addEventListener("blur", onBlur)
    document.addEventListener("visibilitychange", onVisibility)
    navigator.mediaDevices?.addEventListener("devicechange", onDevice)
    ctx.addEventListener("statechange", onState)
    return () => {
      window.removeEventListener("blur", onBlur)
      document.removeEventListener("visibilitychange", onVisibility)
      navigator.mediaDevices?.removeEventListener("devicechange", onDevice)
      ctx.removeEventListener("statechange", onState)
      if (runner.current === r) {
        runner.current = null
        r.abort()
      }
    }
  }, [machine.phase, current, ctx, config.mode, settings, sounds])

  // Rest between blocks, then the next block.
  useEffect(() => {
    if (machine.phase !== "rest") return
    const end = performance.now() + REST_MS
    setRestLeft(Math.ceil(REST_MS / 1000))
    const t = window.setInterval(() => {
      const left = end - performance.now()
      if (left <= 0) {
        window.clearInterval(t)
        setMachine(nextBlock)
      } else setRestLeft(Math.ceil(left / 1000))
    }, 200)
    return () => window.clearInterval(t)
  }, [machine.phase, machine.blocks.length])

  // Finished: hand the record up once.
  const ended = useRef(false)
  useEffect(() => {
    if (machine.phase !== "done" || ended.current) return
    ended.current = true
    onEnd(
      toRecord(machine, {
        id: meta.current.id,
        started_at: meta.current.started_at,
        ended_at: Date.now() / 1000,
        timing: { ...latencies(ctx), calibrated },
        checkin: EMPTY_CHECKIN,
      }),
    )
  }, [machine, ctx, calibrated, onEnd])

  const press = (stream: Stream, timeStamp: number) => {
    if (!streams[stream]) return
    runner.current?.respond(stream, timeStamp)
    setPressed((p) => ({ ...p, [stream]: true }))
    window.setTimeout(() => setPressed((p) => ({ ...p, [stream]: false })), 120)
  }

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.repeat) return
      if (e.code === settings.keys.position) press("position", e.timeStamp)
      else if (e.code === settings.keys.letter) press("letter", e.timeStamp)
      else return
      e.preventDefault()
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  })

  const stop = () => {
    const r = runner.current
    runner.current = null
    const trials = r?.abort()
    setMachine((m) => stopSession(trials ? voidBlock(m, "stopped", trials) : m))
  }

  const blockNo = (current?.index ?? machine.blocks.length) + 1
  const n = current?.n ?? machine.n
  const stim = current && shown !== null ? current.stimuli[shown] : null
  const last = machine.blocks[machine.blocks.length - 1]

  return (
    <div className="flex flex-col items-center gap-6">
      <div className="flex w-full max-w-2xl flex-wrap items-baseline justify-between gap-3">
        <div className="flex items-baseline gap-4">
          <span className="font-mono text-3xl font-semibold">n = {n}</span>
          <span className="text-ink-2 text-sm">
            {machine.phase === "rest" ? "Rest · next: block " : "Block "}
            {Math.min(blockNo, settings.blocks)} of {settings.blocks}
            {transfer && " · measurement"}
          </span>
        </div>
        <Button variant="outline" size="sm" onClick={stop}>
          Stop session
        </Button>
      </div>
      <p className="text-ink-3 w-full max-w-2xl text-xs">
        {transfer
          ? `Fixed level: n stays at ${n} for this check. No adaptation.`
          : `Rule: ${ruleText(settings)}`}{" "}
        Press when the {streams.position && labels.position.toLowerCase()}
        {streams.position && streams.letter && " or "}
        {streams.letter && labels.letter.toLowerCase()} matches the one {n} back.
      </p>

      {machine.phase === "rest" && last ? (
        <BlockSummary block={last} labels={labels} transfer={transfer} restLeft={restLeft} settings={settings} />
      ) : transfer ? (
        <ShapeStage shape={stim?.position ?? null} />
      ) : (
        <Grid active={streams.position ? (stim?.position ?? null) : null} dim={!streams.position} />
      )}

      <div className="flex gap-4">
        {streams.position && (
          <Keycap
            code={settings.keys.position}
            label={`${labels.position} match`}
            pressed={pressed.position}
            mark={marks.position}
            onPress={(t) => press("position", t)}
          />
        )}
        {streams.letter && (
          <Keycap
            code={settings.keys.letter}
            label={`${labels.letter} match`}
            pressed={pressed.letter}
            mark={marks.letter}
            onPress={(t) => press("letter", t)}
          />
        )}
      </div>
    </div>
  )
}

export function BlockSummary({
  block,
  labels,
  transfer,
  restLeft,
  settings,
}: {
  block: Block
  labels: { position: string; letter: string }
  transfer: boolean
  restLeft: number
  settings: { up_max_errors: number; down_min_errors: number }
}) {
  const errs = [block.metrics.position, block.metrics.letter].flatMap((m) => (m ? [errors(m)] : []))
  const change = block.next_n > block.n ? "up" : block.next_n < block.n ? "down" : "same"
  const why = block.void
    ? `Void (${block.void_reason}): logged but not scored, and the level stays at ${block.n}.`
    : transfer
      ? `Fixed level ${block.n}.`
      : change === "up"
        ? `Next block: n = ${block.next_n}. Every stream had ${settings.up_max_errors} or fewer errors.`
        : change === "down"
          ? `Next block: n = ${block.next_n}. A stream had ${settings.down_min_errors} or more errors.`
          : `Next block: n = ${block.next_n}. Errors per stream: ${errs.join(" and ")}.`
  return (
    <div className="border-hairline bg-card w-full max-w-2xl rounded-xl border p-5" aria-live="polite">
      <div className="mb-3 flex items-baseline justify-between">
        <h2 className="text-sm font-semibold">
          Block {block.index + 1} at n = {block.n}
        </h2>
        <span className="text-ink-3 font-mono text-xs">rest {restLeft}s</span>
      </div>
      {!block.void && (
        <MetricsTable
          rows={[
            { label: labels.position, m: block.metrics.position },
            { label: labels.letter, m: block.metrics.letter },
          ]}
        />
      )}
      <p className={block.void ? "text-warn mt-3 text-sm" : "text-ink-2 mt-3 text-sm"}>{why}</p>
    </div>
  )
}
