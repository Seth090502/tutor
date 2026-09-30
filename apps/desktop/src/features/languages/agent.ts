// The per-language role-play partner (ElevenLabs agent, one per language; configs in
// apps/desktop/agent/agent_configs/lang-<code>.json). This file is the app's side of it: the dynamic
// variables the agent's prompt reads, the two client tools it can call, and the help rule.
//
// The two tools draw on the screen only. Nothing they do is spoken, and neither one reaches a
// service: `show_gloss` puts English on screen (logged as assistance) and `log_error` feeds the
// end-of-session digest.
import type { LoggedError } from "./digest"

export type SessionMode = "fluency" | "accuracy"
export type Band = "A1" | "A2" | "B1" | "B2" | "C1" | "C2"
export const BANDS: readonly Band[] = ["A1", "A2", "B1", "B2", "C1", "C2"]

export const MODE_COPY: Record<SessionMode, { title: string; detail: string }> = {
  fluency: {
    title: "Fluency",
    detail: "No interruptions. The partner keeps the conversation going and gives you a short digest of one to three errors at the end.",
  },
  accuracy: {
    title: "Accuracy",
    detail: "The partner prompts you to fix slips as they happen: a question first, then a clue, and only then the correct form.",
  },
}

export type Scenario = { id: string; title: string; prompt: string }
export const SCENARIOS: readonly Scenario[] = [
  { id: "cafe", title: "Ordering at a café", prompt: "You are a waiter or barista taking an order at a café." },
  { id: "directions", title: "Asking for directions", prompt: "You are a passer-by giving directions to a visitor who is lost." },
  { id: "introductions", title: "Meeting someone new", prompt: "You are meeting the learner for the first time at a small gathering." },
  { id: "station", title: "At the train station", prompt: "You are a ticket clerk at a train station selling a ticket." },
  { id: "shop", title: "Shopping for a gift", prompt: "You are a shop assistant helping the learner choose a gift." },
]

/** Dynamic variables the agent's prompt reads as {{name}}. The prompt and these names must stay in step (agent.test.ts). */
export function dynamicVariables(opts: {
  language: string
  mode: SessionMode
  band: Band
  scenario: Scenario
}): Record<string, string> {
  return {
    language_name: opts.language,
    mode: opts.mode,
    level: opts.band,
    scenario: opts.scenario.prompt,
  }
}

// ---- client tools ------------------------------------------------------------------------------

/** `gloss` is English the learner asked for (assistance); `rule` is a brief explicit rule the partner offers (accuracy mode). */
export type Gloss = { kind: "gloss" | "rule"; forText: string; english: string }

export type ToolSink = {
  onGloss: (gloss: Gloss) => void
  onError: (error: LoggedError) => void
}

type Params = Record<string, unknown>
export type LanguageTool = (params: Params) => Promise<string>

const str = (v: unknown) => (typeof v === "string" ? v.trim() : "")

function flag(v: unknown): boolean {
  return v === true || v === "true"
}

/** The tools the language agents declare (agent/tool_configs/show_gloss.json and log_error.json). */
export function createLanguageTools(sink: ToolSink): { show_gloss: LanguageTool; log_error: LanguageTool } {
  return {
    show_gloss: async (params) => {
      const forText = str(params.for_text)
      const english = str(params.english)
      if (!english) return "Error: english is required."
      sink.onGloss({ kind: str(params.kind) === "rule" ? "rule" : "gloss", forText, english })
      return "Shown on screen. Do not say the English aloud."
    },
    log_error: async (params) => {
      const corrected = str(params.corrected)
      const english = str(params.english)
      if (!corrected) return "Error: corrected is required."
      sink.onError({
        said: str(params.said),
        corrected,
        english,
        rule: str(params.rule),
        breaksComprehension: flag(params.breaks_comprehension),
      })
      return "Logged. Do not mention it to the learner."
    },
  }
}

// ---- transcript and help -----------------------------------------------------------------------

/** A transcript line. The learner's lines are what the recogniser heard, not what they said. */
export type Line = { role: "learner" | "partner"; text: string }

export const learnerTurns = (lines: Line[]) => lines.filter((l) => l.role === "learner").length

/** Attempt first: English help opens only once the learner has tried at least one turn this session. */
export function canAskHelp(lines: Line[]): boolean {
  return learnerTurns(lines) > 0
}

export const HELP_NUDGE = "Try a reply first, even a single word. English help opens after your first attempt."

/** The message the help button sends; the agent answers by calling show_gloss, not by speaking English. */
export const HELP_MESSAGE = "English help, please. Put it on the screen."
