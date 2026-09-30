import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react"
import { ConversationProvider, useConversation } from "@elevenlabs/react"

import { idleDimClass } from "@/components/kit"
import { ConversationBar } from "@/components/ui/conversation-bar"
import { type AgentState, Orb } from "@/components/ui/orb"
import { ShimmeringText } from "@/components/ui/shimmering-text"
import { TutorScreen, type TutorNotice } from "@/features/tutor/TutorScreen"
import { inTauri, shell } from "@/lib/backend"
import type { StudyAttempt } from "@/lib/study"
import { cn } from "@/lib/utils"
import { AttemptPanel } from "./AttemptPanel"
import { confidenceWord } from "./attempt"
import { type PanelAction, type VoicePractice, createVoicePractice, openNext, panelActions, studyErrorText } from "./practice"
import { SessionChrome } from "./SessionChrome"
import { stageState } from "./session"
import { OrbStage, VOICES, VoiceSwatches } from "./stage"
import { TutorFrame } from "./TutorFrame"
import { type ChatMessage, Transcript } from "./Transcript"
import { type TutorToolName, createTutorTools, screenMessage } from "./tools"

// The orb stage, the transcript and the call bar adapt ElevenLabs UI's voice-chat-03 block
// (github.com/elevenlabs/ui, MIT).

type Setup = { kind: "loading" } | { kind: "blocked"; message: string } | { kind: "ready"; agentId: string }

const TOOL_STATUS: Record<TutorToolName, string> = {
  get_next_question: "Finding a question",
  propose_answer: "Showing your answer to confirm",
  set_confidence: "Noting your confidence",
  get_hint: "Getting a hint",
  check_answer: "Checking your answer",
  give_up: "Showing the answer",
  search_library: "Searching your library",
  get_progress: "Reading your progress",
}

/** The transcript line for something the learner did on screen. Never answer text. */
function screenCaption(action: PanelAction, attempt: StudyAttempt): string {
  switch (action) {
    case "next":
      return "Next question"
    case "answer":
      return "Answered on screen"
    case "confidence": {
      const level = attempt.confidence?.level
      return level ? `Confidence ${level}, ${confidenceWord(level)}` : "Skipped confidence"
    }
    case "check":
      return "Checked"
    case "hint":
      return "Asked for a hint"
    case "repair":
      return "Tried again"
    case "give_up":
      return "Gave up"
    case "self_assess":
      return attempt.self_assessment?.judged === "correct" ? "Got it" : "Missed it"
  }
}

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Whether the shell, the ElevenLabs key and the agent id are all in place. */
function useSetup(): Setup {
  const [setup, setSetup] = useState<Setup>(() =>
    inTauri
      ? { kind: "loading" }
      : {
          kind: "blocked",
          message:
            "Voice runs in the desktop app, which holds your ElevenLabs key. Start it with npm run tauri dev, then finish setup in Settings.",
        },
  )

  useEffect(() => {
    if (!inTauri) return
    let cancelled = false
    Promise.all([shell.secretsStatus(), shell.configGet()]).then(
      ([secrets, config]) => {
        if (cancelled) return
        if (!secrets.elevenlabs) {
          setSetup({ kind: "blocked", message: "Add your ElevenLabs API key in Settings to talk with the tutor." })
        } else if (!config.elevenlabs_agent_id) {
          setSetup({
            kind: "blocked",
            message:
              "Add the ElevenLabs agent ID in Settings. Create the agent first (apps/desktop/agent/README.md).",
          })
        } else {
          setSetup({ kind: "ready", agentId: config.elevenlabs_agent_id })
        }
      },
      (e: unknown) => {
        if (!cancelled) setSetup({ kind: "blocked", message: `Could not read settings: ${errorText(e)}` })
      },
    )
    return () => {
      cancelled = true
    }
  }, [])

  return setup
}

/** Voice teacher: an ElevenLabs Agents conversation whose tools call the local services. */
export function VoiceTutorScreen() {
  const setup = useSetup()
  if (setup.kind === "ready") return <VoiceSession agentId={setup.agentId} />

  const notice: TutorNotice =
    setup.kind === "blocked"
      ? { tone: "warn", state: "Voice offline", message: setup.message }
      : { tone: "off", state: "Checking", message: "Reading your voice settings." }
  return <TutorScreen notice={notice} />
}

function VoiceSession({ agentId }: { agentId: string }) {
  // One study session per Tutor screen (C12), shared by the voice tools and the typed controls.
  const [practice] = useState(createVoicePractice)
  // Tool calls in flight, so the orb can show "thinking" while the tutor waits on a tool.
  const [pending, setPending] = useState<TutorToolName[]>([])

  const clientTools = useMemo(() => {
    const tools = createTutorTools(practice)
    return Object.fromEntries(
      (Object.keys(tools) as TutorToolName[]).map((name) => [
        name,
        async (params: Record<string, unknown>) => {
          setPending((p) => [...p, name])
          try {
            return await tools[name](params)
          } finally {
            setPending((p) => {
              const i = p.indexOf(name)
              return i < 0 ? p : [...p.slice(0, i), ...p.slice(i + 1)]
            })
          }
        },
      ]),
    )
  }, [practice])

  return (
    <ConversationProvider clientTools={clientTools}>
      <VoiceTutor agentId={agentId} practice={practice} pendingTool={pending[pending.length - 1] ?? null} />
    </ConversationProvider>
  )
}

function VoiceTutor({
  agentId,
  practice,
  pendingTool,
}: {
  agentId: string
  practice: VoicePractice
  pendingTool: TutorToolName | null
}) {
  const [voice, setVoice] = useState(0)
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [startError, setStartError] = useState<string | null>(null)
  const [practiceError, setPracticeError] = useState<string | null>(null)
  // The session and attempt exactly as the server last accepted them.
  const snapshot = useSyncExternalStore(practice.subscribe, practice.getSnapshot)

  const { status, message, isSpeaking, getInputVolume, getOutputVolume, sendUserMessage } = useConversation({
    onConnect: () => {
      setMessages([])
    },
    onMessage: (m) => {
      if (!m.message) return
      setMessages((prev) => [...prev, { role: m.role === "user" ? "user" : "assistant", content: m.message }])
    },
  })

  // Resume this screen's study session after a restart; a new one starts with the first question.
  useEffect(() => {
    practice.current().catch((e: unknown) => setPracticeError(`Could not resume practice: ${studyErrorText(e)}.`))
  }, [practice])

  const getConversationToken = useCallback(async () => {
    setStartError(null)
    try {
      return (await shell.elevenlabsConversationToken(agentId)).token
    } catch (e) {
      setStartError(`Could not start the call: ${errorText(e)}`)
      throw e
    }
  }, [agentId])

  const connected = status === "connected"

  // What the learner does on screen goes to the server first; once accepted, the tutor hears about
  // it (contract C8), so it carries on from there instead of repeating the step with a tool.
  const tell = useRef<(action: PanelAction, attempt: StudyAttempt) => void>(() => {})
  tell.current = (action, attempt) => {
    if (!connected) return
    sendUserMessage(screenMessage(action, attempt))
    setMessages((prev) => [...prev, { role: "user", kind: "action", content: screenCaption(action, attempt) }])
  }
  const session = snapshot ? practice.session : null
  const actions = useMemo(
    () => (session ? panelActions(session, (action, attempt) => tell.current(action, attempt)) : undefined),
    [session],
  )

  // Typed practice without a call: start (or resume) the session and open the next due item.
  const startTyped = useCallback(async () => {
    setPracticeError(null)
    try {
      const opened = await openNext(await practice.ensure("ui"), "ui")
      if (!opened) setPracticeError("Nothing is due right now.")
    } catch (e) {
      setPracticeError(`Could not start practice: ${studyErrorText(e)}.`)
    }
  }, [practice])

  const agentState: AgentState =
    pendingTool || status === "connecting" ? "thinking" : !connected ? null : isSpeaking ? "talking" : "listening"
  const state = stageState(agentState)

  let statusText: { text: string; shimmer: boolean }
  if (startError) statusText = { text: startError, shimmer: false }
  else if (status === "error") statusText = { text: message ?? "The call failed.", shimmer: false }
  else if (status === "connecting") statusText = { text: "Connecting", shimmer: true }
  else if (pendingTool) statusText = { text: TOOL_STATUS[pendingTool], shimmer: true }
  else if (connected) statusText = { text: isSpeaking ? "" : "Say your answer, or type it below.", shimmer: false }
  else statusText = { text: "Start a call to begin a lesson. Once connected you can also type.", shimmer: false }

  return (
    <TutorFrame
      state={state}
      chrome={
        <SessionChrome live={connected}>
          <div className={idleDimClass}>
            <VoiceSwatches value={voice} onChange={setVoice} />
          </div>
        </SessionChrome>
      }
      stage={
        <OrbStage
          state={state}
          error={Boolean(startError) || status === "error"}
          status={statusText.shimmer ? <ShimmeringText text={statusText.text} /> : statusText.text}
          orb={
            <Orb
              colors={VOICES[voice].colors}
              seed={(voice + 1) * 1000}
              agentState={agentState}
              // Live volumes while talking or listening; the orb's own animation while thinking.
              volumeMode={connected && agentState !== "thinking" ? "manual" : "auto"}
              getInputVolume={getInputVolume}
              getOutputVolume={getOutputVolume}
            />
          }
        />
      }
      attempt={
        <AttemptPanel
          attempt={snapshot?.attempt ?? null}
          actions={actions}
          notice={snapshot?.attempt ? null : practiceError}
          onStart={() => void startTyped()}
        />
      }
      transcript={<Transcript messages={messages} className="h-full" />}
      composer={
        <ConversationBar
          className={cn("p-0", idleDimClass)}
          agentId={agentId}
          label="Tutor"
          getConversationToken={getConversationToken}
          onSendMessage={(text) => setMessages((prev) => [...prev, { role: "user", content: text }])}
        />
      }
    />
  )
}
