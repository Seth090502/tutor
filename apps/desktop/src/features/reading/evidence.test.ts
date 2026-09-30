import { describe, expect, it } from "vitest"

import { blankRow, evidenceText, hasEvidence, type EvidenceRow } from "./evidence"

const row = (claim = "", evidence = "", where = ""): EvidenceRow => ({ claim, evidence, where })

describe("evidence table", () => {
  it("starts with empty rows and counts only rows with something in them", () => {
    expect(blankRow()).toEqual(row())
    expect(hasEvidence([row(), row("  ", "", "")])).toBe(false)
    expect(hasEvidence([row(), row("", "Table 2", "")])).toBe(true)
  })
  it("serialises filled rows, one line each, leaving out empty cells and empty rows", () => {
    expect(evidenceText([row("Spacing raised 12-week scores", "Cluster trial, 38 classes", "p. 6"), row(), row("No subject effect", "", "Fig 3")])).toBe(
      "Claim: Spacing raised 12-week scores | Evidence: Cluster trial, 38 classes | Where: p. 6\nClaim: No subject effect | Where: Fig 3",
    )
  })
  it("trims cells", () => {
    expect(evidenceText([row("  a  ", " b", "c ")])).toBe("Claim: a | Evidence: b | Where: c")
  })
})
