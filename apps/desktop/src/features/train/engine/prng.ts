// Seeded PRNG. A session stores one 32-bit seed; each block's seed is derived from it, so a
// session's stimuli can be regenerated exactly.

/** mulberry32: small, fast, well-distributed 32-bit generator. Returns floats in [0, 1). */
export function mulberry32(seed: number): () => number {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

/** The seed for block `k` of a session (a murmur3-style mix of both). */
export function deriveSeed(seed: number, k: number): number {
  let h = (seed ^ Math.imul(k + 1, 0x9e3779b1)) >>> 0
  h = Math.imul(h ^ (h >>> 16), 0x85ebca6b)
  h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35)
  return (h ^ (h >>> 16)) >>> 0
}

/** A fresh session seed from the platform's CSPRNG. */
export function newSeed(): number {
  return crypto.getRandomValues(new Uint32Array(1))[0]
}

/** Integer in [0, max). */
export const randInt = (rng: () => number, max: number) => Math.floor(rng() * max)

/** Fisher-Yates shuffle, in place. */
export function shuffle<T>(rng: () => number, xs: T[]): T[] {
  for (let i = xs.length - 1; i > 0; i--) {
    const j = randInt(rng, i + 1)
    ;[xs[i], xs[j]] = [xs[j], xs[i]]
  }
  return xs
}
