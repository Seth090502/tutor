// Pure helpers for the document browser (contract C9): the list query, filter chips and labels.
import type { Doc, DocDetail, OutlineEntry, SortKey } from "./types"

export type Filters = { q: string; domain: string | null; sourceType: string | null; sort: SortKey }
export const DEFAULT_FILTERS: Filters = { q: "", domain: null, sourceType: null, sort: "title" }

export const PAGE_LIMIT = 50

export const SORTS: { id: SortKey; label: string }[] = [
  { id: "title", label: "Title" },
  { id: "added", label: "Recently added" },
  { id: "pages", label: "Most pages" },
]

export const KNOWN_DOMAINS = ["math", "english", "science", "chemistry", "unsorted"]
export const KNOWN_SOURCE_TYPES = ["book", "paper", "note", "unknown"]

/** The query string of GET /documents. Only the C9 parameters; the engine rejects any other. */
export function documentsQuery(f: Filters, offset: number, limit = PAGE_LIMIT): string {
  const p = new URLSearchParams()
  if (f.domain) p.set("domain", f.domain)
  if (f.sourceType) p.set("source_type", f.sourceType)
  const q = f.q.trim()
  if (q) p.set("q", q)
  p.set("sort", f.sort)
  p.set("limit", String(limit))
  p.set("offset", String(offset))
  return p.toString()
}

/** Whether the list is narrowed by search text or a chip (sort alone does not narrow it). */
export const hasFilters = (f: Filters): boolean => f.q.trim() !== "" || f.domain !== null || f.sourceType !== null

/** Chip values: the known ones first, in order, then any other value by count. Counts are null when unknown. */
export function chipOptions(known: string[], counts: Record<string, number> | undefined): { id: string; count: number | null }[] {
  const extra = Object.entries(counts ?? {})
    .filter(([id]) => !known.includes(id))
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([id]) => id)
  return [...known, ...extra].map((id) => ({ id, count: counts ? (counts[id] ?? 0) : null }))
}

export const isPaper = (doc: Pick<Doc, "source_type">): boolean => doc.source_type === "paper"

export const shelfLabel = (id: string): string => id.charAt(0).toUpperCase() + id.slice(1)

const plural = (n: number, one: string) => `${n.toLocaleString("en-US")} ${one}${n === 1 ? "" : "s"}`
export const sizeLabel = (d: Pick<Doc, "pages" | "chunks">): string => `${plural(d.pages, "page")} · ${plural(d.chunks, "chunk")}`

export function relativeAdded(addedAt: number | null, nowSeconds = Date.now() / 1000): string {
  if (addedAt == null) return ""
  const days = Math.floor((nowSeconds - addedAt) / 86_400)
  if (days < 1) return "today"
  if (days === 1) return "yesterday"
  if (days < 30) return `${days} days ago`
  const months = Math.floor(days / 30)
  return months === 1 ? "1 month ago" : `${months} months ago`
}

export const outlineDepth = (e: OutlineEntry): number => Math.max(0, e.heading_path.length - 1)

/** The entry after `entry` in reading order, or null at the end (or when `entry` is not in the outline). */
export function nextEntry(doc: Pick<DocDetail, "outline">, entry: OutlineEntry): OutlineEntry | null {
  const key = (e: OutlineEntry) => JSON.stringify([e.chunk_id, e.heading_path])
  const i = doc.outline.findIndex((e) => key(e) === key(entry))
  return i >= 0 ? (doc.outline[i + 1] ?? null) : null
}
