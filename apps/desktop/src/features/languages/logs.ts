// Small local logs (localStorage). They are per-viewer conveniences and records for the learner's own
// review: every read and write is wrapped, and the screen works with storage missing or full.
// Keys are `tutor.languages.*`; each log keeps its most recent entries only.

export const LOG_KEYS = {
  hvpt: "tutor.languages.hvpt",
  reading: "tutor.languages.reading",
  speaking: "tutor.languages.speaking",
  conversations: "tutor.languages.conversations",
} as const

type Store = Pick<Storage, "getItem" | "setItem">

function defaultStore(): Store | null {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

export function readLog<T>(key: string, store: Store | null = defaultStore()): T[] {
  try {
    const raw = store?.getItem(key)
    const parsed: unknown = raw ? JSON.parse(raw) : []
    return Array.isArray(parsed) ? (parsed as T[]) : []
  } catch {
    return []
  }
}

/** Appends one entry, keeping the newest `cap`. Returns false when it could not be stored. */
export function appendLog<T>(key: string, entry: T, cap = 200, store: Store | null = defaultStore()): boolean {
  if (!store) return false
  try {
    const next = [...readLog<T>(key, store), entry].slice(-cap)
    store.setItem(key, JSON.stringify(next))
    return true
  } catch {
    return false
  }
}
