// The Guided Reading flow as a pure state machine (spec: docs/tutor/CORPUS.md, Stage 1; calls: C11).
// Order: prequestions, read, closed-book recall, learner summary, missed idea units, atoms, prequestion
// answers. One direction only: there is no way back to the text once recall has begun.
import type { ServiceName } from "./api"
import type { AnswerResult, Confidence, Prepared, Prequestion, SectionRef, SummaryResult } from "./types"

/** A paper keeps the same steps and routes; only the learner's task in three of them differs. */
export type Mode = "section" | "paper"
export type Step = "prequestions" | "reading" | "recall" | "summary" | "missed" | "atoms" | "explanations"

export const STEPS: Step[] = ["prequestions", "reading", "recall", "summary", "missed", "atoms", "explanations"]

export const STEP_LABEL: Record<Mode, Record<Step, string>> = {
  section: { prequestions: "Predict", reading: "Read", recall: "Recall", summary: "Summary", missed: "Gaps", atoms: "Cards", explanations: "Answers" },
  paper: { prequestions: "Predict", reading: "Read", recall: "Evidence", summary: "Critique", missed: "Gaps", atoms: "Cards", explanations: "Answers" },
}

export type PreAnswer = { answer: string; confidence: Confidence; correct: boolean | null; explanation: string | null }
export type Failure = { stage: "session" | "prepare"; service: ServiceName; message: string; offline: boolean }

export type FlowState = {
  mode: Mode
  phase: "opening" | "preparing" | "failed" | "step" | "done"
  step: Step
  sessionId: string | null
  section: SectionRef | null
  prep: Prepared | null
  answers: Record<string, PreAnswer>
  recall: { text: string } | "skipped" | null
  summary: { text: string; judgement: Confidence; result: SummaryResult } | null
  atoms: { accepted: (string | number)[]; entryIds: (string | number)[] } | null
  /** A request for the current step is in flight. */
  busy: boolean
  /** The last request for this step failed; the learner's input is still on screen to resend. */
  error: string | null
  failure: Failure | null
}

export type FlowAction =
  | { type: "sessionOpened"; sessionId: string; section: SectionRef }
  | { type: "prepared"; prep: Prepared }
  | { type: "failed"; failure: Failure }
  | { type: "retry" }
  | { type: "busy" }
  | { type: "stepFailed"; message: string }
  | { type: "answered"; id: string | number; answer: string; confidence: Confidence; result: AnswerResult }
  | { type: "recalled"; text: string }
  | { type: "recallSkipped" }
  | { type: "summarized"; text: string; judgement: Confidence; result: SummaryResult }
  | { type: "atomsDecided"; accepted: (string | number)[]; entryIds: (string | number)[] }
  | { type: "advance" }

export const initFlow = (mode: Mode): FlowState => ({
  mode,
  phase: "opening",
  step: "prequestions",
  sessionId: null,
  section: null,
  prep: null,
  answers: {},
  recall: null,
  summary: null,
  atoms: null,
  busy: false,
  error: null,
  failure: null,
})

/** The first prequestion without an answer. */
export const nextQuestion = (s: FlowState): Prequestion | null => s.prep?.prequestions.find((q) => !(String(q.id) in s.answers)) ?? null

/** Confidence 3-4 on a wrong answer: the case the tutor sends straight back to study (PEDAGOGY). */
export const highConfidenceMiss = (a: PreAnswer): boolean => a.correct === false && a.confidence >= 3

export function canAdvance(s: FlowState): boolean {
  if (s.phase !== "step") return false
  switch (s.step) {
    case "prequestions":
      return nextQuestion(s) === null
    case "recall":
      return s.recall !== null
    case "summary":
      return s.summary !== null
    case "atoms":
      return s.atoms !== null
    default:
      return true
  }
}

const hasStep = (s: FlowState, step: Step): boolean => {
  if (step === "prequestions" || step === "explanations") return (s.prep?.prequestions.length ?? 1) > 0
  if (step === "atoms") return (s.prep?.atoms.length ?? 1) > 0
  return true
}

/** The steps this section will have: a section with no prequestions or no candidate atoms skips them. */
export const visibleSteps = (s: FlowState): Step[] => STEPS.filter((step) => hasStep(s, step))

function advance(s: FlowState): FlowState {
  if (!canAdvance(s)) return s
  const after = visibleSteps(s).filter((step) => STEPS.indexOf(step) > STEPS.indexOf(s.step))[0]
  return after ? { ...s, step: after, busy: false, error: null } : { ...s, phase: "done", busy: false, error: null }
}

const settled = { busy: false, error: null }

export function flowReducer(s: FlowState, a: FlowAction): FlowState {
  const inStep = (step: Step) => s.phase === "step" && s.step === step
  switch (a.type) {
    case "sessionOpened":
      return s.phase === "opening" ? { ...s, phase: "preparing", sessionId: a.sessionId, section: a.section, failure: null } : s
    case "prepared": {
      if (s.phase !== "preparing") return s
      const asked = a.prep.prequestions.length > 0
      return { ...s, phase: "step", step: asked ? "prequestions" : "reading", prep: a.prep, ...settled }
    }
    case "failed":
      return { ...s, phase: "failed", failure: a.failure, ...settled }
    case "retry":
      if (s.phase !== "failed") return s
      return { ...s, phase: s.failure?.stage === "prepare" && s.sessionId ? "preparing" : "opening", sessionId: s.failure?.stage === "prepare" ? s.sessionId : null, failure: null }
    case "busy":
      return { ...s, busy: true, error: null }
    case "stepFailed":
      return { ...s, busy: false, error: a.message }
    case "answered": {
      const key = String(a.id)
      if (!inStep("prequestions") || key in s.answers) return s
      const answer: PreAnswer = { answer: a.answer, confidence: a.confidence, correct: a.result.correct, explanation: a.result.explanation ?? null }
      return { ...s, answers: { ...s.answers, [key]: answer }, ...settled }
    }
    case "recalled":
      return inStep("recall") ? { ...s, recall: { text: a.text }, ...settled } : s
    case "recallSkipped":
      return inStep("recall") ? { ...s, recall: "skipped", ...settled } : s
    case "summarized":
      return inStep("summary") ? { ...s, summary: { text: a.text, judgement: a.judgement, result: a.result }, ...settled } : s
    case "atomsDecided":
      return inStep("atoms") ? { ...s, atoms: { accepted: a.accepted, entryIds: a.entryIds }, ...settled } : s
    case "advance":
      return advance(s)
  }
}
