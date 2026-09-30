import { describe, expect, it } from "vitest"

import { PAPER_CHUNKS, SECTION, SECTION_CHUNKS } from "./fixtures"
import { findAbstractEntry, loadChunks, ordinalHint, paragraphs, readerBlocks, readingProgress, sectionTitle } from "./reader"
import type { Chunk, ChunkPage } from "./types"

describe("paragraphs", () => {
  it("joins hard-wrapped lines and splits on blank lines", () => {
    expect(paragraphs("one two\nthree four\n\nfive\nsix")).toEqual(["one two three four", "five six"])
  })
  it("removes a hyphen that only marks a line break", () => {
    expect(paragraphs("a con-\nvergent sequence")).toEqual(["a convergent sequence"])
  })
  it("keeps a hyphen before a capital, a digit or a space", () => {
    expect(paragraphs("Hahn-\nBanach and 3-\n4 and x -\ny")).toEqual(["Hahn- Banach and 3- 4 and x - y"])
  })
  it("drops soft hyphens and empty paragraphs", () => {
    expect(paragraphs("\n\n  \n\nab­cd\n\n\n")).toEqual(["abcd"])
  })
})

describe("readerBlocks", () => {
  it("puts a page marker before the first paragraph and again whenever the page changes", () => {
    const blocks = readerBlocks(SECTION_CHUNKS)
    const labels = blocks.flatMap((b) => (b.kind === "page" ? [b.label] : []))
    expect(labels).toEqual(["p. 32", "p. 33", "pp. 34–35", "p. 36", "pp. 37–38", "p. 39"])
    expect(blocks[0].kind).toBe("page")
    expect(blocks[1].kind).toBe("para")
  })
  it("does not repeat the marker when the next chunk starts on the page the last one ended on", () => {
    const a: Chunk = { ...SECTION_CHUNKS[0], page_start: 5, page_end: 6, chunk_id: "x:1" }
    const b: Chunk = { ...SECTION_CHUNKS[1], page_start: 6, page_end: 6, chunk_id: "x:2" }
    expect(readerBlocks([a, b]).flatMap((x) => (x.kind === "page" ? [x.label] : []))).toEqual(["pp. 5–6"])
  })
  it("emits no marker for chunks without pages", () => {
    const c: Chunk = { ...SECTION_CHUNKS[0], page_start: null, page_end: null }
    expect(readerBlocks([c]).every((b) => b.kind === "para")).toBe(true)
  })
  it("carries the chunk id on every paragraph", () => {
    const ids = new Set(readerBlocks(SECTION_CHUNKS).flatMap((b) => (b.kind === "para" ? [b.chunkId] : [])))
    expect([...ids]).toEqual(SECTION.chunk_ids)
  })
})

describe("ordinalHint", () => {
  it("reads the trailing ordinal of a chunk id", () => {
    expect(ordinalHint("d-analysis:41")).toBe(41)
    expect(ordinalHint("a:b:7")).toBe(7)
  })
  it("returns null when the id has no ordinal suffix", () => {
    expect(ordinalHint("c9f2")).toBeNull()
    expect(ordinalHint("doc:")).toBeNull()
    expect(ordinalHint("doc:1x")).toBeNull()
  })
})

// A rag-engine stand-in over a list of chunks: `from` is an ordinal; a page holds up to `size` chunks.
function engine(all: Chunk[], size = 20) {
  const calls: number[] = []
  const chunks = async (_doc: string, from: number): Promise<ChunkPage> => {
    calls.push(from)
    const rest = all.filter((c) => c.ordinal >= from)
    return { chunks: rest.slice(0, size), next: rest.length > size ? rest[size].ordinal : null }
  }
  return { chunks, calls }
}

describe("loadChunks", () => {
  const want = new Set(SECTION.chunk_ids)
  const inSection = (c: Chunk) => want.has(c.chunk_id)

  it("starts at the ordinal in the chunk id and stops where the run ends", async () => {
    const after: Chunk = { ...SECTION_CHUNKS[0], chunk_id: "d-analysis:47", ordinal: 47 }
    const { chunks, calls } = engine([...SECTION_CHUNKS, after])
    const got = await loadChunks({ chunks }, "d-analysis", SECTION.chunk_ids[0], inSection, 50)
    expect(got.map((c) => c.chunk_id)).toEqual(SECTION.chunk_ids)
    expect(calls).toEqual([41])
  })
  it("follows `next` when the run is longer than one page", async () => {
    const { chunks, calls } = engine(SECTION_CHUNKS, 2)
    const got = await loadChunks({ chunks }, "d", SECTION.chunk_ids[0], inSection, 50)
    expect(got).toHaveLength(6)
    expect(calls).toEqual([41, 43, 45])
  })
  it("stops at the cap", async () => {
    const { chunks } = engine(SECTION_CHUNKS)
    expect(await loadChunks({ chunks }, "d", SECTION.chunk_ids[0], () => true, 2)).toHaveLength(2)
  })
  it("scans from the start when the id carries no ordinal", async () => {
    const renamed = SECTION_CHUNKS.map((c) => ({ ...c, chunk_id: `h${c.ordinal}` }))
    const ids = new Set(renamed.map((c) => c.chunk_id))
    const before: Chunk = { ...renamed[0], chunk_id: "h0", ordinal: 0 }
    const { chunks, calls } = engine([before, ...renamed], 3)
    const got = await loadChunks({ chunks }, "d", "h41", (c) => ids.has(c.chunk_id), 50)
    expect(got.map((c) => c.chunk_id)).toEqual(renamed.map((c) => c.chunk_id))
    expect(calls[0]).toBe(0)
  })
  it("scans from the start when the hint points at the wrong chunk", async () => {
    const shifted = SECTION_CHUNKS.map((c) => ({ ...c, chunk_id: `d:${c.ordinal + 100}` }))
    const ids = new Set(shifted.map((c) => c.chunk_id))
    const { chunks, calls } = engine(shifted)
    const got = await loadChunks({ chunks }, "d", shifted[0].chunk_id, (c) => ids.has(c.chunk_id), 50)
    expect(got).toHaveLength(6)
    expect(calls).toEqual([141, 0])
  })
  it("fails clearly when the chunk is not in the document", async () => {
    const { chunks } = engine(SECTION_CHUNKS)
    await expect(loadChunks({ chunks }, "d", "nope:9", () => true, 1)).rejects.toThrow(/nope:9/)
  })
})

describe("findAbstractEntry", () => {
  const entry = (heading: string, id = "p:0") => ({ heading_path: [heading], page_start: 1, page_end: 1, chunk_id: id })
  it("finds a heading named Abstract, ignoring case, numbering and punctuation", () => {
    expect(findAbstractEntry([entry("Title", "p:0"), entry("ABSTRACT", "p:1")])?.chunk_id).toBe("p:1")
    expect(findAbstractEntry([entry("0. Abstract.")])?.chunk_id).toBe("p:0")
  })
  it("returns null when there is none", () => {
    expect(findAbstractEntry([entry("1 Introduction"), entry("Abstract algebra")])).toBeNull()
  })
  it("matches the fixture paper's abstract", () => {
    expect(findAbstractEntry([{ heading_path: PAPER_CHUNKS[0].heading_path, page_start: 1, page_end: 1, chunk_id: "d-paper:0" }])).not.toBeNull()
  })
})

describe("readingProgress and sectionTitle", () => {
  it("is scrolled distance over scrollable distance, clamped", () => {
    expect(readingProgress(0, 2000, 800)).toBe(0)
    expect(readingProgress(600, 2000, 800)).toBe(0.5)
    expect(readingProgress(5000, 2000, 800)).toBe(1)
    expect(readingProgress(-20, 2000, 800)).toBe(0)
  })
  it("is 0 when nothing scrolls", () => {
    expect(readingProgress(0, 500, 800)).toBe(0)
  })
  it("titles a section by its last heading", () => {
    expect(sectionTitle(["2 Sequences", "2.1 Limits of sequences"])).toBe("2.1 Limits of sequences")
    expect(sectionTitle([])).toBe("Untitled section")
  })
})
