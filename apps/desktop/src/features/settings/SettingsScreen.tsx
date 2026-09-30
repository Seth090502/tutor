import { PendingSection } from "@/features/tracker/PendingSection"

export function SettingsScreen() {
  return (
    <PendingSection
      tracks={[
        "API keys (ElevenLabs, OpenCode, Anthropic, OpenRouter, Voyage), stored in Windows Credential Manager",
        "Backend and rag-engine service status, with restart",
        "ElevenLabs agent id",
      ]}
      source="Tauri shell commands (contract C3)"
    />
  )
}
