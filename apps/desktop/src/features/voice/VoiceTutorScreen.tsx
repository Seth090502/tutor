import { useCallback, useEffect, useMemo, useReducer, useState } from "react"
import { ConversationProvider, useConversation } from "@elevenlabs/react"

import { idleDimClass } from "@/components/kit"
import { ConversationBar } from "@/components/ui/conversation-bar"
import { type AgentState, Orb } from "@/components/ui/orb"
import { ShimmeringText } from "@/components/ui/shimmering-text"
import { TutorScreen, type TutorNotice } from "@/features/tutor/TutorScreen"
import { inTauri, shell } from "@/lib/backend"
import { cn } from "@/lib/utils"
import { AttemptPanel } from "./AttemptPanel"
import { answerMessage, attemptReducer, confidenceMessage, confidenceWord, type Confidence } from "./attempt"
import { SessionChrome } from "./SessionChrome"
import { stageState } from "./session"
import { OrbStage, VOICES, VoiceSwatches } from "./stage"
import { TutorFrame } from "./TutorFrame"
import { type ChatMessage, Transcript } from "./Transcript"
import { type TutorToolName, onTutorEvent, tutorTools } from "./tools"

// The orb stage, the transcript and the call bar adapt ElevenLabs UI's voice-chat-03 block
// (github.com/elevenlabs/ui, MIT).

type Setup = { kind: "loading" } | { kind: "blocked"; message: string } | { kind: "ready"; agentId: string }

const TOOL_STATUS: Record<TutorToolName, string> = {
  get_next_question: "Finding a question",
  set_confidence: "Noting your confidence",
  get_hint: "Getting a hint",
  check_answer: "Checking your answer",
  record_review: "Scheduling your next review",
  search_library: "Searching your library",
  get_progress: "Reading your progress",
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
  // Tool calls in flight, so the orb can show "thinking" while the tutor waits on a tool.
  const [pending, setPending] = useState<TutorToolName[]>([])

  const clientTools = useMemo(
    () =>
      Object.fromEntries(
        (Object.keys(tutorTools) as TutorToolName[]).map((name) => [
          name,
          async (params: Record<string, unknown>) => {
            setPending((p) => [...p, name])
            try {
              return await tutorTools[name](params)
            } finally {
              setPending((p) => {
                const i = p.indexOf(name)
                return i < 0 ? p : [...p.slice(0, i), ...p.slice(i + 1)]
              })
            }
          },
        ]),
      ),
    [],
  )

  return (
    <ConversationProvider clientTools={clientTools}>
      <VoiceTutor agentId={agentId} pendingTool={pending[pending.length - 1] ?? null} />
    </ConversationProvider>
  )
}

function VoiceTutor({ agentId, pendingTool }: { agentId: string; pendingTool: TutorToolName | null }) {
  const [voice, setVoice] = useState(0)
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [startError, setStartError] = useState<string | null>(null)
  const [attempt, dispatch] = useReducer(attemptReducer, null)

  const { status, message, isSpeaking, getInputVolume, getOutputVolume, sendUserMessage } = useConversation({
    onConnect: () => {
      setMessages([])
      dispatch({ type: "reset" })
    },
    onMessage: (m) => {
      if (!m.message) return
      setMessages((prev) => [...prev, { role: m.role === "user" ? "user" : "assistant", content: m.message }])
    },
  })

  // What the agent's tools did (contract C8) becomes the attempt panel.
  useEffect(() => onTutorEvent(dispatch), [])

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
  const attemptId = attempt?.attemptId

  // Keys send what the learner pressed as a user message, so the agent calls the tools itself.
  const sendKey = useCallback(
    (text: string, caption: string) => {
      sendUserMessage(text)
      setMessages((prev) => [...prev, { role: "user", kind: "action", content: caption }])
    },
    [sendUserMessage],
  )
  const onAnswer = useCallback(
    (choice: string) => {
      if (!attemptId) return
      sendKey(answerMessage(choice), `Answered ${choice}`)
      dispatch({ type: "answered", attempt_id: attemptId, choice })
    },
    [attemptId, sendKey],
  )
  const onRate = useCallback(
    (level: Confidence) => {
      if (!attemptId) return
      sendKey(confidenceMessage(level), `Confidence ${level}, ${confidenceWord(level)}`)
      dispatch({ type: "rated", attempt_id: attemptId, level })
    },
    [attemptId, sendKey],
  )

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
        <AttemptPanel attempt={attempt} onAnswer={connected ? onAnswer : undefined} onRate={connected ? onRate : undefined} />
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
