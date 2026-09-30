import { describe, expect, it } from "vitest"

import { CONTRASTS, contrastsFor } from "./contrasts"
import {
  buildRecord,
  comparison,
  formatVoiceIds,
  isCorrect,
  makeTrials,
  parseVoiceIds,
  planSession,
  scoreBlock,
  splitPairs,
  splitVoices,
  type Response,
  type Trial,
} from "./hvpt"

const pairs = contrastsFor("es").find((c) => c.id === "es-r-rr")!.pairs // 9 pairs
const VOICES = ["v1", "v2", "v3", "v4"]

const trial = (over: Partial<Trial> = {}): Trial => ({
  index: 0,
  block: "train",
  pairId: "x|y",
  target: "a",
  voiceId: "v1",
  flip: false,
  ...over,
})
const answer = (t: Partial<Trial>, chosen: "a" | "b", ms = 800): Response => ({ trial: trial(t), chosen, ms })

describe("contrast lists", () => {
  it("covers the five languages and every contrast the spec names", () => {
    expect(new Set(CONTRASTS.map((c) => c.lang))).toEqual(new Set(["es", "fr", "de", "ja", "zh"]))
    const ids = CONTRASTS.map((c) => c.id)
    for (const id of ["es-r-rr", "es-stress", "fr-u-ou", "fr-nasal-an-on", "de-u-ue", "de-o-oe", "de-ich-ach", "ja-vowel-length", "ja-gemination"])
      expect(ids).toContain(id)
    expect(ids.filter((i) => i.startsWith("zh-tone")).length).toBeGreaterThanOrEqual(2)
  })

  it("marks every list unverified until a native speaker has checked it", () => {
    expect(CONTRASTS.every((c) => c.verified === false)).toBe(true)
  })

  it("has enough well-formed pairs to hold some words out", () => {
    const seen = new Set<string>()
    for (const c of CONTRASTS) {
      expect(c.pairs.length, c.id).toBeGreaterThanOrEqual(8)
      expect(new Set(c.pairs.map((p) => p.id)).size, c.id).toBe(c.pairs.length)
      expect(seen.has(c.id)).toBe(false)
      seen.add(c.id)
      for (const p of c.pairs) {
        expect(p.a.text, p.id).not.toBe(p.b.text)
        expect(p.a.text && p.b.text && p.a.label && p.b.label, p.id).toBeTruthy()
      }
    }
  })
})

describe("splitting words and voices", () => {
  it("keeps test words out of training", () => {
    for (const seed of [1, 2, 3, 99]) {
      const { train, test } = splitPairs(pairs, seed)
      expect(test.length).toBeGreaterThanOrEqual(2)
      expect(train.length).toBeGreaterThanOrEqual(2)
      expect(train.length + test.length).toBe(pairs.length)
      expect(test.filter((t) => train.some((r) => r.id === t.id))).toEqual([])
    }
  })

  it("holds out one voice from three up, two from six, and says when it cannot", () => {
    const three = splitVoices(["a", "b", "c"], 5)
    expect(three.heldOut).toBe(true)
    expect(three.train).toHaveLength(2)
    expect(three.test).toHaveLength(1)
    expect(three.train).not.toContain(three.test[0])
    expect(splitVoices(["a", "b", "c", "d", "e", "f"], 5).test).toHaveLength(2)

    const two = splitVoices(["a", "b"], 5)
    expect(two).toMatchObject({ heldOut: true })
    expect(two.train).toHaveLength(1)
    expect(two.test).toHaveLength(1)
    expect(two.train[0]).not.toBe(two.test[0])

    const one = splitVoices(["a"], 5)
    expect(one).toEqual({ train: ["a"], test: ["a"], heldOut: false })
    expect(splitVoices([], 5)).toEqual({ train: [], test: [], heldOut: false })
    expect(splitVoices(["a", "a"], 5).heldOut).toBe(false) // duplicates are one voice
  })
})

describe("trial generation", () => {
  const opts = { pairs, voices: VOICES, count: 24, block: "train" as const, seed: 42 }

  it("is deterministic for a seed and varies across seeds", () => {
    expect(makeTrials(opts)).toEqual(makeTrials(opts))
    expect(makeTrials({ ...opts, seed: 43 })).not.toEqual(makeTrials(opts))
  })

  it("balances targets, voices and words", () => {
    for (const count of [24, 12, 13]) {
      const trials = makeTrials({ ...opts, count })
      expect(trials).toHaveLength(count)
      const a = trials.filter((t) => t.target === "a").length
      expect(Math.abs(a - (count - a))).toBeLessThanOrEqual(1)
      const perVoice = VOICES.map((v) => trials.filter((t) => t.voiceId === v).length)
      expect(Math.max(...perVoice) - Math.min(...perVoice)).toBeLessThanOrEqual(1)
      const perPair = new Map<string, number>()
      for (const t of trials) perPair.set(t.pairId, (perPair.get(t.pairId) ?? 0) + 1)
      expect(Math.max(...perPair.values()) - Math.min(...perPair.values())).toBeLessThanOrEqual(1)
      if (count >= pairs.length) expect(perPair.size).toBe(pairs.length)
      expect(trials.map((t) => t.index)).toEqual(trials.map((_, i) => i))
    }
  })

  it("never runs more than three of the same target in a row, and uses both positions", () => {
    for (let seed = 0; seed < 40; seed++) {
      const trials = makeTrials({ ...opts, seed })
      let run = 1
      for (let i = 1; i < trials.length; i++) {
        run = trials[i].target === trials[i - 1].target ? run + 1 : 1
        expect(run).toBeLessThanOrEqual(3)
      }
    }
    const flips = makeTrials({ ...opts, count: 60 }).filter((t) => t.flip).length
    expect(flips).toBeGreaterThan(10)
    expect(flips).toBeLessThan(50)
  })

  it("does not repeat the same word back to back", () => {
    for (let seed = 0; seed < 40; seed++) {
      const trials = makeTrials({ ...opts, seed, count: 30 })
      for (let i = 1; i < trials.length; i++) expect(trials[i].pairId).not.toBe(trials[i - 1].pairId)
    }
  })

  it("returns nothing without words, voices or a count", () => {
    expect(makeTrials({ ...opts, pairs: [] })).toEqual([])
    expect(makeTrials({ ...opts, voices: [] })).toEqual([])
    expect(makeTrials({ ...opts, count: 0 })).toEqual([])
  })
})

describe("a session plan", () => {
  it("tests on held-out words and held-out voices only", () => {
    for (const seed of [1, 7, 21]) {
      const plan = planSession({ pairs, voices: VOICES, seed })
      expect(plan.train).toHaveLength(24)
      expect(plan.test).toHaveLength(12)
      expect(plan.voices.heldOut).toBe(true)
      for (const t of plan.train) {
        expect(plan.trainPairIds).toContain(t.pairId)
        expect(plan.voices.train).toContain(t.voiceId)
        expect(t.block).toBe("train")
      }
      for (const t of plan.test) {
        expect(plan.testPairIds).toContain(t.pairId)
        expect(plan.trainPairIds).not.toContain(t.pairId)
        expect(plan.voices.test).toContain(t.voiceId)
        expect(plan.voices.train).not.toContain(t.voiceId)
        expect(t.block).toBe("test")
      }
    }
  })

  it("trains on several voices when it can", () => {
    const plan = planSession({ pairs, voices: VOICES, seed: 3 })
    expect(new Set(plan.train.map((t) => t.voiceId)).size).toBe(3)
  })

  it("falls back to new words only with one voice", () => {
    const plan = planSession({ pairs, voices: ["solo"], seed: 3 })
    expect(plan.voices.heldOut).toBe(false)
    expect(plan.test.every((t) => t.voiceId === "solo")).toBe(true)
    expect(plan.test.every((t) => !plan.trainPairIds.includes(t.pairId))).toBe(true)
  })
})

describe("scoring", () => {
  it("counts correct choices", () => {
    expect(isCorrect(trial({ target: "a" }), "a")).toBe(true)
    expect(isCorrect(trial({ target: "b" }), "a")).toBe(false)
    const s = scoreBlock([answer({ target: "a" }, "a"), answer({ target: "b" }, "b"), answer({ target: "b" }, "a"), answer({ target: "a" }, "a")])
    expect(s.n).toBe(4)
    expect(s.correct).toBe(3)
    expect(s.accuracy).toBe(0.75)
  })

  it("reports d-prime: high for accurate, about zero at chance, zero for always choosing one side", () => {
    const perfect = Array.from({ length: 12 }, (_, i) => answer({ target: i % 2 ? "a" : "b" }, i % 2 ? "a" : "b"))
    expect(scoreBlock(perfect).dPrime).toBeGreaterThan(2.5)
    const always = Array.from({ length: 12 }, (_, i) => answer({ target: i % 2 ? "a" : "b" }, "a"))
    expect(scoreBlock(always).dPrime).toBeCloseTo(0, 6)
    const chance = Array.from({ length: 12 }, (_, i) => answer({ target: i % 2 ? "a" : "b" }, i % 4 < 2 ? "a" : "b"))
    expect(Math.abs(scoreBlock(chance).dPrime as number)).toBeLessThan(0.5)
    const worse = Array.from({ length: 12 }, (_, i) => answer({ target: i % 2 ? "a" : "b" }, i % 2 ? "b" : "a"))
    expect(scoreBlock(worse).dPrime).toBeLessThan(-2.5)
  })

  it("splits accuracy by voice and takes the median time", () => {
    const s = scoreBlock([
      answer({ voiceId: "v1", target: "a" }, "a", 500),
      answer({ voiceId: "v1", target: "a" }, "b", 900),
      answer({ voiceId: "v2", target: "b" }, "b", 700),
    ])
    expect(s.byVoice).toEqual({ v1: { n: 2, correct: 1 }, v2: { n: 1, correct: 1 } })
    expect(s.medianMs).toBe(700)
    expect(scoreBlock([answer({}, "a", 100), answer({}, "a", 300)]).medianMs).toBe(200)
  })

  it("scores an empty block as nothing rather than zero", () => {
    expect(scoreBlock([])).toEqual({ n: 0, correct: 0, accuracy: null, dPrime: null, byVoice: {}, medianMs: null })
  })
})

describe("the session record", () => {
  const plan = planSession({ pairs, voices: VOICES, seed: 9 })
  const right = (t: Trial): Response => ({ trial: t, chosen: t.target, ms: 600 })

  it("records both blocks, the voices and that the list was unverified", () => {
    const r = buildRecord({ at: 1, lang: "es", contrastId: "es-r-rr", unverified: true, plan, train: plan.train.map(right), test: plan.test.map(right) })
    expect(r.train.accuracy).toBe(1)
    expect(r.test?.accuracy).toBe(1)
    expect(r.unverified).toBe(true)
    expect(r.voicesHeldOut).toBe(true)
    expect(r.testVoices).toEqual(plan.voices.test)
  })

  it("has no test block when the learner stopped early", () => {
    const r = buildRecord({ at: 1, lang: "es", contrastId: "es-r-rr", unverified: true, plan, train: plan.train.slice(0, 5).map(right), test: [] })
    expect(r.test).toBeNull()
    expect(comparison(r.train, r.test, true)).toBe("No test block was completed.")
  })

  it("compares test with training in plain words", () => {
    const r = buildRecord({ at: 1, lang: "es", contrastId: "es-r-rr", unverified: true, plan, train: plan.train.map(right), test: plan.test.map((t, i) => ({ ...right(t), chosen: i < 9 ? t.target : t.target === "a" ? "b" : "a" })) })
    expect(comparison(r.train, r.test, true)).toBe("Training 100%, test on new words in new voices 75%.")
    expect(comparison(r.train, r.test, false)).toMatch(/in a voice you trained on/)
  })
})

describe("voice config", () => {
  it("parses and formats comma-separated ids", () => {
    expect(parseVoiceIds(" a, b ,,a,c ")).toEqual(["a", "b", "c"])
    expect(parseVoiceIds(null)).toEqual([])
    expect(parseVoiceIds("")).toEqual([])
    expect(formatVoiceIds(["a", "b", "a"])).toBe("a,b")
  })
})
