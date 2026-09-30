// Colour strings for the chart kit. Everything resolves through CSS variables, so a theme or world change
// re-colours the charts without a re-render.

const CHART_VARS = [1, 2, 3, 4, 5].map((n) => `var(--chart-${n})`)

/**
 * --chart-1 is the world accent; the rest are fixed categorical hues. Series are dealt out in the order
 * 1, 3, 5, 4, 2 because --chart-2 in dark mode (#56d4e0) is the same cyan as the Science accent, so a
 * second series would otherwise be indistinguishable from the first in that world.
 */
const ORDER = [0, 2, 4, 3, 1]
export const seriesColor = (i: number): string => CHART_VARS[ORDER[((i % 5) + 5) % 5]]

/** Flat-fill outline: the same hue, darker (oklab mix toward black). */
export const outlineOf = (c: string): string => `color-mix(in oklab, ${c} 58%, black)`

/** Translucent wash of a colour, e.g. the 10% band and area under a line. */
export const washOf = (c: string, pct: number): string => `color-mix(in oklab, ${c} ${pct}%, transparent)`

/** A colour mixed into a surface, for tints that stay readable behind ink text. */
export const tintOf = (c: string, pct: number, base = "var(--card)"): string => `color-mix(in oklab, ${c} ${pct}%, ${base})`

export const INK_2 = "var(--ink-2)"
export const INK_3 = "var(--ink-3)"
export const HAIRLINE = "var(--hairline)"
export const HAIRLINE_STRONG = "var(--hairline-strong)"
