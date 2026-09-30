// Shared by the Skills and Plan sections (contract C13): how a shell command's result is held, and the
// one-line states every section shows instead of data.
import { RefreshCw } from "lucide-react"

import { Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { Notice, Quiet } from "./parts"

export type LoadState<T> =
  | { kind: "loading" }
  | { kind: "app-required" }
  | { kind: "error"; message: string }
  | { kind: "ready"; data: T }

/** The command's own error sentence. Tauri rejects with a bare string; a thrown Error has a message. */
export function commandError(e: unknown): string {
  const text = typeof e === "string" ? e : e instanceof Error ? e.message : String(e)
  return text.trim() || "The command failed without saying why."
}

/** The browser's IANA timezone, or UTC where it cannot be read. */
export function browserTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC"
  } catch {
    return "UTC"
  }
}

/** A section panel that shows a state in place of data: loading, desktop app needed, or the command's error. */
export function StatePanel({
  eyebrow,
  title,
  state,
  needsApp,
  onRetry,
}: {
  eyebrow: string
  title: string
  state: Exclude<LoadState<unknown>, { kind: "ready" }>
  /** One sentence for the browser case: what the section needs the desktop app for. */
  needsApp: string
  onRetry?: () => void
}) {
  return (
    <Panel eyebrow={eyebrow} title={title}>
      {state.kind === "loading" ? (
        <Quiet>Loading…</Quiet>
      ) : state.kind === "app-required" ? (
        <Quiet>{needsApp}</Quiet>
      ) : (
        <div className="flex flex-col items-start gap-3">
          <Notice>
            <p className="font-medium">This could not be loaded.</p>
            <p className="text-ink-2 mt-1">{state.message}</p>
          </Notice>
          {onRetry ? (
            <Button variant="outline" size="sm" onClick={onRetry}>
              <RefreshCw />
              Try again
            </Button>
          ) : null}
        </div>
      )}
    </Panel>
  )
}
