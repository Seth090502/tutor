// Speak: a role-play partner through the per-language ElevenLabs agent (agent id in config key
// `agent_<lang>`, conversation token through the shell), plus the speaking check. Fluency or accuracy mode,
// the level and the scenario go in as dynamic variables. English help is an on-screen client tool, never
// spoken. When the call ends, the digest shows 1-3 logged errors that can become cards.
import { useCallback, useMemo, useRef, useState } from "react"
import { ConversationProvider, useConversation } from "@elevenlabs/react"
import { KeyRound, Loader2, Mic, PhoneCall } from "lucide-react"

import { Eyebrow, Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { languageMeta, type TargetLanguage } from "@/lib/language"
import { addCards, errorText } from "./api"
import {
  BANDS,
  canAskHelp,
  createLanguageTools,
  dynamicVariables,
  HELP_MESSAGE,
  MODE_COPY,
  SCENARIOS,
  type Band,
  type Gloss,
  type Line,
  type Scenario,
  type SessionMode,
} from "./agent"
import { buildDigest, digestCards, type DigestItem, type LoggedError } from "./digest"
import type { AudioEnv } from "./hooks"
import { useAgentId } from "./hooks"
import { appendLog, LOG_KEYS } from "./logs"
import { FIELD, Notice, Segmented, StateBlock } from "./parts"
import { platform } from "./platform"
import { DigestView, LiveView, type Digest, type LiveStatus } from "./SpeakView"
import { SpeakingCheck } from "./SpeakingCheck"

export type Settings = { mode: SessionMode; band: Band; scenarioId: string }

export function SpeakTab({ language, audio }: { language: TargetLanguage; audio: AudioEnv }) {
  const meta = languageMeta(language)
  const agent = useAgentId(language)

  if (audio.loading || agent.state.status === "loading") {
    return (
      <div role="status" className="text-ink-2 flex items-center justify-center gap-3 py-24 text-sm">
        <Loader2 aria-hidden className="size-4 animate-spin" /> Checking your voice setup
      </div>
    )
  }
  if (audio.blocker) {
    return (
      <div className="flex flex-col gap-5">
        <StateBlock icon={audio.inTauri ? KeyRound : Mic} title="The role-play partner needs the desktop app and a key">
          {audio.blocker} The partner is an ElevenLabs agent, so the call only works in the desktop app with your key stored.
        </StateBlock>
      </div>
    )
  }
  if (agent.state.status === "error") {
    return (
      <StateBlock icon={KeyRound} title="Could not read the app settings">
        {agent.state.message}
      </StateBlock>
    )
  }
  if (!agent.state.value) {
    return <AgentSetup language={language} onSave={agent.save} />
  }

  return (
    <ConversationProviderGate key={`${language}:${agent.state.value}`} language={language} agentId={agent.state.value} audio={audio} label={meta.english} />
  )
}

function AgentSetup({ language, onSave }: { language: TargetLanguage; onSave: (id: string) => Promise<void> }) {
  const meta = languageMeta(language)
  const [id, setId] = useState("")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const save = async () => {
    if (!id.trim()) return
    setBusy(true)
    setError(null)
    try {
      await onSave(id)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }
  return (
    <StateBlock icon={KeyRound} title={`No ${meta.english} partner is set up yet`}>
      <p>
        Each language has its own ElevenLabs agent. Create the {meta.english} one from <code className="font-mono text-[12px]">apps/desktop/agent/agent_configs/lang-{language}.json</code> (see the agent README), then paste its id here. It is saved as the app setting <code className="font-mono text-[12px]">agent_{language}</code>.
      </p>
      <div className="mt-4 flex w-full max-w-sm items-center gap-2">
        <label htmlFor="lg-agent" className="sr-only">Agent id for {meta.english}</label>
        <input id="lg-agent" className={FIELD} value={id} placeholder="agent_…" spellCheck={false} autoComplete="off" onChange={(e) => setId(e.target.value)} onKeyDown={(e) => e.key === "Enter" && void save()} />
        <Button onClick={() => void save()} disabled={!id.trim() || busy}>
          {busy ? <Loader2 aria-hidden className="animate-spin" /> : null}
          Save
        </Button>
      </div>
      {error ? <Notice tone="warn" className="mt-3 text-left">{error}</Notice> : null}
    </StateBlock>
  )
}

/** The session state the tools and the call both write to. */
function useSession() {
  const [lines, setLines] = useState<Line[]>([])
  const [glosses, setGlosses] = useState<Gloss[]>([])
  const refs = useRef({ lines: [] as Line[], glosses: [] as Gloss[], errors: [] as LoggedError[], help: 0 })

  const reset = useCallback(() => {
    refs.current = { lines: [], glosses: [], errors: [], help: 0 }
    setLines([])
    setGlosses([])
  }, [])
  const addLine = useCallback((line: Line) => {
    refs.current.lines = [...refs.current.lines, line]
    setLines(refs.current.lines)
  }, [])
  const tools = useMemo(
    () =>
      createLanguageTools({
        onGloss: (g) => {
          refs.current.glosses = [...refs.current.glosses, g]
          setGlosses(refs.current.glosses)
        },
        onError: (e) => {
          refs.current.errors = [...refs.current.errors, e]
        },
      }),
    [],
  )
  return { lines, glosses, refs, reset, addLine, tools }
}

function ConversationProviderGate(props: { language: TargetLanguage; agentId: string; audio: AudioEnv; label: string }) {
  const session = useSession()
  return (
    <ConversationProvider clientTools={session.tools}>
      <SpeakReady {...props} session={session} />
    </ConversationProvider>
  )
}

function SpeakReady({ language, agentId, audio, session }: { language: TargetLanguage; agentId: string; audio: AudioEnv; label: string; session: ReturnType<typeof useSession> }) {
  const meta = languageMeta(language)
  const [settings, setSettings] = useState<Settings>({ mode: "fluency", band: "A2", scenarioId: SCENARIOS[0].id })
  const [digest, setDigest] = useState<Digest | null>(null)
  const [startError, setStartError] = useState<string | null>(null)
  const [nudge, setNudge] = useState(false)
  const [starting, setStarting] = useState(false)
  const live = useRef<{ settings: Settings; scenario: Scenario } | null>(null)
  const scenario = SCENARIOS.find((s) => s.id === settings.scenarioId) ?? SCENARIOS[0]

  const finish = useCallback(() => {
    const ctx = live.current
    if (!ctx) return
    live.current = null
    const { lines, glosses, errors, help } = session.refs.current
    if (lines.length === 0) {
      setStartError("The call ended before anything was said.")
      return
    }
    const items = buildDigest(errors)
    const d: Digest = {
      items,
      mode: ctx.settings.mode,
      band: ctx.settings.band,
      scenario: ctx.scenario.title,
      lines,
      helpRequests: help,
      rules: glosses.filter((g) => g.kind === "rule").length,
      errorsLogged: errors.length,
    }
    appendLog(
      LOG_KEYS.conversations,
      { at: Date.now() / 1000, lang: language, mode: d.mode, band: d.band, scenario: d.scenario, helpRequests: d.helpRequests, errorsLogged: d.errorsLogged, digest: items, lines },
      20,
    )
    setDigest(d)
  }, [language, session.refs])

  const conv = useConversation({
    onConnect: () => {
      session.reset()
      setNudge(false)
    },
    onMessage: (m) => {
      if (m.message) session.addLine({ role: m.role === "user" ? "learner" : "partner", text: m.message })
    },
    onDisconnect: finish,
    onError: (message: string) => setStartError(message || "The call failed."),
  })

  const start = async () => {
    setStartError(null)
    setStarting(true)
    try {
      // Ask for the microphone inside the click, so the prompt is tied to a user action.
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      stream.getTracks().forEach((t) => t.stop())
      const token = await platform().conversationToken(agentId)
      live.current = { settings, scenario }
      setDigest(null)
      conv.startSession({
        conversationToken: token,
        connectionType: "webrtc",
        dynamicVariables: dynamicVariables({ language: meta.english, mode: settings.mode, band: settings.band, scenario }),
      })
    } catch (e) {
      live.current = null
      setStartError(e instanceof DOMException && e.name === "NotAllowedError" ? "The microphone is blocked. Allow microphone access for this app and try again." : `Could not start the call: ${errorText(e)}`)
    } finally {
      setStarting(false)
    }
  }

  const help = () => {
    if (!canAskHelp(session.lines)) return setNudge(true)
    setNudge(false)
    session.refs.current.help++
    conv.sendUserMessage(HELP_MESSAGE)
  }

  const inCall = conv.status === "connecting" || conv.status === "connected"
  if (inCall) {
    const status: LiveStatus = conv.status === "connecting" ? "connecting" : conv.isSpeaking ? "speaking" : "listening"
    return (
      <LiveView
        language={language}
        mode={settings.mode}
        band={settings.band}
        scenario={scenario.title}
        status={status}
        lines={session.lines}
        glosses={session.glosses}
        nudge={nudge}
        onHelp={help}
        onEnd={() => conv.endSession()}
      />
    )
  }

  if (digest) {
    return (
      <DigestView
        language={language}
        digest={digest}
        onAddCards={async (items: DigestItem[]) => (await addCards(language, digestCards(items))).created}
        onAgain={() => setDigest(null)}
      />
    )
  }

  return (
    <div className="flex flex-col gap-5">
      <SetupPanel settings={settings} onChange={setSettings} onStart={() => void start()} starting={starting} error={startError} language={language} />
      <SpeakingCheck language={language} audio={audio} />
    </div>
  )
}

export function SetupPanel({
  settings,
  onChange,
  onStart,
  starting,
  error,
  language,
}: {
  settings: Settings
  onChange: (s: Settings) => void
  onStart: () => void
  starting: boolean
  error: string | null
  language: TargetLanguage
}) {
  const meta = languageMeta(language)
  const scenario = SCENARIOS.find((s) => s.id === settings.scenarioId) ?? SCENARIOS[0]
  return (
    <Panel eyebrow="Role-play" title={`Talk with a ${meta.english} partner`} className="lg-enter p-6">
      <div className="@container">
      <div className="grid gap-6 @3xl:grid-cols-2">
        <div className="flex flex-col gap-5">
          <div>
            <Eyebrow>Mode</Eyebrow>
            <Segmented
              label="Mode"
              value={settings.mode}
              onChange={(mode) => onChange({ ...settings, mode })}
              options={(Object.keys(MODE_COPY) as SessionMode[]).map((m) => ({ id: m, label: MODE_COPY[m].title }))}
              className="mt-2"
            />
            <p className="text-ink-2 mt-2 text-sm leading-relaxed">{MODE_COPY[settings.mode].detail}</p>
          </div>
          <div>
            <Eyebrow>Level</Eyebrow>
            <Segmented label="Level" value={settings.band} onChange={(band) => onChange({ ...settings, band })} options={BANDS.map((b) => ({ id: b, label: b }))} className="mt-2" />
            <p className="text-ink-3 mt-2 text-[13px]">Your own estimate. It sets how simply the partner speaks.</p>
          </div>
        </div>
        <div className="flex flex-col gap-5">
          <div>
            <Eyebrow>Scenario</Eyebrow>
            <Segmented
              label="Scenario"
              value={settings.scenarioId}
              onChange={(scenarioId) => onChange({ ...settings, scenarioId })}
              options={SCENARIOS.map((s) => ({ id: s.id, label: s.title }))}
              className="mt-2"
            />
            <p className="text-ink-2 mt-2 text-sm leading-relaxed">{scenario.prompt}</p>
          </div>
        </div>
      </div>
      </div>

      <div className="border-hairline mt-6 flex flex-wrap items-center gap-4 border-t pt-5">
        <Button onClick={onStart} disabled={starting}>
          {starting ? <Loader2 aria-hidden className="animate-spin" /> : <PhoneCall aria-hidden />}
          Start the call
        </Button>
        <p className="text-ink-3 max-w-[60ch] text-[13px] leading-relaxed">
          The partner speaks only {meta.english} and starts with a short greeting. Audio is not stored by ElevenLabs, and its copy of the text is kept for one day. The transcript stays on this device.
        </p>
      </div>
      {error ? <Notice tone="warn" className="mt-4">{error}</Notice> : null}
    </Panel>
  )
}
