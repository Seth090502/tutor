import { Conversation, ConversationContent, ConversationScrollButton } from "@/components/ui/conversation"
import { Response } from "@/components/ui/response"
import { useReducedMotion } from "@/lib/motion"
import { cn } from "@/lib/utils"
import "./tutor.css"

/** "action" is a key press sent from the attempt panel; it is shown as a quiet caption. */
export type ChatMessage = { role: "user" | "assistant"; content: string; kind?: "text" | "action" }

const LABEL = "text-ink-3 font-mono text-[11px] tracking-[0.18em] uppercase"

function Line({ m }: { m: ChatMessage }) {
  if (m.kind === "action") {
    return <p className={cn(LABEL, "tv-fade self-end")}>{m.content}</p>
  }
  if (m.role === "user") {
    return (
      <div className="tv-fade flex max-w-[85%] flex-col items-end gap-1.5 self-end">
        <span className={LABEL}>You</span>
        <p className="bg-secondary border-hairline rounded-2xl rounded-tr-md border px-4 py-2.5 text-[15px] leading-normal [overflow-wrap:anywhere] whitespace-pre-wrap">
          {m.content}
        </p>
      </div>
    )
  }
  return (
    <div className="tv-fade flex max-w-[68ch] flex-col items-start gap-1.5 self-start">
      <span className={cn(LABEL, "flex items-center gap-3")}>
        <span aria-hidden className="bg-hairline-strong h-px w-4" />
        Tutor
      </span>
      <Response className="tv-tutor-text w-auto whitespace-pre-wrap">{m.content}</Response>
    </div>
  )
}

/**
 * The conversation so far. It follows new messages while the learner is at the bottom and stops
 * following as soon as they scroll up; the arrow button brings them back.
 */
export function Transcript({ messages, className }: { messages: ChatMessage[]; className?: string }) {
  const reduced = useReducedMotion()
  const mode = reduced ? "instant" : "smooth"

  return (
    <section
      aria-label="Conversation"
      className={cn("tv-scroll bg-card/70 border-hairline relative min-h-40 overflow-hidden rounded-2xl border", className)}
    >
      <Conversation className="absolute inset-0" initial={mode} resize={mode}>
        <ConversationContent className="flex min-w-0 flex-col gap-5 p-5">
          {messages.map((m, i) => (
            <Line key={i} m={m} />
          ))}
        </ConversationContent>
        <ConversationScrollButton aria-label="Jump to the latest message" />
      </Conversation>
      {messages.length === 0 ? (
        <div className="pointer-events-none absolute inset-0 grid place-items-center p-8 text-center">
          <div className="flex max-w-xs flex-col gap-2">
            <p className="font-serif text-xl">No lesson yet</p>
            <p className="text-ink-2 font-serif text-[15px] leading-relaxed italic">
              Ask for a practice question, how you are doing, or anything in your library.
            </p>
          </div>
        </div>
      ) : null}
    </section>
  )
}
