import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"

import type { LearningSkills, SkillState } from "@/lib/backend"
import { SkillsSection, SkillsView } from "./Skills"
import { EMPTY_SKILLS, delayedText, estimateText, evidenceNeeded, excludedRows, sortSkills } from "./skills-logic"

const app = vi.hoisted(() => ({ inTauri: true }))
vi.mock("@/lib/backend", () => ({
  get inTauri() {
    return app.inTauri
  },
  shell: { learningSkills: vi.fn(), learningPlan: vi.fn() },
}))

const skill = (over: Partial<SkillState> = {}): SkillState => ({
  skill_id: "math.fractions",
  policy_version: "skill-policy/1",
  alpha: 9,
  beta: 3,
  effective_weight: 10,
  eligible_observations: 17,
  dependent_observations: 2,
  distinct_families: 3,
  families_available: 4,
  distinct_dates: 5,
  last_exposure_at: 1_790_000_000,
  delayed_evidence: [],
  p_mastery: 0.874,
  status: "developing",
  limitations: ["fewer than 3 families authored"],
  ...over,
})

const data = (skills: SkillState[], over: Partial<LearningSkills> = {}): LearningSkills => ({
  store_id: "s",
  policy_version: "skill-policy/1",
  last_event_seq: 10,
  skills,
  excluded: {},
  skipped_unknown: 0,
  limitations: [],
  ...over,
})

const view = (d: LearningSkills) => renderToStaticMarkup(<SkillsView state={{ kind: "ready", data: d }} />)

beforeEach(() => {
  app.inTauri = true
})

describe("status words and order", () => {
  it("shows each status in words", () => {
    const html = view(
      data([
        skill({ skill_id: "a", status: "insufficient" }),
        skill({ skill_id: "b", status: "developing" }),
        skill({ skill_id: "c", status: "provisionally_mastered" }),
        skill({ skill_id: "d", status: "needs_review" }),
      ]),
    )
    for (const words of ["Not enough evidence yet", "Developing", "Provisionally mastered", "Needs review"]) expect(html).toContain(words)
    expect(html).not.toContain("provisionally_mastered")
    expect(html).not.toContain("needs_review")
  })

  it("sorts needs review, developing, provisionally mastered, insufficient, then by id", () => {
    const sorted = sortSkills([
      skill({ skill_id: "z", status: "insufficient" }),
      skill({ skill_id: "b", status: "provisionally_mastered" }),
      skill({ skill_id: "y", status: "developing" }),
      skill({ skill_id: "a", status: "developing" }),
      skill({ skill_id: "m", status: "needs_review" }),
      skill({ skill_id: "a2", status: "insufficient" }),
    ])
    expect(sorted.map((s) => s.skill_id)).toEqual(["m", "a", "y", "b", "a2", "z"])
  })

  it("does not mutate the list it was given", () => {
    const input = [skill({ skill_id: "b", status: "insufficient" }), skill({ skill_id: "a", status: "needs_review" })]
    sortSkills(input)
    expect(input.map((s) => s.skill_id)).toEqual(["b", "a"])
  })

  it("renders rows in that order", () => {
    const html = view(data([skill({ skill_id: "zeta", status: "insufficient" }), skill({ skill_id: "alpha", status: "needs_review" })]))
    expect(html.indexOf("alpha")).toBeLessThan(html.indexOf("zeta"))
  })
})

describe("the estimate", () => {
  it("never shows a percentage for an insufficient skill, whatever p_mastery is", () => {
    const s = skill({ status: "insufficient", p_mastery: 0.99, eligible_observations: 6, dependent_observations: 0, distinct_dates: 2 })
    expect(estimateText(s)).toBeNull()
    const html = view(data([s]))
    expect(html).not.toContain("Chance this skill")
    expect(html).not.toContain("99%")
    expect(html).toContain("6 of 15 independent attempts, 2 of 3 days")
  })

  it("counts only independent attempts toward the evidence still needed", () => {
    expect(evidenceNeeded(skill({ eligible_observations: 8, dependent_observations: 3, distinct_dates: 1 }))).toBe(
      "5 of 15 independent attempts, 1 of 3 days",
    )
  })

  it("states the chance of being at least 70% reliable for other statuses", () => {
    expect(estimateText(skill({ p_mastery: 0.874 }))).toBe("Chance this skill is at least 70% reliable: 87%")
    expect(view(data([skill({ p_mastery: 0.874 })]))).toContain("Chance this skill is at least 70% reliable: 87%")
  })

  it("does not round up to certainty or down to impossibility", () => {
    expect(estimateText(skill({ p_mastery: 0.9996 }))).toContain("99%")
    expect(estimateText(skill({ p_mastery: 0.0003 }))).toContain(": 1%")
  })

  it("says so when the estimate is not a number", () => {
    expect(estimateText(skill({ p_mastery: Number.NaN }))).toContain("unavailable")
  })
})

describe("denominators and delayed check", () => {
  it("renders every denominator", () => {
    const html = view(data([skill()]))
    expect(html).toContain("Independent eligible attempts")
    expect(html).toContain("Dependent attempts (flagged, no weight)")
    expect(html).toContain("Distinct dates")
    expect(html).toContain("3 of 4") // families seen of available
    expect(html).toMatch(/Independent eligible attempts<\/dt><dd[^>]*>15</)
    expect(html).toMatch(/Dependent attempts \(flagged, no weight\)<\/dt><dd[^>]*>2</)
    expect(html).toMatch(/Distinct dates<\/dt><dd[^>]*>5</)
  })

  it("reports no delayed check when there is none", () => {
    expect(delayedText(skill())).toContain("No delayed check yet")
  })

  it("reports the latest delayed check by time, correct or incorrect", () => {
    const evidence = [
      { attempt_id: "late", at: 2000, gap_days: 9.4, y: 0 as const },
      { attempt_id: "early", at: 1000, gap_days: 8, y: 1 as const },
    ]
    expect(delayedText(skill({ delayed_evidence: evidence }), () => "Oct 1")).toBe(
      "Most recent delayed check: incorrect, 9 days after the previous attempt (Oct 1).",
    )
    const one = [{ attempt_id: "x", at: 1, gap_days: 1, y: 1 as const }]
    expect(delayedText(skill({ delayed_evidence: one }), () => "Oct 1")).toContain("correct, 1 day after")
  })

  it("lists the skill's limitations", () => {
    expect(view(data([skill()]))).toContain("fewer than 3 families authored")
  })
})

describe("header, exclusions and limitations", () => {
  it("states the policy version and that estimates are provisional and advisory", () => {
    const html = view(data([skill()]))
    expect(html).toContain("skill-policy/1")
    expect(html).toContain("provisional and advisory")
    expect(html).toContain("never decide what you may study")
  })

  it("turns excluded reasons into words, largest first, and drops zeros", () => {
    const rows = excludedRows({ exposed_within_24h: 2, assisted: 7, unverified: 0, some_new_reason: 1 })
    expect(rows.map((r) => r.reason)).toEqual(["assisted", "exposed_within_24h", "some_new_reason"])
    const html = view(data([skill()], { excluded: { assisted: 7, exposed_within_24h: 2, no_checked_response: 1 } }))
    expect(html).toContain("Assisted (a hint or the answer was shown first)")
    expect(html).toContain("Exposed within 24 hours")
    expect(html).toContain("No checked response")
    expect(html).not.toContain("exposed_within_24h")
    expect(html).toContain("Left out of the estimates")
  })

  it("mentions skipped_unknown only when it is non-zero", () => {
    expect(view(data([skill()]))).not.toContain("skipped_unknown")
    const html = view(data([skill()], { skipped_unknown: 3 }))
    expect(html).toContain("3 closed attempts were skipped")
    expect(html).toContain("skipped_unknown")
  })

  it("shows the store's own limitations", () => {
    expect(view(data([skill()], { limitations: ["pilot half-life of 30 days"] }))).toContain("pilot half-life of 30 days")
  })
})

describe("states", () => {
  it("shows the empty message when there are no skills", () => {
    const html = view(data([]))
    expect(html).toContain(EMPTY_SKILLS.replace("<", "&lt;").replace(">", "&gt;"))
    expect(html).toContain("skill-policy/1")
  })

  it("says the desktop app is needed when not in it", () => {
    app.inTauri = false
    const html = renderToStaticMarkup(<SkillsSection refreshKey={0} />)
    expect(html).toContain("need the desktop app")
    expect(html).toContain("practice history")
  })

  it("shows loading first in the desktop app", () => {
    expect(renderToStaticMarkup(<SkillsSection refreshKey={0} />)).toContain("Loading")
  })

  it("shows the command's error sentence", () => {
    const html = renderToStaticMarkup(<SkillsView state={{ kind: "error", message: "The study feed is not reachable." }} />)
    expect(html).toContain("The study feed is not reachable.")
  })
})
