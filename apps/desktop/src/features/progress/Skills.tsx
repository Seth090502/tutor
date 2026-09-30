// Skills: what the practice history says about each tagged skill (contract C13, read through shell.learningSkills).
// Estimates are provisional and advisory. Denominators, exclusions and limitations sit next to every number.
import { useCallback, useEffect, useState } from "react"

import { Eyebrow, LedStatus, Panel, type LedTone } from "@/components/kit"
import { inTauri, shell, type LearningSkills, type SkillState } from "@/lib/backend"
import { StatePanel, browserTimezone, commandError, type LoadState } from "./learner"
import { Quiet } from "./parts"
import {
  EMPTY_SKILLS,
  delayedText,
  denominators,
  estimateText,
  evidenceNeeded,
  excludedRows,
  policyLine,
  sortSkills,
  statusWord,
} from "./skills-logic"

const TONES: Record<string, LedTone> = { needs_review: "warn", provisionally_mastered: "ok" }

export const SKILLS_NEEDS_APP =
  "Skill estimates need the desktop app, which reads the practice history on this machine. Open the Tutor app to see them."

/** Loads the skill estimates; `refreshKey` changes when the screen is refreshed. */
export function SkillsSection({ refreshKey }: { refreshKey: number }) {
  const [state, setState] = useState<LoadState<LearningSkills>>(() => (inTauri ? { kind: "loading" } : { kind: "app-required" }))
  const [retry, setRetry] = useState(0)

  useEffect(() => {
    if (!inTauri) {
      setState({ kind: "app-required" })
      return
    }
    let live = true
    shell
      .learningSkills(browserTimezone())
      .then((data) => live && setState({ kind: "ready", data }))
      .catch((e: unknown) => live && setState({ kind: "error", message: commandError(e) }))
    return () => {
      live = false
    }
  }, [refreshKey, retry])

  const onRetry = useCallback(() => {
    setState({ kind: "loading" })
    setRetry((n) => n + 1)
  }, [])

  return <SkillsView state={state} onRetry={onRetry} />
}

export function SkillsView({ state, onRetry }: { state: LoadState<LearningSkills>; onRetry?: () => void }) {
  if (state.kind !== "ready") {
    return <StatePanel eyebrow="Skills" title="Skill estimates" state={state} needsApp={SKILLS_NEEDS_APP} onRetry={onRetry} />
  }
  const { data } = state
  const skills = sortSkills(data.skills)
  const excluded = excludedRows(data.excluded)

  return (
    <Panel eyebrow="Skills" title="Skill estimates">
      <p className="text-ink-2 -mt-2 mb-5 max-w-[68ch] text-sm leading-relaxed text-pretty">{policyLine(data)}</p>

      {skills.length === 0 ? (
        <Quiet>{EMPTY_SKILLS}</Quiet>
      ) : (
        <ul aria-label="Skills" className="m-0 flex list-none flex-col p-0">
          {skills.map((s) => (
            <SkillRow key={s.skill_id} skill={s} />
          ))}
        </ul>
      )}

      {excluded.length > 0 || data.skipped_unknown > 0 ? (
        <div className="border-hairline mt-6 border-t pt-5">
          <Eyebrow>Left out of the estimates</Eyebrow>
          {excluded.length > 0 ? (
            <>
              <p className="text-ink-2 mt-3 text-[13px] leading-relaxed">
                Attempts that do not count as evidence, by reason. One attempt can have more than one reason.
              </p>
              <ul aria-label="Excluded attempts by reason" className="m-0 mt-2 flex list-none flex-col gap-1 p-0 text-[13px]">
                {excluded.map((r) => (
                  <li key={r.reason} className="flex items-baseline gap-3">
                    <span className="min-w-0 flex-1">{r.label}</span>
                    <span className="font-mono text-[12px] tabular-nums">{r.count}</span>
                  </li>
                ))}
              </ul>
            </>
          ) : null}
          {data.skipped_unknown > 0 ? (
            <p className="text-ink-2 mt-3 text-[13px] leading-relaxed">
              {data.skipped_unknown} closed {data.skipped_unknown === 1 ? "attempt was" : "attempts were"} skipped because its opening
              record is missing (skipped_unknown).
            </p>
          ) : null}
        </div>
      ) : null}

      {data.limitations.length > 0 ? (
        <div className="border-hairline mt-6 border-t pt-5">
          <Eyebrow>Limitations</Eyebrow>
          <ul className="text-ink-2 m-0 mt-3 list-disc space-y-1 pl-5 text-[13px] leading-relaxed">
            {data.limitations.map((l, i) => (
              <li key={i}>{l}</li>
            ))}
          </ul>
        </div>
      ) : null}
    </Panel>
  )
}

function SkillRow({ skill: s }: { skill: SkillState }) {
  const estimate = estimateText(s)
  return (
    <li className="border-hairline border-b py-5 first:pt-0 last:border-b-0 last:pb-0">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-2">
        <h3 className="min-w-0 font-mono text-[14px] break-all">{s.skill_id}</h3>
        <LedStatus tone={TONES[s.status] ?? "off"} state={statusWord(s.status)} />
      </div>

      <p className="mt-2 text-[13px] leading-relaxed">
        {estimate ?? <span className="text-ink-2">Evidence so far: {evidenceNeeded(s)}.</span>}
      </p>

      <dl className="mt-3 grid grid-cols-2 gap-x-6 gap-y-2 text-[13px] sm:grid-cols-4">
        {denominators(s).map((d) => (
          <div key={d.label} className="min-w-0">
            <dt className="text-ink-3 text-[11px] leading-snug">{d.label}</dt>
            <dd className="m-0 font-mono text-[13px] tabular-nums">{d.value}</dd>
          </div>
        ))}
      </dl>

      <p className="text-ink-2 mt-3 text-[13px] leading-relaxed">{delayedText(s)}</p>

      {s.limitations.length > 0 ? (
        <ul className="text-ink-3 m-0 mt-2 list-disc space-y-0.5 pl-5 text-[12px] leading-relaxed">
          {s.limitations.map((l, i) => (
            <li key={i}>{l}</li>
          ))}
        </ul>
      ) : null}
    </li>
  )
}
