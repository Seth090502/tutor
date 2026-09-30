import { PendingSection } from "./PendingSection"

export function LibraryScreen() {
  return (
    <PendingSection
      tracks={[
        "Files discovered, deduplicated, parsed, and indexed, per domain",
        "Pages triaged: born-digital, scanned, math-heavy (which need OCR)",
        "Measured tokens and pages per GB, projected cost for the full library",
        "API spend so far against the approved ingestion budget",
      ]}
      source="rag-engine service /status and /survey/latest (contract C1)"
    />
  )
}
