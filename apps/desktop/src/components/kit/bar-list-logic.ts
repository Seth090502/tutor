/** One bar: the label, its count and, optionally, its own fill (any CSS colour). */
export type BarRow = { label: string; value: number; color?: string }

/** Bar length as a share of the longest bar, 0..1. Zero when there is nothing to compare against. */
export const barRatio = (value: number, max: number): number =>
  max > 0 && Number.isFinite(value) ? Math.min(1, Math.max(0, value / max)) : 0

/** "39.1%", or an em dash when the whole is empty. */
export const shareText = (part: number, whole: number, decimals = 1): string =>
  whole > 0 ? `${((100 * part) / whole).toFixed(decimals)}%` : "—"

export function barTotals(rows: BarRow[]): { total: number; max: number } {
  let total = 0
  let max = 0
  for (const r of rows) {
    total += r.value
    if (r.value > max) max = r.value
  }
  return { total, max }
}
