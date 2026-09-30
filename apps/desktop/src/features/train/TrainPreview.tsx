// Dev-only: every Train state inside the real app shell, with fixtures. Served at /train.html, never part
// of the production build. No audio, no storage; interactions that need them stay in the real screen.
import { StrictMode, useState, type ComponentType } from "react"
import { createRoot } from "react-dom/client"
import { Brain, Settings } from "lucide-react"

import "@/index.css"
import { AppShell } from "@/components/shell/AppShell"
import { WorldProvider, writeStoredWorld, isWorld } from "@/lib/world"
import { CalibrationPanel } from "./Calibration"
import { CheckinForm } from "./Checkin"
import { HistoryPanel, TransferChecks } from "./History"
import { SetupPanel, TransferStart, type TrainMode } from "./Setup"
import { RunStage, type RunStageProps } from "./Stage"
import { BlockSummary, SessionSummary } from "./Summary"
import { TrainScreen } from "./TrainScreen"
import { DEFAULT_SETTINGS, REST_MS } from "./engine/session"
import { ruleText } from "./engine/adapt"
import { streamLabels } from "./parts"
import type { Block, Metrics, NBackSession, NBackSummary, Settings as TrainSettings } from "./types"
import type { Calibration } from "./storage"

// ---- fixtures -------------------------------------------------------------------------------------------

const m = (hits: number, misses: number, false_alarms: number, correct_rejections: number, d_prime: number, c: number): Metrics => ({
  hits,
  misses,
  false_alarms,
  correct_rejections,
  d_prime,
  c,
})

const POS_OK = m(5, 1, 1, 13, 2.21, 0.07)
const LET_OK = m(6, 0, 2, 12, 2.34, -0.31)
const POS_BAD = m(3, 3, 3, 11, 0.85, 0.02)
const LET_HARD = m(2, 4, 4, 10, 0.27, 0.41)

function block(index: number, n: number, next_n: number, pos: Metrics | null, letter: Metrics | null, void_reason?: string): Block {
  return { index, n, next_n, void: !!void_reason, void_reason: void_reason ?? null, trials: [], metrics: void_reason ? { position: null, letter: null } : { position: pos, letter } }
}

const PATH: [number, number][] = [[1, 2], [2, 3], [3, 3], [3, 4], [4, 3], [3, 3], [3, 4], [4, 4], [4, 3], [3, 3], [3, 4], [4, 4], [4, 4], [4, 5], [5, 5]]
const blocks: Block[] = PATH.map(([n, next], i) =>
  i === 4 ? block(i, n, n, null, null, "focus lost") : block(i, n, next, next < n ? POS_BAD : POS_OK, next < n ? LET_HARD : LET_OK),
)
const NOW = Date.now() / 1000
const record = (over: Partial<NBackSession> = {}): NBackSession => ({
  id: "fixture",
  started_at: NOW - 18 * 60,
  ended_at: NOW,
  mode: "dual",
  seed: 7,
  settings: DEFAULT_SETTINGS,
  blocks,
  comparable: true,
  timing: { output_latency: 0.021, base_latency: 0.01, calibrated: false },
  checkin: { expectation: null, sleep_hours: null, exercise: null, fatigue: null, strategy: null },
  ...over,
})

const session = (i: number, mode: string, mean_n: number, comparable = true): NBackSummary => ({
  id: `${mode}-${i}`,
  started_at: NOW - (30 - i * 2) * 86_400,
  mode,
  blocks: 15,
  void_blocks: i % 5 === 3 ? 1 : 0,
  start_n: Math.max(1, Math.floor(mean_n)),
  max_n: Math.ceil(mean_n) + 1,
  mean_n,
  d_prime: { position: 1.2 + i * 0.08, letter: mode === "position" ? null : 1.6 + i * 0.05 - (i % 3) * 0.2 },
  comparable,
})
const LEVELS = [1.3, 1.6, 2.1, 1.9, 2.4, 2.7, 2.5, 3.1, 3.0, 3.4, 3.2, 3.7]
const SESSIONS: NBackSummary[] = [
  ...LEVELS.map((l, i) => session(i, "dual", l, i !== 4)),
  ...[1.2, 1.8, 2.2].map((l, i) => session(i + 6, "position", l)),
  { ...session(3, "transfer", 2), id: "t1", start_n: 2, d_prime: { position: 1.62, letter: 1.94 } },
  { ...session(9, "transfer", 3), id: "t2", start_n: 3, d_prime: { position: 1.41, letter: 1.77 }, void_blocks: 1 },
].reverse()

const CALIBRATION: Calibration = {
  at: NOW - 3 * 86_400,
  output_latency: 0.021,
  base_latency: 0.01,
  mean_offset_ms: 3.4,
  max_offset_ms: 8.1,
  in_sync: true,
}

// ---- states ---------------------------------------------------------------------------------------------

const noop = () => {}
const stage = (over: Partial<RunStageProps>): RunStageProps => ({
  mode: "dual",
  n: 3,
  blockNo: 7,
  blocks: 15,
  phase: "block",
  trial: 11,
  trials: 23,
  lit: 2,
  keys: DEFAULT_SETTINGS.keys,
  pressed: { position: false, letter: false },
  marks: { position: null, letter: null },
  onPress: noop,
  onStop: noop,
  restLeft: 6,
  restTotal: Math.ceil(REST_MS / 1000),
  rule: ruleText(DEFAULT_SETTINGS),
  ...over,
})

function Home({ history }: { history: boolean }) {
  const [mode, setMode] = useState<TrainMode>("dual")
  const [settings, setSettings] = useState<TrainSettings>(DEFAULT_SETTINGS)
  const [n, setN] = useState(3)
  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-5 px-6 py-6">
      <SetupPanel
        mode={mode}
        setMode={setMode}
        settings={settings}
        setSettings={setSettings}
        startN={n}
        setStartN={setN}
        lastLevel={history ? 3 : null}
        week={history ? { minutes: 38, sessions: 2 } : null}
        starting={false}
        error={null}
        onStart={noop}
      />
      <CalibrationPanel calibration={history ? CALIBRATION : null} onChange={noop} />
      {history ? (
        <HistoryPanel sessions={SESSIONS} error={null} exported={null} onExport={noop} />
      ) : (
        <p className="text-ink-2 border-hairline-strong rounded-2xl border border-dashed p-5 text-sm leading-relaxed">
          History, saving and CSV export need the desktop app. Training works here, but sessions are not stored.
        </p>
      )}
      <TransferChecks checks={SESSIONS.filter((s) => s.mode === "transfer")} days={history ? 26 : null} available={history}>
        <TransferStart level={3} detail={history ? "Last check 26 days ago; next in 2 days." : "Take the first one when you are ready."} starting={false} onStart={noop} />
      </TransferChecks>
    </div>
  )
}

function Summary({ checkedIn = false }: { checkedIn?: boolean }) {
  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-5 px-6 py-6">
      <SessionSummary record={record()} />
      <div className="px-1">
        <span className="text-ink-2 text-sm">Saving needs the desktop app in a real run; shown here as saved.</span>
      </div>
      {!checkedIn && <CheckinForm onSubmit={noop} saving={false} />}
    </div>
  )
}

const restSummary = (b: Block, transfer = false) => (
  <BlockSummary block={b} labels={streamLabels(transfer)} transfer={transfer} restLeft={6} settings={DEFAULT_SETTINGS} />
)

const VIEWS: Record<string, ComponentType> = {
  setup: () => <Home history={false} />,
  home: () => <Home history />,
  run: () => <RunStage {...stage({ pressed: { position: true, letter: false } })} />,
  "run-idle": () => <RunStage {...stage({ lit: null })} />,
  "run-letter": () => <RunStage {...stage({ mode: "letter", lit: null, pressed: { position: false, letter: true } })} />,
  "run-feedback": () => (
    <RunStage {...stage({ pressed: { position: true, letter: false }, marks: { position: true, letter: false } })} />
  ),
  rest: () => <RunStage {...stage({ phase: "rest", blockNo: 5, summary: restSummary(blocks[3]) })} />,
  "rest-down": () => <RunStage {...stage({ phase: "rest", blockNo: 10, summary: restSummary(blocks[8]) })} />,
  "rest-void": () => <RunStage {...stage({ phase: "rest", blockNo: 6, summary: restSummary(blocks[4]) })} />,
  summary: () => <Summary />,
  history: () => <Home history />,
  transfer: () =>
    <RunStage {...stage({ mode: "transfer", n: 2, lit: 4, trials: 22, pressed: { position: false, letter: true } })} />,
  calibration: () => (
    <div className="mx-auto flex max-w-4xl flex-col gap-5 px-6 py-6">
      <CalibrationPanel calibration={null} onChange={noop} />
      <CalibrationPanel calibration={CALIBRATION} onChange={noop} />
    </div>
  ),
  real: TrainScreen,
}

export function mountTrainPreview(el: HTMLElement | null) {
  if (!el) return
  const q = new URLSearchParams(window.location.search)
  const theme = q.get("theme")
  document.documentElement.classList.toggle("dark", theme ? theme === "dark" : window.matchMedia("(prefers-color-scheme: dark)").matches)
  document.documentElement.dataset.quality = "lite"
  const world = q.get("world")
  if (isWorld(world)) writeStoredWorld(world)
  const View = VIEWS[q.get("view") ?? "setup"] ?? VIEWS.setup
  const sections = [{ id: "train", label: "Train", icon: Brain, subtitle: "Dual N-Back, optional skill practice", Screen: View }]
  const settings = { id: "settings", label: "Settings", icon: Settings, subtitle: "Keys and services", Screen: () => null }
  createRoot(el).render(
    <StrictMode>
      <WorldProvider>
        <AppShell sections={sections} settings={settings} />
      </WorldProvider>
    </StrictMode>,
  )
}
