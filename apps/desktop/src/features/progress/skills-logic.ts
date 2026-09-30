// Pure helpers for the Skills section (contract C13, skill-policy/1). Wording follows the contract: every
// number is a provisional local policy, advisory and never a gate on a lesson.
import type { LearningSkills, SkillState, SkillStatus } from "@/lib/backend"

/** Floors from skill-policy/1 below which a skill is "insufficient". Mirrors the contract, not read from the backend. */
export const MIN_INDEPENDENT = 15
export const MIN_DATES = 3
export const DELAYED_GAP_DAYS = 7

export const EMPTY_SKILLS = "Skill estimates appear once practice items carry skill tags (skill:<id>)."

export const STATUS_WORDS: Record<SkillStatus, string> = {
  insufficient: "Not enough evidence yet",
  developing: "Developing",
  provisionally_mastered: "Provisionally mastered",
  needs_review: "Needs review",
}

const STATUS_ORDER: Record<SkillStatus, number> = {
  needs_review: 0,
  developing: 1,
  provisionally_mastered: 2,
  insufficient: 3,
}

/** A status the app does not know (a newer backend) is shown as sent and sorted last. */
export const statusWord = (status: string): string => STATUS_WORDS[status as SkillStatus] ?? status
const statusRank = (status: string): number => STATUS_ORDER[status as SkillStatus] ?? 4

/** Needs review, developing, provisionally mastered, insufficient; then by skill id. */
export function sortSkills(skills: SkillState[]): SkillState[] {
  return [...skills].sort(
    (a, b) => statusRank(a.status) - statusRank(b.status) || (a.skill_id < b.skill_id ? -1 : a.skill_id > b.skill_id ? 1 : 0),
  )
}

/** Eligible attempts that carry weight; dependent ones (same family, same local date) are kept but weigh nothing. */
export const independentAttempts = (s: SkillState): number => Math.max(0, s.eligible_observations - s.dependent_observations)

/**
 * "Chance this skill is at least 70% reliable: NN%", or null for an insufficient skill: below the evidence floor a
 * percentage would read as a finding. A rounded 100% or 0% is held to 99% and 1% because the posterior is never certain.
 */
export function estimateText(s: SkillState): string | null {
  if (s.status === "insufficient") return null
  const p = s.p_mastery
  if (!Number.isFinite(p)) return "Chance this skill is at least 70% reliable: unavailable"
  let pct = Math.round(p * 100)
  if (pct >= 100 && p < 1) pct = 99
  if (pct <= 0 && p > 0) pct = 1
  return `Chance this skill is at least 70% reliable: ${Math.min(100, Math.max(0, pct))}%`
}

/** What an insufficient skill still needs, e.g. "6 of 15 independent attempts, 2 of 3 days". */
export const evidenceNeeded = (s: SkillState): string =>
  `${independentAttempts(s)} of ${MIN_INDEPENDENT} independent attempts, ${s.distinct_dates} of ${MIN_DATES} days`

export type Denominator = { label: string; value: string }

export function denominators(s: SkillState): Denominator[] {
  return [
    { label: "Independent eligible attempts", value: String(independentAttempts(s)) },
    { label: "Dependent attempts (flagged, no weight)", value: String(s.dependent_observations) },
    { label: "Distinct dates", value: String(s.distinct_dates) },
    { label: "Families seen", value: `${s.distinct_families} of ${s.families_available}` },
  ]
}

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`

export function shortDate(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toLocaleDateString("en-US", { month: "short", day: "numeric" })
}

/** The most recent delayed check (the latest by time), in words; its gap is to the previous exposure of the skill. */
export function delayedText(s: SkillState, date: (epochSeconds: number) => string = shortDate): string {
  const last = s.delayed_evidence.reduce<SkillState["delayed_evidence"][number] | null>((a, d) => (a === null || d.at > a.at ? d : a), null)
  if (!last) return `No delayed check yet. One needs an attempt ${DELAYED_GAP_DAYS} or more days after the skill was last practised.`
  const gap = Math.max(0, Math.round(last.gap_days))
  return `Most recent delayed check: ${last.y === 1 ? "correct" : "incorrect"}, ${plural(gap, "day")} after the previous attempt (${date(last.at)}).`
}

const REASON_WORDS: Record<string, string> = {
  assisted: "Assisted (a hint or the answer was shown first)",
  unverified: "Unverified (the answer could not be checked)",
  exposed_within_24h: "Exposed within 24 hours (the item was seen recently)",
  no_checked_response: "No checked response",
  instruction_mode: "Instruction mode (teaching, not practice)",
  invalid_item: "Invalid item (withdrawn afterwards)",
  unspecified: "No reason recorded",
}

export const reasonWords = (reason: string): string => REASON_WORDS[reason] ?? reason.replace(/_/g, " ")

export type ExcludedRow = { reason: string; label: string; count: number }

/** Excluded counts by reason, largest first; zero counts are dropped. */
export function excludedRows(excluded: Record<string, number>): ExcludedRow[] {
  return Object.entries(excluded)
    .filter(([, count]) => count > 0)
    .map(([reason, count]) => ({ reason, label: reasonWords(reason), count }))
    .sort((a, b) => b.count - a.count || (a.reason < b.reason ? -1 : 1))
}

export const policyLine = (data: LearningSkills): string =>
  `Policy ${data.policy_version}. These estimates are provisional and advisory: they never decide what you may study, and they are not evidence that you have learned.`
