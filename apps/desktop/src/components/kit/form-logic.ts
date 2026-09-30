/** Parse a typed integer and clamp it into range; null while the text is not a whole number. */
export function commitInt(draft: string, min: number, max: number): number | null {
  if (draft.trim() === "") return null
  const v = Number(draft)
  return Number.isFinite(v) ? Math.min(max, Math.max(min, Math.round(v))) : null
}

/** Index a radio group moves to for an arrow, Home or End key; null for any other key. */
export function radioTarget(key: string, current: number, count: number): number | null {
  if (count <= 0) return null
  const at = current < 0 ? 0 : current
  if (key === "ArrowRight" || key === "ArrowDown") return (at + 1) % count
  if (key === "ArrowLeft" || key === "ArrowUp") return (at - 1 + count) % count
  if (key === "Home") return 0
  if (key === "End") return count - 1
  return null
}
