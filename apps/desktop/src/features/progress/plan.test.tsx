import { renderToStaticMarkup } from "react-dom/server"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { PlanItem, StudyPlan } from "@/lib/backend"

const app = vi.hoisted(() => ({ inTauri: true }))
vi.mock("@/lib/backend", () => ({
  get inTauri() {
    return app.inTauri
  },
  shell: { learningSkills: vi.fn(), learningPlan: vi.fn() },
}))

import { PlanSection, PlanView } from "./Plan"
import {
  TRACKS_KEY,
  WIDE_TRACKS,
  buildRequest,
  groupItems,
  nextMinutes,
  readTracks,
  seenTracks,
  toggleTrack,
  writeTracks,
} from "./plan-logic"

const item = (entry_id: number, track: string, seconds = 20, reason = "due; lowest retrievability"): PlanItem => ({
  entry_id,
  track,
  reason,
  estimated_seconds: seconds,
  priority_components: { retrievability: 0.6, overdue_days: 1, track_share: 0.5 },
  due_at: null,
})

const PLAN: StudyPlan = {
  items: [item(1, "math", 180, "due procedures keep a share"), item(2, "lang:es", 15), item(3, "lang:es", 15), item(4, "math", 20)],
  summary: {
    minutes_by_track: { math: 3.3, "lang:es": 0.5 },
    deferred_by_track: { math: 4, chemistry: 2, english: 0 },
    total_minutes: 3.8,
    budget_minutes: 45,
    notes: ["4 due items did not fit the budget and stay due.", "plan-policy/1: every number here is a provisional local default."],
  },
}

const view = (over: Partial<Parameters<typeof PlanView>[0]> = {}) =>
  renderToStaticMarkup(
    <PlanView
      state={{ kind: "ready", data: PLAN }}
      minutes={45}
      fatigue="none"
      newItems={10}
      active={["math", "lang:es"]}
      chips={["chemistry", "lang:es", "math"]}
      {...over}
    />,
  )

function memoryStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial))
  return { data, getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => void data.set(k, v) }
}

beforeEach(() => {
  app.inTauri = true
})
afterEach(() => vi.unstubAllGlobals())

describe("grouping and minutes", () => {
  it("groups planned items by track in label order, keeping the plan's order, with the summary's minutes", () => {
    const groups = groupItems(PLAN)
    expect(groups.map((g) => [g.track, g.items.map((i) => i.entry_id), g.minutes])).toEqual([
      ["math", [1, 4], 3.3],
      ["lang:es", [2, 3], 0.5],
    ])
  })

  it("renders the budget used, minutes by track, deferred items by track, notes and items with reason and time", () => {
    const html = view()
    expect(html).toContain("3.8 min")
    expect(html).toContain("45 min budget")
    expect(html).toContain("4 items")
    expect(html).toContain('aria-label="Planned minutes by track"')
    expect(html).toContain("Spanish")
    expect(html).toContain("3.3 min")
    expect(html).toContain('aria-label="Deferred items by track"')
    expect(html).toContain("Chemistry")
    expect(html).toContain("2 items")
    expect(html).not.toContain("English") // a zero deferred count is not listed
    expect(html).toContain("4 due items did not fit the budget")
    expect(html).toContain("Item 1")
    expect(html).toContain("due procedures keep a share")
    expect(html).toContain("3 min") // the 180 s procedure
    expect(html).toContain("15 s")
  })

  it("says the plan is a suggestion and starts nothing", () => {
    expect(view()).toContain("nothing starts automatically")
  })

  it("says so when nothing is planned", () => {
    const empty: StudyPlan = { items: [], summary: { minutes_by_track: {}, deferred_by_track: {}, total_minutes: 0, budget_minutes: 45, notes: [] } }
    const html = view({ state: { kind: "ready", data: empty } })
    expect(html).toContain("nothing to plan")
    expect(html).not.toContain("Planned items")
  })

  it("lists every track a plan mentions", () => {
    expect(new Set(seenTracks(PLAN))).toEqual(new Set(["math", "lang:es", "chemistry", "english"]))
  })
})

describe("controls", () => {
  it("shows the defaults, fatigue choices with the undo note, and track chips", () => {
    const html = view()
    expect(html).toContain('value="45"')
    expect(html).toContain('value="10"')
    for (const label of ["None", "Tired", "Rested"]) expect(html).toContain(label)
    expect(html).toContain("Tired shrinks today")
    expect(html).toContain("undo")
    expect(html).toContain('aria-pressed="true"')
    expect(html).toContain('aria-pressed="false"') // chemistry is known but off
  })

  it("moves minutes in steps of 5 within 10 to 180", () => {
    expect(nextMinutes(45, 46)).toBe(50)
    expect(nextMinutes(45, 44)).toBe(40)
    expect(nextMinutes(10, 9)).toBe(10)
    expect(nextMinutes(180, 181)).toBe(180)
    expect(nextMinutes(175, 176)).toBe(180)
    expect(nextMinutes(45, 92)).toBe(90)
    expect(nextMinutes(45, 3)).toBe(10)
    expect(nextMinutes(45, 400)).toBe(180)
  })

  it("builds the request from the controls", () => {
    expect(buildRequest(1_790_000_000, "Europe/London", { minutes: 45, fatigue: "tired", newItems: 10, tracks: ["math"] })).toEqual({
      now: 1_790_000_000,
      timezone: "Europe/London",
      available_minutes: 45,
      active_tracks: ["math"],
      new_item_budget: 10,
      fatigue: "tired",
    })
  })

  it("toggles a track on and off", () => {
    expect(toggleTrack(["math"], "lang:es")).toEqual(["math", "lang:es"]) // label order: Math, Spanish
    expect(toggleTrack(["lang:es", "math"], "math")).toEqual(["lang:es"])
  })
})

describe("remembered tracks", () => {
  it("reads what the learner last chose", () => {
    expect(readTracks(memoryStorage({ [TRACKS_KEY]: JSON.stringify(["math", "lang:fr"]) }))).toEqual(["math", "lang:fr"])
  })

  it("returns null when nothing usable is stored", () => {
    expect(readTracks(memoryStorage())).toBeNull()
    expect(readTracks(memoryStorage({ [TRACKS_KEY]: "not json" }))).toBeNull()
    expect(readTracks(memoryStorage({ [TRACKS_KEY]: JSON.stringify({ a: 1 }) }))).toBeNull()
    expect(readTracks(memoryStorage({ [TRACKS_KEY]: JSON.stringify([]) }))).toBeNull()
    expect(readTracks(null)).toBeNull()
  })

  it("falls back to null, and write reports false, when storage throws", () => {
    const blocked = {
      getItem: () => {
        throw new Error("blocked")
      },
      setItem: () => {
        throw new Error("blocked")
      },
    }
    expect(readTracks(blocked)).toBeNull()
    expect(writeTracks(["math"], blocked)).toBe(false)
    expect(writeTracks(["math"], null)).toBe(false)
  })

  it("writes under tutor.plan.tracks", () => {
    const store = memoryStorage()
    expect(writeTracks(["math"], store)).toBe(true)
    expect(store.data.get("tutor.plan.tracks")).toBe('["math"]')
  })

  it("the section starts from the remembered tracks", () => {
    vi.stubGlobal("window", { localStorage: memoryStorage({ [TRACKS_KEY]: JSON.stringify(["math", "lang:ja"]) }) })
    const html = renderToStaticMarkup(<PlanSection refreshKey={0} />)
    expect(html).toContain("Japanese")
    expect(html).toContain("Math")
    expect(html).not.toContain("Spanish")
  })

  it("the section still renders, with no chips yet, when storage throws", () => {
    const blocked = () => {
      throw new Error("blocked")
    }
    vi.stubGlobal("window", { localStorage: { getItem: blocked, setItem: blocked } })
    const html = renderToStaticMarkup(<PlanSection refreshKey={0} />)
    expect(html).toContain("Today&#x27;s plan")
    expect(html).toContain("Tracks appear once the plan finds due or new items")
  })

  it("the first request's default covers every track the app teaches", () => {
    expect(WIDE_TRACKS).toEqual(["general", "math", "english", "science", "chemistry", "lang:es", "lang:fr", "lang:de", "lang:ja", "lang:zh"])
  })
})

describe("states", () => {
  it("says the desktop app is needed when not in it", () => {
    app.inTauri = false
    const html = renderToStaticMarkup(<PlanSection refreshKey={0} />)
    expect(html).toContain("needs the desktop app")
    expect(html).not.toContain("Available minutes")
  })

  it("shows loading with the controls in the desktop app", () => {
    const html = renderToStaticMarkup(<PlanSection refreshKey={0} />)
    expect(html).toContain("Loading")
    expect(html).toContain("Available minutes")
  })

  it("shows the command's error sentence and keeps the controls", () => {
    const html = view({ state: { kind: "error", message: "The backend did not answer on port 8001." } })
    expect(html).toContain("The backend did not answer on port 8001.")
    expect(html).toContain("Available minutes")
  })
})
