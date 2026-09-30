import { PendingSection } from "./PendingSection"

export function PlanScreen() {
  return (
    <PendingSection
      tracks={[
        "Phases with status and the check that proves each done",
        "Decisions waiting on you",
        "Total API spend by provider",
      ]}
      source="docs/tutor/PLAN.md"
    />
  )
}
