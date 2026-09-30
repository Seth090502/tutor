// Pure helpers for the Today's plan section (contract C13, plan-policy/1).
import type { PlanItem, PlanRequest, StudyPlan } from "@/lib/backend"

export type Fatigue = "none" | "tired" | "rested"

export const TRACKS_KEY = "tutor.plan.tracks"
/** The first request, before the learner has chosen tracks: every track the app can teach. */
export const WIDE_TRACKS = ["general", "math", "english", "science", "chemistry", "lang:es", "lang:fr", "lang:de", "lang:ja", "lang:zh"]

export const MINUTES = { min: 10, max: 180, step: 5, initial: 45 } as const
export const NEW_ITEMS = { min: 0, max: 50, initial: 10 } as const

const LANGUAGES: Record<string, string> = { es: "Spanish", fr: "French", de: "German", ja: "Japanese", zh: "Chinese" }

export function trackLabel(track: string): string {
  if (track.startsWith("lang:")) {
    const code = track.slice(5)
    return LANGUAGES[code] ?? `Language ${code}`
  }
  return track.charAt(0).toUpperCase() + track.slice(1)
}

type Store = Pick<Storage, "getItem" | "setItem">

function defaultStore(): Store | null {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

/** The tracks the learner last chose, or null when none are stored or storage cannot be read. */
export function readTracks(store: Store | null = defaultStore()): string[] | null {
  try {
    const raw = store?.getItem(TRACKS_KEY)
    if (!raw) return null
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return null
    const tracks = parsed.filter((t): t is string => typeof t === "string" && t.length > 0)
    return tracks.length > 0 ? tracks : null
  } catch {
    return null
  }
}

/** Remembers the chosen tracks. Returns false when storage refused; the choice still applies for this session. */
export function writeTracks(tracks: string[], store: Store | null = defaultStore()): boolean {
  try {
    if (!store) return false
    store.setItem(TRACKS_KEY, JSON.stringify(tracks))
    return true
  } catch {
    return false
  }
}

const byLabel = (a: string, b: string) => trackLabel(a).localeCompare(trackLabel(b)) || (a < b ? -1 : a > b ? 1 : 0)

/** Every track a plan mentions: its minutes, deferred counts and items. */
export function seenTracks(plan: StudyPlan): string[] {
  const seen = new Set<string>([
    ...Object.keys(plan.summary.minutes_by_track),
    ...Object.keys(plan.summary.deferred_by_track),
    ...plan.items.map((i) => i.track),
  ])
  return [...seen].sort(byLabel)
}

export const mergeTracks = (...lists: string[][]): string[] => [...new Set(lists.flat())].sort(byLabel)

export const toggleTrack = (active: string[], track: string): string[] =>
  active.includes(track) ? active.filter((t) => t !== track) : mergeTracks(active, [track])

/**
 * The stepper moves by 1; the plan's minutes move by 5. A one-step change becomes a five-minute step in that
 * direction, and a typed value is rounded to the nearest 5. The result stays within 10 to 180.
 */
export function nextMinutes(current: number, requested: number): number {
  const diff = requested - current
  const raw = Math.abs(diff) === 1 ? current + Math.sign(diff) * MINUTES.step : Math.round(requested / MINUTES.step) * MINUTES.step
  return Math.min(MINUTES.max, Math.max(MINUTES.min, raw))
}

export type PlanControls = { minutes: number; fatigue: Fatigue; newItems: number; tracks: string[] }

export function buildRequest(nowSeconds: number, timezone: string, c: PlanControls): PlanRequest {
  return {
    now: nowSeconds,
    timezone,
    available_minutes: c.minutes,
    active_tracks: c.tracks,
    new_item_budget: c.newItems,
    fatigue: c.fatigue,
  }
}

export type TrackGroup = { track: string; label: string; items: PlanItem[]; minutes: number }

/** Planned items grouped by track, groups in label order, items in the plan's own order. Minutes come from the plan's summary. */
export function groupItems(plan: StudyPlan): TrackGroup[] {
  const groups = new Map<string, PlanItem[]>()
  for (const item of plan.items) groups.set(item.track, [...(groups.get(item.track) ?? []), item])
  return [...groups.entries()]
    .map(([track, items]) => ({
      track,
      label: trackLabel(track),
      items,
      minutes: plan.summary.minutes_by_track[track] ?? items.reduce((a, i) => a + i.estimated_seconds, 0) / 60,
    }))
    .sort((a, b) => byLabel(a.track, b.track))
}

export function fmtMinutes(minutes: number): string {
  return `${Number(minutes.toFixed(1))} min`
}

export function fmtSeconds(seconds: number): string {
  return seconds < 90 ? `${Math.round(seconds)} s` : fmtMinutes(seconds / 60)
}

export const FATIGUE_NOTE = "Tired shrinks today's plan. Choose None to undo it."
