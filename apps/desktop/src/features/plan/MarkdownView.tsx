// Renders parsed plan markdown as an editorial reading column: serif body, Geist headings with
// mono eyebrows, hairline tables. Each `##` section is one panel that rises in once on mount.
import { Fragment, type ReactNode } from "react"

import { Eyebrow, riseStyle } from "@/components/kit"
import { cn } from "@/lib/utils"

import type { Block, Heading, List } from "./markdown"

export const headingDomId = (h: Heading) => `plan-${h.id}`

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
        <code
          key={at}
          className="rounded-[5px] bg-[color-mix(in_oklab,var(--ink)_7%,transparent)] px-[0.3em] py-[0.1em] font-mono text-[0.8em] [overflow-wrap:anywhere]"
        >
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
        <span key={at} className="underline decoration-hairline-strong underline-offset-[3px]" title={m[5]}>
          {inline(m[4])}
        </span>,
      )
    }
    last = at + m[0].length
  }
  if (last < text.length) out.push(text.slice(last))
  return out
}

function renderList(list: List, depth: number): ReactNode {
  const Tag = list.ordered ? "ol" : "ul"
  return (
    <Tag
      className={cn(
        "marker:text-muted-foreground flex flex-col gap-[0.45em] pl-[1.4em]",
        list.ordered ? "list-decimal marker:font-mono marker:text-[0.78em]" : depth % 2 === 0 ? "list-disc" : "list-[circle]",
        depth > 0 && "mt-[0.45em]",
      )}
    >
      {list.items.map((item, i) => (
        <li key={i} className="pl-[0.2em]">
          {inline(item.text)}
          {item.sub.map((sub, j) => (
            <Fragment key={j}>{renderList(sub, depth + 1)}</Fragment>
          ))}
        </li>
      ))}
    </Tag>
  )
}

/** Headings shift down one level: the page already has an h1 (the section title in the shell). */
function headingTag(level: number) {
  return `h${Math.min(level + 1, 6)}` as "h2"
}

function renderBlock(block: Block, key: number, heading?: Heading): ReactNode {
  switch (block.kind) {
    case "heading": {
      const Tag = headingTag(block.level)
      const common = { id: heading ? headingDomId(heading) : undefined, tabIndex: -1 }
      if (block.level === 1) {
        return (
          <Tag
            key={key}
            {...common}
            className="text-foreground scroll-mt-10 font-sans text-[clamp(1.75rem,3.2vw,2.3rem)] leading-[1.12] font-light tracking-[-0.035em] text-balance outline-none"
          >
            {inline(block.text)}
          </Tag>
        )
      }
      return (
        <Tag
          key={key}
          {...common}
          className={cn(
            "text-foreground font-sans font-medium text-balance outline-none",
            block.level === 2 && "scroll-mt-24 text-[1.5rem] leading-[1.2] tracking-[-0.022em]",
            block.level === 3 && "mt-4 scroll-mt-10 text-[1.05rem] tracking-[-0.01em]",
            block.level > 3 && "scroll-mt-10 text-[0.95rem]",
          )}
        >
          {inline(block.text)}
        </Tag>
      )
    }
    case "paragraph":
      return <p key={key}>{inline(block.text)}</p>
    case "code":
      return (
        <pre
          key={key}
          className="border-hairline overflow-x-auto rounded-lg border bg-[color-mix(in_oklab,var(--ink)_5%,transparent)] p-4 font-mono text-[13px] leading-relaxed"
        >
          <code>{block.text}</code>
        </pre>
      )
    case "list":
      return <div key={key}>{renderList(block.list, 0)}</div>
    case "table":
      return (
        <div key={key} className="overflow-x-auto font-sans text-[14px] leading-snug tabular-nums">
          <table className="w-full border-collapse">
            <thead>
              <tr>
                {block.head.map((cell, i) => (
                  <th
                    key={i}
                    scope="col"
                    className="border-hairline-strong text-muted-foreground border-b py-2.5 pr-5 font-mono text-[11px] font-normal tracking-[0.14em] uppercase last:pr-0"
                    style={{ textAlign: block.align[i] ?? "left" }}
                  >
                    {inline(cell)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row, r) => (
                <tr key={r} className="border-hairline border-b last:border-b-0">
                  {row.map((cell, i) => (
                    <td
                      key={i}
                      className="py-3 pr-5 align-top last:pr-0 first:whitespace-nowrap"
                      style={{ textAlign: block.align[i] }}
                    >
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
        <blockquote key={key} className="border-hairline-strong text-ink-2 border-l pl-5 italic">
          {inline(block.text)}
        </blockquote>
      )
    case "rule":
      return <hr key={key} className="border-hairline" />
  }
}

type Group = { heading: Heading | null; items: { block: Block; heading?: Heading }[] }

/** Splits the document into its title block and one group per `##` section. */
function groupSections(blocks: Block[], headings: Heading[]): Group[] {
  const groups: Group[] = [{ heading: null, items: [] }]
  let next = 0
  for (const block of blocks) {
    const heading = block.kind === "heading" ? headings[next++] : undefined
    if (block.kind === "heading" && block.level === 2) groups.push({ heading: heading!, items: [] })
    groups[groups.length - 1].items.push({ block, heading })
  }
  return groups[0].items.length ? groups : groups.slice(1)
}

/** The number shown in the outline and in each section's eyebrow: 01, 02, ... */
export function sectionNumbers(headings: Heading[]): Map<string, string> {
  const numbers = new Map<string, string>()
  for (const h of headings) if (h.level === 2) numbers.set(h.id, String(numbers.size + 1).padStart(2, "0"))
  return numbers
}

export function MarkdownView({ blocks, headings }: { blocks: Block[]; headings: Heading[] }) {
  const numbers = sectionNumbers(headings)
  return (
    <div className="flex flex-col">
      {groupSections(blocks, headings).map((group, g) => {
        const style = riseStyle(g + 1)
        const [head, ...rest] = group.items
        if (group.heading) {
          return (
            <section
              key={g}
              style={style}
              aria-labelledby={headingDomId(group.heading)}
              className="kit-rise border-hairline mt-14 flex flex-col gap-[1.15em] border-t pt-9"
            >
              <header className="mb-1 flex flex-col gap-3.5">
                <Eyebrow rule className="text-muted-foreground">{numbers.get(group.heading.id)}</Eyebrow>
                {renderBlock(head.block, 0, head.heading)}
              </header>
              {rest.map((item, i) => renderBlock(item.block, i + 1, item.heading))}
            </section>
          )
        }
        return (
          <div key={g} style={style} className="kit-rise flex flex-col gap-[1.15em]">
            {group.items.map((item, i) => renderBlock(item.block, i, item.heading))}
          </div>
        )
      })}
    </div>
  )
}
