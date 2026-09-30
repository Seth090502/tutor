import { useState, type ComponentType } from "react"
import { AudioLines, FlaskConical, Library, ListChecks, Settings, TrendingUp } from "lucide-react"

import { cn } from "@/lib/utils"
import { SettingsScreen } from "@/features/settings/SettingsScreen"
import { EvalsScreen } from "@/features/tracker/EvalsScreen"
import { LibraryScreen } from "@/features/tracker/LibraryScreen"
import { PlanScreen } from "@/features/tracker/PlanScreen"
import { ProgressScreen } from "@/features/tracker/ProgressScreen"
import { VoiceTutorScreen } from "@/features/voice/VoiceTutorScreen"

type Section = {
  id: string
  label: string
  icon: ComponentType<{ className?: string }>
  subtitle: string
  Screen: ComponentType
}

// Screen ownership: docs/tutor/CONTRACTS.md (C4).
const SECTIONS: Section[] = [
  { id: "tutor", label: "Tutor", icon: AudioLines, subtitle: "Voice teacher", Screen: VoiceTutorScreen },
  { id: "library", label: "Library", icon: Library, subtitle: "Your books and papers", Screen: LibraryScreen },
  { id: "evals", label: "Evals", icon: FlaskConical, subtitle: "Hillclimbing, train vs held-out test", Screen: EvalsScreen },
  { id: "progress", label: "Progress", icon: TrendingUp, subtitle: "Your learning", Screen: ProgressScreen },
  { id: "plan", label: "Plan", icon: ListChecks, subtitle: "Build phases, decisions, spend", Screen: PlanScreen },
]
const SETTINGS: Section = {
  id: "settings",
  label: "Settings",
  icon: Settings,
  subtitle: "Keys and services",
  Screen: SettingsScreen,
}

function NavButton({ s, active, onSelect }: { s: Section; active: boolean; onSelect: () => void }) {
  return (
    <button
      onClick={onSelect}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors",
        active
          ? "bg-sidebar-accent text-sidebar-accent-foreground"
          : "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground",
      )}
    >
      <s.icon className="size-4" />
      {s.label}
    </button>
  )
}

export default function App() {
  const [active, setActive] = useState(SECTIONS[0].id)
  const section = [...SECTIONS, SETTINGS].find((s) => s.id === active) ?? SECTIONS[0]

  return (
    <div className="bg-background text-foreground flex h-screen">
      <nav className="bg-sidebar text-sidebar-foreground flex w-56 shrink-0 flex-col gap-1 border-r p-3">
        <div className="px-3 pt-2 pb-5 text-sm font-semibold tracking-tight">Tutor</div>
        {SECTIONS.map((s) => (
          <NavButton key={s.id} s={s} active={s.id === active} onSelect={() => setActive(s.id)} />
        ))}
        <div className="mt-auto">
          <NavButton s={SETTINGS} active={active === SETTINGS.id} onSelect={() => setActive(SETTINGS.id)} />
        </div>
      </nav>
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="border-b px-8 py-4">
          <h1 className="text-base font-semibold">{section.label}</h1>
          <p className="text-muted-foreground text-sm">{section.subtitle}</p>
        </header>
        <div className="min-h-0 flex-1 overflow-auto">
          <section.Screen />
        </div>
      </main>
    </div>
  )
}
