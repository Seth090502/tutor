import { createElement } from "react"
import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it } from "vitest"

import { WORLDS } from "@/lib/world"

import { WorldMotif, cardioidPath } from "./motifs"

describe("cardioidPath", () => {
  it("draws one chord per point except point 0, which maps to itself", () => {
    const chords = cardioidPath(0, 0, 10, 24, 2).match(/M/g) ?? []
    expect(chords.length).toBe(23)
  })

  it("stays on the circle", () => {
    const numbers = (cardioidPath(0, 0, 10, 36, 2).match(/-?\d+(\.\d+)?/g) ?? []).map(Number)
    for (let i = 0; i < numbers.length; i += 2) {
      expect(Math.hypot(numbers[i], numbers[i + 1])).toBeCloseTo(10, 1)
    }
  })
})

describe("WorldMotif", () => {
  it.each(WORLDS.map((w) => w.id))("renders a hidden-from-assistive-tech SVG for %s", (world) => {
    const html = renderToStaticMarkup(createElement(WorldMotif, { world }))
    expect(html).toContain('aria-hidden="true"')
    expect(html).toContain('pathLength="1"')
    expect(html).not.toContain("NaN")
    // Every world shows its accent in the world colour next to the hairline strokes.
    expect(html).toContain("stroke-world")
    expect(html).toContain("stroke-hairline-strong")
  })
})
