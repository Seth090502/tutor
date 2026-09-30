// The paper-mode evidence table. Its rows go to the recall route as text (C11 has no table field).
export type EvidenceRow = { claim: string; evidence: string; where: string }

export const blankRow = (): EvidenceRow => ({ claim: "", evidence: "", where: "" })

const cells = (r: EvidenceRow): [string, string][] =>
  ([["Claim", r.claim], ["Evidence", r.evidence], ["Where", r.where]] as [string, string][])
    .map(([k, v]): [string, string] => [k, v.trim()])
    .filter(([, v]) => v !== "")

export const hasEvidence = (rows: EvidenceRow[]): boolean => rows.some((r) => cells(r).length > 0)

export const evidenceText = (rows: EvidenceRow[]): string =>
  rows
    .map((r) => cells(r).map(([k, v]) => `${k}: ${v}`).join(" | "))
    .filter(Boolean)
    .join("\n")
