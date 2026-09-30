import { renderToStaticMarkup } from "react-dom/server"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { StudySession } from "@/lib/study"
import { CardView } from "./PracticeTab"
import { cardActions, openNext, openPractice, sessionKey, SKIPPED_REASON } from "./practice"
import { createStudyBackend, type FixtureCard, type StudyBackend } from "./studyFixture"

const RIVER: FixtureCard = {
  id: 101,
  lang: "es",
  kind: "production",
  question: "the river",
  answer: "el río",
  hints: ["It is a masculine noun.", "The accent falls on the second letter of the noun."],
  explanation: "Río is stressed on the i, so it is written with an accent.",
  misconception: "Río is not stressed on the o.",
}
const MORNING: FixtureCard = {
  id: 104,
  lang: "es",
  kind: "production",
  question: "Describe your morning in one sentence.",
  answer: "Me levanto a las siete y tomo un café.",
  hints: [],
  explanation: "A model answer. Yours can differ.",
  free: true,
}
const FRENCH: FixtureCard = { id: 401, lang: "fr", kind: "production", question: "the station", answer: "la gare", hints: [], explanation: "Feminine." }

const AUDIO = { loading: false, inTauri: true, hasKey: true, blocker: null }

function memoryStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial))
  return {
    data,
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
    removeItem: (k: string) => void data.delete(k),
  }
}

let storage: ReturnType<typeof memoryStorage>
beforeEach(() => {
  storage = memoryStorage()
  vi.stubGlobal("window", { localStorage: storage })
})
afterEach(() => vi.unstubAllGlobals())

const commands = (b: StudyBackend) => b.calls.filter((c) => c.path === "/api/study/commands")
const lastCommand = (b: StudyBackend) => commands(b)[commands(b).length - 1].body
const kinds = (b: StudyBackend) => commands(b).map((c) => c.body?.kind)

/** The card as the tab draws it, from the session's accepted snapshot, with `run` doing nothing. */
function markup(session: StudySession, card = RIVER) {
  return renderToStaticMarkup(
    <CardView
      attempt={session.state.attempt!}
      kind={card.kind}
      language="es"
      position="Card 1 of 1"
      tally={{ reviewed: 0, assisted: 0 }}
      pending={false}
      error={null}
      actions={cardActions(session)}
      run={() => {}}
      onNext={() => {}}
      isLast
      audio={AUDIO}
      say={async () => {}}
      stop={() => {}}
      sayStatus="idle"
      sayError={null}
    />,
  )
}

async function openCard(backend: StudyBackend, lang: "es" | "fr" = "es") {
  const { session, queue } = await openPractice(lang, backend.fetch)
  await openNext(session!, queue, [])
  return session!
}

describe("opening practice", () => {
  it("asks the server for this language's due cards and starts one practice session as the UI", async () => {
    const backend = createStudyBackend([RIVER, FRENCH, MORNING])
    const { session, queue } = await openPractice("es", backend.fetch)

    expect(backend.calls[0]).toMatchObject({ method: "GET", path: "/api/study/queue?limit=100&tag=lang%3Aes" })
    expect(queue.map((q) => q.entry_id)).toEqual([101, 104])
    expect(commands(backend)).toHaveLength(1)
    expect(commands(backend)[0].body).toMatchObject({ kind: "start_session", payload: { mode: "practice" }, expected_version: null })
    expect(commands(backend)[0].headers["x-tutor-source"]).toBe("ui")
    expect(storage.data.get(sessionKey("es"))).toBe(session!.state.session.session_id)
    expect(sessionKey("es")).toBe("tutor.study.languages.es")
  })

  it("resumes the stored session instead of starting another", async () => {
    const backend = createStudyBackend([RIVER])
    const first = await openPractice("es", backend.fetch)
    const again = await openPractice("es", backend.fetch)
    expect(again.session!.state.session.session_id).toBe(first.session!.state.session.session_id)
    expect(kinds(backend)).toEqual(["start_session"])
  })

  it("resumes an open attempt where the learner left it", async () => {
    const backend = createStudyBackend([RIVER])
    const before = await openCard(backend)
    await cardActions(before).answer("el rio")
    const resumed = (await openPractice("es", backend.fetch)).session!
    expect(resumed.state.attempt).toMatchObject({ status: "committed", entry_id: 101 })
    expect(resumed.state.attempt?.first_response?.response).toBe("el rio")
  })

  it("starts fresh when the stored session is gone", async () => {
    const backend = createStudyBackend([RIVER])
    storage.setItem(sessionKey("es"), "f".repeat(32))
    const { session } = await openPractice("es", backend.fetch)
    expect(backend.calls.some((c) => c.path === `/api/study/sessions/${"f".repeat(32)}?after_seq=0`)).toBe(true)
    expect(kinds(backend)).toEqual(["start_session"])
    expect(storage.data.get(sessionKey("es"))).toBe(session!.state.session.session_id)
  })

  it("starts no session when nothing is due", async () => {
    const backend = createStudyBackend([FRENCH])
    const { session, queue } = await openPractice("es", backend.fetch)
    expect(session).toBeNull()
    expect(queue).toEqual([])
    expect(commands(backend)).toHaveLength(0)
  })

  it("works when storage is blocked", async () => {
    const blocked = () => {
      throw new Error("blocked")
    }
    vi.stubGlobal("window", { localStorage: { getItem: blocked, setItem: blocked, removeItem: blocked } })
    const backend = createStudyBackend([RIVER])
    const { session } = await openPractice("es", backend.fetch)
    expect(session).not.toBeNull()
  })
})

describe("the card flow", () => {
  it("sends every command as the UI, and nothing for a move the status does not allow", async () => {
    const backend = createStudyBackend([RIVER])
    const session = await openCard(backend)
    const actions = cardActions(session)
    const before = commands(backend).length

    // The attempt is open: no confidence, check, repair, hint, give up or self-assessment.
    expect(actions.confidence(3)).toBeNull()
    expect(actions.skipConfidence()).toBeNull()
    expect(actions.check()).toBeNull()
    expect(actions.repair("el río")).toBeNull()
    expect(actions.hint()).toBeNull()
    expect(actions.giveUp()).toBeNull()
    expect(actions.selfAssess("correct")).toBeNull()
    expect(actions.answer("   ")).toBeNull()
    expect(commands(backend)).toHaveLength(before)

    await actions.answer("el rio")
    await actions.confidence(3)
    await actions.check()
    await actions.hint()
    await actions.repair("el río")

    expect(kinds(backend)).toEqual(["start_session", "open_item", "commit_response", "record_confidence", "check", "request_assistance", "submit_repair"])
    expect(commands(backend).every((c) => c.headers["x-tutor-source"] === "ui")).toBe(true)
  })

  it("records the confidence only after the answer is committed", async () => {
    const backend = createStudyBackend([RIVER])
    const session = await openCard(backend)
    const actions = cardActions(session)
    expect(actions.confidence(4)).toBeNull()
    await actions.answer("el rio")
    expect(actions.answer("another")).toBeNull() // sealed
    await actions.confidence(4)
    expect(commands(backend).map((c) => c.body?.kind)).toEqual(["start_session", "open_item", "commit_response", "record_confidence"])
    expect(commands(backend)[3].body?.payload).toEqual({ level: 4 })
    expect(commands(backend)[2].body?.payload).toEqual({ response: "el rio", input_mode: "typed" })
  })

  it("shows no confidence control before the answer is committed", async () => {
    const session = await openCard(createStudyBackend([RIVER]))
    const open = markup(session)
    expect(open).not.toContain('role="radiogroup"')
    expect(open).not.toContain("Skip")
    expect(open).toMatch(/once your answer is submitted/)

    await cardActions(session).answer("el rio")
    const committed = markup(session)
    expect(committed).toContain('role="radiogroup"')
    expect(committed).toContain("Skip")
    expect(committed).toContain("Certain")
  })

  it("never renders the answer key, the explanation or the misconception before the server releases them", async () => {
    const backend = createStudyBackend([RIVER])
    const session = await openCard(backend)
    const actions = cardActions(session)
    const secrets = [RIVER.answer, RIVER.explanation, RIVER.misconception!]
    const seen: string[] = []
    const check = (label: string) => {
      const html = markup(session)
      seen.push(label)
      for (const s of secrets) expect(html, `${label}: ${s}`).not.toContain(s)
      return html
    }

    check("open")
    await actions.answer("el rio")
    expect(check("committed")).toContain("el rio")
    await actions.confidence(3)
    check("ready")
    await actions.check()
    expect(session.state.attempt?.status).toBe("remediating")
    const missed = check("remediating")
    expect(missed).toContain("Check your accents")
    await actions.hint()
    check("hint")
    expect(markup(session)).toContain("It is a masculine noun.")
    await actions.repair("el rio")
    await actions.repair("rio")
    check("two repairs missed")
    expect(markup(session)).toContain("Give up to see the answer")
    expect(seen).toEqual(["open", "committed", "ready", "remediating", "hint", "two repairs missed"])

    await actions.giveUp()
    const revealed = markup(session)
    expect(revealed).toContain(RIVER.answer)
    expect(revealed).toContain(RIVER.explanation)
    expect(revealed).toContain("Answer shown")
    expect(revealed).toContain("Finish")
    expect(revealed).not.toContain("Two repairs have missed")
  })

  it("closes a correct repair with the rating and its provenance in words", async () => {
    const backend = createStudyBackend([RIVER])
    const session = await openCard(backend)
    const actions = cardActions(session)
    await actions.answer("el rio")
    await actions.confidence(2)
    await actions.check()
    await actions.repair("el río")

    expect(session.state.attempt).toMatchObject({ status: "closed", closure: { reason: "correct" } })
    const html = markup(session)
    expect(html).toContain("el río")
    expect(html).toContain("Correct on a repair")
    expect(html).toContain("Again")
    expect(html).toContain("The app checked your first answer and you had no help before it.")
    expect(html).toMatch(/Finish/)
  })

  it("shows a correct first answer as correct, rated Good", async () => {
    const session = await openCard(createStudyBackend([RIVER]))
    const actions = cardActions(session)
    await actions.answer("El río")
    await actions.confidence(4)
    await actions.check()
    const html = markup(session)
    expect(session.state.attempt?.status).toBe("closed")
    expect(html).toContain("Good")
    expect(html).toContain("Hear it again")
    expect(html).toContain(RIVER.explanation)
  })

  it("records a skipped confidence with a reason", async () => {
    const backend = createStudyBackend([MORNING])
    const session = await openCard(backend)
    const actions = cardActions(session)
    await actions.answer("Me levanto temprano.")
    await actions.skipConfidence()
    expect(commands(backend)[3].body?.payload).toEqual({ level: null, skipped_reason: SKIPPED_REASON })
    expect(markup(session, MORNING)).toContain("Skipped")
  })

  it("holds an unverifiable answer back until the check, then asks the learner to judge it", async () => {
    const backend = createStudyBackend([MORNING])
    const session = await openCard(backend)
    const actions = cardActions(session)
    await actions.answer("Me levanto temprano.")
    await actions.confidence(3)
    expect(markup(session, MORNING)).not.toContain(MORNING.answer)
    await actions.check()

    expect(session.state.attempt?.status).toBe("awaiting_self_assessment")
    const html = markup(session, MORNING)
    expect(html).toContain(MORNING.answer)
    expect(html).toContain("I got it")
    expect(html).toContain("I missed it")

    await actions.selfAssess("correct")
    const closed = markup(session, MORNING)
    expect(lastCommand(backend)).toMatchObject({ kind: "self_assess", payload: { judged: "correct" } })
    expect(closed).toContain("You had it")
    expect(closed).toContain("your own judgement")
  })

  it("records a don't-know without text and shows it as such", async () => {
    const backend = createStudyBackend([RIVER])
    const session = await openCard(backend)
    const actions = cardActions(session)
    await actions.dontKnow()
    expect(lastCommand(backend)?.payload).toEqual({ response: "", input_mode: "dont_know" })
    expect(markup(session)).toContain("You said you did not know.")
    await actions.confidence(1)
    await actions.check()
    expect(session.state.attempt?.status).toBe("remediating")
    expect(markup(session)).not.toContain(RIVER.answer)
  })

  it("moves on to the next card only after this one closes", async () => {
    const backend = createStudyBackend([RIVER, MORNING])
    const { session, queue } = await openPractice("es", backend.fetch)
    await openNext(session!, queue, [])
    // The first card is still open: the server refuses a second item, and the session reports it.
    await expect(openNext(session!, queue, [])).rejects.toMatchObject({ code: "invalid_transition" })
    await cardActions(session!).dontKnow()
    await cardActions(session!).confidence(1)
    await cardActions(session!).check()
    await cardActions(session!).giveUp()
    await openNext(session!, queue, [])
    expect(session!.state.attempt).toMatchObject({ entry_id: 104, status: "open" })
    expect(openNext(session!, queue, [101, 104])).toBeNull()
  })
})
