// A small noise tile, generated once at startup and used as a data-URL background. No filters, no
// per-frame work: the browser just repeats the image.

const TILE = 160

/** mulberry32: a tiny seeded generator so the tile is the same on every launch. */
function rng(seed: number) {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

/** RGBA pixels: half black, half white specks at roughly 2 to 4 percent alpha (mean about 3). */
export function grainPixels(size = TILE, seed = 0x7a17): Uint8ClampedArray<ArrayBuffer> {
  const next = rng(seed)
  const data = new Uint8ClampedArray(new ArrayBuffer(size * size * 4))
  for (let i = 0; i < data.length; i += 4) {
    const v = next() < 0.5 ? 0 : 255
    data[i] = data[i + 1] = data[i + 2] = v
    data[i + 3] = 5 + Math.floor(next() * 6)
  }
  return data
}

let cached: string | null | undefined

/** The tile as a `url(...)` value, or null when a canvas is not available. Built once. */
export function grainBackground(): string | null {
  if (cached !== undefined) return cached
  cached = null
  try {
    const canvas = document.createElement("canvas")
    canvas.width = canvas.height = TILE
    const ctx = canvas.getContext("2d")
    if (ctx) {
      ctx.putImageData(new ImageData(grainPixels(), TILE, TILE), 0, 0)
      cached = `url(${canvas.toDataURL("image/png")})`
    }
  } catch {
    // No canvas (or it is blocked): the ambient layer simply goes without grain.
  }
  return cached
}
