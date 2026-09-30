// Minimal-pair lists for the Listen trainer (HVPT, LANGUAGES.md item 6). One list per contrast.
//
// EVERY LIST IS UNVERIFIED. The pairs were written from general knowledge of each language and have
// not been checked by a native speaker, so `verified` is false and the screen says so. Some German
// pairs are near-minimal (a plural against its singular), and the TTS voices have not been checked
// on any of them. Flip `verified` to true for a contrast only after a native speaker has gone
// through its pairs and listened to the audio.
import type { TargetLanguage } from "@/lib/language"

/** One word: `text` is what the voice says, `label` is what the choice button shows. */
export type Token = { text: string; label: string }
export type Pair = { id: string; a: Token; b: Token }
export type Contrast = {
  id: string
  lang: TargetLanguage
  title: string
  /** What differs, in a sentence. */
  note: string
  verified: boolean
  pairs: Pair[]
}

const plain = (a: string, b: string): Pair => ({ id: `${a}|${b}`, a: { text: a, label: a }, b: { text: b, label: b } })
/** A pair whose choice labels carry a reading aid (romaji or pinyin). */
const aided = (a: string, ra: string, b: string, rb: string): Pair => ({
  id: `${a}|${b}`,
  a: { text: a, label: `${a}  ${ra}` },
  b: { text: b, label: `${b}  ${rb}` },
})

export const CONTRASTS: readonly Contrast[] = [
  {
    id: "es-r-rr",
    lang: "es",
    title: "r and rr",
    note: "A single tap (pero) against a trill (perro).",
    verified: false,
    pairs: [
      plain("pero", "perro"),
      plain("caro", "carro"),
      plain("cero", "cerro"),
      plain("para", "parra"),
      plain("coro", "corro"),
      plain("ahora", "ahorra"),
      plain("pera", "perra"),
      plain("moro", "morro"),
      plain("foro", "forro"),
    ],
  },
  {
    id: "es-stress",
    lang: "es",
    title: "Stress",
    note: "The stressed syllable moves the meaning: papa (potato) against papá (dad).",
    verified: false,
    pairs: [
      plain("papa", "papá"),
      plain("hablo", "habló"),
      plain("termino", "terminó"),
      plain("esta", "está"),
      plain("canto", "cantó"),
      plain("miro", "miró"),
      plain("cambio", "cambió"),
      plain("trabajo", "trabajó"),
      plain("llamo", "llamó"),
    ],
  },
  {
    id: "fr-u-ou",
    lang: "fr",
    title: "u and ou",
    note: "The rounded front vowel in tu against the back vowel in tout.",
    verified: false,
    pairs: [
      plain("tu", "tout"),
      plain("rue", "roue"),
      plain("su", "sous"),
      plain("vu", "vous"),
      plain("lu", "loup"),
      plain("pur", "pour"),
      plain("sur", "sourd"),
      plain("nu", "nous"),
      plain("bu", "boue"),
      plain("dû", "doux"),
    ],
  },
  {
    id: "fr-nasal-an-on",
    lang: "fr",
    title: "Nasal vowels: an and on",
    note: "The nasal vowel in sans against the one in son.",
    verified: false,
    pairs: [
      plain("sans", "son"),
      plain("banc", "bon"),
      plain("temps", "ton"),
      plain("blanc", "blond"),
      plain("dans", "dont"),
      plain("vent", "vont"),
      plain("pan", "pont"),
      plain("rang", "rond"),
      plain("lent", "long"),
    ],
  },
  {
    id: "fr-nasal-on-in",
    lang: "fr",
    title: "Nasal vowels: on and in",
    note: "The nasal vowel in bon against the one in bain.",
    verified: false,
    pairs: [
      plain("bon", "bain"),
      plain("mon", "main"),
      plain("pont", "pain"),
      plain("son", "sain"),
      plain("ton", "thym"),
      plain("front", "frein"),
      plain("long", "lin"),
      plain("rond", "rein"),
    ],
  },
  {
    id: "de-u-ue",
    lang: "de",
    title: "u and ü",
    note: "The back vowel in Kur against the rounded front vowel in Kür. Some pairs are a singular and its plural.",
    verified: false,
    pairs: [
      plain("Tour", "Tür"),
      plain("Kur", "Kür"),
      plain("Gruß", "Grüße"),
      plain("Zug", "Züge"),
      plain("Hut", "Hüte"),
      plain("Stuhl", "Stühle"),
      plain("Kuh", "Kühe"),
      plain("Fuß", "Füße"),
    ],
  },
  {
    id: "de-o-oe",
    lang: "de",
    title: "o and ö",
    note: "The back vowel in schon against the rounded front vowel in schön.",
    verified: false,
    pairs: [
      plain("schon", "schön"),
      plain("mochte", "möchte"),
      plain("lose", "löse"),
      plain("Sohn", "Söhne"),
      plain("Hof", "Höfe"),
      plain("Ton", "Töne"),
      plain("Bogen", "Bögen"),
      plain("Ofen", "Öfen"),
    ],
  },
  {
    id: "de-ich-ach",
    lang: "de",
    title: "ich-Laut and ach-Laut",
    note: "The ch after a front vowel (ich) against the ch after a back vowel (ach). Most pairs are a word and its plural.",
    verified: false,
    pairs: [
      plain("Nacht", "nicht"),
      plain("Dach", "dich"),
      plain("Kuchen", "Kirchen"),
      plain("Buch", "Bücher"),
      plain("Tuch", "Tücher"),
      plain("Loch", "Löcher"),
      plain("Bach", "Bäche"),
      plain("Macht", "Mächte"),
      plain("Fach", "Fächer"),
    ],
  },
  {
    id: "ja-vowel-length",
    lang: "ja",
    title: "Vowel length",
    note: "A short vowel against a long one changes the word: obasan (aunt) against obaasan (grandmother).",
    verified: false,
    pairs: [
      aided("おばさん", "obasan", "おばあさん", "obaasan"),
      aided("おじさん", "ojisan", "おじいさん", "ojiisan"),
      aided("ゆき", "yuki", "ゆうき", "yuuki"),
      aided("とる", "toru", "とおる", "tooru"),
      aided("ビル", "biru", "ビール", "biiru"),
      aided("せき", "seki", "せいき", "seiki"),
      aided("しゅじん", "shujin", "しゅうじん", "shuujin"),
      aided("ここ", "koko", "こうこう", "koukou"),
    ],
  },
  {
    id: "ja-gemination",
    lang: "ja",
    title: "Gemination (small っ)",
    note: "A held consonant against a single one: kite against kitte.",
    verified: false,
    pairs: [
      aided("きて", "kite", "きって", "kitte"),
      aided("おと", "oto", "おっと", "otto"),
      aided("さか", "saka", "さっか", "sakka"),
      aided("いた", "ita", "いった", "itta"),
      aided("かこ", "kako", "かっこ", "kakko"),
      aided("まち", "machi", "マッチ", "macchi"),
      aided("ぶか", "buka", "ぶっか", "bukka"),
      aided("きた", "kita", "きった", "kitta"),
    ],
  },
  {
    id: "zh-tone-1-2",
    lang: "zh",
    title: "Tones 1 and 2",
    note: "High level (mā) against rising (má).",
    verified: false,
    pairs: [
      aided("妈", "mā", "麻", "má"),
      aided("八", "bā", "拔", "bá"),
      aided("书", "shū", "熟", "shú"),
      aided("衣", "yī", "姨", "yí"),
      aided("汤", "tāng", "糖", "táng"),
      aided("歌", "gē", "格", "gé"),
      aided("花", "huā", "滑", "huá"),
      aided("低", "dī", "敌", "dí"),
    ],
  },
  {
    id: "zh-tone-2-3",
    lang: "zh",
    title: "Tones 2 and 3",
    note: "Rising (yú) against dipping (yǔ).",
    verified: false,
    pairs: [
      aided("鱼", "yú", "雨", "yǔ"),
      aided("糖", "táng", "躺", "tǎng"),
      aided("房", "fáng", "访", "fǎng"),
      aided("熟", "shú", "鼠", "shǔ"),
      aided("姨", "yí", "椅", "yǐ"),
      aided("图", "tú", "土", "tǔ"),
      aided("文", "wén", "稳", "wěn"),
      aided("爷", "yé", "也", "yě"),
    ],
  },
  {
    id: "zh-tone-3-4",
    lang: "zh",
    title: "Tones 3 and 4",
    note: "Dipping (mǎi) against falling (mài).",
    verified: false,
    pairs: [
      aided("买", "mǎi", "卖", "mài"),
      aided("马", "mǎ", "骂", "mà"),
      aided("水", "shuǐ", "睡", "shuì"),
      aided("吻", "wěn", "问", "wèn"),
      aided("米", "mǐ", "密", "mì"),
      aided("椅", "yǐ", "义", "yì"),
      aided("少", "shǎo", "哨", "shào"),
      aided("五", "wǔ", "物", "wù"),
    ],
  },
]

export const contrastsFor = (lang: TargetLanguage): Contrast[] => CONTRASTS.filter((c) => c.lang === lang)
export const contrastById = (id: string): Contrast | undefined => CONTRASTS.find((c) => c.id === id)
