// The running session: blocks with lookahead-scheduled stimuli, voids on focus loss, window hiding
// or an audio device change, 10 s rests with a block-end summary, then the finished record.
import { useEffect, useMemo, useRef, useState } from "react"

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
import { streamLabels } from "./parts"
import { BlockRunner, type Feedback, type Stream } from "./runner"
import { RunStage } from "./Stage"
import { BlockSummary } from "./Summary"
import type { NBackSession } from "./types"

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
  const [trialNo, setTrialNo] = useState(0) // trials started in this block, for the progress rule
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
      onShow: (i) => {
        setShown(i)
        if (i !== null) setTrialNo(i + 1)
      },
      onFeedback: feedback,
      onDone: (trials) => {
        runner.current = null
        setMachine((m) => finishBlock(m, trials))
      },
    })
    runner.current = r
    setTrialNo(0)
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
    <RunStage
      mode={config.mode}
      n={n}
      blockNo={blockNo}
      blocks={settings.blocks}
      phase={machine.phase === "rest" ? "rest" : "block"}
      trial={trialNo}
      trials={current?.stimuli.length ?? 0}
      lit={stim?.position ?? null}
      keys={settings.keys}
      pressed={pressed}
      marks={marks}
      onPress={press}
      onStop={stop}
      restLeft={restLeft}
      restTotal={Math.ceil(REST_MS / 1000)}
      rule={ruleText(settings)}
      summary={
        last && <BlockSummary block={last} labels={labels} transfer={transfer} restLeft={restLeft} settings={settings} />
      }
    />
  )
}
