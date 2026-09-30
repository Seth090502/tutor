import type { ComponentProps, CSSProperties } from "react"
import { Check, X } from "lucide-react"

import { cn } from "@/lib/utils"
import { typedChars } from "./log-logic"
import "./kit.css"

/** The log: a mono list that announces new lines. */
export function LogList({ label, className, ...props }: Omit<ComponentProps<"ol">, "aria-label"> & { label: string }) {
  return (
    <ol
      data-slot="kit-log-list"
      role="log"
      aria-label={label}
      className={cn(
        "border-hairline m-0 flex list-none flex-col gap-1.5 border-t p-0 pt-4 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap",
        className,
      )}
      {...props}
    />
  )
}

type LogLineProps = Omit<ComponentProps<"li">, "children"> & {
  /** Already formatted, e.g. "16:39:24". */
  time: string
  /** The state word: queued, running, done, failed. */
  label: string
  detail: string
  tone?: "default" | "bad"
  /** A mark that stamps down after the line types in: a check for success, a cross for failure. */
  mark?: "check" | "cross"
  /** A caret after the detail, for the newest line of something still running. */
  caret?: boolean
}

/**
 * A typed log line: time, state word and detail in mono columns. The line types in once, character
 * by character (`steps()` lands on character edges because it is mono), and a check stamps down with a
 * little overshoot. Reduced motion shows the final line at once.
 */
export function LogLine({ time, label, detail, tone = "default", mark, caret = false, className, style, ...props }: LogLineProps) {
  return (
    <li
      data-slot="kit-log-line"
      className={cn("kit-type grid grid-cols-[8ch_8ch_minmax(0,1fr)_auto] items-start gap-x-3", className)}
      style={{ "--ch": typedChars(time, label, detail), ...style } as CSSProperties}
      {...props}
    >
      <span className="text-ink-3">{time}</span>
      <span className={tone === "bad" ? "text-bad" : "text-foreground"}>{label}</span>
      <span className="text-ink-2 min-w-0 break-words">
        {detail}
        {caret ? (
          <span aria-hidden className="text-world ml-1.5">
            ▍
          </span>
        ) : null}
      </span>
      {mark === "check" ? (
        <span aria-hidden className="kit-stamp border-ok/60 bg-ok/10 text-ok mt-px grid size-[18px] place-items-center rounded-full border">
          <Check className="size-3" strokeWidth={3} />
        </span>
      ) : null}
      {mark === "cross" ? (
        <span aria-hidden className="border-bad/60 bg-bad/10 text-bad mt-px grid size-[18px] place-items-center rounded-full border">
          <X className="size-3" strokeWidth={3} />
        </span>
      ) : null}
    </li>
  )
}
