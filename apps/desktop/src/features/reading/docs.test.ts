import { describe, expect, it } from "vitest"

import { BOOK, DOCS, OUTLINE, PAPER } from "./fixtures"
import {
  DEFAULT_FILTERS,
  chipOptions,
  documentsQuery,
  hasFilters,
  isPaper,
  nextEntry,
  outlineDepth,
  relativeAdded,
  shelfLabel,
  sizeLabel,
} from "./docs"

describe("documentsQuery", () => {
  it("lists the C9 parameters in a fixed order and leaves empty ones out", () => {
    expect(documentsQuery(DEFAULT_FILTERS, 0)).toBe("sort=title&limit=50&offset=0")
    expect(documentsQuery({ q: "  limits ", domain: "math", sourceType: "book", sort: "pages" }, 50)).toBe(
      "domain=math&source_type=book&q=limits&sort=pages&limit=50&offset=50",
    )
  })
  it("encodes the search text", () => {
    expect(documentsQuery({ ...DEFAULT_FILTERS, q: "a&b c" }, 0)).toBe("q=a%26b+c&sort=title&limit=50&offset=0")
  })
  it("never sends a parameter the engine does not know (it rejects unknown fields)", () => {
    const keys = [...new URLSearchParams(documentsQuery({ q: "x", domain: "math", sourceType: "paper", sort: "added" }, 100)).keys()]
    expect(keys.sort()).toEqual(["domain", "limit", "offset", "q", "sort", "source_type"])
  })
})

describe("hasFilters", () => {
  it("is true for any search text or chip, false for sort alone", () => {
    expect(hasFilters(DEFAULT_FILTERS)).toBe(false)
    expect(hasFilters({ ...DEFAULT_FILTERS, sort: "pages" })).toBe(false)
    expect(hasFilters({ ...DEFAULT_FILTERS, q: " " })).toBe(false)
    expect(hasFilters({ ...DEFAULT_FILTERS, q: "x" })).toBe(true)
    expect(hasFilters({ ...DEFAULT_FILTERS, domain: "math" })).toBe(true)
    expect(hasFilters({ ...DEFAULT_FILTERS, sourceType: "note" })).toBe(true)
  })
})

describe("chipOptions", () => {
  it("lists the known values first, with counts, then any other shelf by count", () => {
    const rows = chipOptions(["math", "english"], { english: 4, math: 9, lab: 2, zoo: 7 })
    expect(rows).toEqual([
      { id: "math", count: 9 },
      { id: "english", count: 4 },
      { id: "zoo", count: 7 },
      { id: "lab", count: 2 },
    ])
  })
  it("keeps known values visible with no counts", () => {
    expect(chipOptions(["math"], undefined)).toEqual([{ id: "math", count: null }])
  })
})

describe("labels", () => {
  it("knows papers, and only papers", () => {
    expect(isPaper(PAPER)).toBe(true)
    expect(isPaper(BOOK)).toBe(false)
    expect(isPaper({ ...BOOK, source_type: "note" })).toBe(false)
  })
  it("names shelves and sizes", () => {
    expect(shelfLabel("unsorted")).toBe("Unsorted")
    expect(shelfLabel("math")).toBe("Math")
    expect(sizeLabel(DOCS[0])).toBe("412 pages · 1,180 chunks")
    expect(sizeLabel({ ...BOOK, pages: 1, chunks: 1 })).toBe("1 page · 1 chunk")
  })
  it("words the age of a document", () => {
    const now = 1_790_000_000
    expect(relativeAdded(null, now)).toBe("")
    expect(relativeAdded(now - 30, now)).toBe("today")
    expect(relativeAdded(now - 86_400 * 1.2, now)).toBe("yesterday")
    expect(relativeAdded(now - 86_400 * 9, now)).toBe("9 days ago")
    expect(relativeAdded(now - 86_400 * 70, now)).toBe("2 months ago")
  })
})

describe("outline", () => {
  it("indents by heading depth", () => {
    expect(outlineDepth(OUTLINE[0])).toBe(0)
    expect(outlineDepth(OUTLINE[1])).toBe(1)
  })
  it("finds the next section in reading order", () => {
    expect(nextEntry({ outline: OUTLINE }, OUTLINE[4])).toBe(OUTLINE[5])
    expect(nextEntry({ outline: OUTLINE }, OUTLINE[0])).toBe(OUTLINE[1])
  })
  it("has none after the last, or for an entry that is not in the outline", () => {
    expect(nextEntry({ outline: OUTLINE }, OUTLINE[5])).toBeNull()
    expect(nextEntry({ outline: OUTLINE }, { heading_path: ["x"], page_start: 1, page_end: 1, chunk_id: "d:1" })).toBeNull()
  })
  it("tells two entries on one chunk apart by heading", () => {
    const a = { heading_path: ["A"], page_start: 1, page_end: 1, chunk_id: "d:1" }
    const b = { heading_path: ["A", "A.1"], page_start: 1, page_end: 1, chunk_id: "d:1" }
    expect(nextEntry({ outline: [a, b] }, a)).toBe(b)
  })
})
