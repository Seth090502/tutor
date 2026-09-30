import { useState, type ComponentType, type ReactNode } from "react"
import { AudioLines, FlaskConical, Library, ListChecks, TrendingUp } from "lucide-react"

import { cn } from "@/lib/utils"
import { PendingSection } from "@/features/tracker/PendingSection"
import { TutorScreen } from "@/features/tutor/TutorScreen"

type Section = {
  id: string
  label: string
  icon: ComponentType<{ className?: string }>
  subtitle: string
  render: () => ReactNode
}

const SECTIONS: Section[] = [
  {
    id: "tutor",
    label: "Tutor",
    icon: AudioLines,
    subtitle: "Voice teacher",
    render: () => <TutorScreen />,
  },
  {
    id: "library",
    label: "Library",
    icon: Library,
    subtitle: "Ingestion of your teaching material",
    render: () => (
      <PendingSection
        tracks={[
          "Files discovered, deduplicated, parsed, and indexed, per domain",
          "Pages triaged: born-digital, scanned, math-heavy (which need OCR)",
          "Measured tokens and pages per GB, projected cost for the full library",
          "API spend so far against the approved ingestion budget",
        ]}
        source="rag-engine survey (phase P1) and full ingestion (P4)"
      />
    ),
  },
  {
    id: "evals",
    label: "Evals",
    icon: FlaskConical,
    subtitle: "Hillclimbing, train vs held-out test",
    render: () => (
      <PendingSection
        tracks={[
          "Every round: the one change made, train and test scores with 95% CIs",
          "Keep/revert decision per round and the noise floor",
          "Per-domain retrieval breakdown (math, English, science, chemistry)",
          "Scored transcripts for reading failures, cost per eval run",
        ]}
        source="retrieval eval (P2), paid pilots (P3), voice-teacher tests (P7)"
      />
    ),
  },
  {
    id: "progress",
    label: "Progress",
    icon: TrendingUp,
    subtitle: "Your learning",
    render: () => (
      <PendingSection
        tracks={[
          "Reviews due today and upcoming (FSRS schedule)",
          "Accuracy by domain and topic, misconceptions you hit most",
          "Mastery paths and what to study next",
        ]}
        source="DeepTutor practice and mastery-path APIs (P5)"
      />
    ),
  },
  {
    id: "plan",
    label: "Plan",
    icon: ListChecks,
    subtitle: "Build phases, decisions, spend",
    render: () => (
      <PendingSection
        tracks={[
          "Phases P0-P7 with status and the check that proves each done",
          "Decisions waiting on you",
          "Total API spend by provider",
        ]}
        source="FORK_PLAN.md and the spend ledger"
      />
    ),
  },
]

export default function App() {
  const [active, setActive] = useState(SECTIONS[0].id)
  const section = SECTIONS.find((s) => s.id === active) ?? SECTIONS[0]

  return (
    <div className="bg-background text-foreground flex h-screen">
      <nav className="bg-sidebar text-sidebar-foreground flex w-56 shrink-0 flex-col gap-1 border-r p-3">
        <div className="px-3 pt-2 pb-5 text-sm font-semibold tracking-tight">Tutor</div>
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            onClick={() => setActive(s.id)}
            aria-current={s.id === active ? "page" : undefined}
            className={cn(
              "flex items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors",
              s.id === active
                ? "bg-sidebar-accent text-sidebar-accent-foreground"
                : "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground",
            )}
          >
            <s.icon className="size-4" />
            {s.label}
          </button>
        ))}
      </nav>
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="border-b px-8 py-4">
          <h1 className="text-base font-semibold">{section.label}</h1>
          <p className="text-muted-foreground text-sm">{section.subtitle}</p>
        </header>
        <div className="min-h-0 flex-1 overflow-auto">{section.render()}</div>
      </main>
    </div>
  )
}
