// Block generator: 20 + n trials, 6 planned matches per stream (2 of them dual in dual mode),
// no unplanned matches, and optional lures (an n-1 or n+1 repeat on a non-match), tagged.
import { mulberry32, randInt, shuffle } from "./prng"

/** One generated trial. Values are indices 0-7 (grid cell or shape; letter or tone). */
export type Stim = {
  position: number | null
  letter: number | null
  pos_match: boolean
  let_match: boolean
  lure: boolean
}

export type Streams = { position: boolean; letter: boolean }

export const STIMULI = 8
export const BASE_TRIALS = 20
export const MATCHES = 6
export const DUAL_MATCHES = 2
export const LURES_PER_STREAM = 2

export function generateBlock(opts: { n: number; streams: Streams; seed: number; lures: boolean }): Stim[] {
  const { n, streams, lures } = opts
  const rng = mulberry32(opts.seed)
  const length = BASE_TRIALS + n
  const scoreable = Array.from({ length: BASE_TRIALS }, (_, k) => n + k)

  // Planned matches. In dual mode the single-stream matches are disjoint, so exactly 2 are dual.
  const order = shuffle(rng, scoreable.slice())
  const targets: Record<keyof Streams, Set<number>> = { position: new Set(), letter: new Set() }
  if (streams.position && streams.letter) {
    const dual = order.slice(0, DUAL_MATCHES)
    const single = MATCHES - DUAL_MATCHES
    targets.position = new Set([...dual, ...order.slice(DUAL_MATCHES, DUAL_MATCHES + single)])
    targets.letter = new Set([...dual, ...order.slice(DUAL_MATCHES + single, DUAL_MATCHES + 2 * single)])
  } else {
    const key = streams.position ? "position" : "letter"
    targets[key] = new Set(order.slice(0, MATCHES))
  }

  const values: Record<keyof Streams, number[] | null> = { position: null, letter: null }
  const lured = new Set<number>()
  for (const key of ["position", "letter"] as const) {
    if (!streams[key]) continue
    const planned = new Set(
      lures
        ? shuffle(
            rng,
            scoreable.filter((i) => !targets[key].has(i) && (i - n - 1 >= 0 || n > 1)),
          ).slice(0, LURES_PER_STREAM)
        : [],
    )
    const v: number[] = []
    for (let i = 0; i < length; i++) {
      if (i < n) {
        v.push(randInt(rng, STIMULI))
        continue
      }
      if (targets[key].has(i)) {
        v.push(v[i - n])
        continue
      }
      // Neighbours of the n-back item: repeating one is a lure (n-1 only exists for n > 1).
      const near = [i - n - 1, n > 1 ? i - n + 1 : -1].filter((j) => j >= 0 && v[j] !== v[i - n])
      if (planned.has(i) && near.length > 0) {
        v.push(v[near[randInt(rng, near.length)]])
        lured.add(i)
        continue
      }
      const banned = new Set([v[i - n], ...near.map((j) => v[j])])
      const allowed = Array.from({ length: STIMULI }, (_, s) => s).filter((s) => !banned.has(s))
      v.push(allowed[randInt(rng, allowed.length)])
    }
    values[key] = v
  }

  return Array.from({ length }, (_, i) => ({
    position: values.position?.[i] ?? null,
    letter: values.letter?.[i] ?? null,
    pos_match: targets.position.has(i),
    let_match: targets.letter.has(i),
    lure: lured.has(i),
  }))
}
