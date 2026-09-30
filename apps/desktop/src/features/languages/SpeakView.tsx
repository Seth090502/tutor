// The drawn parts of the Speak tab: the live call and the end-of-session digest. They take plain props so
// the preview page can show every state without a call. The transcript is framed honestly: the learner's
// lines are what the recogniser heard, not what they said (LANGUAGES.md item 8).
import { useEffect, useRef, useState } from "react"
import { Loader2, PhoneOff, Plus, Languages } from "lucide-react"

import { Eyebrow, Panel, Stat, StatRow, StateChip } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { TargetLanguage } from "@/lib/language"
import { HELP_NUDGE, MODE_COPY, learnerTurns, type Gloss, type Line, type SessionMode } from "./agent"
import { cardable, EMPTY_DIGEST, type DigestItem } from "./digest"
import { Notice } from "./parts"

export type LiveStatus = "connecting" | "listening" | "speaking"

const STATUS_WORD: Record<LiveStatus, string> = { connecting: "Connecting", listening: "Listening", speaking: "Partner speaking" }

export function TranscriptList({ lines, language, className }: { lines: Line[]; language: TargetLanguage; className?: string }) {
  const box = useRef<HTMLDivElement>(null)
  // Scroll the log itself, not the page: scrollIntoView would move every scrolling ancestor too.
  useEffect(() => {
    const el = box.current
    if (el) el.scrollTop = el.scrollHeight
  }, [lines.length])
  return (
    <div
      ref={box}
      role="log"
      aria-label="Conversation"
      className={cn("bg-card border-hairline flex min-h-56 flex-col gap-4 overflow-y-auto rounded-2xl border p-5", className)}
    >
      {lines.length === 0 ? (
        <p className="text-ink-2 m-auto max-w-xs text-center text-sm leading-relaxed">The conversation appears here. Your lines show what the recogniser heard.</p>
      ) : null}
      {lines.map((l, i) =>
        l.role === "learner" ? (
          <div key={i} className="flex max-w-[85%] flex-col items-end gap-1 self-end">
            <span className="text-ink-3 font-mono text-[11px] tracking-[0.14em] uppercase">The recogniser heard</span>
            <p lang={language} className="bg-secondary border-hairline rounded-2xl rounded-tr-md border px-4 py-2.5 text-[16px] leading-normal [overflow-wrap:anywhere]">
              {l.text}
            </p>
          </div>
        ) : (
          <div key={i} className="flex max-w-[85%] flex-col items-start gap-1 self-start">
            <span className="text-ink-3 flex items-center gap-3 font-mono text-[11px] tracking-[0.14em] uppercase">
              <span aria-hidden className="bg-hairline-strong h-px w-4" /> Partner
            </span>
            <p lang={language} className="font-serif text-[18px] leading-snug [overflow-wrap:anywhere]">{l.text}</p>
          </div>
        ),
      )}
    </div>
  )
}

export function LiveView({
  language,
  mode,
  band,
  scenario,
  status,
  lines,
  glosses,
  nudge,
  onHelp,
  onEnd,
}: {
  language: TargetLanguage
  mode: SessionMode
  band: string
  scenario: string
  status: LiveStatus
  lines: Line[]
  glosses: Gloss[]
  nudge: boolean
  onHelp: () => void
  onEnd: () => void
}) {
  const current = glosses[glosses.length - 1]
  return (
    <div className="@container flex flex-col gap-5">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <StateChip state={STATUS_WORD[status]} tone={status === "connecting" ? "warn" : "world"} pulse={status !== "connecting"} />
        <span className="text-ink-2 text-sm">
          {MODE_COPY[mode].title} mode · {band} · {scenario}
        </span>
        <Button variant="outline" size="sm" className="ml-auto" onClick={onEnd}>
          <PhoneOff aria-hidden /> End call
        </Button>
      </div>

      <div className="grid gap-5 @3xl:grid-cols-[minmax(0,1fr)_minmax(0,19rem)]">
        <TranscriptList lines={lines} language={language} className="h-[22rem]" />
        <div className="flex flex-col gap-5">
          <Panel eyebrow="On screen only" title="English help" className="p-5">
            {current ? (
              <div className="lg-enter">
                {current.forText ? <p lang={language} className="text-ink-2 mb-2 text-[15px]">{current.forText}</p> : null}
                <p className="text-[16px] leading-relaxed">
                  {current.kind === "rule" ? <span className="text-ink-3 mr-2 font-mono text-[11px] tracking-[0.14em] uppercase">Rule</span> : null}
                  {current.english}
                </p>
              </div>
            ) : (
              <p className="text-ink-2 text-sm leading-relaxed">Nothing yet. Try a reply first; English help opens after your first attempt.</p>
            )}
            <Button variant="outline" className="mt-4" onClick={onHelp} disabled={status === "connecting"}>
              <Languages aria-hidden /> Help in English
            </Button>
            {nudge ? <p role="status" className="text-ink-2 mt-3 text-sm">{HELP_NUDGE}</p> : null}
            <p className="text-ink-3 mt-3 text-[13px]">Each help request is counted as assistance in your session summary.</p>
          </Panel>
          <p className="text-ink-3 px-1 text-[13px] leading-relaxed">
            The partner reads text from a speech recogniser, not sound. It will not judge your accent, and it can miss or repair words.
            {learnerTurns(lines) === 0 ? "" : ` ${learnerTurns(lines)} ${learnerTurns(lines) === 1 ? "turn" : "turns"} so far.`}
          </p>
        </div>
      </div>
    </div>
  )
}

export type Digest = {
  items: DigestItem[]
  mode: SessionMode
  band: string
  scenario: string
  lines: Line[]
  helpRequests: number
  rules: number
  errorsLogged: number
}

export function DigestView({
  language,
  digest,
  onAddCards,
  onAgain,
}: {
  language: TargetLanguage
  digest: Digest
  onAddCards: (items: DigestItem[]) => Promise<number>
  onAgain: () => void
}) {
  const cards = digest.items.filter(cardable)
  const [state, setState] = useState<"idle" | "busy" | { created: number } | { error: string }>("idle")
  const add = async () => {
    setState("busy")
    try {
      setState({ created: await onAddCards(cards) })
    } catch (e) {
      setState({ error: e instanceof Error ? e.message : String(e) })
    }
  }
  return (
    <div className="@container flex flex-col gap-5">
      <Panel eyebrow="End of session" title={digest.items.length ? `${digest.items.length} to work on` : "Nothing flagged"} className="lg-enter p-6">
        <StatRow className="mb-5">
          <Stat size="md" label="Your turns" value={learnerTurns(digest.lines)} animate={false} />
          <Stat size="md" label="English help" value={digest.helpRequests} animate={false} />
          <Stat size="md" label="Errors logged" value={digest.errorsLogged} animate={false} />
        </StatRow>

        {digest.items.length === 0 ? (
          <p className="text-ink-2 text-[15px] leading-relaxed">{EMPTY_DIGEST}</p>
        ) : (
          <>
            <p className="text-ink-2 mb-4 text-[15px] leading-relaxed">
              {MODE_COPY[digest.mode].title === "Fluency" ? "You were not interrupted. " : ""}These are at most three errors, picked because they broke meaning or came up more than once. Turn them into production cards to practise them.
            </p>
            <ol className="flex flex-col gap-4">
              {digest.items.map((it, i) => (
                <li key={i} className="border-hairline bg-secondary/50 rounded-xl border p-4">
                  <p lang={language} className="font-serif text-[20px] leading-snug">{it.corrected}</p>
                  {it.english ? <p className="text-ink-2 mt-0.5 text-[15px]">{it.english}</p> : null}
                  {it.said.length ? (
                    <p className="text-ink-2 mt-2 text-sm">
                      <span className="text-ink-3 mr-2 font-mono text-[11px] tracking-[0.14em] uppercase">Heard</span>
                      <span lang={language}>{it.said.join(" / ")}</span>
                      {it.count > 1 ? <span className="text-ink-3"> · {it.count} times</span> : null}
                      {it.breaksComprehension ? <span className="text-ink-3"> · it made your meaning unclear</span> : null}
                    </p>
                  ) : null}
                  {it.rule ? <p className="mt-1.5 text-sm leading-relaxed">{it.rule}</p> : null}
                  {!cardable(it) ? <p className="text-ink-3 mt-1.5 text-[13px]">No meaning was logged, so this one cannot become a card.</p> : null}
                </li>
              ))}
            </ol>
            <div className="mt-5 flex flex-wrap items-center gap-3">
              <Button onClick={() => void add()} disabled={cards.length === 0 || state === "busy" || (typeof state === "object" && "created" in state)}>
                {state === "busy" ? <Loader2 aria-hidden className="animate-spin" /> : <Plus aria-hidden />}
                Add {cards.length} {cards.length === 1 ? "card" : "cards"}
              </Button>
              <span className="text-sm" aria-live="polite">
                {typeof state === "object" && "created" in state ? <span className="text-ink-2">Added. {state.created} {state.created === 1 ? "card" : "cards"} created.</span> : null}
                {typeof state === "object" && "error" in state ? <span className="text-bad">{state.error}</span> : null}
              </span>
            </div>
          </>
        )}
        {digest.rules > 0 ? <p className="text-ink-3 mt-4 text-[13px]">The partner showed {digest.rules} {digest.rules === 1 ? "rule" : "rules"} on screen. These were not counted as English help.</p> : null}
        <div className="mt-5">
          <Button variant="outline" onClick={onAgain}>New session</Button>
        </div>
      </Panel>

      <details className="border-hairline bg-card rounded-2xl border px-5 py-3">
        <summary className="text-ink-2 cursor-pointer text-sm font-medium">Transcript, as the recogniser heard it</summary>
        <div className="mt-3">
          <TranscriptList lines={digest.lines} language={language} className="border-0 p-0" />
        </div>
      </details>
      <Notice>The transcript is kept on this device. The recogniser can repair or miss words, so it is not a record of exactly what you said.</Notice>
    </div>
  )
}

export { Eyebrow }
