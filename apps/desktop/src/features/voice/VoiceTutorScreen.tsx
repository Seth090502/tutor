import { useCallback, useEffect, useMemo, useState } from "react"
import { ConversationProvider, useConversation } from "@elevenlabs/react"
import { Settings } from "lucide-react"

import { Card } from "@/components/ui/card"
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from "@/components/ui/conversation"
import { ConversationBar } from "@/components/ui/conversation-bar"
import { Message, MessageContent } from "@/components/ui/message"
import { type AgentState, Orb } from "@/components/ui/orb"
import { Response } from "@/components/ui/response"
import { ShimmeringText } from "@/components/ui/shimmering-text"
import { OrbFrame, TutorScreen, VOICES } from "@/features/tutor/TutorScreen"
import { inTauri, shell } from "@/lib/backend"
import { cn } from "@/lib/utils"
import { type TutorToolName, tutorTools } from "./tools"

// Adapted from ElevenLabs UI's voice-chat-03 block (github.com/elevenlabs/ui, MIT).

type Setup = { kind: "loading" } | { kind: "blocked"; message: string } | { kind: "ready"; agentId: string }
type ChatMessage = { role: "user" | "assistant"; content: string }

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

  return (
    <div className="flex h-full flex-col">
      {setup.kind === "blocked" && (
        <div
          role="status"
          className="bg-muted/50 mx-auto mt-6 flex max-w-xl items-start gap-3 rounded-lg border px-4 py-3 text-sm"
        >
          <Settings className="text-muted-foreground mt-0.5 size-4 shrink-0" />
          <p>{setup.message}</p>
        </div>
      )}
      <div className="min-h-0 flex-1">
        <TutorScreen />
      </div>
    </div>
  )
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

  const { status, message, isSpeaking, getInputVolume, getOutputVolume } = useConversation({
    onConnect: () => setMessages([]),
    onMessage: (m) => {
      if (!m.message) return
      setMessages((prev) => [...prev, { role: m.role === "user" ? "user" : "assistant", content: m.message }])
    },
  })

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
  const agentState: AgentState =
    pendingTool || status === "connecting" ? "thinking" : !connected ? null : isSpeaking ? "talking" : "listening"

  let statusText: { text: string; shimmer: boolean }
  if (startError) statusText = { text: startError, shimmer: false }
  else if (status === "error") statusText = { text: message ?? "The call failed.", shimmer: false }
  else if (status === "connecting") statusText = { text: "Connecting", shimmer: true }
  else if (pendingTool) statusText = { text: TOOL_STATUS[pendingTool], shimmer: true }
  else if (connected) statusText = { text: isSpeaking ? "Speaking" : "Listening", shimmer: false }
  else statusText = { text: "Start a call to begin a lesson. Once connected you can also type.", shimmer: false }

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col gap-4 px-6 py-5">
      <div className="flex flex-col items-center gap-3">
        <OrbFrame className="size-44">
          <Orb
            colors={VOICES[voice].colors}
            seed={(voice + 1) * 1000}
            agentState={agentState}
            // Live volumes while talking or listening; the orb's own animation while thinking.
            volumeMode={connected && agentState !== "thinking" ? "manual" : "auto"}
            getInputVolume={getInputVolume}
            getOutputVolume={getOutputVolume}
          />
        </OrbFrame>

        <p
          aria-live="polite"
          className={cn(
            "min-h-5 text-center text-sm",
            startError || status === "error" ? "text-destructive" : "text-muted-foreground",
          )}
        >
          {statusText.shimmer ? <ShimmeringText text={statusText.text} /> : statusText.text}
        </p>

        <div className="flex gap-4" role="radiogroup" aria-label="Voice style">
          {VOICES.map((v, i) => (
            <button
              key={v.name}
              role="radio"
              aria-checked={voice === i}
              onClick={() => setVoice(i)}
              className="flex items-center gap-2 text-xs"
            >
              <OrbFrame
                className={cn(
                  "size-7 transition",
                  voice === i ? "ring-ring ring-offset-background ring-2 ring-offset-2" : "opacity-60 hover:opacity-100",
                )}
              >
                <Orb colors={v.colors} seed={(i + 1) * 1000} agentState={null} />
              </OrbFrame>
              <span className={voice === i ? "text-foreground" : "text-muted-foreground"}>{v.name}</span>
            </button>
          ))}
        </div>
      </div>

      <Card className="relative min-h-0 flex-1 gap-0 p-0">
        <Conversation className="absolute inset-0">
          <ConversationContent className="flex min-w-0 flex-col gap-2 p-6">
            {messages.length === 0 ? (
              <ConversationEmptyState
                title="No lesson yet"
                description="Ask for a practice question, how you're doing, or anything in your library."
              />
            ) : (
              messages.map((m, i) => (
                <Message key={i} from={m.role}>
                  <MessageContent className="max-w-full min-w-0">
                    <Response className="w-auto [overflow-wrap:anywhere] whitespace-pre-wrap">{m.content}</Response>
                  </MessageContent>
                </Message>
              ))
            )}
          </ConversationContent>
          <ConversationScrollButton />
        </Conversation>
      </Card>

      <ConversationBar
        className="p-0"
        agentId={agentId}
        label="Tutor"
        getConversationToken={getConversationToken}
        onSendMessage={(text) => setMessages((prev) => [...prev, { role: "user", content: text }])}
      />
    </div>
  )
}
