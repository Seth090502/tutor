// Dev-only fixtures for the preview page (preview.html): a fake backend (fetch) and a fake platform (speech,
// secrets, config), so every tab and state can be drawn and driven without services or keys. Nothing here is
// imported by the app.
import { setPlatform } from "./platform"
import { createStudyBackend, type FixtureCard } from "./studyFixture"

export type Scenario =
  | "ready"
  | "notauri"
  | "nokey"
  | "noagent"
  | "offline"
  | "nocards"
  | "novoices"
  | "nogloss"

export const CARDS: FixtureCard[] = [
  {
    id: 101,
    lang: "es",
    kind: "production",
    question: "the river",
    answer: "el río",
    hints: ["It is a masculine noun.", "It starts with r and has an accent on the second letter.", "Two words: the article, then the noun.", "el r_o, with the accent on i."],
    explanation: "Source: Tatoeba sentence pair, CC BY 2.0 FR. Río carries a written accent because the stress falls on the i.",
    misconception: "Río is not stressed on the o, so it is not written rio.",
  },
  {
    id: 102,
    lang: "es",
    kind: "production",
    question: "Where is the station?",
    answer: "¿Dónde está la estación?",
    hints: [],
    explanation: "Source: Tatoeba, CC BY 2.0 FR. Question words always carry an accent.",
  },
  {
    id: 103,
    lang: "es",
    kind: "recognition",
    question: "¿Cuánto cuesta este libro?",
    answer: "How much does this book cost?",
    hints: ["Cuánto asks about an amount.", "Cuesta is a form of costar."],
    explanation: "Source: Tatoeba, CC BY 2.0 FR.",
  },
  {
    id: 104,
    lang: "es",
    kind: "production",
    question: "Describe your morning in one sentence.",
    answer: "Me levanto a las siete y tomo un café.",
    hints: ["Start with a reflexive verb."],
    explanation: "A model answer. Yours can differ: compare the verb forms and the word order.",
    free: true,
  },
  { id: 201, lang: "ja", kind: "production", question: "Thank you very much.", answer: "ありがとうございます。", hints: ["Five kana, then the polite ending."], explanation: "Source: Tatoeba, CC BY 2.0 FR." },
  { id: 301, lang: "zh", kind: "production", question: "I want a cup of coffee.", answer: "我想要一杯咖啡。", hints: ["Start with the subject."], explanation: "Source: Tatoeba, CC BY 2.0 FR." },
  { id: 401, lang: "fr", kind: "production", question: "the station", answer: "la gare", hints: ["Feminine."], explanation: "Source: Tatoeba, CC BY 2.0 FR." },
  { id: 501, lang: "de", kind: "production", question: "the train station", answer: "der Bahnhof", hints: ["Masculine."], explanation: "Source: Tatoeba, CC BY 2.0 FR." },
]

const TEXTS: Record<string, { text: string; sentences: string[]; coverage: number; unknown: string[]; questions: { q: string; a: string }[] }> = {
  es: {
    text: "",
    sentences: [
      "Todos los sábados, Marta va al mercado con su abuelo.",
      "Compran tomates, pan y un poco de queso.",
      "El vendedor siempre les regala una manzana.",
      "Después, se sientan en un banco y comen juntos.",
      "A Marta le gusta escuchar las historias de su abuelo.",
    ],
    coverage: 0.962,
    unknown: ["vendedor", "regala", "banco"],
    questions: [
      { q: "¿Con quién va Marta al mercado?", a: "Va con su abuelo." },
      { q: "¿Qué les regala el vendedor?", a: "Les regala una manzana." },
      { q: "Where do they eat afterwards?", a: "On a bench." },
    ],
  },
  fr: {
    text: "",
    sentences: ["Chaque samedi, Marc va au marché avec sa grand-mère.", "Ils achètent des tomates et du pain."],
    coverage: 0.97,
    unknown: ["marché"],
    questions: [{ q: "Where does Marc go on Saturdays?", a: "To the market." }],
  },
  de: {
    text: "",
    sentences: ["Jeden Samstag geht Anna mit ihrem Opa auf den Markt.", "Sie kaufen Tomaten und Brot."],
    coverage: 0.97,
    unknown: ["Markt"],
    questions: [{ q: "Where does Anna go on Saturdays?", a: "To the market." }],
  },
  ja: {
    text: "",
    sentences: ["毎週土曜日に、田中さんはおじいさんと市場に行きます。", "トマトとパンとチーズを買います。", "市場の人はいつもりんごをくれます。"],
    coverage: 0.955,
    unknown: ["市場", "くれます"],
    questions: [{ q: "田中さんは誰と市場に行きますか。", a: "おじいさんと行きます。" }],
  },
  zh: {
    text: "",
    sentences: ["每个星期六，小王和爷爷去市场。", "他们买西红柿、面包和奶酪。"],
    coverage: 0.972,
    unknown: ["市场"],
    questions: [{ q: "小王和谁去市场？", a: "和爷爷去。" }],
  },
}

const GLOSSES: Record<string, string> = {
  mercado: "market",
  abuelo: "grandfather",
  vendedor: "seller, vendor",
  regala: "gives as a gift (regalar)",
  banco: "bench; also bank",
  市场: "market",
}

/** A short sine tone as a WAV, so the audio path really plays something in the preview. */
function tone(freq: number): ArrayBuffer {
  const rate = 16000
  const n = Math.round(rate * 0.22)
  const buf = new ArrayBuffer(44 + n * 2)
  const v = new DataView(buf)
  const w = (o: number, s: string) => [...s].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)))
  w(0, "RIFF")
  v.setUint32(4, 36 + n * 2, true)
  w(8, "WAVEfmt ")
  v.setUint32(16, 16, true)
  v.setUint16(20, 1, true)
  v.setUint16(22, 1, true)
  v.setUint32(24, rate, true)
  v.setUint32(28, rate * 2, true)
  v.setUint16(32, 2, true)
  v.setUint16(34, 16, true)
  w(36, "data")
  v.setUint32(40, n * 2, true)
  for (let i = 0; i < n; i++) v.setInt16(44 + i * 2, Math.sin((2 * Math.PI * freq * i) / rate) * 1200 * Math.min(1, (n - i) / 800), true)
  return buf
}

export function installFixtures(scenario: Scenario) {
  const study = createStudyBackend(scenario === "nocards" ? [] : CARDS)
  const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } })

  window.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    const method = init?.method ?? "GET"
    const body = init?.body ? JSON.parse(String(init.body)) : {}
    await new Promise((r) => setTimeout(r, 40))
    if (scenario === "offline") throw new TypeError("Failed to fetch")
    const path = url.replace("http://127.0.0.1:8001", "")

    if (path.startsWith("/api/study/")) return study.fetch(input, init)
    if (path === "/api/languages/graded-text") {
      await new Promise((r) => setTimeout(r, 250))
      return json({ ...(TEXTS[body.lang] ?? TEXTS.es), text: (TEXTS[body.lang] ?? TEXTS.es).sentences.join(" ") })
    }
    if (path === "/api/languages/cards") return json({ created: 3, entry_ids: [901, 902, 903] })
    return json({ detail: `no fixture for ${method} ${path}` }, 404)
  }) as typeof fetch

  const voices = ["Lucía", "Mateo", "Sofía", "Diego", "Carmen", "Pablo"].map((name, i) => ({
    voice_id: `v${i + 1}`,
    name,
    labels: { accent: i % 2 ? "latin american" : "castilian", gender: i % 2 ? "male" : "female", age: "adult" },
    preview_url: null,
  }))

  setPlatform({
    inTauri: scenario !== "notauri",
    hasElevenLabsKey: async () => scenario !== "nokey",
    configValue: async (key) => {
      if (key.startsWith("agent_")) return scenario === "noagent" ? null : "agent_fixture"
      if (key.startsWith("voice_")) return scenario === "novoices" ? "" : "v1,v2,v3,v4"
      return null
    },
    configSet: async () => {},
    ttsAudio: async (text) => tone(300 + (text.length % 7) * 60),
    ttsVoices: async () => voices,
    sttTranscribe: async () => ({ text: "quisiera un café leche por favor", words: [] }),
    conversationToken: async () => "fixture-token",
    gloss: async (_lang, word) => (scenario === "nogloss" ? null : (GLOSSES[word.toLowerCase()] ?? null)),
  })
}
