// Block and session summaries: raw counts next to d' and c per stream, the level change and the rule that
// caused it, void blocks flagged. Counts are always shown; d' and c never stand alone.
import { ArrowDown, ArrowUp, Minus, TriangleAlert } from "lucide-react"

import { DeltaChip, Eyebrow, LedStatus, Panel, Stat, StatRow } from "@/components/kit"
import { cn } from "@/lib/utils"
import { errors } from "./engine/scoring"
import { streamsOf } from "./engine/session"
import { streamLabels } from "./parts"
import type { Block, Metrics, NBackSession } from "./types"
import { levelChange, pooled, type LevelChange } from "./view"

const fmt2 = (x: number) => x.toFixed(2)
const HONEST = "n-back scores are noisy and do not measure intelligence."

type Row = { label: string; m: Metrics | null }

/** Counts, d' and c per stream as one quiet table: a row per stream, hairline dividers, light numerals. */
export function StreamTable({ rows, caption, big = false }: { rows: Row[]; caption: string; big?: boolean }) {
  const shown = rows.filter((r): r is { label: string; m: Metrics } => r.m !== null)
  if (shown.length === 0) return null
  const th = "text-ink-3 px-1 pb-3 text-right align-bottom font-mono text-[11px] leading-tight font-normal tracking-[0.04em] uppercase"
  const num = cn("px-1 py-3 text-right font-light tracking-[-0.02em] tabular-nums", big ? "text-[28px]" : "text-[24px]")
  return (
    <table className="w-full table-fixed border-collapse">
      <caption className="sr-only">{caption}</caption>
      <thead>
        <tr>
          <th scope="col" className={cn(th, "w-[4.5rem] pl-0 text-left")}>
            Stream
          </th>
          <th scope="col" className={th}>Hits</th>
          <th scope="col" className={th}>Misses</th>
          <th scope="col" className={th}>False alarms</th>
          <th scope="col" className={th}>Correct rejections</th>
          <th scope="col" className={cn(th, "border-hairline-strong w-[5.25rem] border-l text-[13px] tracking-normal normal-case")}>d′</th>
          <th scope="col" className={cn(th, "w-[5.25rem] text-[13px] tracking-normal normal-case")}>c</th>
        </tr>
      </thead>
      <tbody>
        {shown.map(({ label, m }) => (
          <tr key={label} className="border-hairline border-t">
            <th scope="row" className="py-3 pr-1 text-left align-middle text-[15px] font-medium">
              {label}
            </th>
            <td className={num}>{m.hits}</td>
            <td className={num}>{m.misses}</td>
            <td className={num}>{m.false_alarms}</td>
            <td className={num}>{m.correct_rejections}</td>
            <td className={cn(num, "border-hairline-strong border-l")}>{fmt2(m.d_prime)}</td>
            <td className={num}>{fmt2(m.c)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

/** The level move as a small chip: an arrow and the numbers, never colour alone and never a verdict. */
export function LevelChip({ change }: { change: LevelChange }) {
  const Icon = change.dir === "up" ? ArrowUp : change.dir === "down" ? ArrowDown : Minus
  const word = change.dir === "up" ? "Up" : change.dir === "down" ? "Down" : change.dir === "fixed" ? "Fixed" : "Stays"
  return (
    <span className="border-hairline-strong inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 font-mono text-[11px] leading-none tracking-[0.04em]">
      <Icon aria-hidden className="size-3" strokeWidth={2.25} />
      {word} · n {change.from}
      {change.dir === "up" || change.dir === "down" ? ` to ${change.to}` : ""}
    </span>
  )
}

/** One sentence: what the next level is and the rule that decided it. */
function levelReason(block: Block, change: LevelChange, rule: { up_max_errors: number; down_min_errors: number }, errs: string) {
  if (change.dir === "void") return `Void (${block.void_reason}): logged but not scored, and the level stays at ${block.n}.`
  if (change.dir === "fixed") return `Fixed level ${block.n}: the transfer check does not adapt.`
  if (change.dir === "up")
    return `Next block: n = ${change.to}. Every stream had ${rule.up_max_errors} or fewer errors (${errs}).`
  if (change.dir === "down")
    return `Next block: n = ${change.to}. A stream had ${rule.down_min_errors} or more errors (${errs}).`
  return `Next block: n = ${change.to}. Errors per stream: ${errs}. Misses plus false alarms.`
}

export function BlockSummary({
  block,
  labels,
  transfer,
  restLeft,
  settings,
}: {
  block: Block
  labels: { position: string; letter: string }
  transfer: boolean
  restLeft: number
  settings: { up_max_errors: number; down_min_errors: number }
}) {
  const change = levelChange(block, transfer)
  const errs = [
    block.metrics.position && `${labels.position} ${errors(block.metrics.position)}`,
    block.metrics.letter && `${labels.letter} ${errors(block.metrics.letter)}`,
  ]
    .filter(Boolean)
    .join(", ")
  return (
    <Panel
      className="w-full max-w-2xl"
      eyebrow={`Block ${block.index + 1} at n = ${block.n}`}
      title={block.void ? "Void block" : "Block summary"}
      actions={<span className="text-ink-3 font-mono text-xs tabular-nums">Rest {restLeft} s</span>}
    >
      {block.void ? (
        <DeltaChip tone="warn" text={`Void · ${block.void_reason}`} />
      ) : (
        <StreamTable
          caption={`Block ${block.index + 1}: hits, misses, false alarms, correct rejections, d-prime and c per stream`}
          rows={[
            { label: labels.position, m: block.metrics.position },
            { label: labels.letter, m: block.metrics.letter },
          ]}
        />
      )}
      <div className="border-hairline mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 border-t pt-4" role="status">
        <LevelChip change={change} />
        <p className="text-ink-2 min-w-0 flex-1 text-[13px] leading-snug text-pretty">
          {levelReason(block, change, settings, errs)}
        </p>
      </div>
    </Panel>
  )
}

/** Level by block: a number, an arrow, and a void mark with its reason. */
function LevelPath({ blocks, transfer }: { blocks: Block[]; transfer: boolean }) {
  return (
    <ol className="m-0 flex list-none flex-wrap gap-1.5 p-0">
      {blocks.map((b) => {
        const c = levelChange(b, transfer)
        const Icon = c.dir === "void" ? TriangleAlert : c.dir === "up" ? ArrowUp : c.dir === "down" ? ArrowDown : Minus
        const name =
          c.dir === "void"
            ? `Block ${b.index + 1}: void, ${b.void_reason}`
            : `Block ${b.index + 1}: n ${b.n}${c.dir === "up" || c.dir === "down" ? `, then ${c.dir} to ${c.to}` : ", level stays"}`
        return (
          <li key={b.index} aria-label={name} title={name} className="bg-card border-hairline flex w-14 flex-col items-center gap-1 rounded-xl border px-1 py-2.5">
            <span className="text-ink-3 font-mono text-[11px]">{b.index + 1}</span>
            <span className={cn("font-mono text-[17px] tabular-nums", c.dir === "void" && "text-ink-3 line-through")}>{b.n}</span>
            <Icon aria-hidden className={cn("size-3.5", c.dir === "void" ? "text-warn" : "text-ink-3")} strokeWidth={2.25} />
            {c.dir === "void" && <span className="text-warn font-mono text-[11px] leading-none">void</span>}
          </li>
        )
      })}
    </ol>
  )
}

export function SessionSummary({ record }: { record: NBackSession }) {
  const transfer = record.mode === "transfer"
  const scored = record.blocks.filter((b) => !b.void)
  const voids = record.blocks.length - scored.length
  const labels = streamLabels(transfer)
  const streams = streamsOf(record.mode)
  const first = record.blocks[0]?.n
  const last = record.blocks[record.blocks.length - 1]?.next_n
  const minutes = Math.round((record.ended_at - record.started_at) / 60)
  return (
    <Panel
      eyebrow={transfer ? "Measurement" : "Session complete"}
      title={transfer ? "Transfer check" : `Level ${first} to ${last}`}
      className="train-rise"
    >
      <div className="flex flex-col gap-6">
        <StatRow>
          <Stat
            label="Blocks"
            value={record.blocks.length}
            size="md"
            delta={voids > 0 ? { text: `${voids} void`, tone: "warn" } : undefined}
          />
          <Stat label="Time" value={minutes} unit="min" size="md" />
          <Stat label={transfer ? "Fixed level" : "Level"} value={transfer ? `n = ${first}` : `${first} → ${last}`} size="md" />
        </StatRow>

        <StreamTable
          caption="Session totals: hits, misses, false alarms, correct rejections, d-prime and c per stream"
          big
          rows={[
            { label: labels.position, m: streams.position ? pooled(scored.flatMap((b) => b.metrics.position ?? [])) : null },
            { label: labels.letter, m: streams.letter ? pooled(scored.flatMap((b) => b.metrics.letter ?? [])) : null },
          ]}
        />
        <p className="text-ink-3 -mt-3 text-xs">Counts are summed over scored blocks; d′ and c are block means.</p>

        <div>
          <Eyebrow as="h3" rule className="mb-3">
            Level by block
          </Eyebrow>
          <LevelPath blocks={record.blocks} transfer={transfer} />
          <p className="text-ink-3 mt-2 text-xs leading-relaxed">
            {transfer
              ? "The transfer check does not adapt: n stays fixed."
              : `Rule: up when each stream has ${record.settings.up_max_errors} or fewer errors, down when either has ${record.settings.down_min_errors} or more.`}{" "}
            Void blocks are not scored and keep their level.
          </p>
        </div>

        <div className="border-hairline flex flex-wrap items-center gap-x-6 gap-y-2 border-t pt-4">
          <LedStatus
            tone={record.comparable ? "ok" : "warn"}
            state={record.comparable ? "Comparable" : "Not comparable"}
          />
          <LedStatus
            tone={record.timing.calibrated ? "ok" : "warn"}
            state={record.timing.calibrated ? "Timing checked" : "Timing unmeasured"}
          />
          <p className="text-ink-2 font-serif text-[15px] italic">{HONEST}</p>
        </div>
      </div>
    </Panel>
  )
}
