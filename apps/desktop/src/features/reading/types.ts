// Wire types for the Reading feature. Contracts: docs/tutor/CONTRACTS.md C9 (rag-engine library browsing
// and page crops) and C11 (DeepTutor /api/reading). Only fields the screens read are listed.

export type SortKey = "title" | "added" | "pages"

export type Doc = {
  doc_id: string
  path: string
  title: string
  domain: string
  source_type: string // "book" | "paper" | "note" | "unknown"
  pages: number
  chunks: number
  bytes: number
  added_at: number | null
  ocr_pending: number
}
export type DocList = { total: number; documents: Doc[] }

export type OutlineEntry = {
  heading_path: string[]
  page_start: number | null
  page_end: number | null
  chunk_id: string
}
export type DocDetail = Doc & { outline: OutlineEntry[] }

export type Chunk = {
  chunk_id: string
  ordinal: number
  text: string
  page_start: number | null
  page_end: number | null
  heading_path: string[]
}
export type ChunkPage = { chunks: Chunk[]; next: number | null }

/** The slice of GET /status the browser uses, for filter counts. */
export type LibraryCounts = {
  documents: { total: number; by_domain: Record<string, number>; by_source_type: Record<string, number> }
}

export type Confidence = 1 | 2 | 3 | 4

export type SectionRef = {
  heading_path: string[]
  chunk_ids: string[]
  page_start: number | null
  page_end: number | null
}
export type OpenedSession = { session_id: string; section: SectionRef }

export type Prequestion = { id: string | number; q: string }
export type Atom = { id: string | number; front: string; back: string; chunk_id: string }
export type Prepared = {
  prequestions: Prequestion[]
  idea_units: number
  atoms: Atom[]
  warrants: unknown[]
}

/** C11 returns `correct` only. `explanation` is read when a backend supplies it; the contract does not. */
export type AnswerResult = { correct: boolean | null; explanation?: string | null }
export type MissedUnit = { unit: string; chunk_id: string }
export type SummaryResult = { covered: number; missed: MissedUnit[] }
export type AcceptResult = { entry_ids: (number | string)[] }

export type Citation = { chunk_id: string; quote: string; page?: number }
export type VerifyReason = null | "no_chunk" | "not_substring" | "page_out_of_range"
export type VerifyResult = { ok: boolean; reason: VerifyReason }
