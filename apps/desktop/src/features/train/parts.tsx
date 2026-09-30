// Small shared helpers of the Train screen.

/** Grid cell (0-8, row-major) of a position (0-7): the centre cell is never used. */
export const cellOf = (position: number) => (position < 4 ? position : position + 1)

/** A printable name for a KeyboardEvent.code: KeyA is A, ArrowLeft is Arrow Left. */
export const keyLabel = (code: string) => code.replace(/^(Key|Digit)/, "").replace(/^Arrow/, "Arrow ")

/** Stream labels by mode: transfer uses shapes and tones. */
export const streamLabels = (transfer: boolean) =>
  transfer ? { position: "Shape", letter: "Tone" } : { position: "Position", letter: "Letter" }
