// Plan: docs/tutor/PLAN.md rendered by a small in-file markdown renderer (no dependency).
import { Fragment, useCallback, useEffect, useState, type ReactNode } from "react"
import { Monitor, RefreshCw } from "lucide-react"

import { Button } from "@/components/ui/button"
import { inTauri, shell } from "@/lib/backend"
import { cn } from "@/lib/utils"
import { Notice, ScreenBoundary, StateMessage, errorText } from "./ui"

export function PlanScreen() {
  return (
    <ScreenBoundary>
      {inTauri ? <PlanLoader /> : <StateMessage icon={Monitor} title="Open in the desktop app" detail="The plan is read from disk by the desktop shell." />}
    </ScreenBoundary>
  )
}

function PlanLoader() {
  const [markdown, setMarkdown] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setMarkdown((await shell.planMarkdown()).markdown)
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-5 p-8">
      <div className="flex items-center justify-between gap-3">
        <span className="text-muted-foreground text-xs">docs/tutor/PLAN.md</span>
        <Button variant="outline" size="sm" onClick={() => void load()} disabled={loading}>
          <RefreshCw />
          Refresh
        </Button>
      </div>
      {error && <Notice>Could not read the plan: {error}</Notice>}
      {markdown !== null ? <Markdown source={markdown} /> : !error && <p className="text-muted-foreground text-sm">Loading…</p>}
    </div>
  )
}

// ---------- markdown: block parser ----------

type Align = "left" | "right" | "center" | undefined
type List = { ordered: boolean; items: { text: string; sub: List[] }[] }
type Block =
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

function parseBlocks(source: string): Block[] {
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

// ---------- markdown: inline and rendering ----------

// Code, bold, italic, link. Links render as text: navigating the app window away would lose the app.
const INLINE = /`([^`]+)`|\*\*(.+?)\*\*|\*([^\s*](?:[^*]*[^\s*])?)\*|\[([^\]]+)\]\(([^)\s]+)\)/g

function inline(text: string): ReactNode[] {
  const out: ReactNode[] = []
  let last = 0
  for (const m of text.matchAll(INLINE)) {
    const at = m.index
    if (at > last) out.push(text.slice(last, at))
    if (m[1] !== undefined) {
      out.push(
        <code key={at} className="bg-muted rounded px-1 py-0.5 font-mono text-[0.85em]">
          {m[1]}
        </code>,
      )
    } else if (m[2] !== undefined) {
      out.push(
        <strong key={at} className="font-semibold">
          {inline(m[2])}
        </strong>,
      )
    } else if (m[3] !== undefined) {
      out.push(<em key={at}>{inline(m[3])}</em>)
    } else {
      out.push(
        <span key={at} className="underline underline-offset-2" title={m[5]}>
          {inline(m[4])}
        </span>,
      )
    }
    last = at + m[0].length
  }
  if (last < text.length) out.push(text.slice(last))
  return out
}

const HEADING_CLASS = [
  "text-xl font-semibold tracking-tight",
  "mt-4 border-b pb-1 text-base font-semibold",
  "mt-2 text-sm font-semibold",
  "text-sm font-medium",
  "text-muted-foreground text-sm font-medium",
  "text-muted-foreground text-sm font-medium",
]

function renderList(list: List, depth: number): ReactNode {
  const Tag = list.ordered ? "ol" : "ul"
  return (
    <Tag className={cn("space-y-1 pl-5", list.ordered ? "list-decimal" : depth % 2 === 0 ? "list-disc" : "list-[circle]", depth > 0 && "mt-1")}>
      {list.items.map((item, i) => (
        <li key={i} className="pl-1">
          {inline(item.text)}
          {item.sub.map((sub, j) => (
            <Fragment key={j}>{renderList(sub, depth + 1)}</Fragment>
          ))}
        </li>
      ))}
    </Tag>
  )
}

function renderBlock(block: Block, key: number): ReactNode {
  switch (block.kind) {
    case "heading": {
      const Tag = `h${block.level}` as "h1"
      return (
        <Tag key={key} className={HEADING_CLASS[block.level - 1]}>
          {inline(block.text)}
        </Tag>
      )
    }
    case "paragraph":
      return (
        <p key={key} className="text-sm leading-relaxed">
          {inline(block.text)}
        </p>
      )
    case "code":
      return (
        <pre key={key} className="bg-muted overflow-x-auto rounded-lg p-3 font-mono text-xs leading-relaxed">
          <code>{block.text}</code>
        </pre>
      )
    case "list":
      return (
        <div key={key} className="text-sm leading-relaxed">
          {renderList(block.list, 0)}
        </div>
      )
    case "table":
      return (
        <div key={key} className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr>
                {block.head.map((cell, i) => (
                  <th key={i} className="border-b px-3 py-1.5 text-left font-medium" style={{ textAlign: block.align[i] }}>
                    {inline(cell)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row, r) => (
                <tr key={r}>
                  {row.map((cell, i) => (
                    <td key={i} className="border-b px-3 py-1.5 align-top" style={{ textAlign: block.align[i] }}>
                      {inline(cell)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )
    case "quote":
      return (
        <blockquote key={key} className="text-muted-foreground border-l-2 pl-4 text-sm leading-relaxed">
          {inline(block.text)}
        </blockquote>
      )
    case "rule":
      return <hr key={key} />
  }
}

function Markdown({ source }: { source: string }) {
  return <div className="flex flex-col gap-4">{parseBlocks(source).map(renderBlock)}</div>
}
