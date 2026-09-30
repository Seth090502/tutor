import { useState, type ReactNode } from "react"
import { createRoot } from "react-dom/client"

import "@/index.css"
import {
  BarList,
  CountUp,
  DecisionBadge,
  DeltaChip,
  Eyebrow,
  IdleDim,
  Kbd,
  KbdHint,
  Keycap,
  KeycapGroup,
  Led,
  LedStatus,
  LogLine,
  LogList,
  Panel,
  PasswordField,
  QuestionCard,
  QuietNote,
  Rise,
  Scrim,
  Segmented,
  ServiceRow,
  Stat,
  StatRow,
  StateChip,
  Stepper,
  SwitchRow,
  TextField,
  idleDimClass,
  type KeycapChoice,
} from "@/components/kit"

/**
 * Dev-only gallery of every kit component in every state. kit.html sets the theme and world
 * from the query string: ?theme=dark|light&world=math|english|science|chemistry|neutral
 * (and &quality=rich|lite).
 */

const WORLDS = ["neutral", "math", "english", "science", "chemistry", "languages"] as const

function applyQuery() {
  const q = new URLSearchParams(window.location.search)
  const root = document.documentElement
  const theme = q.get("theme")
  root.classList.toggle("dark", theme ? theme === "dark" : window.matchMedia("(prefers-color-scheme: dark)").matches)
  const world = q.get("world")
  root.dataset.world = WORLDS.find((w) => w === world) ?? "neutral"
  root.dataset.quality = q.get("quality") === "rich" ? "rich" : "lite"
}

function Section({ id, eyebrow, title, children }: { id: string; eyebrow: string; title: string; children: ReactNode }) {
  return (
    <Panel id={id} eyebrow={eyebrow} title={title}>
      {children}
    </Panel>
  )
}

function Sub({ children }: { children: ReactNode }) {
  return <p className="text-ink-3 mb-3 font-mono text-[11px] tracking-[0.16em] uppercase">{children}</p>
}

const CHOICES: KeycapChoice[] = [
  { id: "a", label: "The limit exists and equals f(a)" },
  { id: "b", label: "Both one-sided limits agree" },
  { id: "c", label: "f is defined at a" },
  { id: "d", label: "f is differentiable at a" },
]

function LiveKeys({ active = false }: { active?: boolean }) {
  const [value, setValue] = useState<string | null>(null)
  const [graded, setGraded] = useState(false)
  const choices = graded
    ? CHOICES.map((c) => ({
        ...c,
        status: c.id === "b" ? ("correct" as const) : c.id === value ? ("wrong" as const) : undefined,
      }))
    : CHOICES
  return (
    <div>
      <KeycapGroup
        choices={choices}
        value={value}
        onChange={setValue}
        onSubmit={() => setGraded(true)}
        active={active}
        aria-label="Which statement makes a two-sided limit exist?"
      />
      <div className="mt-4 flex items-center gap-4">
        <KbdHint>
          <Kbd>A</Kbd>–<Kbd>D</Kbd> or <Kbd>1</Kbd>–<Kbd>4</Kbd> to choose, <Kbd>Enter</Kbd> to submit
        </KbdHint>
        {graded ? (
          <button
            type="button"
            className="border-hairline-strong rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.14em] uppercase"
            onClick={() => {
              setGraded(false)
              setValue(null)
            }}
          >
            Reset
          </button>
        ) : null}
      </div>
    </div>
  )
}

const Q = (
  <p className="font-serif text-[21px] leading-[1.45] text-pretty">
    Which condition guarantees that <span className="italic">lim</span> <span className="italic">f(x)</span> exists as x approaches a?
  </p>
)

function CardDemo({ mastery, id, defaultFlipped, live }: { mastery: "new" | "learning" | "mastered"; id: string; defaultFlipped?: boolean; live?: boolean }) {
  const [value, setValue] = useState<string | null>(mastery === "new" ? null : "b")
  return (
    <div id={id}>
      <QuestionCard
        mastery={mastery}
        defaultFlipped={defaultFlipped}
        eyebrow="Limits · 3 of 10"
        explanation={
          <p>
            A two-sided limit exists exactly when the left-hand and right-hand limits are equal. The value of f at a itself
            plays no part in it.
          </p>
        }
        misconception={
          <p>
            Students often assume the limit must equal f(a). The limit describes nearby behaviour, so f(a) can differ, or not
            exist at all.
          </p>
        }
      >
        <div className="flex flex-col gap-5">
          {Q}
          <KeycapGroup
            choices={CHOICES}
            value={value}
            onChange={setValue}
            active={Boolean(live)}
            aria-label={`Answers, ${mastery} card`}
          />
        </div>
      </QuestionCard>
    </div>
  )
}

function CountOnceDemo() {
  const [n, setN] = useState(41)
  return (
    <div className="flex flex-col gap-3">
      <span className="text-[40px] leading-none font-extralight tracking-[-0.04em]">
        <CountUp value={n} once />
      </span>
      <button
        type="button"
        onClick={() => setN((v) => v + 7)}
        className="border-hairline-strong hover:bg-world-soft w-fit rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.14em] uppercase"
      >
        Add 7 (snaps)
      </button>
    </div>
  )
}

function CountUpDemo() {
  const [n, setN] = useState(128)
  return (
    <div className="flex flex-wrap items-center gap-6">
      <span id="countup" className="text-[56px] leading-none font-extralight tracking-[-0.04em]">
        <CountUp value={n} />
      </span>
      {[12, 128, 2048].map((v) => (
        <button
          key={v}
          type="button"
          onClick={() => setN(v)}
          className="border-hairline-strong hover:bg-world-soft rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.14em] uppercase"
        >
          To {v}
        </button>
      ))}
    </div>
  )
}

/** Replays the one-shot entrance by remounting it. */
function RiseDemo() {
  const [run, setRun] = useState(0)
  return (
    <div className="flex flex-col gap-4">
      <div key={run} id="rise-demo" className="grid gap-3 sm:grid-cols-4">
        {[0, 1, 2, 3].map((i) => (
          <Rise key={i} i={i} className="border-hairline bg-card rounded-xl border p-4">
            <Eyebrow>Step {i}</Eyebrow>
            <p className="text-ink-2 mt-2 text-sm">Waits {i * 70} ms, then rises 8px.</p>
          </Rise>
        ))}
      </div>
      <button
        type="button"
        onClick={() => setRun((n) => n + 1)}
        className="border-hairline-strong hover:bg-world-soft w-fit rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.14em] uppercase"
      >
        Replay
      </button>
    </div>
  )
}

function FormDemo() {
  const [seg, setSeg] = useState<string | null>("dual")
  const [optional, setOptional] = useState<number | null>(null)
  const [n, setN] = useState(3)
  const [on, setOn] = useState(false)
  const [off, setOff] = useState(true)
  return (
    <div className="grid gap-8 md:grid-cols-2">
      <div className="flex flex-col gap-6">
        <div id="form-seg" className="flex flex-col gap-3">
          <Sub>Segmented</Sub>
          <Segmented
            label="Mode"
            value={seg}
            onChange={setSeg}
            options={[
              { value: "dual", label: "Dual" },
              { value: "position", label: "Position only" },
              { value: "letter", label: "Letter only" },
            ]}
          />
          <Segmented
            label="Optional answer"
            value={optional}
            onChange={setOptional}
            onClear={() => setOptional(null)}
            options={[1, 2, 3, 4, 5].map((v) => ({ value: v, label: String(v) }))}
          />
        </div>
        <div id="form-stepper" className="flex flex-col gap-3">
          <Sub>Stepper</Sub>
          <Stepper label="starting n" value={n} min={1} max={9} onChange={setN} />
        </div>
      </div>
      <div className="flex flex-col gap-6">
        <div id="form-switch" className="divide-hairline flex flex-col divide-y">
          <SwitchRow title="Lures" detail="n±1 repeats, tagged in the log." checked={on} onChange={setOn} note={on ? <DeltaChip tone="warn" text="Not comparable" /> : undefined} />
          <SwitchRow title="Sound" detail="A quiet tick on each answer." checked={off} onChange={setOff} />
        </div>
        <div id="form-text" className="flex flex-col gap-3">
          <Sub>TextField and PasswordField</Sub>
          <TextField aria-label="Agent id" placeholder="agent_…" defaultValue="agent_01jx8k4v2e9f" />
          <PasswordField aria-label="API key" placeholder="Paste the key" />
        </div>
      </div>
    </div>
  )
}

function HotkeyDemo({ id, hotkeys, sendOnPress }: { id: string; hotkeys: "all" | "letters" | "none"; sendOnPress?: boolean }) {
  const [value, setValue] = useState<string | null>(null)
  const [sent, setSent] = useState<string | null>(null)
  return (
    <div id={id} className="flex flex-col gap-3">
      <KeycapGroup
        aria-label={`Hotkeys ${hotkeys}`}
        choices={CHOICES.slice(0, 3)}
        value={sendOnPress ? sent : value}
        onChange={sendOnPress ? undefined : setValue}
        onSubmit={sendOnPress ? setSent : undefined}
        sendOnPress={sendOnPress}
        hotkeys={hotkeys}
      />
      <p className="text-ink-3 font-mono text-[11px]" data-testid={`${id}-out`}>
        {sendOnPress ? `sent: ${sent ?? "nothing"}` : `chosen: ${value ?? "nothing"}`}
      </p>
    </div>
  )
}

function BusyBackdrop({ children }: { children: ReactNode }) {
  return (
    <div
      className="relative flex min-h-56 items-center justify-center overflow-hidden rounded-2xl p-8"
      style={{
        background:
          "radial-gradient(circle at 22% 30%, #e4573a 0, transparent 38%), radial-gradient(circle at 78% 26%, #56d4e0 0, transparent 40%), radial-gradient(circle at 55% 85%, #dcb86e 0, transparent 42%), conic-gradient(from 40deg at 50% 50%, #2a1f3d, #6a5acd, #f0c27b, #1f2a6b, #2a1f3d)",
      }}
    >
      {children}
    </div>
  )
}

export function KitGallery() {
  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6 px-4 py-10 sm:px-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div className="flex flex-col gap-3">
          <Eyebrow rule>Component kit</Eyebrow>
          <h1 className="text-[clamp(2.25rem,5vw,3.25rem)] leading-none font-extralight tracking-[-0.04em]">Every state, one page</h1>
          <KbdHint>Hover the cards, press the keys, then leave the page alone for a few seconds.</KbdHint>
        </div>
        <nav aria-label="Theme and world" className="text-ink-2 flex flex-wrap gap-x-4 gap-y-1 font-mono text-[11px] tracking-[0.12em] uppercase">
          <a className="underline-offset-4 hover:underline" href="?theme=light">Light</a>
          <a className="underline-offset-4 hover:underline" href="?theme=dark">Dark</a>
          {WORLDS.map((w) => (
            <a key={w} className="underline-offset-4 hover:underline" href={`?world=${w}${new URLSearchParams(location.search).get("theme") ? `&theme=${new URLSearchParams(location.search).get("theme")}` : ""}`}>
              {w}
            </a>
          ))}
        </nav>
      </header>

      <Section id="type" eyebrow="Type" title="Eyebrow, Panel, KbdHint">
        <div className="grid gap-8 md:grid-cols-2">
          <div className="flex flex-col gap-4">
            <Sub>Eyebrow</Sub>
            <Eyebrow>Plain eyebrow</Eyebrow>
            <Eyebrow rule>With leading rule</Eyebrow>
            <KbdHint>
              Press <Kbd>Space</Kbd> to flip, <Kbd>Esc</Kbd> to leave.
            </KbdHint>
          </div>
          <Panel eyebrow="Session" title="Panel with actions" actions={<LedStatus tone="ok" state="Live" />}>
            <p className="text-ink-2 font-serif text-[17px] leading-[1.6]">
              A card surface with a 1px hairline, a 16px radius and an inner top highlight.
            </p>
          </Panel>
        </div>
      </Section>

      <Section id="stats" eyebrow="Numbers" title="Stat, StatRow, DeltaChip, CountUp">
        <div className="flex flex-col gap-6">
          <StatRow id="statrow">
            <Stat label="Due today" value={12} unit="cards" delta={{ text: "+3 vs yesterday", tone: "warn" }} />
            <Stat label="Mastered" value={248} delta={{ text: "+18 this week", tone: "ok" }} />
            <Stat label="Retention" value={87.4} decimals={1} unit="%" delta={{ text: "-2.1 pts", tone: "bad" }} />
            <Stat label="Streak" value="9 days" animate={false} />
          </StatRow>
          <div className="flex flex-wrap items-center gap-3">
            <DeltaChip tone="ok" text="+12%" />
            <DeltaChip tone="warn" text="3 overdue" />
            <DeltaChip tone="bad" text="-4 pts" />
          </div>
          <CountUpDemo />
          <div className="grid gap-6 md:grid-cols-[2fr_1fr_1fr]">
            <div id="stat-sizes">
              <Sub>Numeral follows its cell: wide, narrow, narrower</Sub>
              <div className="grid grid-cols-[2fr_1.2fr_0.9fr] gap-px overflow-hidden rounded-2xl border border-hairline bg-hairline">
                <Stat label="Wide" value={58764} caption="of 61,432 · 95.7%" />
                <Stat label="Narrow" value={8731} />
                <Stat label="Tiny" value={184} />
              </div>
            </div>
            <div id="stat-md">
              <Sub>Size md</Sub>
              <StatRow className="grid-cols-1">
                <Stat size="md" label="Errors" value={3} delta={{ text: "needs a look", tone: "bad" }} />
              </StatRow>
            </div>
            <div id="countup-once">
              <Sub>CountUp once</Sub>
              <CountOnceDemo />
            </div>
          </div>
        </div>
      </Section>

      <Section id="chips" eyebrow="Over busy visuals" title="StateChip and Scrim">
        <div className="grid gap-5 md:grid-cols-2">
          <BusyBackdrop>
            <div id="chips-row" className="flex flex-wrap justify-center gap-3">
              <StateChip state="Listening" pulse />
              <StateChip state="Thinking" tone="warn" pulse />
              <StateChip state="Speaking" tone="ok" />
              <StateChip state="Idle" dot={false} />
              <StateChip state="Error" tone="bad" />
            </div>
          </BusyBackdrop>
          <BusyBackdrop>
            <Scrim id="scrim" className="max-w-xs text-center">
              <p className="font-serif text-[19px] leading-snug italic">A local scrim keeps this sentence readable over colour.</p>
            </Scrim>
          </BusyBackdrop>
        </div>
      </Section>

      <Section id="chips-light" eyebrow="On a plain surface" title="StateChip, light variant">
        <div id="chips-light-row" className="flex flex-wrap gap-3">
          <StateChip variant="light" state="Listening" pulse />
          <StateChip variant="light" state="Thinking" tone="warn" pulse />
          <StateChip variant="light" state="Speaking" tone="ok" />
          <StateChip variant="light" state="Idle" dot={false} />
          <StateChip variant="light" state="Error" tone="bad" />
        </div>
      </Section>

      <Section id="status" eyebrow="Settings" title="LedStatus and ServiceRow">
        <div className="grid gap-8 md:grid-cols-2">
          <div id="service-rows" className="divide-hairline flex flex-col divide-y">
            <ServiceRow label="Backend" detail="127.0.0.1:8001" tone="ok" state="Running" />
            <ServiceRow label="rag-engine" detail="127.0.0.1:8811" tone="warn" state="Starting" pulse />
            <ServiceRow label="Voice agent" tone="off" state="Stopped" />
            <ServiceRow label="Embeddings" detail="voyage" tone="bad" state="Failed" />
          </div>
          <div className="flex flex-col gap-4">
            <Sub>LED tones</Sub>
            <div className="flex flex-wrap gap-x-8 gap-y-3">
              <LedStatus tone="ok" state="Ok" />
              <LedStatus tone="warn" state="Warn" pulse />
              <LedStatus tone="bad" state="Bad" />
              <LedStatus tone="off" state="Off" />
            </div>
            <div className="flex gap-4">
              <Led tone="ok" />
              <Led tone="warn" />
              <Led tone="bad" />
              <Led tone="off" />
            </div>
          </div>
        </div>
      </Section>

      <Section id="keys" eyebrow="Answer keys" title="Keycap and KeycapGroup">
        <div className="flex flex-col gap-8">
          <div>
            <Sub>Key states: idle, latched, correct, wrong, disabled</Sub>
            <div id="key-states" className="flex flex-wrap gap-x-4 gap-y-3">
              <Keycap id="key-idle" legend="A" aria-label="Idle" />
              <Keycap id="key-latched" legend="B" latched aria-label="Latched" />
              <Keycap id="key-correct" legend="C" status="correct" aria-label="Correct" />
              <Keycap id="key-wrong" legend="D" status="wrong" aria-label="Wrong" />
              <Keycap id="key-disabled" legend="A" disabled aria-label="Disabled" />
            </div>
          </div>
          <div className="grid gap-10 md:grid-cols-2">
            <div id="group-live">
              <Sub>Live group (hotkeys on)</Sub>
              <LiveKeys active />
            </div>
            <div id="group-graded">
              <Sub>Graded: chose C, answer B</Sub>
              <KeycapGroup
                active={false}
                value="c"
                onChange={() => {}}
                aria-label="Graded answers"
                choices={CHOICES.map((c) => ({ ...c, status: c.id === "b" ? "correct" : c.id === "c" ? "wrong" : undefined }))}
              />
            </div>
          </div>
        </div>
      </Section>

      <Section id="cards" eyebrow="Study cards" title="QuestionCard: new, learning, mastered, flipped">
        <div className="grid gap-8 lg:grid-cols-2">
          <CardDemo id="card-new" mastery="new" />
          <CardDemo id="card-learning" mastery="learning" />
          <CardDemo id="card-mastered" mastery="mastered" />
          <CardDemo id="card-flipped" mastery="learning" defaultFlipped />
        </div>
      </Section>

      <Section id="hotkeys" eyebrow="Answer keys" title="KeycapGroup hotkeys and send on press">
        <div className="grid gap-8 md:grid-cols-3">
          <div>
            <Sub>Letters only (A-C; 1-3 do nothing)</Sub>
            <HotkeyDemo id="hk-letters" hotkeys="letters" />
          </div>
          <div>
            <Sub>None (the page owns the keys)</Sub>
            <HotkeyDemo id="hk-none" hotkeys="none" />
          </div>
          <div>
            <Sub>Send on press (click; arrows only move)</Sub>
            <HotkeyDemo id="hk-send" hotkeys="letters" sendOnPress />
          </div>
        </div>
      </Section>

      <Section id="verdict" eyebrow="Study cards" title="QuestionCard: no mastery, with a verdict">
        <div className="grid gap-8 lg:grid-cols-2">
          <div id="card-nomastery">
            <QuestionCard mastery={null} eyebrow="Practice question" verdict={<DeltaChip tone="bad" text="Not quite" />} explanation={<p>The one-sided limits must agree.</p>}>
              {Q}
            </QuestionCard>
          </div>
          <div id="card-verdict">
            <QuestionCard mastery={null} eyebrow="Practice question" defaultFlipped verdict={<DeltaChip tone="ok" text="Correct" />} explanation={<p>The one-sided limits agree, so the two-sided limit exists.</p>}>
              {Q}
            </QuestionCard>
          </div>
        </div>
      </Section>

      <Section id="lists" eyebrow="Shared pieces" title="BarList, DecisionBadge, QuietNote">
        <div className="grid gap-8 md:grid-cols-2">
          <div id="barlist">
            <BarList
              label="Documents by domain"
              capitalize
              empty="No documents yet."
              rows={[
                { label: "math", value: 72, color: "var(--chart-1)" },
                { label: "english", value: 41, color: "var(--chart-2)" },
                { label: "science", value: 33, color: "var(--chart-3)" },
                { label: "unsorted", value: 16 },
              ]}
            />
            <div className="mt-6">
              <Sub>Empty</Sub>
              <BarList label="Nothing" rows={[]} empty="No documents yet. Put files in a subject shelf, then run Ingest." />
            </div>
          </div>
          <div className="flex flex-col gap-6">
            <div id="badges" className="flex flex-wrap gap-3">
              <DecisionBadge shape="diamond" label="Baseline" color="var(--ink-2)" />
              <DecisionBadge shape="circle" label="Keep" color="var(--ok)" />
              <DecisionBadge shape="cross" label="Revert" color="var(--bad)" />
              <DecisionBadge shape="triangle" label="Pending" color="var(--warn)" />
            </div>
            <div id="notes" className="flex flex-col gap-3">
              <QuietNote tone="warn">Could not refresh: rag-engine answered slowly. Showing the last reading.</QuietNote>
              <QuietNote tone="bad">Could not refresh: cannot read the runs directory.</QuietNote>
            </div>
          </div>
        </div>
      </Section>

      <Section id="log" eyebrow="Jobs" title="LogList and LogLine (typed, with a check stamp)">
        <div id="log-demo">
          <LogList label="Ingest log">
            <LogLine time="16:39:24" label="queued" detail="ingest 3f2a91c4" />
            <LogLine time="16:39:26" label="running" detail="ingest 3f2a91c4" />
            <LogLine time="16:39:32" label="done" detail="finished in 6.0s" mark="check" />
            <LogLine time="16:40:01" label="failed" detail="embedding provider returned 429: rate limited" tone="bad" mark="cross" />
            <LogLine time="16:40:09" label="running" detail="ingest 9b1c77d0" caret />
          </LogList>
        </div>
      </Section>

      <Section id="rise" eyebrow="Motion" title="Rise: one-shot staggered entrance">
        <RiseDemo />
      </Section>

      <Section id="forms" eyebrow="Controls" title="Segmented, Stepper, SwitchRow, TextField, PasswordField">
        <FormDemo />
      </Section>

      <Section id="idle" eyebrow="Session chrome" title="IdleDim and useIdle">
        <IdleDim ms={3000} id="idle-demo" className="flex flex-wrap items-center gap-4">
          <span className={idleDimClass + " text-ink-2 font-serif text-[15px] italic"}>
            This chrome dims to 40% over 1.8 s after 3 s without input, and returns instantly on any movement.
          </span>
          <StateChip className={idleDimClass} state="Session 12:40" dot={false} />
        </IdleDim>
      </Section>
    </div>
  )
}

export function mountKitGallery(el: HTMLElement | null) {
  if (!el) return
  applyQuery()
  createRoot(el).render(<KitGallery />)
}
