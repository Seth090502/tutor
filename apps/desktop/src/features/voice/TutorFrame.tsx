import type { ReactNode } from "react"

import { IdleDim } from "@/components/kit"
import type { StageState } from "./session"
import { StateTint } from "./stage"
import "./tutor.css"

type TutorFrameProps = {
  state: StageState
  /** Session timer, voice style and the break note. Dims when idle. */
  chrome: ReactNode
  stage: ReactNode
  /** The attempt panel. Right-hand column when wide, between stage and conversation when narrow. */
  attempt: ReactNode
  transcript: ReactNode
  composer: ReactNode
}

/**
 * Layout for the live tutor. It answers to the width of the screen, not the window, so the
 * sidebar never skews it: two columns from 56rem, one below. The page tint sits behind both.
 * IdleDim only sets a flag; each piece of chrome opts in to the dim, the question card does not.
 */
export function TutorFrame({ state, chrome, stage, attempt, transcript, composer }: TutorFrameProps) {
  return (
    <div className="@container relative isolate h-full">
      <StateTint state={state} />
      <IdleDim className="tv-grid">
        <div data-area="chrome" className="flex flex-col gap-3">
          {chrome}
        </div>
        <div data-area="stage">{stage}</div>
        <div data-area="attempt" className="tv-scroll -m-1.5 p-1.5">
          {attempt}
        </div>
        <div data-area="transcript" className="relative min-h-0">
          {transcript}
        </div>
        <div data-area="composer">{composer}</div>
      </IdleDim>
    </div>
  )
}
