// Today's plan: a suggestion from the study plan (contract C13, read through shell.learningPlan). Changing a
// control asks for a new plan. Nothing starts from here.
import { useCallback, useEffect, useState, type ReactNode } from "react"
import { RefreshCw } from "lucide-react"

import { BarList, Eyebrow, Panel, Segmented, Stepper } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { inTauri, shell, type StudyPlan } from "@/lib/backend"
import { cn } from "@/lib/utils"
import { StatePanel, browserTimezone, commandError, type LoadState } from "./learner"
import { Notice, Quiet } from "./parts"
import {
  FATIGUE_NOTE,
  MINUTES,
  NEW_ITEMS,
  WIDE_TRACKS,
  buildRequest,
  fmtMinutes,
  fmtSeconds,
  groupItems,
  mergeTracks,
  nextMinutes,
  readTracks,
  seenTracks,
  toggleTrack,
  trackLabel,
  writeTracks,
  type Fatigue,
} from "./plan-logic"

export const PLAN_NEEDS_APP =
  "The study plan needs the desktop app, which reads the practice history on this machine. Open the Tutor app to see today's plan."

const FATIGUE_OPTIONS = [
  { value: "none" as const, label: "None" },
  { value: "tired" as const, label: "Tired" },
  { value: "rested" as const, label: "Rested" },
]

const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

/** Loads the plan for the current controls; `refreshKey` changes when the screen is refreshed. */
export function PlanSection({ refreshKey }: { refreshKey: number }) {
  const [minutes, setMinutes] = useState<number>(MINUTES.initial)
  const [fatigue, setFatigue] = useState<Fatigue>("none")
  const [newItems, setNewItems] = useState<number>(NEW_ITEMS.initial)
  // The learner's own choice, or null until they make one: the first request then uses the wide default.
  const [chosen, setChosen] = useState<string[] | null>(() => readTracks())
  const [known, setKnown] = useState<string[]>([])
  const [state, setState] = useState<LoadState<StudyPlan>>(() => (inTauri ? { kind: "loading" } : { kind: "app-required" }))
  const [updating, setUpdating] = useState(false)
  const [retry, setRetry] = useState(0)

  useEffect(() => {
    if (!inTauri) {
      setState({ kind: "app-required" })
      return
    }
    let live = true
    setUpdating(true)
    const timer = setTimeout(() => {
      const request = buildRequest(Date.now() / 1000, browserTimezone(), {
        minutes,
        fatigue,
        newItems,
        tracks: chosen ?? WIDE_TRACKS,
      })
      shell
        .learningPlan(request)
        .then((plan) => {
          if (!live) return
          setKnown((k) => mergeTracks(k, seenTracks(plan)))
          setState({ kind: "ready", data: plan })
        })
        .catch((e: unknown) => live && setState({ kind: "error", message: commandError(e) }))
        .finally(() => live && setUpdating(false))
    }, 200)
    return () => {
      live = false
      clearTimeout(timer)
    }
  }, [refreshKey, retry, minutes, fatigue, newItems, chosen])

  const active = chosen ?? (state.kind === "ready" ? seenTracks(state.data) : [])
  const chips = mergeTracks(known, active)

  const onToggleTrack = useCallback(
    (track: string) => {
      const next = toggleTrack(active, track)
      if (next.length === 0) return
      setChosen(next)
      writeTracks(next)
    },
    [active],
  )

  const onRetry = useCallback(() => {
    setState({ kind: "loading" })
    setRetry((n) => n + 1)
  }, [])

  return (
    <PlanView
      state={state}
      updating={updating}
      minutes={minutes}
      fatigue={fatigue}
      newItems={newItems}
      active={active}
      chips={chips}
      onMinutes={(v) => setMinutes((m) => nextMinutes(m, v))}
      onFatigue={setFatigue}
      onNewItems={setNewItems}
      onToggleTrack={onToggleTrack}
      onRetry={onRetry}
    />
  )
}

type PlanViewProps = {
  state: LoadState<StudyPlan>
  updating?: boolean
  minutes: number
  fatigue: Fatigue
  newItems: number
  active: string[]
  /** Every track the learner can switch on or off. */
  chips: string[]
  onMinutes?: (v: number) => void
  onFatigue?: (v: Fatigue) => void
  onNewItems?: (v: number) => void
  onToggleTrack?: (track: string) => void
  onRetry?: () => void
}

export function PlanView(p: PlanViewProps) {
  if (p.state.kind === "app-required") {
    return <StatePanel eyebrow="Today" title="Today's plan" state={p.state} needsApp={PLAN_NEEDS_APP} />
  }
  const none = () => undefined

  return (
    <Panel
      eyebrow="Today"
      title="Today's plan"
      aria-busy={p.updating ? true : undefined}
      actions={p.updating ? <span className="text-ink-3 font-mono text-[11px] tracking-[0.08em]">Updating…</span> : null}
    >
      <p className="text-ink-2 -mt-2 mb-5 max-w-[68ch] text-sm leading-relaxed text-pretty">
        A suggestion, not a schedule: nothing starts automatically. One time budget covers every subject and language, and
        every number is a provisional local default.
      </p>

      <div className="grid gap-x-8 gap-y-5 sm:grid-cols-2 lg:grid-cols-3">
        <Control label="Available minutes">
          <Stepper label="available minutes" value={p.minutes} min={MINUTES.min} max={MINUTES.max} onChange={p.onMinutes ?? none} />
        </Control>
        <Control label="New items per day">
          <Stepper label="new items per day" value={p.newItems} min={NEW_ITEMS.min} max={NEW_ITEMS.max} onChange={p.onNewItems ?? none} />
        </Control>
        <Control label="Fatigue" note={FATIGUE_NOTE}>
          <Segmented label="Fatigue" value={p.fatigue} options={FATIGUE_OPTIONS} onChange={p.onFatigue ?? none} />
        </Control>
      </div>

      <div className="mt-5">
        <Eyebrow>Active tracks</Eyebrow>
        {p.chips.length === 0 ? (
          <p className="text-ink-2 mt-2 text-[13px] leading-relaxed">Tracks appear once the plan finds due or new items.</p>
        ) : (
          <ul aria-label="Active tracks" className="m-0 mt-2 flex list-none flex-wrap gap-2 p-0">
            {p.chips.map((t) => {
              const on = p.active.includes(t)
              return (
                <li key={t}>
                  <button
                    type="button"
                    aria-pressed={on}
                    onClick={() => p.onToggleTrack?.(t)}
                    className={cn(
                      "rounded-full border px-3.5 py-1.5 text-sm transition-colors",
                      FOCUS,
                      on ? "bg-card border-hairline-strong text-foreground font-medium" : "border-hairline text-ink-2 hover:text-foreground",
                    )}
                    style={{ transitionDuration: "var(--dur-base)" }}
                  >
                    {trackLabel(t)}
                  </button>
                </li>
              )
            })}
          </ul>
        )}
      </div>

      <div className="border-hairline mt-6 border-t pt-5">
        {p.state.kind === "loading" ? (
          <Quiet>Loading…</Quiet>
        ) : p.state.kind === "error" ? (
          <div className="flex flex-col items-start gap-3">
            <Notice>
              <p className="font-medium">The plan could not be loaded.</p>
              <p className="text-ink-2 mt-1">{p.state.message}</p>
            </Notice>
            {p.onRetry ? (
              <Button variant="outline" size="sm" onClick={p.onRetry}>
                <RefreshCw />
                Try again
              </Button>
            ) : null}
          </div>
        ) : (
          <PlanBody plan={p.state.data} />
        )}
      </div>
    </Panel>
  )
}

function Control({ label, note, children }: { label: string; note?: string; children: ReactNode }) {
  return (
    <div className="flex min-w-0 flex-col items-start gap-2">
      <Eyebrow>{label}</Eyebrow>
      {children}
      {note ? <p className="text-ink-3 max-w-[34ch] text-[12px] leading-snug">{note}</p> : null}
    </div>
  )
}

function PlanBody({ plan }: { plan: StudyPlan }) {
  const { summary } = plan
  const groups = groupItems(plan)
  const minutesRows = Object.entries(summary.minutes_by_track)
    .map(([track, value]) => ({ label: trackLabel(track), value }))
    .sort((a, b) => b.value - a.value || a.label.localeCompare(b.label))
  const deferred = Object.entries(summary.deferred_by_track)
    .filter(([, n]) => n > 0)
    .sort(([a], [b]) => trackLabel(a).localeCompare(trackLabel(b)))

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <span className="text-[28px] leading-none font-light tracking-[-0.02em] tabular-nums">{fmtMinutes(summary.total_minutes)}</span>
        <span className="text-ink-2 text-sm">
          planned of a {fmtMinutes(summary.budget_minutes)} budget, {plan.items.length} {plan.items.length === 1 ? "item" : "items"}
        </span>
      </div>

      {plan.items.length === 0 ? (
        <Quiet>Nothing is due or new in the active tracks, so there is nothing to plan.</Quiet>
      ) : (
        <>
          <div>
            <Eyebrow>Minutes by track</Eyebrow>
            <BarList
              className="mt-3"
              label="Planned minutes by track"
              rows={minutesRows}
              format={fmtMinutes}
              empty="No minutes are planned."
            />
          </div>

          {deferred.length > 0 ? (
            <div>
              <Eyebrow>Due but deferred</Eyebrow>
              <ul aria-label="Deferred items by track" className="m-0 mt-3 flex list-none flex-col gap-1 p-0 text-[13px]">
                {deferred.map(([track, n]) => (
                  <li key={track} className="flex items-baseline gap-3">
                    <span className="min-w-0 flex-1">{trackLabel(track)}</span>
                    <span className="font-mono text-[12px] tabular-nums">
                      {n} {n === 1 ? "item" : "items"}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          <div>
            <Eyebrow>Planned items</Eyebrow>
            <div className="mt-3 flex flex-col gap-2">
              {groups.map((g, i) => (
                <details key={g.track} open={i === 0} className="border-hairline rounded-xl border">
                  <summary className={cn("flex cursor-pointer flex-wrap items-baseline gap-x-3 px-4 py-3 text-sm", FOCUS)}>
                    <span className="font-medium">{g.label}</span>
                    <span className="text-ink-2 font-mono text-[12px] tabular-nums">
                      {g.items.length} {g.items.length === 1 ? "item" : "items"}, {fmtMinutes(g.minutes)}
                    </span>
                  </summary>
                  <ul className="m-0 list-none p-0">
                    {g.items.map((item) => (
                      <li key={item.entry_id} className="border-hairline flex items-baseline gap-3 border-t px-4 py-2.5 text-[13px]">
                        <span className="text-ink-3 w-20 shrink-0 font-mono text-[12px] tabular-nums">Item {item.entry_id}</span>
                        <span className="text-ink-2 min-w-0 flex-1 leading-snug">{item.reason}</span>
                        <span className="shrink-0 font-mono text-[12px] tabular-nums">{fmtSeconds(item.estimated_seconds)}</span>
                      </li>
                    ))}
                  </ul>
                </details>
              ))}
            </div>
          </div>
        </>
      )}

      {summary.notes.length > 0 ? (
        <div>
          <Eyebrow>Notes</Eyebrow>
          <ul className="text-ink-2 m-0 mt-3 list-disc space-y-1 pl-5 text-[13px] leading-relaxed">
            {summary.notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  )
}
