// Target language for the Languages world (contract C10). Remembered in localStorage like the
// world, when storage is available.

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react"

export type TargetLanguage = "es" | "fr" | "de" | "ja" | "zh"

export type LanguageMeta = {
  /** ISO 639-1; also the `lang` for text to speech and the voice listing. */
  id: TargetLanguage
  english: string
  native: string
  /** ISO 15924 script code. */
  script: "Latn" | "Jpan" | "Hans"
  /** BCP-47 tag for `lang` attributes and Intl. */
  bcp47: string
  /** ISO 639-3 code, as Scribe lists its languages; the `lang` for speech to text. */
  scribe: string
}

export const LANGUAGES: readonly LanguageMeta[] = [
  { id: "es", english: "Spanish", native: "Español", script: "Latn", bcp47: "es", scribe: "spa" },
  { id: "fr", english: "French", native: "Français", script: "Latn", bcp47: "fr", scribe: "fra" },
  { id: "de", english: "German", native: "Deutsch", script: "Latn", bcp47: "de", scribe: "deu" },
  { id: "ja", english: "Japanese", native: "日本語", script: "Jpan", bcp47: "ja", scribe: "jpn" },
  { id: "zh", english: "Mandarin Chinese", native: "中文", script: "Hans", bcp47: "zh-Hans", scribe: "zho" },
]

export const LANGUAGE_STORAGE_KEY = "tutor.language"

const DEFAULT_LANGUAGE: TargetLanguage = "es"

export function isTargetLanguage(value: unknown): value is TargetLanguage {
  return LANGUAGES.some((l) => l.id === value)
}

export function languageMeta(language: TargetLanguage): LanguageMeta {
  return LANGUAGES.find((l) => l.id === language) ?? LANGUAGES[0]
}

function defaultStorage(): Storage | null {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

/** The remembered language, or Spanish when nothing valid is stored or storage is unavailable. */
export function readStoredLanguage(storage: Pick<Storage, "getItem"> | null = defaultStorage()): TargetLanguage {
  try {
    const value = storage?.getItem(LANGUAGE_STORAGE_KEY)
    return isTargetLanguage(value) ? value : DEFAULT_LANGUAGE
  } catch {
    return DEFAULT_LANGUAGE
  }
}

export function writeStoredLanguage(
  language: TargetLanguage,
  storage: Pick<Storage, "setItem"> | null = defaultStorage(),
) {
  try {
    storage?.setItem(LANGUAGE_STORAGE_KEY, language)
  } catch {
    // Private windows and blocked site data: the choice just is not remembered.
  }
}

type LanguageContextValue = { language: TargetLanguage; setLanguage: (language: TargetLanguage) => void }

const LanguageContext = createContext<LanguageContextValue | null>(null)

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<TargetLanguage>(() => readStoredLanguage())

  const setLanguage = useCallback((next: TargetLanguage) => {
    setLanguageState(next)
    writeStoredLanguage(next)
  }, [])

  const value = useMemo(() => ({ language, setLanguage }), [language, setLanguage])
  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>
}

export function useLanguage(): LanguageContextValue {
  const value = useContext(LanguageContext)
  if (!value) throw new Error("useLanguage must be used inside <LanguageProvider>")
  return value
}
