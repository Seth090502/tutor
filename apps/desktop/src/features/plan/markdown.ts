// Markdown for the Plan screen: a small block parser (no dependency) plus the pure helpers the
// reader needs (heading ids, outline, reading time). Rendering lives in MarkdownView.tsx.

export type Align = "left" | "right" | "center" | undefined
export type List = { ordered: boolean; items: { text: string; sub: List[] }[] }
export type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "paragraph"; text: string }
  | { kind: "code"; text: string }
  | { kind: "list"; list: List }
  | { kind: "table"; head: string[]; align: Align[]; rows: string[][] }
  | { kind: "quote"; text: string }
  | { kind: "rule" }

const FENCE = /^\s{0,3}(`{3,}|~{3,})/
const HEADING = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/
const RULE = /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/
const ITEM = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/
const QUOTE = /^\s{0,3}>\s?(.*)$/
const TABLE_SEPARATOR = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/

const startsTable = (lines: string[], i: number) =>
  lines[i].includes("|") && i + 1 < lines.length && lines[i + 1].includes("|") && TABLE_SEPARATOR.test(lines[i + 1])

function startsBlock(lines: string[], i: number) {
  const line = lines[i]
  return FENCE.test(line) || HEADING.test(line) || RULE.test(line) || QUOTE.test(line) || ITEM.test(line) || startsTable(lines, i)
}

function splitRow(line: string) {
  let s = line.trim()
  if (s.startsWith("|")) s = s.slice(1)
  if (s.endsWith("|") && !s.endsWith("\\|")) s = s.slice(0, -1)
  return s.split(/(?<!\\)\|/).map((cell) => cell.trim().replace(/\\\|/g, "|"))
}

const alignOf = (cell: string): Align =>
  cell.startsWith(":") && cell.endsWith(":") ? "center" : cell.endsWith(":") ? "right" : cell.startsWith(":") ? "left" : undefined

/** Bullet and numbered items, nested by indentation; indented lines continue the last item. */
function parseList(lines: string[], start: number): [List, number] {
  const first = lines[start].match(ITEM)!
  const root: List = { ordered: /\d/.test(first[2]), items: [] }
  const stack = [{ indent: first[1].length, list: root }]
  let i = start
  while (i < lines.length) {
    const line = lines[i]
    if (!line.trim()) {
      let next = i + 1
      while (next < lines.length && !lines[next].trim()) next++
      if (next < lines.length && ITEM.test(lines[next])) {
        i = next
        continue
      }
      break
    }
    const m = line.match(ITEM)
    if (!m) {
      if (!/^\s+\S/.test(line) || FENCE.test(line)) break
      const { items } = stack[stack.length - 1].list
      items[items.length - 1].text += ` ${line.trim()}`
      i++
      continue
    }
    const indent = m[1].length
    const ordered = /\d/.test(m[2])
    while (stack.length > 1 && indent < stack[stack.length - 1].indent) stack.pop()
    let top = stack[stack.length - 1]
    if (indent > top.indent && top.list.items.length > 0) {
      const nested: List = { ordered, items: [] }
      top.list.items[top.list.items.length - 1].sub.push(nested)
      top = { indent, list: nested }
      stack.push(top)
    } else if (ordered !== top.list.ordered) {
      // Switching between bullets and numbers at the same depth starts a new list.
      if (stack.length === 1) break
      stack.pop()
      const parent = stack[stack.length - 1].list
      const sibling: List = { ordered, items: [] }
      parent.items[parent.items.length - 1].sub.push(sibling)
      top = { indent: top.indent, list: sibling }
      stack.push(top)
    }
    top.list.items.push({ text: m[3], sub: [] })
    i++
  }
  return [root, i]
}

export function parseBlocks(source: string): Block[] {
  const lines = source.replace(/\r\n?/g, "\n").split("\n")
  const blocks: Block[] = []
  let i = 0
  while (i < lines.length) {
    const line = lines[i]
    if (!line.trim()) {
      i++
      continue
    }
    const fence = line.match(FENCE)
    if (fence) {
      const marker = fence[1]
      const body: string[] = []
      i++
      while (i < lines.length) {
        const t = lines[i].trim()
        if (t.length >= marker.length && t === marker[0].repeat(t.length)) break
        body.push(lines[i++])
      }
      i++ // closing fence (or end of input)
      blocks.push({ kind: "code", text: body.join("\n") })
      continue
    }
    const heading = line.match(HEADING)
    if (heading) {
      blocks.push({ kind: "heading", level: heading[1].length, text: heading[2] })
      i++
      continue
    }
    if (RULE.test(line)) {
      blocks.push({ kind: "rule" })
      i++
      continue
    }
    if (startsTable(lines, i)) {
      const head = splitRow(line)
      const align = splitRow(lines[i + 1]).map(alignOf)
      const rows: string[][] = []
      i += 2
      while (i < lines.length && lines[i].trim() && lines[i].includes("|")) rows.push(splitRow(lines[i++]))
      blocks.push({ kind: "table", head, align, rows })
      continue
    }
    if (QUOTE.test(line)) {
      const quoted: string[] = []
      while (i < lines.length && QUOTE.test(lines[i])) quoted.push(lines[i++].match(QUOTE)![1])
      blocks.push({ kind: "quote", text: quoted.join(" ").trim() })
      continue
    }
    if (ITEM.test(line)) {
      const [list, next] = parseList(lines, i)
      blocks.push({ kind: "list", list })
      i = next
      continue
    }
    const text: string[] = []
    while (i < lines.length && lines[i].trim() && (text.length === 0 || !startsBlock(lines, i))) text.push(lines[i++].trim())
    blocks.push({ kind: "paragraph", text: text.join(" ") })
  }
  return blocks
}

// ---------- outline and reading time ----------

/** Heading text without inline markup: the label shown in the outline. */
export function plainText(text: string): string {
  return text
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*(.+?)\*\*/g, "$1")
    .replace(/\*([^\s*](?:[^*]*[^\s*])?)\*/g, "$1")
    .replace(/\[([^\]]+)\]\([^)\s]+\)/g, "$1")
}

export function slugify(text: string): string {
  return (
    plainText(text)
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, "-")
      .replace(/^-+|-+$/g, "") || "section"
  )
}

export type Heading = { id: string; level: number; text: string }

/** Every heading with a unique id (`slug`, `slug-2`, ...), in document order. */
export function headingsOf(blocks: Block[]): Heading[] {
  const seen = new Map<string, number>()
  const out: Heading[] = []
  for (const b of blocks) {
    if (b.kind !== "heading") continue
    const base = slugify(b.text)
    const n = (seen.get(base) ?? 0) + 1
    seen.set(base, n)
    out.push({ id: n === 1 ? base : `${base}-${n}`, level: b.level, text: plainText(b.text) })
  }
  return out
}

function listText(list: List): string {
  return list.items.map((item) => [item.text, ...item.sub.map(listText)].join(" ")).join(" ")
}

/** Whole minutes at 230 words per minute, never less than 1. */
export function readingMinutes(blocks: Block[]): number {
  let words = 0
  for (const b of blocks) {
    const text =
      b.kind === "list"
        ? listText(b.list)
        : b.kind === "table"
          ? [...b.head, ...b.rows.flat()].join(" ")
          : b.kind === "rule"
            ? ""
            : b.text
    words += text.split(/\s+/).filter(Boolean).length
  }
  return Math.max(1, Math.round(words / 230))
}
