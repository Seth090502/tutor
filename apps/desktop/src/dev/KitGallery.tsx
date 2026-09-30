import { useState, type ReactNode } from "react"
import { createRoot } from "react-dom/client"

import "@/index.css"
import {
  CountUp,
  DeltaChip,
  Eyebrow,
  IdleDim,
  Kbd,
  KbdHint,
  Keycap,
  KeycapGroup,
  Led,
  LedStatus,
  Panel,
  QuestionCard,
  Scrim,
  ServiceRow,
  Stat,
  StatRow,
  StateChip,
  idleDimClass,
  type KeycapChoice,
} from "@/components/kit"

/**
 * Dev-only gallery of every kit component in every state. kit.html sets the theme and world
 * from the query string: ?theme=dark|light&world=math|english|science|chemistry|neutral
 * (and &quality=rich|lite).
 */

const WORLDS = ["neutral", "math", "english", "science", "chemistry"] as const

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
