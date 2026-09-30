// Languages (contract C4: Screen file for the fifth subject). A language picker and four tabs: Practice
// (typed drill over cards), Listen (minimal pairs), Read (graded text) and Speak (role-play and a
// speaking check). The world accent colours the chrome only (the active tab, focus rings); the learning
// panes stay neutral. Method: docs/tutor/LANGUAGES.md.
import { useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react"
import { BookOpen, Headphones, Mic, PenLine, type LucideIcon } from "lucide-react"

import { Eyebrow } from "@/components/kit"
import { LANGUAGES, useLanguage, type TargetLanguage } from "@/lib/language"
import { useAudioEnv } from "./hooks"
import { ListenTab } from "./ListenTab"
import { Segmented } from "./parts"
import { PracticeTab } from "./PracticeTab"
import { ReadTab } from "./ReadTab"
import { SpeakTab } from "./SpeakTab"
import "./languages.css"

export type TabId = "practice" | "listen" | "read" | "speak"
const TABS: { id: TabId; label: string; icon: LucideIcon; hint: string }[] = [
  { id: "practice", label: "Practice", icon: PenLine, hint: "Type answers to your cards" },
  { id: "listen", label: "Listen", icon: Headphones, hint: "Tell sounds apart" },
  { id: "read", label: "Read", icon: BookOpen, hint: "Graded texts" },
  { id: "speak", label: "Speak", icon: Mic, hint: "Role-play and a speaking check" },
]

/** The picker and the tab row around whatever the active tab draws. */
export function LanguagesFrame({
  language,
  onLanguage,
  tab,
  onTab,
  children,
}: {
  language: TargetLanguage
  onLanguage: (l: TargetLanguage) => void
  tab: TabId
  onTab: (t: TabId) => void
  children: ReactNode
}) {
  const base = useId()
  const refs = useRef<(HTMLButtonElement | null)[]>([])

  const onKeyDown = (e: KeyboardEvent, i: number) => {
    const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0
    const to = e.key === "Home" ? 0 : e.key === "End" ? TABS.length - 1 : delta ? (i + delta + TABS.length) % TABS.length : -1
    if (to < 0) return
    e.preventDefault()
    onTab(TABS[to].id)
    refs.current[to]?.focus()
  }

  return (
    <div className="lg-root @container mx-auto flex max-w-[1120px] flex-col gap-5 px-6 py-5 lg:px-8">
      <div>
        <Eyebrow rule>Language</Eyebrow>
        <Segmented
          label="Target language"
          value={language}
          onChange={onLanguage}
          options={LANGUAGES.map((l) => ({
            id: l.id,
            lang: l.bcp47,
            hint: l.english,
            label: (
              <span className="flex items-baseline gap-2">
                <span>{l.native}</span>
                <span className="text-[12px] opacity-70 @max-2xl:hidden" lang="en">
                  {l.english}
                </span>
              </span>
            ),
          }))}
          className="mt-2.5"
          compact
        />
      </div>

      <div role="tablist" aria-label="Languages sections" className="border-hairline flex gap-6 border-b">
        {TABS.map((t, i) => (
          <button
            key={t.id}
            ref={(el) => {
              refs.current[i] = el
            }}
            id={`${base}-tab-${t.id}`}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            aria-controls={`${base}-panel-${t.id}`}
            tabIndex={tab === t.id ? 0 : -1}
            title={t.hint}
            className="lg-tab"
            onClick={() => onTab(t.id)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            <t.icon aria-hidden className="size-4" />
            {t.label}
          </button>
        ))}
      </div>

      <div role="tabpanel" id={`${base}-panel-${tab}`} aria-labelledby={`${base}-tab-${tab}`} tabIndex={0} className="min-w-0 outline-none">
        {children}
      </div>
    </div>
  )
}

export function LanguagesScreen({ initialTab = "practice" }: { initialTab?: TabId }) {
  const { language, setLanguage } = useLanguage()
  const audio = useAudioEnv()
  const [tab, setTab] = useState<TabId>(initialTab)

  return (
    <LanguagesFrame language={language} onLanguage={setLanguage} tab={tab} onTab={setTab}>
      {tab === "practice" ? <PracticeTab key={language} language={language} audio={audio} /> : null}
      {tab === "listen" ? <ListenTab key={language} language={language} audio={audio} /> : null}
      {tab === "read" ? <ReadTab key={language} language={language} audio={audio} /> : null}
      {tab === "speak" ? <SpeakTab key={language} language={language} audio={audio} /> : null}
    </LanguagesFrame>
  )
}
