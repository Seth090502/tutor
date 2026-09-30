// PENDING, REMOVE AT INTEGRATION. A local stand-in for `src/lib/language.tsx` (contract C10), which
// another worker builds. It mirrors the C10 surface: `TargetLanguage`, `LANGUAGES`,
// `<LanguageProvider>` and `useLanguage()`. Switch the one import in `../platform.ts`-adjacent files
// (`import ... from "./_pending/language"`) to `@/lib/language`, then delete this folder.
import { useCallback, useSyncExternalStore, type ReactNode } from "react"

export type TargetLanguage = "es" | "fr" | "de" | "ja" | "zh"

export type LanguageMeta = {
  id: TargetLanguage
  english: string
  native: string
  script: string
  /** BCP-47 tag. */
  tag: string
  /** Scribe language code. */
  scribe: string
}

export const LANGUAGES: readonly LanguageMeta[] = [
  { id: "es", english: "Spanish", native: "Español", script: "Latin", tag: "es-ES", scribe: "spa" },
  { id: "fr", english: "French", native: "Français", script: "Latin", tag: "fr-FR", scribe: "fra" },
  { id: "de", english: "German", native: "Deutsch", script: "Latin", tag: "de-DE", scribe: "deu" },
  { id: "ja", english: "Japanese", native: "日本語", script: "Kana and kanji", tag: "ja-JP", scribe: "jpn" },
  { id: "zh", english: "Mandarin", native: "中文", script: "Hanzi", tag: "zh-CN", scribe: "zho" },
]

const KEY = "tutor.language"
const isLanguage = (v: unknown): v is TargetLanguage => LANGUAGES.some((l) => l.id === v)

function read(): TargetLanguage {
  try {
    const v = window.localStorage.getItem(KEY)
    return isLanguage(v) ? v : "es"
  } catch {
    return "es"
  }
}

let current: TargetLanguage = "es"
let loaded = false
const listeners = new Set<() => void>()

function snapshot(): TargetLanguage {
  if (!loaded) {
    current = read()
    loaded = true
  }
  return current
}

function subscribe(fn: () => void) {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}

/** The stand-in works without a provider; the real one throws outside `<LanguageProvider>`. */
export function LanguageProvider({ children }: { children: ReactNode }) {
  return <>{children}</>
}

export function useLanguage(): { language: TargetLanguage; setLanguage: (language: TargetLanguage) => void } {
  const language = useSyncExternalStore(subscribe, snapshot, () => "es" as TargetLanguage)
  const setLanguage = useCallback((next: TargetLanguage) => {
    current = next
    loaded = true
    try {
      window.localStorage.setItem(KEY, next)
    } catch {
      // Private window or blocked storage: the choice is not remembered.
    }
    for (const fn of [...listeners]) fn()
  }, [])
  return { language, setLanguage }
}

export const languageMeta = (id: TargetLanguage): LanguageMeta => LANGUAGES.find((l) => l.id === id) ?? LANGUAGES[0]
