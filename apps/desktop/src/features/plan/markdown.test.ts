import { describe, expect, it } from "vitest"

import { headingsOf, parseBlocks, plainText, readingMinutes, slugify } from "./markdown"

describe("plan markdown", () => {
  it("parses headings, nested lists, tables and fences", () => {
    const blocks = parseBlocks(
      [
        "# Title",
        "",
        "Some **bold** text",
        "continues here.",
        "",
        "- top",
        "  - nested",
        "  - second `code`",
        "- next",
        "",
        "| Name | Count |",
        "|:--|--:|",
        "| a | 1 |",
        "",
        "```",
        "# not a heading",
        "```",
      ].join("\n"),
    )
    expect(blocks.map((b) => b.kind)).toEqual(["heading", "paragraph", "list", "table", "code"])
    expect(blocks[1]).toEqual({ kind: "paragraph", text: "Some **bold** text continues here." })
    const list = blocks[2]
    if (list.kind !== "list") throw new Error("expected a list")
    expect(list.list.items).toHaveLength(2)
    expect(list.list.items[0].sub[0].items.map((i) => i.text)).toEqual(["nested", "second `code`"])
    const table = blocks[3]
    if (table.kind !== "table") throw new Error("expected a table")
    expect(table.align).toEqual(["left", "right"])
    expect(blocks[4]).toEqual({ kind: "code", text: "# not a heading" })
  })

  it("strips inline markup for outline labels and slugs", () => {
    expect(plainText("Goals (**owner**, `2026-09-30`)")).toBe("Goals (owner, 2026-09-30)")
    expect(slugify("Goals (owner, 2026-09-30)")).toBe("goals-owner-2026-09-30")
    expect(slugify("???")).toBe("section")
  })

  it("gives repeated headings unique ids in document order", () => {
    const headings = headingsOf(parseBlocks("# A\n\n## Notes\n\ntext\n\n## Notes\n\n### Notes"))
    expect(headings.map((h) => h.id)).toEqual(["a", "notes", "notes-2", "notes-3"])
    expect(headings.map((h) => h.level)).toEqual([1, 2, 2, 3])
  })

  it("estimates reading time at 230 words a minute, at least one", () => {
    expect(readingMinutes(parseBlocks("one two three"))).toBe(1)
    const words = Array.from({ length: 690 }, () => "word").join(" ")
    expect(readingMinutes(parseBlocks(`${words}\n\n- ${words}`))).toBe(6)
  })
})
