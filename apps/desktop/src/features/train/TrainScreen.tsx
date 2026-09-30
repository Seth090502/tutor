// Train: Dual N-Back (docs/tutor/NBACK.md, contract C6). Setup, timing check, the run, a check-in
// and history. Training works in a plain browser; saving and history need the desktop app.
import { useCallback, useEffect, useState } from "react"

import { Button } from "@/components/ui/button"
import { inTauri } from "@/lib/backend"
import { audioContext, loadClips, type Clips } from "./audio"
import { CalibrationPanel } from "./Calibration"
import { CheckinForm, SessionSummary } from "./Checkin"
import { weekMinutes } from "./engine/history"
import { newSeed } from "./engine/prng"
import { DEFAULT_SETTINGS, TRANSFER_SETTINGS, transferLevel, type SessionConfig } from "./engine/session"
import { HistoryPanel } from "./History"
import { RunView } from "./Run"
import { SetupPanel } from "./Setup"
import { loadCalibration, nbackStore, type Calibration } from "./storage"
import type { Checkin, Mode, NBackSession, NBackSummary, Settings } from "./types"

type Levels = Partial<Record<Mode, number>>
type Save = { state: "saving" | "saved" | "error" | "unavailable"; detail?: string }

const errText = (e: unknown) => (e instanceof Error ? e.message : String(e))
/** A session's level at its end: the last block's next level. */
const finalLevel = (s: NBackSession) => s.blocks[s.blocks.length - 1]?.next_n

export function TrainScreen() {
  const [mode, setMode] = useState<Mode>("dual")
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS)
  const [startN, setStartN] = useState<number | null>(null)
  const [calibration, setCalibration] = useState<Calibration | null>(loadCalibration)
  const [starting, setStarting] = useState(false)
  const [startError, setStartError] = useState<string | null>(null)
  const [run, setRun] = useState<{ config: SessionConfig; ctx: AudioContext; clips: Clips } | null>(null)
  const [record, setRecord] = useState<NBackSession | null>(null)
  const [save, setSave] = useState<Save | null>(null)
  const [checkedIn, setCheckedIn] = useState(false)

  const [sessions, setSessions] = useState<NBackSummary[]>([])
  const [levels, setLevels] = useState<Levels>({})
  const [week, setWeek] = useState<{ minutes: number; sessions: number } | null>(null)
  const [historyError, setHistoryError] = useState<string | null>(null)
  const [exported, setExported] = useState<string | null>(null)

  // Decode the letter clips at startup (no AudioContext yet).
  useEffect(() => {
    loadClips().catch((e) => setStartError(`Could not load the letter sounds: ${errText(e)}`))
  }, [])

  const refresh = useCallback(async () => {
    if (!inTauri) return
    try {
      const { sessions } = await nbackStore.sessions()
      const now = Date.now() / 1000
      // Full records for the last week (minutes) and each mode's latest session (its end level).
      const latest = (["dual", "position", "letter"] as const).flatMap((m) => sessions.find((s) => s.mode === m) ?? [])
      const ids = new Set([...sessions.filter((s) => s.started_at > now - 7 * 86_400), ...latest].map((s) => s.id))
      const full = await Promise.all([...ids].map((id) => nbackStore.session(id)))
      const byId = new Map(full.map((s) => [s.id, s]))
      setSessions(sessions)
      setWeek(weekMinutes(full.filter((s) => s.started_at > now - 7 * 86_400), now))
      setLevels(Object.fromEntries(latest.map((s) => [s.mode, finalLevel(byId.get(s.id)!) ?? 1])))
      setHistoryError(null)
    } catch (e) {
      setHistoryError(`Could not load history: ${errText(e)}`)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const level = levels.dual ?? 1
  const n = startN ?? (mode === "transfer" ? level : (levels[mode] ?? 1))

  const start = async () => {
    const ctx = audioContext() // the one AudioContext, created inside the Start click
    setStarting(true)
    setStartError(null)
    try {
      const clips = await loadClips()
      const transfer = mode === "transfer"
      setRun({
        ctx,
        clips,
        config: {
          mode,
          settings: transfer ? { ...TRANSFER_SETTINGS, keys: settings.keys } : settings,
          seed: newSeed(),
          startN: transfer ? transferLevel(level) : n,
        },
      })
      setRecord(null)
      setSave(null)
      setCheckedIn(false)
    } catch (e) {
      setStartError(`Could not start: ${errText(e)}`)
    } finally {
      setStarting(false)
    }
  }

  const persist = useCallback(
    async (r: NBackSession) => {
      if (!inTauri) {
        setSave({ state: "unavailable" })
        return
      }
      setSave({ state: "saving" })
      try {
        await nbackStore.save(r)
        setSave({ state: "saved" })
        void refresh()
      } catch (e) {
        setSave({ state: "error", detail: errText(e) })
      }
    },
    [refresh],
  )

  const onEnd = useCallback(
    (r: NBackSession) => {
      setRun(null)
      setRecord(r)
      void persist(r) // saved now, and again with the check-in (idempotent on id)
    },
    [persist],
  )

  const submitCheckin = async (checkin: Checkin) => {
    if (!record) return
    const r = { ...record, checkin }
    setRecord(r)
    await persist(r)
    setCheckedIn(true)
  }

  const exportCsv = async () => {
    try {
      setExported((await nbackStore.exportCsv()).path)
    } catch (e) {
      setHistoryError(`Export failed: ${errText(e)}`)
    }
  }

  if (run) {
    return (
      <div className="p-8">
        <RunView
          config={run.config}
          ctx={run.ctx}
          clips={run.clips}
          calibrated={calibration !== null}
          onEnd={onEnd}
        />
      </div>
    )
  }

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-5 p-8">
      {record && (
        <>
          <SessionSummary record={record} />
          <SaveStatus save={save} onRetry={() => void persist(record)} />
          {!checkedIn && <CheckinForm onSubmit={submitCheckin} saving={save?.state === "saving"} />}
          {checkedIn && (
            <div>
              <Button variant="outline" onClick={() => setRecord(null)}>
                Done
              </Button>
            </div>
          )}
        </>
      )}
      {!record && (
        <>
          <SetupPanel
            mode={mode}
            setMode={(m) => {
              setMode(m)
              setStartN(null)
            }}
            settings={settings}
            setSettings={setSettings}
            startN={n}
            setStartN={setStartN}
            level={level}
            lastTransferAt={sessions.find((s) => s.mode === "transfer")?.started_at ?? null}
            starting={starting}
            error={startError}
            onStart={() => void start()}
          />
          <CalibrationPanel calibration={calibration} onChange={setCalibration} />
          {inTauri ? (
            <HistoryPanel
              sessions={sessions}
              week={week}
              error={historyError}
              exported={exported}
              onExport={() => void exportCsv()}
            />
          ) : (
            <p className="border-hairline text-ink-2 rounded-xl border p-5 text-sm">
              History, saving and CSV export need the desktop app. Training works here, but sessions are not stored.
            </p>
          )}
        </>
      )}
    </div>
  )
}

function SaveStatus({ save, onRetry }: { save: Save | null; onRetry: () => void }) {
  if (!save) return null
  if (save.state === "unavailable")
    return <p className="text-warn text-sm">Not saved: saving needs the desktop app. This session is not stored.</p>
  if (save.state === "error")
    return (
      <p className="text-bad flex items-center gap-3 text-sm">
        Not saved: {save.detail}
        <Button size="sm" variant="outline" onClick={onRetry}>
          Retry
        </Button>
      </p>
    )
  return <p className="text-ink-3 text-sm">{save.state === "saving" ? "Saving…" : "Saved."}</p>
}
