/** Which keys pick a choice: A-D and 1-4 ("all"), A-D only ("letters"), or none (the caller owns the keys). */
export type HotkeyMode = "all" | "letters" | "none"

/** Hotkeys: A-D and 1-4 pick the choice at that position. Null when the key maps to nothing. */
export function hotkeyIndex(key: string, count: number, mode: HotkeyMode = "all"): number | null {
  if (mode === "none" || key.length !== 1) return null
  const k = key.toLowerCase()
  const i = "abcd".indexOf(k)
  const n = mode === "all" ? "1234".indexOf(k) : -1
  const idx = i >= 0 ? i : n
  return idx >= 0 && idx < count ? idx : null
}

/** Next index in a wrapping list, for arrow-key movement. */
export function stepIndex(current: number, delta: 1 | -1, count: number): number {
  return (current + delta + count) % count
}

type MaybeElement = { tagName?: string; isContentEditable?: boolean }

/** True for text-entry targets, where typing a letter must not select an answer. */
export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as MaybeElement | null
  if (!el || typeof el.tagName !== "string") return false
  return el.isContentEditable === true || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName)
}
