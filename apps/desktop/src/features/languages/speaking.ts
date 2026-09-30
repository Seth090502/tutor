// The speaking check: the learner records one sentence, Scribe transcribes it with the language pinned,
// and the screen shows what the recogniser heard against the target.
//
// This is an intelligibility probe, never a pronunciation score (LANGUAGES.md item 8). A recogniser
// can quietly repair slips, so a match can flatter the learner and a miss can be the machine's, and
// the screen says both. There is deliberately no percentage or grade here.
import type { TargetLanguage } from "./_pending/language"

export const SPEAKING_TARGETS: Record<TargetLanguage, string[]> = {
  es: ["¿Dónde está la estación de tren?", "Quisiera un café con leche, por favor.", "Me llamo Ana y vivo en Madrid.", "Ayer fuimos al mercado."],
  fr: ["Où est la gare, s'il vous plaît ?", "Je voudrais un café au lait.", "Je m'appelle Marc et j'habite à Lyon.", "Hier, nous sommes allés au marché."],
  de: ["Wo ist der Bahnhof, bitte?", "Ich hätte gern einen Milchkaffee.", "Ich heiße Anna und wohne in Berlin.", "Gestern sind wir zum Markt gegangen."],
  ja: ["駅はどこですか。", "コーヒーをお願いします。", "私の名前は田中です。", "昨日、市場に行きました。"],
  zh: ["火车站在哪里？", "我想要一杯咖啡。", "我叫王明，住在北京。", "昨天我们去了市场。"],
}

/** Comparison units: words for spaced scripts, characters for Japanese and Mandarin. */
export function units(text: string, lang: TargetLanguage): string[] {
  const clean = text
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}\s'’-]+/gu, " ")
  if (lang === "ja" || lang === "zh") return Array.from(clean.replace(/\s+/g, ""))
  return clean
    .split(/\s+/)
    .map((w) => w.replace(/^['’-]+|['’-]+$/g, ""))
    .filter(Boolean)
}

export type TargetUnit = { text: string; heard: boolean }

export type Comparison = {
  /** The target's units in order, each marked heard or not. */
  target: TargetUnit[]
  /** Units the recogniser produced that are not in the target. */
  extra: string[]
  total: number
  matched: number
  /** Every target unit was heard, in order, and nothing else. */
  exact: boolean
}

/** Aligns heard units to the target with a longest common subsequence, so order counts. */
export function compareSpoken(target: string, heard: string, lang: TargetLanguage): Comparison {
  const t = units(target, lang)
  const h = units(heard, lang)
  const dp: number[][] = Array.from({ length: t.length + 1 }, () => new Array<number>(h.length + 1).fill(0))
  for (let i = t.length - 1; i >= 0; i--)
    for (let j = h.length - 1; j >= 0; j--)
      dp[i][j] = t[i] === h[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1])

  const heardT = new Array<boolean>(t.length).fill(false)
  const usedH = new Array<boolean>(h.length).fill(false)
  let i = 0
  let j = 0
  while (i < t.length && j < h.length) {
    if (t[i] === h[j]) {
      heardT[i] = true
      usedH[j] = true
      i++
      j++
    } else if (dp[i + 1][j] >= dp[i][j + 1]) i++
    else j++
  }
  const matched = heardT.filter(Boolean).length
  return {
    target: t.map((text, k) => ({ text, heard: heardT[k] })),
    extra: h.filter((_, k) => !usedH[k]),
    total: t.length,
    matched,
    exact: t.length > 0 && matched === t.length && h.length === t.length,
  }
}

/** One factual sentence about the comparison, with no grade in it. */
export function probeSummary(c: Comparison, heard: string, lang: TargetLanguage): string {
  if (!heard.trim()) return "The recogniser heard nothing. Check the microphone and try again."
  const unit = lang === "ja" || lang === "zh" ? "character" : "word"
  if (c.exact) return `The recogniser heard every ${unit} of the target. It may also have smoothed over slips.`
  const missed = c.total - c.matched
  return `The recogniser did not match ${missed} of the ${c.total} ${unit}${c.total === 1 ? "" : "s"}. That can be your pronunciation, the recording or the recogniser itself.`
}
