// Typed wrappers for the n-back storage commands (contract C6). Only callable inside Tauri.
import { invoke } from "@tauri-apps/api/core"

import type { NBackSession, NBackSummary } from "./types"

export const nbackStore = {
  save: (session: NBackSession) => invoke<{ id: string }>("nback_save_session", { session }),
  sessions: (limit?: number) => invoke<{ sessions: NBackSummary[] }>("nback_sessions", { limit }),
  session: (id: string) => invoke<NBackSession>("nback_session", { id }),
  exportCsv: () => invoke<{ path: string }>("nback_export_csv"),
}

const CALIBRATION_KEY = "tutor.nback.calibration"

export type Calibration = {
  at: number
  output_latency: number | null
  base_latency: number | null
  mean_offset_ms: number
  max_offset_ms: number
  in_sync: boolean
}

/** Last calibration on this device (a per-device convenience; sessions store their own flag). */
export function loadCalibration(): Calibration | null {
  try {
    const raw = localStorage.getItem(CALIBRATION_KEY)
    return raw ? (JSON.parse(raw) as Calibration) : null
  } catch {
    return null
  }
}

export function storeCalibration(c: Calibration) {
  try {
    localStorage.setItem(CALIBRATION_KEY, JSON.stringify(c))
  } catch {
    // storage unavailable: the calibration still applies for this run
  }
}
