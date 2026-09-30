// High-variability phonetic training (LANGUAGES.md item 6): trial generation and scoring.
//
// A session has a training block and a test block. The test block uses words the learner never
// trained on and, when there are enough voices, voices they never heard in training, because the
// evidence for this method is about generalisation to new talkers and new words. Training trials
// give feedback after each answer; test trials do not, so the test is a measurement.
//
// Everything is deterministic for a given seed, so a session can be regenerated and tested.
import { mulberry32, shuffle } from "@/features/train/engine/prng"
import { probit } from "@/features/train/engine/scoring"
import type { Pair } from "./contrasts"

export type Block = "train" | "test"
/** Which member of the pair the voice says. */
export type Side = "a" | "b"

export type Trial = {
  index: number
  block: Block
  pairId: string
  target: Side
  voiceId: string
  /** Show b on the left: choice positions vary so position carries no information. */
  flip: boolean
}

export type Response = { trial: Trial; chosen: Side; ms: number }

export type VoiceSplit = { train: string[]; test: string[]; heldOut: boolean }

/**
 * Voices for training and for the held-out test. One voice cannot be held out, so the test reuses
 * it and `heldOut` is false; two voices train on one and test on the other; from three up, one is
 * held out (two from six).
 */
export function splitVoices(voices: string[], seed: number): VoiceSplit {
  const ids = shuffle(mulberry32(seed ^ 0x51ed), [...new Set(voices)])
  if (ids.length === 0) return { train: [], test: [], heldOut: false }
  if (ids.length === 1) return { train: ids, test: ids, heldOut: false }
  const held = ids.length >= 6 ? 2 : 1
  return { train: ids.slice(0, ids.length - held), test: ids.slice(ids.length - held), heldOut: true }
}

/** Training and held-out test words: about 30% of the pairs, at least two, leaving at least two to train on. */
export function splitPairs(pairs: Pair[], seed: number): { train: Pair[]; test: Pair[] } {
  const shuffled = shuffle(mulberry32(seed ^ 0x9a1f), [...pairs])
  const testCount = Math.min(Math.max(2, Math.round(pairs.length * 0.3)), Math.max(0, pairs.length - 2))
  return { train: shuffled.slice(testCount), test: shuffled.slice(0, testCount) }
}

/** Items in a shuffled cycle: every item is used before any repeats, and a reshuffle never repeats the last one. */
function cycle<T>(rng: () => number, items: T[], count: number): T[] {
  const out: T[] = []
  let round: T[] = []
  while (out.length < count) {
    if (round.length === 0) {
      round = shuffle(rng, [...items])
      if (items.length > 1 && round[0] === out[out.length - 1]) round.push(round.shift() as T)
    }
    out.push(round.shift() as T)
  }
  return out
}

/** Equal numbers of each target (one extra at random for an odd count), never more than three alike in a row. */
function targets(rng: () => number, count: number): Side[] {
  const base: Side[] = Array.from({ length: count }, (_, i) => (i % 2 === 0 ? "a" : "b"))
  if (count % 2 === 1 && rng() < 0.5) base[count - 1] = "b"
  for (let attempt = 0; attempt < 50; attempt++) {
    shuffle(rng, base)
    let run = 1
    let ok = true
    for (let i = 1; i < base.length && ok; i++) {
      run = base[i] === base[i - 1] ? run + 1 : 1
      if (run > 3) ok = false
    }
    if (ok) return base
  }
  return base
}

export function makeTrials(opts: { pairs: Pair[]; voices: string[]; count: number; block: Block; seed: number }): Trial[] {
  const { pairs, voices, count, block, seed } = opts
  if (pairs.length === 0 || voices.length === 0 || count <= 0) return []
  const rng = mulberry32(seed)
  const pairIds = cycle(
    rng,
    pairs.map((p) => p.id),
    count,
  )
  const voiceIds = cycle(rng, voices, count)
  const sides = targets(rng, count)
  return sides.map((target, index) => ({
    index,
    block,
    pairId: pairIds[index],
    target,
    voiceId: voiceIds[index],
    flip: rng() < 0.5,
  }))
}

export type Plan = {
  train: Trial[]
  test: Trial[]
  trainPairIds: string[]
  testPairIds: string[]
  voices: VoiceSplit
}

export const TRAIN_TRIALS = 24
export const TEST_TRIALS = 12

export function planSession(opts: {
  pairs: Pair[]
  voices: string[]
  seed: number
  trainTrials?: number
  testTrials?: number
}): Plan {
  const { pairs, seed } = opts
  const words = splitPairs(pairs, seed)
  const voices = splitVoices(opts.voices, seed)
  return {
    train: makeTrials({ pairs: words.train, voices: voices.train, count: opts.trainTrials ?? TRAIN_TRIALS, block: "train", seed: seed ^ 0x1111 }),
    test: makeTrials({ pairs: words.test, voices: voices.test, count: opts.testTrials ?? TEST_TRIALS, block: "test", seed: seed ^ 0x2222 }),
    trainPairIds: words.train.map((p) => p.id),
    testPairIds: words.test.map((p) => p.id),
    voices,
  }
}

export const isCorrect = (trial: Trial, chosen: Side) => trial.target === chosen

export type BlockScore = {
  n: number
  correct: number
  accuracy: number | null
  /** Sensitivity with "a" as the signal, log-linear corrected so perfect runs stay finite. Null with no trials. */
  dPrime: number | null
  byVoice: Record<string, { n: number; correct: number }>
  /** Median response time in ms, or null with no trials. */
  medianMs: number | null
}

export function scoreBlock(responses: Response[]): BlockScore {
  let hits = 0
  let misses = 0
  let falseAlarms = 0
  let rejections = 0
  let correct = 0
  const byVoice: BlockScore["byVoice"] = {}
  for (const { trial, chosen } of responses) {
    const ok = isCorrect(trial, chosen)
    if (ok) correct++
    if (trial.target === "a") chosen === "a" ? hits++ : misses++
    else chosen === "a" ? falseAlarms++ : rejections++
    const v = (byVoice[trial.voiceId] ??= { n: 0, correct: 0 })
    v.n++
    if (ok) v.correct++
  }
  const n = responses.length
  const times = responses.map((r) => r.ms).sort((x, y) => x - y)
  const mid = Math.floor(times.length / 2)
  return {
    n,
    correct,
    accuracy: n ? correct / n : null,
    dPrime: n
      ? probit((hits + 0.5) / (hits + misses + 1)) - probit((falseAlarms + 0.5) / (falseAlarms + rejections + 1))
      : null,
    byVoice,
    medianMs: n ? (times.length % 2 ? times[mid] : (times[mid - 1] + times[mid]) / 2) : null,
  }
}

/** What was stored about one session (localStorage, see logs.ts). */
export type SessionRecord = {
  at: number
  lang: string
  contrastId: string
  /** The list was unverified when the session ran. */
  unverified: boolean
  trainVoices: string[]
  testVoices: string[]
  voicesHeldOut: boolean
  train: BlockScore
  test: BlockScore | null
}

export function buildRecord(opts: {
  at: number
  lang: string
  contrastId: string
  unverified: boolean
  plan: Plan
  train: Response[]
  test: Response[]
}): SessionRecord {
  return {
    at: opts.at,
    lang: opts.lang,
    contrastId: opts.contrastId,
    unverified: opts.unverified,
    trainVoices: opts.plan.voices.train,
    testVoices: opts.plan.voices.test,
    voicesHeldOut: opts.plan.voices.heldOut,
    train: scoreBlock(opts.train),
    test: opts.test.length ? scoreBlock(opts.test) : null,
  }
}

/** Plain words for how the test compares with training, without claiming a cause. */
export function comparison(train: BlockScore, test: BlockScore | null, voicesHeldOut: boolean): string {
  if (!test || test.accuracy === null || train.accuracy === null) return "No test block was completed."
  const what = voicesHeldOut ? "new words in new voices" : "new words, in a voice you trained on"
  const pct = (x: number) => `${Math.round(x * 100)}%`
  return `Training ${pct(train.accuracy)}, test on ${what} ${pct(test.accuracy)}.`
}

// ---- voice config ------------------------------------------------------------------------------

/** `voice_<lang>` holds comma-separated voice ids (contract C10). */
export function parseVoiceIds(value: string | null | undefined): string[] {
  return [...new Set((value ?? "").split(",").map((v) => v.trim()).filter(Boolean))]
}

export const formatVoiceIds = (ids: string[]): string => [...new Set(ids)].join(",")

/** Voices needed for a session that can hold one out. */
export const MIN_VOICES = 3
