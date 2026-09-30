import { useState } from "react"
import { Mic, MicOff } from "lucide-react"

import { IdleDim, LedStatus, Panel, idleDimClass, type LedTone } from "@/components/kit"
import { Orb } from "@/components/ui/orb"
import { STAGE_WORD, stageState, type StageState } from "@/features/voice/session"
import { OrbStage, StateTint, VOICES, VoiceSwatches } from "@/features/voice/stage"
import { cn } from "@/lib/utils"
import { useMicVolume } from "./useMicVolume"

import "@/features/voice/tutor.css"

/** Why the live call is not available, shown as a quiet status row above the preview. */
export type TutorNotice = { tone: LedTone; state: string; message: string }

const PREVIEW_STATES: { state: StageState; agent: null | "listening" | "thinking" | "talking" }[] = [
  { state: "idle", agent: null },
  { state: "listening", agent: "listening" },
  { state: "thinking", agent: "thinking" },
  { state: "speaking", agent: "talking" },
]

const PILL =
  "border-hairline-strong text-ink-2 hover:bg-world-soft hover:text-foreground focus-visible:outline-world aria-pressed:bg-foreground aria-pressed:text-background aria-pressed:border-foreground disabled:opacity-50 rounded-full border px-3.5 py-1.5 font-mono text-[11px] tracking-[0.14em] uppercase outline-offset-2 transition-colors duration-(--dur-quick) focus-visible:outline-2"

/**
 * The orb preview. It is what the Tutor shows while the voice call is not set up: no Tauri shell,
 * no ElevenLabs key or no agent id. The state buttons and the live microphone drive the orb so
 * the stage can be checked without a call.
 */
export function TutorScreen({ notice }: { notice?: TutorNotice }) {
  const [voice, setVoice] = useState(0)
  const [agent, setAgent] = useState<null | "listening" | "thinking" | "talking">(null)
  const [live, setLive] = useState(false)
  const mic = useMicVolume(live)

  const agentState = live ? "listening" : agent
  const state = stageState(agentState)

  return (
    <div className="@container relative isolate min-h-full">
      <StateTint state={state} />
      <IdleDim className="mx-auto flex min-h-full w-full max-w-2xl flex-col items-center justify-center gap-7 px-6 py-8">
        {notice ? (
          <Panel role="status" className="tv-enter w-full p-4">
            <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
              <LedStatus tone={notice.tone} state={notice.state} />
              <p className="text-ink-2 min-w-0 flex-1 basis-64 text-sm leading-relaxed">{notice.message}</p>
            </div>
          </Panel>
        ) : null}

        <OrbStage
          state={state}
          orbClass="size-44 @2xl:size-56"
          status={
            mic.error
              ? `Microphone unavailable: ${mic.error}`
              : "Preview. The teacher's voice connects once ElevenLabs is set up."
          }
          error={Boolean(mic.error)}
          orb={
            <Orb
              colors={VOICES[voice].colors}
              seed={(voice + 1) * 1000}
              agentState={agentState}
              volumeMode={live ? "manual" : "auto"}
              inputVolumeRef={mic.level}
              getOutputVolume={() => 0.3}
            />
          }
        />

        <div className={cn("flex flex-col items-center gap-4", idleDimClass)}>
          <div role="group" aria-label="Orb state" className="flex flex-wrap justify-center gap-2">
            {PREVIEW_STATES.map((p) => (
              <button
                key={p.state}
                type="button"
                className={PILL}
                aria-pressed={!live && agent === p.agent}
                disabled={live}
                onClick={() => setAgent(p.agent)}
              >
                {STAGE_WORD[p.state]}
              </button>
            ))}
            <button type="button" className={cn(PILL, "inline-flex items-center gap-2")} aria-pressed={live} onClick={() => setLive(!live)}>
              {live ? <MicOff aria-hidden className="size-3.5" /> : <Mic aria-hidden className="size-3.5" />}
              {live ? "Stop mic" : "Live mic"}
            </button>
          </div>
          <VoiceSwatches value={voice} onChange={setVoice} />
          <p className="text-ink-3 max-w-md text-center text-xs">Live mic drives the orb from your microphone.</p>
        </div>
      </IdleDim>
    </div>
  )
}
