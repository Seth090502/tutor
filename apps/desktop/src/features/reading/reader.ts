// Pure helpers for the reader: paragraph text, page markers, chunk loading and the abstract lookup.
import type { Chunk, ChunkPage, OutlineEntry } from "./types"

/** Text of a chunk as paragraphs. PDF text is hard-wrapped, so single newlines become spaces. */
export function paragraphs(text: string): string[] {
  return text
    .replace(/­/g, "")
    .replace(/([a-z])-\n([a-z])/g, "$1$2")
    .split(/\n\s*\n/)
    .map((p) => p.replace(/\s*\n\s*/g, " ").replace(/[ \t]{2,}/g, " ").trim())
    .filter(Boolean)
}

export type ReaderBlock = { kind: "page"; label: string; page: number } | { kind: "para"; chunkId: string; text: string }

export function pageLabel(start: number | null, end: number | null): string {
  if (start == null) return ""
  return end != null && end > start ? `pp. ${start}–${end}` : `p. ${start}`
}

/** Paragraphs in reading order, each chunk's first page marked when it differs from where the last chunk ended. */
export function readerBlocks(chunks: Chunk[]): ReaderBlock[] {
  const out: ReaderBlock[] = []
  let last: number | null = null
  for (const c of chunks) {
    if (c.page_start != null && c.page_start !== last) out.push({ kind: "page", label: pageLabel(c.page_start, c.page_end), page: c.page_start })
    if (c.page_start != null) last = c.page_end ?? c.page_start
    for (const text of paragraphs(c.text)) out.push({ kind: "para", chunkId: c.chunk_id, text })
  }
  return out
}

/** rag-engine builds chunk ids as `<doc_id>:<ordinal>`; C9 does not promise it, so this is only a hint. */
export function ordinalHint(chunkId: string): number | null {
  const m = /:(\d+)$/.exec(chunkId)
  return m ? Number(m[1]) : null
}

const PAGE_SIZE = 20
const MAX_SCAN_PAGES = 500

/**
 * The run of chunks that starts at `firstId` and satisfies `keep`, read from the chunk route (C9).
 * The route pages by ordinal, so the start is looked up from the id's ordinal hint and confirmed; when
 * the hint is missing or wrong the document is scanned from the beginning. The run ends at the first
 * chunk that fails `keep`, at `cap` chunks, or at the end of the document.
 */
export async function loadChunks(
  api: { chunks: (docId: string, from: number, limit: number) => Promise<ChunkPage> },
  docId: string,
  firstId: string,
  keep: (chunk: Chunk, run: Chunk[]) => boolean,
  cap: number,
): Promise<Chunk[]> {
  const notFound = () => new Error(`Chunk ${firstId} was not found in document ${docId}.`)
  const hint = ordinalHint(firstId)
  let page: ChunkPage | null = null
  let index = -1
  if (hint != null) {
    page = await api.chunks(docId, hint, PAGE_SIZE)
    if (page.chunks[0]?.chunk_id === firstId) index = 0
  }
  for (let n = 0, from = 0; index < 0; n++) {
    if (n >= MAX_SCAN_PAGES) throw notFound()
    page = await api.chunks(docId, from, PAGE_SIZE)
    index = page.chunks.findIndex((c) => c.chunk_id === firstId)
    if (index < 0) {
      if (page.next == null) throw notFound()
      from = page.next
    }
  }
  const run: Chunk[] = []
  for (;;) {
    for (const c of page!.chunks.slice(index)) {
      if (!keep(c, run)) return run
      run.push(c)
      if (run.length >= cap) return run
    }
    if (page!.next == null) return run
    page = await api.chunks(docId, page!.next, PAGE_SIZE)
    index = 0
  }
}

const ABSTRACT = /^\W*(?:\d+\.?\s*)?abstract\W*$/i

export function findAbstractEntry(outline: OutlineEntry[]): OutlineEntry | null {
  return outline.find((e) => ABSTRACT.test(e.heading_path[e.heading_path.length - 1] ?? "")) ?? null
}

export const sectionTitle = (path: string[]): string => path[path.length - 1] ?? "Untitled section"

/** Scrolled share of the scrollable distance, in [0, 1]. */
export function readingProgress(scrollTop: number, scrollHeight: number, clientHeight: number): number {
  const max = scrollHeight - clientHeight
  return max > 0 ? Math.min(1, Math.max(0, scrollTop / max)) : 0
}
