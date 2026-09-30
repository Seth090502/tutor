import { PendingSection } from "./PendingSection"

export function EvalsScreen() {
  return (
    <PendingSection
      tracks={[
        "Every round: the one change made, train and test scores with 95% CIs",
        "Keep/revert decision per round and the noise floor",
        "Per-domain retrieval breakdown (math, English, science, chemistry)",
        "Cost per eval run",
      ]}
      source="evals/runs/*.json (contract C2)"
    />
  )
}
