import { AudioLines, BookOpenText, Brain, FlaskConical, Library, ListChecks, Settings, TrendingUp } from "lucide-react"

import { AppShell } from "@/components/shell/AppShell"
import type { Section } from "@/components/shell/types"
import { ReadingScreen } from "@/features/reading/ReadingScreen"
import { SettingsScreen } from "@/features/settings/SettingsScreen"
import { EvalsScreen } from "@/features/tracker/EvalsScreen"
import { LibraryScreen } from "@/features/tracker/LibraryScreen"
import { PlanScreen } from "@/features/tracker/PlanScreen"
import { ProgressScreen } from "@/features/tracker/ProgressScreen"
import { TrainScreen } from "@/features/train/TrainScreen"
import { VoiceTutorScreen } from "@/features/voice/VoiceTutorScreen"

// Screen ownership: docs/tutor/CONTRACTS.md (C4).
const SECTIONS: Section[] = [
  { id: "tutor", label: "Tutor", icon: AudioLines, subtitle: "Voice teacher", Screen: VoiceTutorScreen },
  { id: "library", label: "Library", icon: Library, subtitle: "Your books and papers", Screen: LibraryScreen },
  { id: "read", label: "Read", icon: BookOpenText, subtitle: "Guided reading from your library", Screen: ReadingScreen },
  { id: "evals", label: "Evals", icon: FlaskConical, subtitle: "Hillclimbing, train vs held-out test", Screen: EvalsScreen },
  { id: "progress", label: "Progress", icon: TrendingUp, subtitle: "Your learning", Screen: ProgressScreen },
  { id: "train", label: "Train", icon: Brain, subtitle: "Dual N-Back, optional skill practice", Screen: TrainScreen },
  { id: "plan", label: "Plan", icon: ListChecks, subtitle: "Build phases, decisions, spend", Screen: PlanScreen },
]
const SETTINGS: Section = {
  id: "settings",
  label: "Settings",
  icon: Settings,
  subtitle: "Keys and services",
  Screen: SettingsScreen,
}

export default function App() {
  return <AppShell sections={SECTIONS} settings={SETTINGS} />
}
