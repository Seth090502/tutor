// Shared client for the app's services and Tauri commands.
// Contracts: docs/tutor/CONTRACTS.md (C1 rag-engine, C3 Tauri commands).
import { invoke } from "@tauri-apps/api/core"

export const BACKEND_URL = "http://127.0.0.1:8001"
export const RAG_ENGINE_URL = "http://127.0.0.1:8811"

/** True inside the Tauri window; false in a plain browser (`npm run dev`). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

export class HttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message)
  }
}

export async function fetchJson<T>(base: string, path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(base + path, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  })
  if (!res.ok) {
    const body = await res.text().catch(() => "")
    throw new HttpError(res.status, `${init?.method ?? "GET"} ${path}: ${res.status} ${body}`.trim())
  }
  return (await res.json()) as T
}

export type ServiceState = {
  state: "starting" | "running" | "stopped" | "failed"
  url: string
  detail: string | null
}
export type ServicesStatus = { backend: ServiceState; rag_engine: ServiceState }
export type SecretName = "elevenlabs" | "opencode" | "anthropic" | "openrouter" | "voyage"

/** One eval run file (contract C2). */
export type SplitResult = {
  n: number
  metrics: Record<string, number>
  ci95?: Record<string, [number, number]>
}
export type EvalRun = {
  run_id: string
  created_at: number
  eval: string
  dataset: string
  change: string
  config: Record<string, unknown>
  splits: { train: SplitResult; test: SplitResult }
  per_domain: Record<string, { train: SplitResult; test: SplitResult }>
  decision: "baseline" | "keep" | "revert" | "pending"
  cost_usd: number
}

/** Speech through the shell (contract C10). */
export type TtsOptions = { voiceId?: string; model?: string; speed?: number }
export type TtsVoice = { voice_id: string; name: string; labels: Record<string, string>; preview_url: string | null }
export type SttWord = { text: string; start: number; end: number }
export type SttResult = { text: string; words: SttWord[] }

/** Learner projections (contract C13). Every number is a provisional local policy, not a validated measure. */
export type SkillStatus = "insufficient" | "developing" | "provisionally_mastered" | "needs_review"
export type SkillState = {
  skill_id: string
  policy_version: string
  alpha: number
  beta: number
  effective_weight: number
  eligible_observations: number
  dependent_observations: number
  distinct_families: number
  families_available: number
  distinct_dates: number
  last_exposure_at: number | null
  delayed_evidence: { attempt_id: string; at: number; gap_days: number; y: 0 | 1 }[]
  p_mastery: number
  status: SkillStatus
  limitations: string[]
}
export type LearningSkills = {
  store_id: string
  policy_version: string
  last_event_seq: number
  skills: SkillState[]
  excluded: Record<string, number>
  skipped_unknown: number
  limitations: string[]
}
export type PlanRequest = {
  now: number
  timezone: string
  available_minutes: number
  active_tracks: string[]
  review_budget_minutes?: number
  new_item_budget: number
  study_soft_stop_minutes?: number
  procedure_share_min?: number
  fatigue?: "none" | "tired" | "rested"
  excluded_experiment_units?: number[]
}
export type PlanItem = {
  entry_id: number
  track: string
  reason: string
  estimated_seconds: number
  priority_components: { retrievability: number | null; overdue_days: number; track_share: number }
  due_at: number | null
}
export type StudyPlan = {
  items: PlanItem[]
  summary: {
    minutes_by_track: Record<string, number>
    deferred_by_track: Record<string, number>
    total_minutes: number
    budget_minutes: number
    notes: string[]
  }
}

export const shell = {
  servicesStatus: () => invoke<ServicesStatus>("services_status"),
  servicesRestart: (name: "backend" | "rag_engine") => invoke<void>("services_restart", { name }),
  secretsStatus: () => invoke<Record<SecretName, boolean>>("secrets_status"),
  secretSet: (name: SecretName, value: string) => invoke<void>("secret_set", { name, value }),
  secretDelete: (name: SecretName) => invoke<void>("secret_delete", { name }),
  elevenlabsConversationToken: (agentId: string) =>
    invoke<{ token: string }>("elevenlabs_conversation_token", { agentId }),
  /** Also `agent_<lang>` and `voice_<lang>` for the target languages (C10), when set. */
  configGet: () =>
    invoke<{ elevenlabs_agent_id: string | null } & Partial<Record<string, string | null>>>("config_get"),
  configSet: (key: string, value: string) => invoke<void>("config_set", { key, value }),
  evalsRuns: () => invoke<{ runs: EvalRun[]; warnings: string[] }>("evals_runs"),
  planMarkdown: () => invoke<{ markdown: string }>("plan_markdown"),
  /** MP3 bytes for `text`; `lang` is ISO 639-1. Cached on disk by the shell. */
  ttsAudio: (text: string, lang: string, { voiceId, model, speed }: TtsOptions = {}) =>
    invoke<ArrayBuffer>("tts_audio", { text, lang, voiceId, model, speed }),
  /** The voices for a language (ISO 639-1). */
  ttsVoices: (lang: string) => invoke<{ voices: TtsVoice[] }>("tts_voices", { lang }),
  /** Scribe transcript of WebM/Opus or WAV bytes; `lang` is the Scribe (ISO 639-3) code. */
  sttTranscribe: (audio: Uint8Array, lang: string) =>
    invoke<SttResult>("stt_transcribe", { audio: Array.from(audio), lang }),
  /** Skill estimates, after catching up from the study feed (C13). */
  learningSkills: (timezone: string) => invoke<LearningSkills>("learning_skills", { timezone }),
  /** One global study plan over the due items (C13). */
  learningPlan: (request: PlanRequest) => invoke<StudyPlan>("learning_plan", { request }),
}
