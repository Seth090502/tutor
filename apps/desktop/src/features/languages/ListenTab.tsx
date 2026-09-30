// Listen: a minimal-pair trainer (high-variability phonetic training). Several voices say real words,
// the learner picks which word they heard, and training gives feedback after each answer. The test block
// uses held-out words and, when there are enough voices, held-out voices, and gives no feedback until the
// end. Results are logged locally. The pair lists are unverified (contrasts.ts); ElevenLabs voices stand in
// for natural talkers, which is untested, and the screen says so.
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { Headphones, Loader2, RefreshCw } from "lucide-react"

import { Eyebrow, KeycapGroup, Panel, Stat, StatRow, DeltaChip } from "@/components/kit"
import { Sparkline } from "@/components/charts"
import { Button } from "@/components/ui/button"
import { newSeed } from "@/features/train/engine/prng"
import { cn } from "@/lib/utils"
import { languageMeta, type TargetLanguage } from "@/lib/language"
import { contrastsFor, type Contrast, type Pair, type Token } from "./contrasts"
import type { AudioEnv } from "./hooks"
import { useVoices } from "./hooks"
import {
  buildRecord,
  comparison,
  isCorrect,
  MIN_VOICES,
  planSession,
  scoreBlock,
  splitVoices,
  type Plan,
  type Response,
  type SessionRecord,
  type Side,
  type Trial,
} from "./hvpt"
import { appendLog, LOG_KEYS, readLog } from "./logs"
import { Notice, PlayButton, StateBlock } from "./parts"
import { audioUrl, playUrl, prefetch, stopAudio } from "./speech"

type Phase = "setup" | "train" | "bridge" | "test" | "summary"

const pct = (x: number | null) => (x === null ? "none" : `${Math.round(x * 100)}`)

export function ListenTab({ language, audio }: { language: TargetLanguage; audio: AudioEnv }) {
  const meta = languageMeta(language)
  if (audio.loading) {
    return (
      <div role="status" className="text-ink-2 flex items-center justify-center gap-3 py-24 text-sm">
        <Loader2 aria-hidden className="size-4 animate-spin" /> Checking audio
      </div>
    )
  }
  if (audio.blocker) {
    return (
      <StateBlock icon={Headphones} title="Listening practice needs audio">
        {audio.blocker} The {meta.english} voices are played by ElevenLabs through the desktop app.
      </StateBlock>
    )
  }
  return <ListenSession key={language} language={language} />
}

function ListenSession({ language }: { language: TargetLanguage }) {
  const contrasts = useMemo(() => contrastsFor(language), [language])
  const [contrastId, setContrastId] = useState(contrasts[0]?.id ?? "")
  const contrast = contrasts.find((c) => c.id === contrastId) ?? contrasts[0]
  const voices = useVoices(language, true)
  const [phase, setPhase] = useState<Phase>("setup")
  const [plan, setPlan] = useState<Plan | null>(null)
  const [train, setTrain] = useState<Response[]>([])
  const [record, setRecord] = useState<SessionRecord | null>(null)
  const [saved, setSaved] = useState(true)

  const voiceIds = voices.chosen.status === "ready" ? voices.chosen.value : []
  const names = useMemo(() => {
    const map = new Map<string, string>()
    if (voices.catalogue.status === "ready") for (const v of voices.catalogue.value) map.set(v.voice_id, v.name)
    return map
  }, [voices.catalogue])

  const begin = () => {
    if (!contrast || voiceIds.length === 0) return
    const p = planSession({ pairs: contrast.pairs, voices: voiceIds, seed: newSeed() })
    setPlan(p)
    setTrain([])
    setRecord(null)
    setPhase("train")
  }

  const finish = (p: Plan, trainR: Response[], testR: Response[]) => {
    const r = buildRecord({ at: Date.now() / 1000, lang: language, contrastId: contrast.id, unverified: !contrast.verified, plan: p, train: trainR, test: testR })
    setSaved(appendLog(LOG_KEYS.hvpt, r))
    setRecord(r)
    setPhase("summary")
  }

  if (!contrast) {
    return <StateBlock icon={Headphones} title="No minimal pairs for this language yet" />
  }

  if (phase === "setup") {
    return (
      <Setup
        language={language}
        contrasts={contrasts}
        contrastId={contrast.id}
        onContrast={setContrastId}
        voices={voices}
        voiceIds={voiceIds}
        onStart={begin}
      />
    )
  }

  if (!plan) return null

  if (phase === "bridge") {
    const score = scoreBlock(train)
    return (
      <Panel eyebrow="Training done" title="Now the test" className="lg-enter mx-auto max-w-2xl">
        <p className="text-ink-2 text-[15px] leading-relaxed">
          You got {score.correct} of {score.n} in training. The test uses {plan.testPairIds.length} words you have not heard{plan.voices.heldOut ? ` and ${plan.voices.test.length === 1 ? "a voice" : "voices"} you have not heard` : ""}. There is no feedback until the end, so it measures what carried over.
        </p>
        {!plan.voices.heldOut ? (
          <Notice className="mt-4">With one voice the test can only hold out words, not voices. Choose three or more voices to test on a new talker.</Notice>
        ) : null}
        <Button className="mt-5" onClick={() => setPhase("test")}>Start the test</Button>
      </Panel>
    )
  }

  if (phase === "train" || phase === "test") {
    const trials = phase === "train" ? plan.train : plan.test
    return (
      <TrialRunner
        key={phase}
        language={language}
        contrast={contrast}
        block={phase}
        trials={trials}
        voiceLabel={(id) => labelFor(id, plan, names)}
        onDone={(rs) => {
          if (phase === "train") {
            setTrain(rs)
            if (plan.test.length) setPhase("bridge")
            else finish(plan, rs, [])
          } else {
            finish(plan, train, rs)
          }
        }}
        onExit={() => {
          stopAudio()
          setPhase("setup")
        }}
      />
    )
  }

  return record ? (
    <Summary
      record={record}
      contrast={contrast}
      saved={saved}
      labelFor={(id) => labelFor(id, plan, names)}
      onAgain={begin}
      onChoose={() => setPhase("setup")}
    />
  ) : null
}

/** A stable short name for a voice within this session: the catalogue name, else "Voice 2". */
function labelFor(id: string, plan: Plan, names: Map<string, string>): string {
  const all = [...plan.voices.train, ...plan.voices.test.filter((v) => !plan.voices.train.includes(v))]
  return names.get(id) ?? `Voice ${all.indexOf(id) + 1}`
}

// ---- setup -------------------------------------------------------------------------------------

function Setup({
  language,
  contrasts,
  contrastId,
  onContrast,
  voices,
  voiceIds,
  onStart,
}: {
  language: TargetLanguage
  contrasts: Contrast[]
  contrastId: string
  onContrast: (id: string) => void
  voices: ReturnType<typeof useVoices>
  voiceIds: string[]
  onStart: () => void
}) {
  const history = useMemo(() => readLog<SessionRecord>(LOG_KEYS.hvpt).filter((r) => r.lang === language), [language])
  const split = splitVoices(voiceIds, 1)
  return (
    <div className="@container">
      <div className="grid gap-5 @3xl:grid-cols-[minmax(0,1fr)_minmax(0,22rem)]">
        <Panel eyebrow="Contrast" title="What do you want to tell apart?" className="lg-enter p-6">
          <div role="radiogroup" aria-label="Contrast" className="flex flex-col gap-2.5">
            {contrasts.map((c) => {
              const tests = history.filter((r) => r.contrastId === c.id && r.test?.accuracy != null)
              const last = tests[tests.length - 1]?.test?.accuracy ?? null
              return (
                <button
                  key={c.id}
                  type="button"
                  role="radio"
                  aria-checked={c.id === contrastId}
                  onClick={() => onContrast(c.id)}
                  className="border-hairline-strong hover:bg-world-soft focus-visible:outline-world aria-checked:border-foreground aria-checked:bg-secondary flex flex-col gap-1 rounded-xl border px-4 py-3 text-left outline-offset-2 transition-colors duration-(--dur-quick) focus-visible:outline-2"
                >
                  <span className="flex flex-wrap items-center gap-x-3 gap-y-1">
                    <span className="text-[15px] font-medium">{c.title}</span>
                    {!c.verified ? <DeltaChip text="Unverified" tone="warn" /> : null}
                    <span className="text-ink-3 ml-auto font-mono text-[11px] tracking-[0.08em]">
                      {c.pairs.length} pairs{last !== null ? ` · last test ${pct(last)}%` : ""}
                    </span>
                  </span>
                  <span className="text-ink-2 text-sm leading-relaxed">{c.note}</span>
                </button>
              )
            })}
          </div>
          <Notice className="mt-4">
            These word lists were written without a native speaker and are marked unverified until one checks them. The voices are synthetic
            stand-ins for natural talkers, which has not been tested. Treat a surprising pair as a question for a native speaker, not as a fact about the language.
          </Notice>
        </Panel>

        <VoicePicker language={language} voices={voices} split={split} onStart={onStart} count={voiceIds.length} />
      </div>
    </div>
  )
}

function VoicePicker({
  language,
  voices,
  split,
  onStart,
  count,
}: {
  language: TargetLanguage
  voices: ReturnType<typeof useVoices>
  split: ReturnType<typeof splitVoices>
  onStart: () => void
  count: number
}) {
  const [draft, setDraft] = useState<string[] | null>(null)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const chosen = voices.chosen.status === "ready" ? voices.chosen.value : []
  const picked = draft ?? chosen
  const dirty = draft !== null && draft.join() !== chosen.join()

  const toggle = (id: string) => setDraft(picked.includes(id) ? picked.filter((v) => v !== id) : [...picked, id].slice(0, 6))
  const save = async () => {
    setSaving(true)
    setError(null)
    try {
      await voices.save(picked)
      setDraft(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Panel eyebrow="Voices" title={`${languageMeta(language).english} voices`} className="lg-enter p-6" style={{ "--i": 1 } as React.CSSProperties}>
      {voices.catalogue.status === "loading" || voices.chosen.status === "loading" ? (
        <p role="status" className="text-ink-2 flex items-center gap-2 text-sm">
          <Loader2 aria-hidden className="size-4 animate-spin" /> Loading voices
        </p>
      ) : voices.catalogue.status === "error" ? (
        <Notice tone="warn">Could not load the voice list: {voices.catalogue.message}</Notice>
      ) : (
        <>
          <p className="text-ink-2 mb-3 text-sm leading-relaxed">
            Pick three to six. Training uses most of them; the test uses one you have not heard in training.
          </p>
          <ul className="flex max-h-56 flex-col gap-1 overflow-y-auto pr-1">
            {voices.catalogue.value.map((v) => (
              <li key={v.voice_id}>
                <label className="hover:bg-accent flex cursor-pointer items-center gap-3 rounded-lg px-2 py-1.5 text-sm">
                  <input type="checkbox" className="size-4 accent-[var(--foreground)]" checked={picked.includes(v.voice_id)} onChange={() => toggle(v.voice_id)} />
                  <span className="font-medium">{v.name}</span>
                  <span className="text-ink-3 truncate font-mono text-[11px]">{Object.values(v.labels).slice(0, 3).join(" · ")}</span>
                </label>
              </li>
            ))}
            {voices.catalogue.value.length === 0 ? <li className="text-ink-2 text-sm">No voices came back for this language.</li> : null}
          </ul>
          {error ? <Notice tone="warn" className="mt-3">{error}</Notice> : null}
          <div className="mt-4 flex flex-wrap items-center gap-3">
            <Button variant="outline" size="sm" onClick={() => void save()} disabled={!dirty || saving}>
              {saving ? <Loader2 aria-hidden className="animate-spin" /> : null}
              Save voices
            </Button>
            <span className="text-ink-3 text-[13px]">{picked.length} chosen</span>
          </div>
        </>
      )}
      <div className="border-hairline mt-5 border-t pt-5">
        <p className="text-ink-2 mb-3 text-sm leading-relaxed" aria-live="polite">
          {count === 0
            ? "Save at least one voice to start."
            : count < MIN_VOICES
              ? `${count} saved. ${split.heldOut ? "The test will use a voice you did not train on, but training has little variety." : "With one voice the test holds out words only."}`
              : `${count} saved. The test holds out ${split.test.length} ${split.test.length === 1 ? "voice" : "voices"}.`}
        </p>
        <Button onClick={onStart} disabled={count === 0 || dirty}>
          Start: 24 training trials, then 12 test
        </Button>
        {dirty ? <p className="text-ink-3 mt-2 text-[13px]">Save your voice choice first.</p> : null}
      </div>
    </Panel>
  )
}

// ---- trials ------------------------------------------------------------------------------------

function TrialRunner({
  language,
  contrast,
  block,
  trials,
  voiceLabel,
  onDone,
  onExit,
}: {
  language: TargetLanguage
  contrast: Contrast
  block: "train" | "test"
  trials: Trial[]
  voiceLabel: (id: string) => string
  onDone: (responses: Response[]) => void
  onExit: () => void
}) {
  const pairs = useMemo(() => new Map<string, Pair>(contrast.pairs.map((p) => [p.id, p])), [contrast])
  const [i, setI] = useState(0)
  const [responses, setResponses] = useState<Response[]>([])
  const [chosen, setChosen] = useState<Side | null>(null)
  const [audioError, setAudioError] = useState<string | null>(null)
  const [playing, setPlaying] = useState(false)
  const startedAt = useRef<number | null>(null)
  const alive = useRef(true)
  const nextRef = useRef<HTMLButtonElement>(null)
  const trial = trials[i]
  const pair = pairs.get(trial.pairId)!
  const feedback = block === "train"

  const tokenOf = useCallback((t: Trial, side: Side): Token => {
    const p = pairs.get(t.pairId)!
    return side === "a" ? p.a : p.b
  }, [pairs])

  const play = useCallback(
    async (t: Trial, side: Side = t.target, markStart = false) => {
      setAudioError(null)
      setPlaying(true)
      try {
        const url = await audioUrl(tokenOf(t, side).text, language, { voiceId: t.voiceId })
        if (!alive.current) return
        if (markStart) startedAt.current = performance.now()
        await playUrl(url)
      } catch (e) {
        if (alive.current) setAudioError(e instanceof Error ? e.message : String(e))
      } finally {
        if (alive.current) setPlaying(false)
      }
    },
    [language, tokenOf],
  )

  useEffect(() => {
    alive.current = true
    return () => {
      alive.current = false
      stopAudio()
    }
  }, [])

  // Each trial plays on arrival, and the next few are fetched ahead.
  useEffect(() => {
    startedAt.current = null
    void play(trial, trial.target, true)
    for (const ahead of trials.slice(i + 1, i + 4)) prefetch(tokenOf(ahead, ahead.target).text, language, { voiceId: ahead.voiceId })
  }, [i, trial, trials, play, tokenOf, language])

  const advance = useCallback(
    (rs: Response[]) => {
      stopAudio()
      if (i + 1 >= trials.length) return onDone(rs)
      setI(i + 1)
      setChosen(null)
    },
    [i, trials.length, onDone],
  )

  const answer = (side: Side) => {
    if (chosen) return
    const ms = Math.round(performance.now() - (startedAt.current ?? performance.now()))
    const rs = [...responses, { trial, chosen: side, ms }]
    setChosen(side)
    setResponses(rs)
    if (!feedback) window.setTimeout(() => alive.current && advance(rs), 450)
    else window.setTimeout(() => nextRef.current?.focus(), 0)
  }

  const left: Side = trial.flip ? "b" : "a"
  const right: Side = trial.flip ? "a" : "b"
  const status = (side: Side) =>
    !feedback || !chosen ? undefined : side === trial.target ? ("correct" as const) : side === chosen ? ("wrong" as const) : undefined
  const choices = [left, right].map((side) => ({
    id: side,
    legend: side === left ? "A" : "B",
    label: (
      <span lang={language} className="text-[20px] leading-tight">
        {tokenOf(trial, side).label}
      </span>
    ),
    status: status(side),
  }))
  const correct = chosen ? isCorrect(trial, chosen) : null

  return (
    <div className="@container mx-auto max-w-2xl">
      <Panel
        eyebrow={`${block === "train" ? "Training" : "Test, no feedback"} · ${i + 1} of ${trials.length}`}
        title="Which word did you hear?"
        className="lg-enter p-6"
        actions={
          <Button variant="ghost" size="sm" onClick={onExit}>
            Stop
          </Button>
        }
      >
        <div className="bg-secondary mb-5 h-1.5 overflow-hidden rounded-full" aria-hidden>
          <div className="bg-foreground h-full origin-left rounded-full transition-transform duration-(--dur-base)" style={{ transform: `scaleX(${(i + (chosen ? 1 : 0)) / trials.length})` }} />
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <PlayButton label="Play again" status={playing ? "playing" : "idle"} onPlay={() => void play(trial)} onStop={stopAudio} variant="secondary" size="default" />
          <span className="text-ink-2 text-sm">{voiceLabel(trial.voiceId)}</span>
        </div>
        {audioError ? <Notice tone="warn" className="mt-4">{audioError}</Notice> : null}

        <div className="mt-6">
          <KeycapGroup
            aria-label="Which word did you hear, A or B"
            choices={choices}
            value={chosen}
            onChange={(id) => answer(id as Side)}
            disabled={Boolean(audioError) && !chosen}
          />
        </div>

        {feedback && chosen ? (
          <div className="lg-enter mt-6 flex flex-col gap-3" aria-live="polite">
            <p className="text-[15px]">
              <strong className="font-medium">{correct ? "Correct." : "Not this time."}</strong>{" "}
              <span lang={language}>{tokenOf(trial, trial.target).label}</span> was said
              {correct ? "" : <>, and you chose <span lang={language}>{tokenOf(trial, chosen).label}</span></>}.
            </p>
            <div className="flex flex-wrap items-center gap-3">
              <Button variant="outline" size="sm" onClick={() => void (async () => { await play(trial, "a"); await play(trial, "b") })()}>
                Hear both
              </Button>
              <Button ref={nextRef} onClick={() => advance(responses)}>
                {i + 1 >= trials.length ? "Finish training" : "Next"}
              </Button>
            </div>
            <p className="text-ink-3 text-[13px]">Listen, repeat aloud, then compare. Hearing the pair side by side is the point of the feedback.</p>
          </div>
        ) : (
          <p className={cn("text-ink-3 mt-5 text-[13px]", chosen && "invisible")}>
            The word plays once on its own. Press A or B, or click.
          </p>
        )}
        <p className="sr-only">{pair.a.label} or {pair.b.label}</p>
      </Panel>
    </div>
  )
}

// ---- summary -----------------------------------------------------------------------------------

function Summary({
  record,
  contrast,
  saved,
  labelFor,
  onAgain,
  onChoose,
}: {
  record: SessionRecord
  contrast: Contrast
  saved: boolean
  labelFor: (id: string) => string
  onAgain: () => void
  onChoose: () => void
}) {
  const past = readLog<SessionRecord>(LOG_KEYS.hvpt)
    .filter((r) => r.contrastId === contrast.id && r.test?.accuracy != null)
    .map((r) => (r.test?.accuracy ?? 0) * 100)
  const testVoices = record.testVoices.map(labelFor)
  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-5">
      <Panel eyebrow="Session result" title={contrast.title} className="lg-enter p-6">
        <StatRow>
          <Stat size="md" label="Training" value={pct(record.train.accuracy)} unit="% correct" animate={false} />
          <Stat size="md" label={record.voicesHeldOut ? "Test, new voice" : "Test, new words"} value={pct(record.test?.accuracy ?? null)} unit="% correct" animate={false} />
          <Stat size="md" label="Test d-prime" value={record.test?.dPrime == null ? "none" : record.test.dPrime.toFixed(2)} animate={false} />
        </StatRow>
        <p className="text-ink-2 mt-4 text-[15px] leading-relaxed">{comparison(record.train, record.test, record.voicesHeldOut)}</p>
        <p className="text-ink-3 mt-1 text-[13px]">
          Twelve trials is a small sample: a few points either way is noise. d-prime of 0 is chance; above 2 is strong.
        </p>

        {record.test ? (
          <div className="mt-5">
            <Eyebrow>Test accuracy by voice</Eyebrow>
            <ul className="mt-2 flex flex-col gap-1 text-sm">
              {Object.entries(record.test.byVoice).map(([id, v]) => (
                <li key={id} className="flex justify-between gap-4">
                  <span>{labelFor(id)}</span>
                  <span className="tabular-nums">{v.correct} of {v.n}</span>
                </li>
              ))}
            </ul>
            <p className="text-ink-3 mt-2 text-[13px]">
              {record.voicesHeldOut ? `Held out from training: ${testVoices.join(", ")}.` : "The test used a voice you trained on."}
            </p>
          </div>
        ) : null}

        {past.length >= 2 ? (
          <div className="mt-5 flex items-center gap-4">
            <Eyebrow>Test accuracy over {past.length} sessions</Eyebrow>
            <Sparkline values={past} name="Test accuracy, percent" width={140} height={32} />
          </div>
        ) : null}

        {!contrast.verified ? <Notice className="mt-5">This pair list is unverified. Until a native speaker has checked it, treat these results as practice, not as a measurement.</Notice> : null}
        <p className="text-ink-3 mt-4 text-[13px]">{saved ? "Saved on this device." : "Could not save this session: local storage is unavailable."}</p>
        <div className="mt-5 flex flex-wrap gap-3">
          <Button onClick={onAgain}>
            <RefreshCw aria-hidden /> Another session
          </Button>
          <Button variant="outline" onClick={onChoose}>
            Choose another contrast
          </Button>
        </div>
      </Panel>
    </div>
  )
}
