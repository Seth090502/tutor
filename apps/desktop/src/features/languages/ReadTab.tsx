// Read: the comprehensible-input lane. A graded text at the chosen band, its known-word coverage from the
// learner's own FSRS state, sentence-level audio, a tapped-word panel (gloss on screen only, then a card),
// and comprehension questions that ask for an attempt before showing the reference. Reading is extra
// exposure, not the main source of new cards, so nothing here adds a card without a tap.
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { BookOpen, Loader2, Plus, ServerOff } from "lucide-react"

import { Eyebrow, Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { languageMeta, type TargetLanguage } from "./_pending/language"
import { addCards, errorText, gradedText, isOffline, type GradedText } from "./api"
import { BANDS, type Band } from "./agent"
import type { AudioEnv } from "./hooks"
import { appendLog, LOG_KEYS } from "./logs"
import { CoverageMeter, FIELD, Notice, PlayButton, Segmented, StateBlock } from "./parts"
import { platform } from "./platform"
import { cardItem, isNew, tokenize, unknownSet } from "./reading"
import { useSay } from "./speech"

type Load =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "offline" }
  | { status: "error"; message: string }
  | { status: "ready"; text: GradedText; band: Band }

type Tap = { word: string; sentence: string }
type Stats = { glosses: number; cards: number; answered: number; hadIt: number }
const NO_STATS: Stats = { glosses: 0, cards: 0, answered: 0, hadIt: 0 }

const bandAsks = (band: Band) => (BANDS.indexOf(band) >= BANDS.indexOf("B1") ? "target" : "English")

export function ReadTab({ language, audio }: { language: TargetLanguage; audio: AudioEnv }) {
  const [band, setBand] = useState<Band>("A2")
  const [topic, setTopic] = useState("")
  const [load, setLoad] = useState<Load>({ status: "idle" })
  const stats = useRef<Stats>({ ...NO_STATS })
  const serial = useRef(0)

  // One log entry per text, written when the text is replaced or the tab is left.
  const ready = load.status === "ready" ? load : null
  useEffect(() => {
    if (!ready) return
    return () => {
      appendLog(LOG_KEYS.reading, {
        at: Date.now() / 1000,
        lang: language,
        band: ready.band,
        coverage: ready.text.coverage,
        unknown: ready.text.unknown.length,
        ...stats.current,
      })
      stats.current = { ...NO_STATS }
    }
  }, [ready, language])

  const generate = useCallback(async () => {
    const mine = ++serial.current
    setLoad({ status: "loading" })
    try {
      const text = await gradedText(language, band, topic)
      if (mine === serial.current) setLoad({ status: "ready", text, band })
    } catch (e) {
      if (mine === serial.current) setLoad(isOffline(e) ? { status: "offline" } : { status: "error", message: errorText(e) })
    }
  }, [language, band, topic])

  useEffect(() => () => void serial.current++, [])

  return (
    <div className="flex flex-col gap-5">
      <div className="lg-enter flex flex-col gap-2">
        <div className="flex flex-wrap items-end gap-x-6 gap-y-4">
          <div>
            <Eyebrow id="lg-band-label">Band</Eyebrow>
            <Segmented
              label="Band, A1 easiest to C2 hardest"
              value={band}
              onChange={setBand}
              options={BANDS.map((b) => ({ id: b, label: b }))}
              className="mt-2"
            />
          </div>
          <div className="min-w-72 flex-1">
            <label htmlFor="lg-topic" className="mb-1.5 block">
              <Eyebrow>Topic, optional</Eyebrow>
            </label>
            <input id="lg-topic" className={FIELD} value={topic} placeholder="For example: a day at the market" onChange={(e) => setTopic(e.target.value)} onKeyDown={(e) => e.key === "Enter" && void generate()} />
          </div>
          <Button onClick={() => void generate()} disabled={load.status === "loading"}>
            {load.status === "loading" ? <Loader2 aria-hidden className="animate-spin" /> : <BookOpen aria-hidden />}
            {load.status === "ready" ? "New text" : "Generate a text"}
          </Button>
        </div>
        <p className="text-ink-3 text-[13px]">
          Written to about 97% words you know, from your own review history. The band is your own estimate, not an official level.
        </p>
      </div>

      {load.status === "idle" ? null : load.status === "loading" ? (
        <div role="status" className="text-ink-2 flex items-center justify-center gap-3 py-16 text-sm">
          <Loader2 aria-hidden className="size-4 animate-spin" /> Writing a text and checking your coverage. This can take half a minute.
        </div>
      ) : load.status === "offline" ? (
        <StateBlock icon={ServerOff} title="The backend is not running" actions={<Button variant="outline" size="sm" onClick={() => void generate()}>Try again</Button>}>
          Graded texts are written and checked by the DeepTutor backend on this machine. Start it from Settings, then try again.
        </StateBlock>
      ) : load.status === "error" ? (
        <StateBlock icon={ServerOff} title="Could not make a text" actions={<Button variant="outline" size="sm" onClick={() => void generate()}>Try again</Button>}>
          {load.message}
        </StateBlock>
      ) : (
        <TextView key={load.text.text} language={language} band={load.band} text={load.text} audio={audio} stats={stats} />
      )}
    </div>
  )
}

function TextView({ language, band, text, audio, stats }: { language: TargetLanguage; band: Band; text: GradedText; audio: AudioEnv; stats: React.RefObject<Stats> }) {
  const unknown = useMemo(() => unknownSet(text.unknown), [text.unknown])
  const sentences = text.sentences.length ? text.sentences : [text.text]
  const [tap, setTap] = useState<Tap | null>(null)
  const say = useSay(language)

  return (
    <div className="@container flex flex-col gap-5">
      <div className="grid gap-5 @3xl:grid-cols-[minmax(0,1fr)_minmax(0,20rem)]">
        <Panel eyebrow={`Band ${band}`} title="Read it" className="lg-enter p-6" style={{ "--i": 1 } as React.CSSProperties}>
          <div className="mb-5">
            <CoverageMeter coverage={text.coverage} unknown={text.unknown.length} />
          </div>
          <div className="lg-reading" lang={language}>
            {sentences.map((s, si) => (
              <p key={si} className="mb-3 flex items-start gap-2">
                <span className="mt-1.5 shrink-0">
                  <PlayButton
                    label={`Play sentence ${si + 1}`}
                    size="icon"
                    variant="ghost"
                    className="size-7"
                    status={say.playing === s ? say.status : "idle"}
                    onPlay={() => void say.say(s)}
                    onStop={say.stop}
                    disabledReason={audio.blocker}
                  />
                </span>
                <span>
                  {tokenize(s, language).map((part, pi) =>
                    part.word ? (
                      <button
                        key={pi}
                        type="button"
                        className="lg-word"
                        data-new={isNew(part.text, unknown) ? "" : undefined}
                        aria-pressed={tap?.word === part.text && tap.sentence === s}
                        onClick={() => setTap({ word: part.text, sentence: s })}
                      >
                        {part.text}
                      </button>
                    ) : (
                      <span key={pi}>{part.text}</span>
                    ),
                  )}
                </span>
              </p>
            ))}
          </div>
          {say.error ? <Notice tone="warn" className="mt-3">{say.error}</Notice> : null}
          {audio.blocker ? <p className="text-ink-3 mt-2 text-[13px]">{audio.blocker}</p> : null}
          <p className="text-ink-3 mt-3 text-[13px]">Tap a word for its meaning. Words with a dotted underline are new to you.</p>
        </Panel>

        <WordPanel key={tap ? `${tap.word}|${tap.sentence}` : "none"} language={language} tap={tap} stats={stats} />
      </div>

      <Comprehension language={language} band={band} questions={text.questions} stats={stats} />
    </div>
  )
}

function WordPanel({ language, tap, stats }: { language: TargetLanguage; tap: Tap | null; stats: React.RefObject<Stats> }) {
  const [gloss, setGloss] = useState<{ status: "loading" } | { status: "ready"; text: string | null } | { status: "error"; message: string }>({ status: "loading" })
  const [meaning, setMeaning] = useState("")
  const [adding, setAdding] = useState<"idle" | "busy" | { created: number } | { error: string }>("idle")

  useEffect(() => {
    if (!tap) return
    let cancelled = false
    platform()
      .gloss(language, tap.word, tap.sentence)
      .then(
        (text) => {
          if (cancelled) return
          if (text) stats.current.glosses++
          setGloss({ status: "ready", text })
          if (text) setMeaning(text)
        },
        (e: unknown) => !cancelled && setGloss({ status: "error", message: errorText(e) }),
      )
    return () => {
      cancelled = true
    }
  }, [tap, language, stats])

  if (!tap) {
    return (
      <Panel eyebrow="Word" title="Nothing tapped yet" className="lg-enter p-6" style={{ "--i": 2 } as React.CSSProperties}>
        <p className="text-ink-2 text-sm leading-relaxed">Tap any word in the text to see what it means and to add it as a card.</p>
      </Panel>
    )
  }

  const item = cardItem(tap.word, meaning, tap.sentence)
  const add = async () => {
    if (!item) return
    setAdding("busy")
    try {
      const out = await addCards(language, [item])
      stats.current.cards += out.created
      setAdding({ created: out.created })
    } catch (e) {
      setAdding({ error: errorText(e) })
    }
  }

  return (
    <Panel eyebrow="Word" className="lg-enter p-6" style={{ "--i": 2 } as React.CSSProperties}>
      <p lang={language} className="font-serif text-[28px] leading-tight tracking-[-0.01em]">
        {tap.word}
      </p>
      <p lang={language} className="text-ink-2 mt-2 text-[15px] leading-relaxed">{tap.sentence}</p>

      <div className="mt-5">
        <Eyebrow>Meaning, on screen only</Eyebrow>
        {gloss.status === "loading" ? (
          <p role="status" className="text-ink-2 mt-2 flex items-center gap-2 text-sm"><Loader2 aria-hidden className="size-4 animate-spin" /> Looking it up</p>
        ) : gloss.status === "error" ? (
          <Notice tone="warn" className="mt-2">{gloss.message}</Notice>
        ) : gloss.text ? (
          <p className="mt-2 text-[16px] leading-relaxed">{gloss.text}</p>
        ) : (
          <p className="text-ink-2 mt-2 text-sm leading-relaxed">No dictionary is connected yet. Write the meaning yourself below; that is also a good first retrieval.</p>
        )}
        <label htmlFor="lg-meaning" className="mt-3 mb-1.5 block">
          <Eyebrow>Card meaning</Eyebrow>
        </label>
        <input id="lg-meaning" lang="en" className={FIELD} value={meaning} onChange={(e) => { setMeaning(e.target.value); setAdding("idle") }} placeholder="In English" onKeyDown={(e) => e.key === "Enter" && void add()} />
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-3">
        <Button onClick={() => void add()} disabled={!item || adding === "busy" || (typeof adding === "object" && "created" in adding)}>
          {adding === "busy" ? <Loader2 aria-hidden className="animate-spin" /> : <Plus aria-hidden />}
          Add as card
        </Button>
        <span className="text-sm" aria-live="polite">
          {typeof adding === "object" && "created" in adding ? (
            <span className="text-ink-2">Added. {adding.created} {adding.created === 1 ? "card" : "cards"} created.</span>
          ) : typeof adding === "object" && "error" in adding ? (
            <span className="text-bad">{adding.error}</span>
          ) : null}
        </span>
      </div>
    </Panel>
  )
}

type Q = { shown: boolean; draft: string; had: boolean | null }

function Comprehension({ language, band, questions, stats }: { language: TargetLanguage; band: Band; questions: { q: string; a: string }[]; stats: React.RefObject<Stats> }) {
  const [qs, setQs] = useState<Q[]>(() => questions.map(() => ({ shown: false, draft: "", had: null })))
  const set = (i: number, patch: Partial<Q>) => setQs((all) => all.map((q, k) => (k === i ? { ...q, ...patch } : q)))
  const qLang = bandAsks(band) === "target" ? language : "en"

  if (questions.length === 0) return null
  return (
    <Panel eyebrow="Comprehension" title="Answer from memory of the text" className="lg-enter p-6" style={{ "--i": 3 } as React.CSSProperties}>
      <p className="text-ink-2 mb-5 text-sm leading-relaxed">
        The questions are in {bandAsks(band) === "target" ? "the target language" : "English"} at this band. Write your answer first; the reference only opens after an attempt, and you judge your own answer.
      </p>
      <ol className="flex flex-col gap-6">
        {questions.map((q, i) => (
          <li key={i} className="border-hairline border-t pt-5 first:border-t-0 first:pt-0">
            <p lang={qLang} className="font-serif text-[18px] leading-snug">
              <span className="text-ink-3 mr-2 font-mono text-[11px] tracking-[0.14em]">{i + 1}</span>
              {q.q}
            </p>
            <label htmlFor={`lg-q-${i}`} className="sr-only">Your answer to question {i + 1}</label>
            <textarea id={`lg-q-${i}`} lang={language} rows={2} className={`${FIELD} mt-3 resize-y`} placeholder={`Answer in ${languageMeta(language).english} if you can`} value={qs[i].draft} disabled={qs[i].shown} onChange={(e) => set(i, { draft: e.target.value })} />
            {!qs[i].shown ? (
              <Button className="mt-3" variant="outline" size="sm" disabled={!qs[i].draft.trim()} onClick={() => set(i, { shown: true })}>
                Show the reference answer
              </Button>
            ) : (
              <div className="lg-enter mt-3 flex flex-col gap-3">
                <p className="border-hairline-strong rounded-lg border-l-2 py-1 pl-4 font-serif text-[17px] leading-relaxed" lang={qLang}>
                  <span className="text-ink-3 mr-2 font-mono text-[11px] tracking-[0.14em] uppercase">Reference</span>
                  {q.a}
                </p>
                {qs[i].had === null ? (
                  <div className="flex flex-wrap items-center gap-3">
                    <span className="text-sm">Did yours say the same thing?</span>
                    <Button variant="outline" size="sm" onClick={() => { set(i, { had: true }); stats.current.answered++; stats.current.hadIt++ }}>I had it</Button>
                    <Button variant="outline" size="sm" onClick={() => { set(i, { had: false }); stats.current.answered++ }}>Not quite</Button>
                  </div>
                ) : (
                  <p className="text-ink-2 text-sm">Noted as {qs[i].had ? "understood" : "not yet understood"}. This stays on this device; it does not change your card schedule.</p>
                )}
              </div>
            )}
          </li>
        ))}
      </ol>
    </Panel>
  )
}
