import { renderToStaticMarkup } from "react-dom/server"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { StudySession, type StudyAttempt } from "@/lib/study"
import { AttemptPanel } from "./AttemptPanel"
import { CHOICE, ESSAY, type FakeItem, type FakeStudy, SECRETS, TYPED, fakeStudyServer } from "./fakeStudy"
import { type PanelActions, panelActions } from "./practice"

let server: FakeStudy
let session: StudySession
let actions: PanelActions

/** A fresh server with only `item` due, a session, and the item opened from the screen. */
async function open(item: FakeItem) {
  server = fakeStudyServer([item])
  vi.stubGlobal("fetch", server.fetch)
  session = await StudySession.start("practice")
  actions = panelActions(session)
  expect(await actions.next()).not.toBeNull()
}

const html = (attempt: StudyAttempt | null = session.state.attempt) =>
  renderToStaticMarkup(<AttemptPanel attempt={attempt} actions={actions} />)
/** What the learner reads: tags stripped, entities decoded. */
const text = (markup: string) =>
  markup
    .replace(/<[^>]+>/g, " ")
    .replace(/&#x27;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ")

/** Nothing the server has not released, and no hint the learner has not asked for. */
function expectNothingUnreleased(markup: string, unaskedHints: RegExp) {
  expect(markup).not.toMatch(SECRETS)
  expect(markup).not.toMatch(unaskedHints)
  expect(markup).not.toContain('data-status="correct"')
}

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {})
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe("AttemptPanel", () => {
  it("shows an empty state before any question, with a practice notice when there is one", () => {
    const markup = renderToStaticMarkup(<AttemptPanel attempt={null} notice="Could not resume practice." />)
    expect(text(markup)).toContain("No question yet")
    expect(text(markup)).toContain("Could not resume practice.")
  })

  it("open, choice: the question, the A-D keys, I don't know and a hint that says it costs the unaided count", async () => {
    await open(CHOICE)
    const markup = html()
    const t = text(markup)

    expect(t).toContain(CHOICE.prompt)
    for (const option of Object.values(CHOICE.options)) expect(t).toContain(option)
    expect(t).toContain("I don't know")
    expect(t).toContain("Hint")
    expect(t).toContain("A hint before you answer means this question will not count as unaided.")
    expect(t).not.toContain("How sure are you?")
    expect(markup).not.toContain(">Check</button>")
    expectNothingUnreleased(markup, /HINT-|Think about/)
  })

  it("open, typed: a text field for the answer", async () => {
    await open(TYPED)
    const markup = html()

    expect(markup).toContain('aria-label="Your answer"')
    expect(markup).toContain(">Answer</button>")
    expectNothingUnreleased(markup, /HINT-ONE-HERON/)
  })

  it("a hint before answering shows its rung and says plainly the item will not count as unaided", async () => {
    await open(CHOICE)
    await actions.hint()
    const t = text(html())

    expect(t).toContain(CHOICE.hints[0])
    expect(t).toContain("Rung 1 of 4, nudge")
    expect(t).toContain("You had help before answering, so this question will not count as unaided.")
    expectNothingUnreleased(html(), /HINT-TWO-IBIS/)
  })

  it("a pending proposal: what the tutor heard, an editable field and Confirm", async () => {
    await open(TYPED)
    await session.propose("key zebra forty two")
    const markup = html()

    expect(text(markup)).toContain("Tutor heard: key zebra forty two")
    expect(markup).toContain('value="key zebra forty two"')
    expect(markup).toContain(">Confirm</button>")
    expectNothingUnreleased(markup, /HINT-ONE-HERON/)
  })

  it("committed: confidence 1-4 and skip reasons, nothing else to do yet", async () => {
    await open(CHOICE)
    await actions.answer("A")
    const markup = html()
    const t = text(markup)

    expect(t).toContain("How sure are you?")
    for (const word of ["guess", "unsure", "fairly sure", "certain"]) expect(t).toContain(word)
    expect(t).toContain("Skip, because:")
    expect(t).toContain("can't tell")
    expect(markup).not.toContain(">Check</button>")
    expectNothingUnreleased(markup, /HINT-|Think about/)
  })

  it("ready: the confidence given and a Check button", async () => {
    await open(CHOICE)
    await actions.answer("A")
    await actions.confidence(3)
    const markup = html()

    expect(text(markup)).toContain("Confidence · fairly sure")
    expect(markup).toContain(">Check</button>")
    expect(text(markup)).not.toContain("How sure are you?")
    expectNothingUnreleased(markup, /HINT-|Think about/)
  })

  it("remediating: the verdict and diagnostic, a hint, a retry field and Give up, with the answer still hidden", async () => {
    await open(CHOICE)
    await actions.answer("A")
    await actions.confidence(3)
    await actions.check()
    const markup = html()
    const t = text(markup)

    expect(session.state.attempt?.status).toBe("remediating")
    expect(t).toContain("Not quite")
    expect(t).toContain("Not the expected answer.")
    expect(markup).toContain('aria-label="Your new answer"')
    expect(markup).toContain(">Give up</button>")
    expect(markup).toContain(">Hint</button>")
    // The keys stay pressable for a new try: no marks before the answer is released.
    expect(markup).not.toContain("data-status")
    expectNothingUnreleased(markup, /HINT-|Think about/)
  })

  it("remediating after two missed tries shows the offer", async () => {
    await open(CHOICE)
    await actions.answer("A")
    await actions.confidence(1)
    await actions.check()
    await actions.repair("C")
    await actions.repair("A")

    expect(session.state.attempt?.offer).toBe("worked_example_then_fresh_problem")
    expect(text(html())).toContain("Two tries missed: a worked example on a parallel problem, then a fresh problem of the same kind.")
    expectNothingUnreleased(html(), /HINT-|Think about/)
  })

  it("awaiting self-assessment: the released reference answer and I got it / I missed it", async () => {
    await open(ESSAY)
    await actions.answer("Water moves to the saltier side.")
    await actions.confidence(2)
    await actions.check()
    const t = text(html())

    expect(session.state.attempt?.status).toBe("awaiting_self_assessment")
    expect(t).toContain("Reference answer")
    expect(t).toContain(ESSAY.key)
    expect(t).toContain(ESSAY.explanation)
    expect(t).toContain("I got it")
    expect(t).toContain("I missed it")
    // The learner's own answer sits beside the reference, so there is something to compare.
    expect(t).toContain("Your answer")
    expect(t).toContain("Water moves to the saltier side.")
  })

  it("closed correct: verdict, released answer, explanation, misconception, the rating in words and Next question", async () => {
    await open(CHOICE)
    await actions.answer("B")
    await actions.confidence(4)
    await actions.check()
    const markup = html()
    const t = text(markup)

    expect(t).toContain("Correct")
    expect(t).toContain("Correct answer")
    expect(t).toContain("B. Carbon dioxide")
    expect(t).toContain(CHOICE.explanation)
    expect(t).toContain(CHOICE.misconception)
    expect(t).toContain("Good, correct without help")
    expect(t).toContain("Next review")
    expect(markup).toContain(">Next question</button>")
    expect(markup).toContain('data-status="correct"')
  })

  it("closed after a hint: Again, because a hint was used", async () => {
    await open(CHOICE)
    await actions.hint()
    await actions.answer("B")
    await actions.confidence(4)
    await actions.check()

    expect(text(html())).toContain("Again, because a hint was used")
  })

  it("closed after giving up: the answer shown, marked against the learner's choice", async () => {
    await open(CHOICE)
    await actions.answer("A")
    await actions.confidence(4)
    await actions.check()
    await actions.giveUp()
    const markup = html()
    const t = text(markup)

    expect(t).toContain("Answer shown")
    expect(t).toContain("Again, because the first answer was not correct")
    expect(t).toContain(CHOICE.explanation)
    expect(markup).toContain('data-status="correct"')
    expect(markup).toContain('data-status="wrong"')
  })

  it("closed self-assessed: the learner's judgement and its rating", async () => {
    await open(ESSAY)
    await actions.answer("Water moves to the saltier side.")
    await actions.confidence(2)
    await actions.check()
    await actions.selfAssess("correct")
    const t = text(html())

    expect(t).toContain("You got it")
    expect(t).toContain("Good, by your own judgement")
  })

  it("disables every control while there is no session to send to", async () => {
    await open(CHOICE)
    const markup = renderToStaticMarkup(<AttemptPanel attempt={session.state.attempt} />)
    const controls = markup.match(/<(button|input)[^>]*>/g) ?? []
    expect(controls.length).toBeGreaterThan(4)
    // The attribute, not the `disabled:` utility classes every button carries.
    for (const c of controls) expect(c).toMatch(/\sdisabled=""/)
  })
})

describe("panelActions", () => {
  const sources = () => server.commands().map((c) => [c.body?.kind, c.source])

  it("sends every typed action as ui, with the edited text as confirmed_text", async () => {
    await open(CHOICE)
    await actions.hint()
    await session.propose("see")
    await actions.confirm("see", "C")
    await actions.skipConfidence("can't tell")
    await actions.check()
    await actions.repair("A")
    await session.propose("bee")
    await actions.confirm("bee", "B")

    expect(sources()).toEqual([
      ["start_session", "ui"],
      ["open_item", "ui"],
      ["request_assistance", "ui"],
      ["propose_response", "voice_tool"],
      ["commit_response", "ui"],
      ["record_confidence", "ui"],
      ["check", "ui"],
      ["submit_repair", "ui"],
      ["propose_response", "voice_tool"],
      ["submit_repair", "ui"],
    ])
    const payloads = server.commands().map((c) => c.body?.payload)
    expect(payloads[4]).toEqual({
      response: "C",
      input_mode: "spoken",
      transcript_confirmation: { raw_transcript: "see", confirmed_text: "C" },
    })
    expect(payloads[5]).toEqual({ level: null, skipped_reason: "can't tell" })
    expect(payloads[7]).toEqual({ response: "A", input_mode: "typed" })
    expect(payloads[9]).toEqual({
      response: "B",
      input_mode: "spoken",
      transcript_confirmation: { raw_transcript: "bee", confirmed_text: "B" },
    })
    expect(session.state.attempt?.closure?.reason).toBe("correct")
    expect(await actions.next()).toBeNull()
  })

  it("sends don't know, confidence, give up and self-assessment as ui", async () => {
    await open(TYPED)
    await actions.dontKnow()
    await actions.confidence(1)
    await actions.check()
    await actions.giveUp()
    expect(server.commands().map((c) => c.body?.payload)[2]).toEqual({ response: "", input_mode: "dont_know" })

    await open(ESSAY)
    await actions.answer("Water moves.")
    await actions.confidence(2)
    await actions.check()
    await actions.selfAssess("incorrect")
    expect(sources().every(([, source]) => source === "ui")).toBe(true)
    expect(sources().map(([kind]) => kind)).toEqual([
      "start_session",
      "open_item",
      "commit_response",
      "record_confidence",
      "check",
      "self_assess",
    ])
  })

  it("tells the caller only about accepted actions", async () => {
    await open(CHOICE)
    const accepted: string[] = []
    const told = panelActions(session, (action) => accepted.push(action))

    await expect(told.check()).rejects.toMatchObject({ code: "invalid_transition" })
    await told.answer("B")
    expect(accepted).toEqual(["answer"])
  })
})
