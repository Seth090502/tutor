import { describe, expect, it } from "vitest"

import { nextLevel } from "./adapt"
import { generateBlock, type Stim } from "./generate"
import { deriveSeed, mulberry32 } from "./prng"
import { probit, scoreBlock, scoreStream } from "./scoring"
import {
  DEFAULT_SETTINGS,
  TRANSFER_SETTINGS,
  finishBlock,
  isComparable,
  nextBlock,
  startSession,
  stopSession,
  toRecord,
  toTrial,
  transferLevel,
  voidBlock,
  type Responses,
  type SessionMachine,
} from "./session"
import type { Trial } from "../types"

const BOTH = { position: true, letter: true }
const SEEDS = Array.from({ length: 200 }, (_, k) => 1000 + k * 7919)

describe("mulberry32", () => {
  it("is deterministic per seed and stays in [0, 1)", () => {
    const a = mulberry32(42)
    const b = mulberry32(42)
    const c = mulberry32(43)
    const xs = Array.from({ length: 1000 }, () => a())
    expect(xs).toEqual(Array.from({ length: 1000 }, () => b()))
    expect(xs.slice(0, 5)).not.toEqual(Array.from({ length: 5 }, () => c()))
    expect(xs.every((x) => x >= 0 && x < 1)).toBe(true)
  })

  it("derives distinct per-block seeds", () => {
    const seeds = new Set(Array.from({ length: 20 }, (_, k) => deriveSeed(42, k)))
    expect(seeds.size).toBe(20)
    expect(deriveSeed(42, 3)).toBe(deriveSeed(42, 3))
  })
})

/** Whether stream values at i and j are equal and both present. */
const same = (s: Stim[], key: "position" | "letter", i: number, j: number) =>
  j >= 0 && s[i][key] !== null && s[i][key] === s[j][key]
const matchKey = { position: "pos_match", letter: "let_match" } as const

describe("generateBlock", () => {
  it("makes 20 + n trials with exactly 6 matches per stream, 2 of them dual", () => {
    for (const n of [1, 2, 3, 5]) {
      for (const seed of SEEDS) {
        const s = generateBlock({ n, streams: BOTH, seed, lures: false })
        expect(s).toHaveLength(20 + n)
        expect(s.filter((t) => t.pos_match)).toHaveLength(6)
        expect(s.filter((t) => t.let_match)).toHaveLength(6)
        expect(s.filter((t) => t.pos_match && t.let_match)).toHaveLength(2)
        expect(s.every((t) => t.position! >= 0 && t.position! < 8 && t.letter! >= 0 && t.letter! < 8)).toBe(true)
      }
    }
  })

  it("has no unplanned matches: a match flag is set exactly when the n-back item repeats", () => {
    for (const n of [1, 2, 4]) {
      for (const seed of SEEDS) {
        const s = generateBlock({ n, streams: BOTH, seed, lures: true })
        for (const key of ["position", "letter"] as const) {
          s.forEach((t, i) => expect(t[matchKey[key]]).toBe(i >= n && same(s, key, i, i - n)))
        }
      }
    }
  })

  it("is reproducible from the seed", () => {
    const a = generateBlock({ n: 2, streams: BOTH, seed: 99, lures: true })
    expect(generateBlock({ n: 2, streams: BOTH, seed: 99, lures: true })).toEqual(a)
    expect(generateBlock({ n: 2, streams: BOTH, seed: 100, lures: true })).not.toEqual(a)
  })

  it("without lures, no non-match trial repeats the n-1 or n+1 item", () => {
    for (const n of [1, 2, 3]) {
      for (const seed of SEEDS) {
        const s = generateBlock({ n, streams: BOTH, seed, lures: false })
        expect(s.some((t) => t.lure)).toBe(false)
        for (const key of ["position", "letter"] as const) {
          s.forEach((t, i) => {
            if (i < n || t[matchKey[key]]) return
            expect(same(s, key, i, i - n - 1)).toBe(false)
            if (n > 1) expect(same(s, key, i, i - n + 1)).toBe(false)
          })
        }
      }
    }
  })

  it("with lures, tags exactly the non-match trials that repeat the n-1 or n+1 item", () => {
    let tagged = 0
    for (const n of [1, 2, 3]) {
      for (const seed of SEEDS) {
        const s = generateBlock({ n, streams: BOTH, seed, lures: true })
        s.forEach((t, i) => {
          const isLure = (["position", "letter"] as const).some(
            (key) =>
              i >= n &&
              !t[matchKey[key]] &&
              (same(s, key, i, i - n - 1) || (n > 1 && same(s, key, i, i - n + 1))),
          )
          expect(t.lure).toBe(isLure)
        })
        tagged += s.filter((t) => t.lure).length
      }
    }
    expect(tagged).toBeGreaterThan(SEEDS.length * 3 * 2)
  })

  it("fills only the active stream in single modes", () => {
    const s = generateBlock({ n: 2, streams: { position: true, letter: false }, seed: 5, lures: false })
    expect(s.every((t) => t.letter === null && !t.let_match)).toBe(true)
    expect(s.filter((t) => t.pos_match)).toHaveLength(6)
    const l = generateBlock({ n: 2, streams: { position: false, letter: true }, seed: 5, lures: false })
    expect(l.every((t) => t.position === null && !t.pos_match)).toBe(true)
    expect(l.filter((t) => t.let_match)).toHaveLength(6)
  })
})

/** Trials for one stream with the given outcome counts, after `n` unscored lead-in trials. */
function trials(n: number, hits: number, misses: number, fas: number, crs: number): Trial[] {
  const kinds = [
    ...Array(hits).fill([true, true]),
    ...Array(misses).fill([true, false]),
    ...Array(fas).fill([false, true]),
    ...Array(crs).fill([false, false]),
  ] as [boolean, boolean][]
  // Lead-in trials carry a response that must be ignored.
  const lead = Array.from({ length: n }, () => [false, true] as [boolean, boolean])
  return [...lead, ...kinds].map(([match, resp], i) => ({
    i,
    position: 0,
    letter: "C",
    pos_match: match,
    let_match: false,
    lure: false,
    pos_response: resp,
    let_response: false,
    pos_rt_ms: resp ? 400 : null,
    let_rt_ms: null,
    intended_onset: i * 3000,
    actual_onset: i * 3000,
  }))
}

describe("scoring", () => {
  it("probit matches the normal quantile", () => {
    expect(probit(0.5)).toBeCloseTo(0, 9)
    expect(probit(0.975)).toBeCloseTo(1.959964, 6)
    expect(probit(0.001)).toBeCloseTo(-3.090232, 6)
    expect(probit(0.999999)).toBeCloseTo(4.753424, 5)
  })

  it("counts outcomes and applies the log-linear correction", () => {
    const m = scoreStream(trials(2, 5, 1, 1, 13), "position", 2)
    expect(m).toMatchObject({ hits: 5, misses: 1, false_alarms: 1, correct_rejections: 13 })
    expect(m.d_prime).toBeCloseTo(2.07319, 5)
    expect(m.c).toBeCloseTo(0.244956, 5)
  })

  it("keeps d' finite at perfect and at inverted performance", () => {
    const perfect = scoreStream(trials(1, 6, 0, 0, 14), "position", 1)
    expect(perfect.d_prime).toBeCloseTo(3.299148, 5)
    expect(perfect.c).toBeCloseTo(0.18434, 5)
    const worst = scoreStream(trials(1, 0, 6, 14, 0), "position", 1)
    expect(worst.d_prime).toBeCloseTo(-3.299148, 5)
    const chance = scoreStream(trials(1, 3, 3, 7, 7), "position", 1)
    expect(chance.d_prime).toBeCloseTo(0, 9)
  })

  it("scores only the active streams of a block", () => {
    const t = trials(1, 6, 0, 0, 14)
    expect(scoreBlock(t, 1, { position: true, letter: false }).letter).toBeNull()
    expect(scoreBlock(t, 1, BOTH).letter).toMatchObject({ hits: 0, misses: 0, correct_rejections: 20 })
  })
})

const metric = (misses: number, false_alarms: number) => ({
  hits: 6 - misses,
  misses,
  false_alarms,
  correct_rejections: 14 - false_alarms,
  d_prime: 0,
  c: 0,
})
const RULE = { up_max_errors: 2, down_min_errors: 6 }

describe("nextLevel (Jaeggi per block)", () => {
  it("goes up when each stream has at most 2 errors", () => {
    expect(nextLevel(2, { position: metric(1, 1), letter: metric(0, 2) }, RULE)).toBe(3)
  })
  it("goes down when either stream has more than 5 errors", () => {
    expect(nextLevel(3, { position: metric(0, 0), letter: metric(3, 3) }, RULE)).toBe(2)
  })
  it("stays for 3 to 5 errors and never drops below 1", () => {
    expect(nextLevel(3, { position: metric(2, 1), letter: metric(0, 0) }, RULE)).toBe(3)
    expect(nextLevel(3, { position: metric(5, 0), letter: metric(0, 0) }, RULE)).toBe(3)
    expect(nextLevel(1, { position: metric(6, 6), letter: metric(0, 0) }, RULE)).toBe(1)
  })
  it("uses only active streams and honours configured thresholds", () => {
    expect(nextLevel(2, { position: metric(1, 0), letter: null }, RULE)).toBe(3)
    expect(nextLevel(2, { position: metric(1, 0), letter: null }, { up_max_errors: 0, down_min_errors: 1 })).toBe(1)
  })
})

/** A block's trials answered perfectly (or with every match missed). */
function answer(m: SessionMachine, perfect: boolean): Trial[] {
  const block = m.current!
  return block.stimuli.map((s, i) => {
    const r: Responses = {
      position: perfect && s.pos_match ? 450 : null,
      letter: perfect && s.let_match ? 500 : null,
    }
    return toTrial(m.config.mode, s, i, r, i * 3000, i * 3000 + 2)
  })
}

describe("session state machine", () => {
  const config = { mode: "dual" as const, settings: { ...DEFAULT_SETTINGS, blocks: 3 }, seed: 7, startN: 2 }

  it("runs blocks with rests, adapts per block, and ends after the set count", () => {
    let m = startSession(config)
    expect(m.phase).toBe("block")
    expect(m.current!.n).toBe(2)
    expect(m.current!.stimuli).toHaveLength(22)

    m = finishBlock(m, answer(m, true))
    expect(m.phase).toBe("rest")
    expect(m.blocks[0]).toMatchObject({ index: 0, n: 2, void: false, next_n: 3 })
    expect(m.blocks[0].metrics.position).toMatchObject({ hits: 6, false_alarms: 0 })

    m = nextBlock(m)
    expect(m.current!.n).toBe(3)
    m = finishBlock(m, answer(m, false))
    expect(m.blocks[1]).toMatchObject({ n: 3, next_n: 2 }) // 6 misses per stream: more than 5 errors
    m = nextBlock(m)
    m = finishBlock(m, answer(m, true))
    expect(m.phase).toBe("done")
    expect(m.blocks.map((b) => b.index)).toEqual([0, 1, 2])
  })

  it("voids a block without scoring or adapting, and still counts it", () => {
    let m = startSession(config)
    const partial = answer(m, true).slice(0, 5)
    m = voidBlock(m, "focus lost", partial)
    expect(m.blocks[0]).toMatchObject({ void: true, void_reason: "focus lost", n: 2, next_n: 2 })
    expect(m.blocks[0].metrics).toEqual({ position: null, letter: null })
    expect(m.blocks[0].trials).toHaveLength(5)
    expect(m.phase).toBe("rest")
    m = nextBlock(m)
    expect(m.current!.n).toBe(2)
  })

  it("derives every block's stimuli from the stored seed", () => {
    const a = startSession(config)
    const b = startSession(config)
    expect(a.current!.stimuli).toEqual(b.current!.stimuli)
    const a2 = nextBlock(finishBlock(a, answer(a, true)))
    expect(a2.current!.stimuli).not.toEqual(a.current!.stimuli)
  })

  it("stops early on request", () => {
    const m = stopSession(startSession(config))
    expect(m.phase).toBe("done")
    expect(m.current).toBeNull()
  })

  it("transfer mode is fixed: n = level - 1 (min 1) and no adaptation", () => {
    expect(transferLevel(4)).toBe(3)
    expect(transferLevel(1)).toBe(1)
    let m = startSession({ mode: "transfer", settings: TRANSFER_SETTINGS, seed: 3, startN: transferLevel(3) })
    expect(m.current!.n).toBe(2)
    m = finishBlock(m, answer(m, true))
    expect(m.blocks[0].next_n).toBe(2)
    expect(m.blocks[0].metrics.letter).not.toBeNull()
  })

  it("labels letters in dual mode and tones in transfer mode", () => {
    const s: Stim = { position: 3, letter: 0, pos_match: false, let_match: false, lure: false }
    const r = { position: null, letter: null }
    expect(toTrial("dual", s, 0, r, 0, 0).letter).toBe("C")
    expect(toTrial("transfer", s, 0, r, 0, 0).letter).toMatch(/Hz$/)
    expect(toTrial("dual", s, 0, { position: 812.4, letter: null }, 10, 12)).toMatchObject({
      pos_response: true,
      pos_rt_ms: 812.4,
      let_response: false,
      let_rt_ms: null,
      intended_onset: 10,
      actual_onset: 12,
    })
  })
})

describe("toRecord", () => {
  it("builds the C6 session record from a finished machine", () => {
    let m = startSession({ mode: "dual", settings: { ...DEFAULT_SETTINGS, blocks: 2, lures: true }, seed: 11, startN: 1 })
    m = finishBlock(m, answer(m, true))
    m = stopSession(m)
    const timing = { output_latency: 0.02, base_latency: 0.01, calibrated: false }
    const checkin = { expectation: null, sleep_hours: null, exercise: null, fatigue: null, strategy: null }
    const r = toRecord(m, { id: "x", started_at: 10, ended_at: 70, timing, checkin })
    expect(r).toEqual({
      id: "x",
      started_at: 10,
      ended_at: 70,
      mode: "dual",
      seed: 11,
      settings: m.config.settings,
      blocks: m.blocks,
      comparable: false, // lures on
      timing,
      checkin,
    })
  })
})

describe("comparability", () => {
  it("is true for default settings, even with remapped keys or a different block count", () => {
    expect(isComparable("dual", DEFAULT_SETTINGS)).toBe(true)
    expect(isComparable("dual", { ...DEFAULT_SETTINGS, blocks: 12, keys: { position: "KeyF", letter: "KeyJ" } })).toBe(
      true,
    )
    expect(isComparable("transfer", TRANSFER_SETTINGS)).toBe(true)
  })
  it("is false with in-trial feedback or changed task settings", () => {
    expect(isComparable("dual", { ...DEFAULT_SETTINGS, in_trial_feedback: true })).toBe(false)
    expect(isComparable("dual", { ...DEFAULT_SETTINGS, lures: true })).toBe(false)
    expect(isComparable("dual", { ...DEFAULT_SETTINGS, up_max_errors: 3 })).toBe(false)
    expect(isComparable("dual", { ...DEFAULT_SETTINGS, trial_ms: 2500 })).toBe(false)
  })
  it("defaults follow NBACK.md", () => {
    expect(DEFAULT_SETTINGS).toMatchObject({
      trial_ms: 3000,
      stimulus_ms: 500,
      blocks: 15,
      up_max_errors: 2,
      down_min_errors: 6,
      lures: false,
      in_trial_feedback: false,
      keys: { position: "KeyA", letter: "KeyL" },
    })
  })
})
