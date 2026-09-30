import { useState, type ReactNode } from "react"
import { Mic, MicOff } from "lucide-react"

import { Button } from "@/components/ui/button"
import { type AgentState, Orb } from "@/components/ui/orb"
import { cn } from "@/lib/utils"
import { useMicVolume } from "./useMicVolume"

// Color presets from ElevenLabs' own Orb demo (github.com/elevenlabs/ui, MIT).
const VOICES: { name: string; colors: [string, string] }[] = [
  { name: "Sky", colors: ["#CADCFC", "#A0B9D1"] },
  { name: "Sand", colors: ["#F6E7D8", "#E0CFC2"] },
  { name: "Slate", colors: ["#E5E7EB", "#9CA3AF"] },
]

const STATES: { label: string; value: AgentState }[] = [
  { label: "Idle", value: null },
  { label: "Listening", value: "listening" },
  { label: "Thinking", value: "thinking" },
  { label: "Talking", value: "talking" },
]

/** The inset ring ElevenLabs frames the orb with in their demo. */
function OrbFrame({ className, children }: { className: string; children: ReactNode }) {
  return (
    <div
      className={cn(
        "bg-muted relative rounded-full p-1 shadow-[inset_0_2px_8px_rgba(0,0,0,0.5)]",
        className,
      )}
    >
      <div className="bg-background h-full w-full overflow-hidden rounded-full shadow-[inset_0_0_12px_rgba(0,0,0,0.3)]">
        {children}
      </div>
    </div>
  )
}

export function TutorScreen() {
  const [voice, setVoice] = useState(0)
  const [agent, setAgent] = useState<AgentState>(null)
  const [live, setLive] = useState(false)
  const mic = useMicVolume(live)

  return (
    <div className="flex h-full flex-col items-center justify-center gap-10 p-8">
      <OrbFrame className="size-72">
        <Orb
          colors={VOICES[voice].colors}
          seed={(voice + 1) * 1000}
          agentState={live ? "listening" : agent}
          volumeMode={live ? "manual" : "auto"}
          inputVolumeRef={mic.level}
          getOutputVolume={() => 0.3}
        />
      </OrbFrame>

      <div className="flex flex-col items-center gap-4">
        <div className="flex gap-6" role="radiogroup" aria-label="Voice">
          {VOICES.map((v, i) => (
            <button
              key={v.name}
              role="radio"
              aria-checked={voice === i}
              onClick={() => setVoice(i)}
              className="flex flex-col items-center gap-2 text-xs"
            >
              <OrbFrame
                className={cn(
                  "size-14 transition",
                  voice === i ? "ring-ring ring-2 ring-offset-2 ring-offset-background" : "opacity-60 hover:opacity-100",
                )}
              >
                <Orb colors={v.colors} seed={(i + 1) * 1000} agentState={null} />
              </OrbFrame>
              <span className={voice === i ? "text-foreground" : "text-muted-foreground"}>{v.name}</span>
            </button>
          ))}
        </div>

        <div className="flex flex-wrap justify-center gap-2">
          {STATES.map((s) => (
            <Button
              key={s.label}
              size="sm"
              variant="outline"
              disabled={live || agent === s.value}
              onClick={() => setAgent(s.value)}
            >
              {s.label}
            </Button>
          ))}
          <Button size="sm" variant={live ? "default" : "outline"} onClick={() => setLive(!live)}>
            {live ? <MicOff /> : <Mic />}
            {live ? "Stop mic" : "Live mic"}
          </Button>
        </div>

        <p className="text-muted-foreground max-w-md text-center text-xs">
          {mic.error
            ? `Microphone unavailable: ${mic.error}`
            : "Preview. The teacher's voice connects once ElevenLabs is set up; Live mic drives the orb from your microphone."}
        </p>
      </div>
    </div>
  )
}
