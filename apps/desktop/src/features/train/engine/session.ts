// Session state machine: block -> rest -> block ... -> done. Pure: the screen feeds it events
// (a finished block's trials, a void, the end of a rest, a stop) and renders its state.
import type { Block, Mode, NBackSession, Settings, Trial } from "../types"
import { nextLevel } from "./adapt"
import { generateBlock, type Stim, type Streams } from "./generate"
import { deriveSeed } from "./prng"
import { scoreBlock } from "./scoring"

export const LETTERS = ["C", "H", "K", "L", "Q", "R", "S", "T"] as const
/** Transfer-mode tones (Hz), spread over two octaves so neighbours are easy to tell apart. */
export const TONES = [262, 330, 392, 494, 587, 698, 880, 1047] as const
export const REST_MS = 10_000

export const DEFAULT_SETTINGS: Settings = {
  trial_ms: 3000,
  stimulus_ms: 500,
  blocks: 15,
  up_max_errors: 2,
  down_min_errors: 6,
  lures: false,
  in_trial_feedback: false,
  keys: { position: "KeyA", letter: "KeyL" },
}

/** Transfer check: fixed settings, no feedback, no lures, a short fixed length. */
export const TRANSFER_SETTINGS: Settings = { ...DEFAULT_SETTINGS, blocks: 3 }

/** The transfer check runs at the current trained level minus 1 (at least 1). */
export const transferLevel = (level: number) => Math.max(1, level - 1)

export const streamsOf = (mode: Mode): Streams => ({ position: mode !== "letter", letter: mode !== "position" })

/** Comparable sessions use the default task settings and no in-trial feedback. Keys and block
 * count are free: remapping is an accessibility setting, and metrics are per block. */
export function isComparable(mode: Mode, s: Settings): boolean {
  const ref = mode === "transfer" ? TRANSFER_SETTINGS : DEFAULT_SETTINGS
  return (
    !s.in_trial_feedback &&
    s.trial_ms === ref.trial_ms &&
    s.stimulus_ms === ref.stimulus_ms &&
    s.up_max_errors === ref.up_max_errors &&
    s.down_min_errors === ref.down_min_errors &&
    s.lures === ref.lures &&
    (mode !== "transfer" || s.blocks === ref.blocks)
  )
}

export type SessionConfig = { mode: Mode; settings: Settings; seed: number; startN: number }

export type SessionMachine = {
  config: SessionConfig
  phase: "block" | "rest" | "done"
  /** The block being run, or (during a rest) null. */
  current: { index: number; n: number; stimuli: Stim[] } | null
  /** Level for the next block. */
  n: number
  blocks: Block[]
}

function makeBlock(config: SessionConfig, index: number, n: number) {
  const stimuli = generateBlock({
    n,
    streams: streamsOf(config.mode),
    seed: deriveSeed(config.seed, index),
    lures: config.settings.lures,
  })
  return { index, n, stimuli }
}

export function startSession(config: SessionConfig): SessionMachine {
  const n = Math.max(1, Math.round(config.startN))
  return { config, phase: "block", current: makeBlock(config, 0, n), n, blocks: [] }
}

function append(m: SessionMachine, block: Block): SessionMachine {
  const blocks = [...m.blocks, block]
  const phase = blocks.length >= m.config.settings.blocks ? "done" : "rest"
  return { ...m, phase, current: null, n: block.next_n, blocks }
}

export function finishBlock(m: SessionMachine, trials: Trial[]): SessionMachine {
  if (m.phase !== "block" || !m.current) return m
  const { index, n } = m.current
  const metrics = scoreBlock(trials, n, streamsOf(m.config.mode))
  const next_n = m.config.mode === "transfer" ? n : nextLevel(n, metrics, m.config.settings)
  return append(m, { index, n, void: false, void_reason: null, trials, metrics, next_n })
}

/** A void block keeps its trials for the log but is not scored and does not change the level. */
export function voidBlock(m: SessionMachine, reason: string, trials: Trial[]): SessionMachine {
  if (m.phase !== "block" || !m.current) return m
  const { index, n } = m.current
  return append(m, { index, n, void: true, void_reason: reason, trials, metrics: { position: null, letter: null }, next_n: n })
}

export function nextBlock(m: SessionMachine): SessionMachine {
  if (m.phase !== "rest") return m
  return { ...m, phase: "block", current: makeBlock(m.config, m.blocks.length, m.n) }
}

export function stopSession(m: SessionMachine): SessionMachine {
  return { ...m, phase: "done", current: null }
}

/** Response times in ms from the stimulus onset, or null for no response. */
export type Responses = { position: number | null; letter: number | null }

/** Label for a letter-stream value: a consonant, or in transfer mode a tone. */
export const letterLabel = (mode: Mode, v: number) => (mode === "transfer" ? `${TONES[v]}Hz` : LETTERS[v])

export function toTrial(mode: Mode, s: Stim, i: number, r: Responses, intended: number, actual: number): Trial {
  return {
    i,
    position: s.position,
    letter: s.letter === null ? null : letterLabel(mode, s.letter),
    pos_match: s.pos_match,
    let_match: s.let_match,
    lure: s.lure,
    pos_response: r.position !== null,
    let_response: r.letter !== null,
    pos_rt_ms: r.position,
    let_rt_ms: r.letter,
    intended_onset: intended,
    actual_onset: actual,
  }
}

export function toRecord(
  m: SessionMachine,
  meta: Pick<NBackSession, "id" | "started_at" | "ended_at" | "timing" | "checkin">,
): NBackSession {
  const { mode, seed, settings } = m.config
  return { ...meta, mode, seed, settings, blocks: m.blocks, comparable: isComparable(mode, settings) }
}
