// Dev-only: the Languages screen inside the real app shell, with fixtures (fixtures.ts). Served at
// /src/features/languages/preview.html, never part of the production build. The live call and the digest need
// a real ElevenLabs session, so `view=live` and `view=digest` draw their presentational components with
// sample data instead.
import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { Languages, Settings } from "lucide-react"

import "@/index.css"
import "./preview.css"
import { AppShell } from "@/components/shell/AppShell"
import { WorldProvider } from "@/lib/world"
import { LANGUAGES, useLanguage, type TargetLanguage } from "@/lib/language"
import type { Gloss, Line } from "./agent"
import { buildDigest } from "./digest"
import { installFixtures, type Scenario } from "./fixtures"
import { LanguagesFrame, LanguagesScreen, type TabId } from "./LanguagesScreen"
import { DigestView, LiveView } from "./SpeakView"

const LINES: Line[] = [
  { role: "partner", text: "¡Hola! Empecemos. ¿Cómo estás hoy?" },
  { role: "learner", text: "hola estoy bien gracias" },
  { role: "partner", text: "¡Qué bien! Estás en un café. ¿Qué quieres tomar?" },
  { role: "learner", text: "yo quiero un café con leche y una tostada" },
  { role: "partner", text: "Muy bien. ¿Quieres la tostada con tomate o con mermelada?" },
  { role: "learner", text: "con tomate yo tiene hambre" },
  { role: "partner", text: "Perfecto, un café con leche y una tostada con tomate. ¿Algo más?" },
]
const GLOSSES: Gloss[] = [
  { kind: "gloss", forText: "¿Quieres la tostada con tomate o con mermelada?", english: "Do you want the toast with tomato or with jam?" },
]
const ERRORS = [
  { said: "yo tiene hambre", corrected: "yo tengo hambre", english: "I am hungry", rule: "Tener is irregular in the yo form: tengo.", breaksComprehension: false },
  { said: "yo quiero un café", corrected: "quiero un café", english: "I want a coffee", rule: "The subject pronoun is usually dropped.", breaksComprehension: false },
  { said: "con tomate yo tiene hambre", corrected: "yo tengo hambre", english: "I am hungry", rule: "", breaksComprehension: false },
  { said: "la tostada es grande muy", corrected: "la tostada es muy grande", english: "the toast is very big", rule: "Muy comes before the adjective.", breaksComprehension: true },
  { said: "hablo espanol", corrected: "hablo español", english: "I speak Spanish", rule: "", breaksComprehension: false },
]

function Root({ tab, view }: { tab: TabId; view: string | null }) {
  const { language, setLanguage } = useLanguage()
  if (view === "live" || view === "live-nudge" || view === "connecting") {
    return (
      <LanguagesFrame language={language} onLanguage={setLanguage} tab="speak" onTab={() => {}}>
        <LiveView
          language={language}
          mode="fluency"
          band="A2"
          scenario="Ordering at a café"
          status={view === "connecting" ? "connecting" : "speaking"}
          lines={view === "connecting" ? [] : LINES}
          glosses={view === "live" ? [...GLOSSES, { kind: "rule", forText: "tener", english: "Tener is irregular in the yo form: tengo, not tiene." }] : []}
          nudge={view === "live-nudge"}
          onHelp={() => {}}
          onEnd={() => {}}
        />
      </LanguagesFrame>
    )
  }
  if (view === "digest" || view === "digest-empty") {
    return (
      <LanguagesFrame language={language} onLanguage={setLanguage} tab="speak" onTab={() => {}}>
        <DigestView
          language={language}
          digest={{
            items: view === "digest" ? buildDigest(ERRORS) : [],
            mode: "fluency",
            band: "A2",
            scenario: "Ordering at a café",
            lines: LINES,
            helpRequests: 1,
            rules: 1,
            errorsLogged: view === "digest" ? ERRORS.length : 0,
          }}
          onAddCards={async (items) => items.length}
          onAgain={() => {}}
        />
      </LanguagesFrame>
    )
  }
  return <LanguagesScreen initialTab={tab} />
}

export function mountLanguagesPreview(el: HTMLElement | null) {
  if (!el) return
  const q = new URLSearchParams(window.location.search)
  const theme = q.get("theme")
  document.documentElement.classList.toggle("dark", theme ? theme === "dark" : window.matchMedia("(prefers-color-scheme: dark)").matches)
  document.documentElement.dataset.quality = "lite"
  const lang = q.get("lang")
  try {
    window.localStorage.setItem("tutor.language", LANGUAGES.some((l) => l.id === lang) ? (lang as TargetLanguage) : "es")
  } catch {
    // Storage blocked: the default language is used.
  }
  const scenario = (q.get("scenario") ?? "ready") as Scenario
  installFixtures(scenario)
  const tab = (q.get("tab") ?? "practice") as TabId
  const view = q.get("view")
  const sections = [{ id: "languages", label: "Languages", icon: Languages, subtitle: "Practice, listen, read and speak", Screen: () => <Root tab={tab} view={view} /> }]
  const settings = { id: "settings", label: "Settings", icon: Settings, subtitle: "Keys and services", Screen: () => null }
  createRoot(el).render(
    <StrictMode>
      <WorldProvider>
        <AppShell sections={sections} settings={settings} />
      </WorldProvider>
    </StrictMode>,
  )
  // The languages world is not in index.css yet (contract C10); preview.css supplies its accent.
  window.setTimeout(() => {
    document.documentElement.dataset.world = "languages"
  }, 30)
}
