import { PendingSection } from "./PendingSection"

export function ProgressScreen() {
  return (
    <PendingSection
      tracks={[
        "Reviews due today and upcoming (FSRS schedule)",
        "Accuracy by domain and topic, misconceptions you hit most",
        "Mastery paths and what to study next",
      ]}
      source="DeepTutor practice and mastery-path APIs"
    />
  )
}
