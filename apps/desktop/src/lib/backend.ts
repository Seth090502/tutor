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

export const shell = {
  servicesStatus: () => invoke<ServicesStatus>("services_status"),
  servicesRestart: (name: "backend" | "rag_engine") => invoke<void>("services_restart", { name }),
  secretsStatus: () => invoke<Record<SecretName, boolean>>("secrets_status"),
  secretSet: (name: SecretName, value: string) => invoke<void>("secret_set", { name, value }),
  secretDelete: (name: SecretName) => invoke<void>("secret_delete", { name }),
  elevenlabsConversationToken: (agentId: string) =>
    invoke<{ token: string }>("elevenlabs_conversation_token", { agentId }),
  configGet: () => invoke<{ elevenlabs_agent_id: string | null }>("config_get"),
  configSet: (key: string, value: string) => invoke<void>("config_set", { key, value }),
  evalsRuns: () => invoke<{ runs: EvalRun[]; warnings: string[] }>("evals_runs"),
  planMarkdown: () => invoke<{ markdown: string }>("plan_markdown"),
}
