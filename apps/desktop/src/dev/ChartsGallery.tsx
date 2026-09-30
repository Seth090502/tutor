// Dev-only gallery for the SVG chart kit. Served by Vite at /charts.html, never part of the production build.
//   ?theme=dark|light   sets the `dark` class on <html>
//   ?world=math|english|science|chemistry|neutral   sets data-world
//   ?view=table         opens every frame in its Table view
import { StrictMode, type ReactNode } from "react"
import { createRoot } from "react-dom/client"

import "@/index.css"
import {
  AreaChart,
  ChartFrame,
  Gauge,
  LineChart,
  PhaseDiscs,
  RangeBars,
  Sparkline,
  StackedBar,
  Sunburst,
  lineTable,
  phaseTable,
  rangeTable,
  stackTable,
  sunburstTable,
  type PhaseDay,
  type RangeRow,
  type Series,
  type StackSegment,
  type SunburstDatum,
} from "@/components/charts"

const params = new URLSearchParams(window.location.search)
const theme = params.get("theme") === "dark" ? "dark" : "light"
const world = params.get("world") ?? "neutral"
const defaultView = params.get("view") === "table" ? "table" : "chart"
document.documentElement.classList.toggle("dark", theme === "dark")
document.documentElement.dataset.world = world
document.documentElement.dataset.quality = "lite"

// ---------------------------------------------------------------------------
// Sample data (fixed, so screenshots are comparable run to run)
// ---------------------------------------------------------------------------

const RUNS = 14
const band = (vs: number[], w0: number, w1: number) =>
  vs.map((v, i) => {
    const w = w0 + ((w1 - w0) * i) / (vs.length - 1)
    return { lo: Math.max(0, +(v - w).toFixed(3)), hi: Math.min(1, +(v + w).toFixed(3)) }
  })
const train = [0.52, 0.58, 0.61, 0.66, 0.64, 0.7, 0.73, 0.77, 0.75, 0.8, 0.82, 0.84, 0.85, 0.87]
const test = [0.49, 0.53, 0.57, 0.6, 0.58, 0.63, 0.66, 0.69, 0.66, 0.7, 0.72, 0.71, 0.74, 0.75]
const hit3 = [0.61, 0.66, 0.7, 0.72, 0.73, 0.76, 0.78, 0.8, 0.8, 0.82, 0.83, 0.84, 0.84, 0.86]
const reverted = new Set([4, 8, 11])
const trainBand = band(train, 0.04, 0.022)
const testBand = band(test, 0.065, 0.04)

const evalSeries: Series[] = [
  { id: "train", label: "Train", points: train.map((y, i) => ({ x: i + 1, y, ...trainBand[i] })) },
  {
    id: "test",
    label: "Held-out test",
    points: test.map((y, i) => ({
      x: i + 1,
      y,
      ...testBand[i],
      marker: i === 0 ? "diamond" : reverted.has(i) ? "cross" : "circle",
      note: i === 0 ? "baseline" : reverted.has(i) ? "reverted, test fell" : "kept",
    })),
  },
  { id: "hit3", label: "Hit@3", points: hit3.map((y, i) => ({ x: i + 1, y })) },
]

const evalRows: RangeRow[] = [
  ["recall", "Recall@3", 0.86, 0.8, 0.91, 0.77, 0.69, 0.84],
  ["mrr", "MRR", 0.74, 0.68, 0.79, 0.66, 0.57, 0.74],
  ["faith", "Faithfulness", 0.93, 0.89, 0.96, 0.88, 0.8, 0.93],
  ["cite", "Citation rate", 0.81, 0.75, 0.86, 0.72, 0.62, 0.81],
  ["answer", "Answer score", 0.79, 0.73, 0.84, 0.68, 0.58, 0.77],
].map(([id, label, a, alo, ahi, b, blo, bhi]) => ({
  id: id as string,
  label: label as string,
  groups: [
    { id: "train", label: "Train", value: a as number, lo: alo as number, hi: ahi as number },
    { id: "test", label: "Held-out test", value: b as number, lo: blo as number, hi: bhi as number, hatched: true },
  ],
}))

const DAY = new Date(2026, 8, 1)
const dayAt = (i: number) => new Date(DAY.getFullYear(), DAY.getMonth(), DAY.getDate() + i)
const GOAL_MIN = 30
const minutes = [34, 28, 0, 41, 22, 36, 18, 30, 45, 12, 0, 27, 33, 38, 20, 31, 42, 25, 9, 35, 30, 44, 16, 24]
const TODAY = minutes.length - 1

const activity: Series[] = [
  { id: "min", label: "Minutes", points: minutes.map((y, i) => ({ x: dayAt(i).getTime(), y })) },
]

const due = [18, 22, 19, 25, 21, 17, 23, 26, 20, 18, 24, 22, 19, 16, 14]
const done = [16, 22, 15, 25, 18, 17, 20, 22, 20, 18, 21, 22, 19, 17, 15]
const reviews: Series[] = [
  { id: "due", label: "Cards due", points: due.map((y, i) => ({ x: dayAt(9 + i).getTime(), y })) },
  { id: "done", label: "Cards reviewed", points: done.map((y, i) => ({ x: dayAt(9 + i).getTime(), y })) },
]

const weekday = (d: Date) => d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" })
const phaseDays: PhaseDay[] = Array.from({ length: 30 }, (_, i) => {
  const d = dayAt(i)
  return {
    id: `d${i}`,
    label: String(d.getDate()),
    title: weekday(d),
    value: i <= TODAY ? Math.min(1, minutes[i] / GOAL_MIN) : null,
    isToday: i === TODAY,
  }
})

const skill = (id: string, label: string, value: number, mastery: number): SunburstDatum => ({ id, label, value, mastery })
const tree: SunburstDatum[] = [
  {
    id: "math",
    label: "Mathematics",
    children: [
      { id: "alg", label: "Algebra", children: [skill("lin", "Linear equations", 8, 0.92), skill("quad", "Quadratics", 6, 0.74), skill("ineq", "Inequalities", 4, 0.6), skill("sys", "Systems", 5, 0.8)] },
      { id: "calc", label: "Calculus", children: [skill("lim", "Limits", 6, 0.66), skill("der", "Derivatives", 9, 0.45), skill("int", "Integrals", 5, 0.2)] },
      { id: "stat", label: "Statistics", children: [skill("dist", "Distributions", 6, 0.55), skill("hyp", "Hypothesis tests", 4, 0.3)] },
    ],
  },
  {
    id: "eng",
    label: "English",
    children: [
      { id: "gram", label: "Grammar", children: [skill("comma", "Commas", 5, 0.85), skill("agree", "Agreement", 4, 0.7), skill("clause", "Clauses", 6, 0.4)] },
      { id: "rhet", label: "Rhetoric", children: [skill("claim", "Claims", 5, 0.6), skill("evid", "Evidence", 6, 0.5), skill("tone", "Tone", 3, 0.35)] },
      { id: "vocab", label: "Vocabulary", children: [skill("roots", "Roots", 6, 0.9), skill("ctx", "Context clues", 5, 0.65)] },
    ],
  },
  {
    id: "sci",
    label: "Science",
    children: [
      { id: "phys", label: "Physics", children: [skill("kin", "Kinematics", 7, 0.7), skill("forces", "Forces", 8, 0.5), skill("energy", "Energy", 6, 0.4)] },
      { id: "bio", label: "Biology", children: [skill("cells", "Cells", 6, 0.8), skill("gen", "Genetics", 7, 0.55), skill("evo", "Evolution", 4, 0.3)] },
    ],
  },
  {
    id: "chem",
    label: "Chemistry",
    children: [
      { id: "stoi", label: "Stoichiometry", children: [skill("mole", "Moles", 6, 0.6), skill("limit", "Limiting reagents", 4, 0.35)] },
      { id: "bond", label: "Bonding", children: [skill("lewis", "Lewis structures", 5, 0.5), skill("vsepr", "VSEPR", 4, 0.3)] },
      { id: "eq", label: "Equilibrium", children: [skill("chatel", "Le Chatelier", 4, 0.25), skill("ksp", "Ksp", 3, 0.1)] },
    ],
  },
]
const treeHues = ["var(--chart-1)", "var(--chart-2)", "var(--chart-3)", "var(--chart-4)"]

const triage: StackSegment[] = [
  { id: "read", label: "Read in full", value: 412 },
  { id: "skim", label: "Skimmed", value: 268 },
  { id: "skip", label: "Skipped", value: 97 },
  { id: "todo", label: "Not yet triaged", value: 153 },
]

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const pct2 = (v: number) => v.toFixed(2)

function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <div className="text-ink-3 flex items-center gap-3 font-mono text-[11px] tracking-[0.18em] uppercase">
      <span aria-hidden="true" className="bg-hairline-strong h-px w-6" />
      {children}
    </div>
  )
}

function Stat({ label, value, delta, children }: { label: string; value: string; delta?: string; children?: ReactNode }) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-2 px-6 py-5 first:pl-0 last:pr-0">
      <div className="text-ink-3 font-mono text-[11px] tracking-[0.18em] uppercase">{label}</div>
      <div className="flex items-end justify-between gap-3">
        <div className="text-foreground text-[40px] leading-none font-extralight tracking-[-0.04em] tabular-nums">{value}</div>
        {children}
      </div>
      {delta ? <div className="text-ink-2 font-mono text-[11px]">{delta}</div> : null}
    </div>
  )
}

function ModeLinks() {
  const href = (k: string, v: string) => {
    const p = new URLSearchParams(window.location.search)
    p.set(k, v)
    return `?${p.toString()}`
  }
  const link = (k: string, v: string, on: boolean) => (
    <a
      key={v}
      href={href(k, v)}
      aria-current={on ? "true" : undefined}
      className={`focus-visible:outline-world rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.08em] uppercase focus-visible:outline-2 focus-visible:outline-offset-2 ${on ? "border-hairline-strong bg-card text-foreground" : "border-hairline text-ink-3 hover:text-foreground"}`}
    >
      {v}
    </a>
  )
  return (
    <div className="mt-6 flex flex-wrap items-center gap-2">
      {["light", "dark"].map((t) => link("theme", t, t === theme))}
      <span aria-hidden="true" className="bg-hairline-strong mx-2 h-4 w-px" />
      {["neutral", "math", "english", "science", "chemistry", "languages"].map((w) => link("world", w, w === world))}
    </div>
  )
}

const calLabels = ["Guess", "Unsure", "Fairly sure", "Certain"]
const calibration: Series[] = [
  {
    id: "actual",
    label: "Correct",
    points: [
      { x: 0, y: 29, lo: 12, hi: 55 },
      { x: 1, y: 52, lo: 35, hi: 68 },
      { x: 2, y: 67, lo: 54, hi: 78 },
      { x: 3, y: 84, lo: 71, hi: 92 },
    ],
  },
  { id: "reference", label: "Reference", color: "var(--ink-3)", dashed: true, points: [25, 50, 75, 95].map((y, x) => ({ x, y })) },
]
const oneLevel: Series[] = [{ id: "actual", label: "Correct", points: [{ x: 2, y: 67, lo: 41, hi: 85 }] }]

export function ChartsGallery() {
  return (
    <main className="mx-auto max-w-[1120px] px-8 py-12">
      <header className="mb-10">
        <Eyebrow>Chart kit &middot; {theme} &middot; {world}</Eyebrow>
        <h1 className="mt-4 font-serif text-[44px] leading-[1.05] font-normal tracking-[-0.02em]">
          Charts, <em className="text-world">drawn by hand</em>
        </h1>
        <p className="text-ink-2 mt-3 max-w-[60ch] font-serif text-lg leading-relaxed text-pretty">
          Plain SVG, two-pixel strokes, hairline grids. Every chart has a name for screen readers and a Table view with the same numbers.
        </p>
        <ModeLinks />
      </header>

      <div className="border-hairline mb-8 flex flex-wrap items-stretch divide-x divide-[var(--hairline)] rounded-2xl border bg-card px-6">
        <Stat label="Reviews a day" value="22" delta="+3 on last week">
          <Sparkline name="Cards reviewed per day" values={done} area width={92} height={30} />
        </Stat>
        <Stat label="Minutes today" value={String(minutes[TODAY])} delta="goal 30 minutes">
          <Sparkline name="Minutes practised per day" values={minutes} width={92} height={30} />
        </Stat>
        <Stat label="Retention" value="86%" delta="+2 points">
          <Sparkline name="Retention" values={[78, 80, 79, 82, 84, 83, 85, 86]} area width={92} height={30} />
        </Stat>
        <div className="flex flex-none items-center justify-center px-6 py-4 first:pl-0">
          <Gauge name="Cards due" value={12} max={40} label="Due today" tone="warn" className="scale-[0.72] origin-center -my-6" />
        </div>
      </div>

      <div className="grid gap-6">
        <ChartFrame
          title="Hillclimb runs"
          subtitle="Train climbs steadily; the held-out test score is the one to trust. Three reverted runs cost nothing."
          table={lineTable(evalSeries, { xTitle: "Run", xFormat: (x) => `#${x}`, yFormat: pct2, caption: "Score by run" })}
          defaultView={defaultView}
          footer="Hover the chart or use the arrow keys to read every series at one run. Click a label to isolate it."
        >
          <LineChart
            name={`Score over ${RUNS} hillclimb runs: train, held-out test and hit at 3`}
            series={evalSeries}
            xFormat={(x) => `#${x}`}
            yFormat={pct2}
            height={300}
            bandNote="95% confidence interval"
            markerKey={[
              { shape: "diamond", label: "Baseline" },
              { shape: "circle", label: "Kept" },
              { shape: "cross", label: "Reverted" },
            ]}
          />
        </ChartFrame>

        <div className="grid items-start gap-6 lg:grid-cols-2">
          <ChartFrame
            title="Train vs held-out test"
            subtitle="Whiskers show the 95% interval. A big gap between the pair is overfitting."
            table={rangeTable(evalRows, pct2, "Metrics with 95% intervals")}
            defaultView={defaultView}
          >
            <RangeBars name="Train and held-out test score per metric, with confidence intervals" rows={evalRows} domain={[0, 1]} format={pct2} />
          </ChartFrame>

          <ChartFrame
            title="Practice, September"
            subtitle="Minutes a day, 1 to 24 September."
            table={lineTable(activity, { xTitle: "Day", xFormat: (t) => weekday(new Date(t)), caption: "Minutes practised per day" })}
            defaultView={defaultView}
          >
            <AreaChart name="Minutes practised each day in September" series={activity} xKind="time" height={260} />
          </ChartFrame>
        </div>

        <div className="grid items-start gap-6 lg:grid-cols-[1.15fr_1fr]">
          <ChartFrame
            title="Daily goal"
            subtitle="Each moon fills with the share of the 30-minute goal. A full moon is a goal met; a dashed outline is a day to come."
            table={phaseTable(phaseDays, "Share of the daily goal")}
            defaultView={defaultView}
          >
            <PhaseDiscs name="Share of the daily goal for each day of September" days={phaseDays} />
          </ChartFrame>

          <ChartFrame
            title="Cards due and reviewed"
            subtitle="Labels sit at the line ends and repel when they collide."
            table={lineTable(reviews, { xTitle: "Day", xFormat: (t) => weekday(new Date(t)), caption: "Cards due and reviewed" })}
            defaultView={defaultView}
          >
            <LineChart name="Cards due and cards reviewed per day" series={reviews} xKind="time" height={260} />
          </ChartFrame>
        </div>

        <div className="grid items-start gap-6 lg:grid-cols-2">
          <ChartFrame
            title="Calibration, four levels"
            className="lg:col-span-2"
            subtitle="A categorical x axis: every level is a tick, every point a dot with its own 95% whisker."
            table={lineTable(calibration, { xTitle: "Level", xFormat: (x) => calLabels[x], yFormat: (y) => `${Math.round(y)}%`, caption: "Percent correct by confidence level" })}
            defaultView={defaultView}
          >
            <LineChart
              name="Percent correct by confidence level, against a reference line"
              series={calibration}
              xKind="category"
              xLabels={calLabels}
              yDomain={[0, 100]}
              yFormat={(y) => `${Math.round(y)}%`}
              height={240}
            />
          </ChartFrame>

          <ChartFrame
            title="One level so far"
            subtitle="A single point has no neighbour to shade a band with, so its interval is a whisker."
            table={lineTable(oneLevel, { xTitle: "Level", xFormat: (x) => calLabels[x], yFormat: (y) => `${Math.round(y)}%`, caption: "One level" })}
            defaultView={defaultView}
          >
            <LineChart
              name="Percent correct at one confidence level"
              series={oneLevel}
              xKind="category"
              xLabels={calLabels}
              yDomain={[0, 100]}
              yFormat={(y) => `${Math.round(y)}%`}
              height={240}
            />
          </ChartFrame>

          <ChartFrame
            title="Reviews, last 14 days"
            subtitle="Discs worded for the busiest day instead of a goal: the unit and label props."
            table={phaseTable(phaseDays.slice(0, 14), "Share of the busiest day", "Share of your busiest day")}
            defaultView={defaultView}
          >
            <PhaseDiscs
              name="Reviews for each of the last 14 days, relative to the busiest day"
              days={phaseDays.slice(0, 14)}
              columns={7}
              unit="of your busiest day"
              label="of busiest day"
            />
          </ChartFrame>
        </div>

        <ChartFrame
          title="Mastery map"
          subtitle="Angle is the number of items, tint is mastery. Click a segment to zoom in; Escape or Back zooms out."
          table={sunburstTable(tree, "Items", "Mastery by subject, topic and skill")}
          defaultView={defaultView}
        >
          <Sunburst name="Mastery by subject, topic and skill" data={tree} hues={treeHues} size={460} />
        </ChartFrame>

        <ChartFrame
          title="Library triage"
          subtitle="930 pages across the shelf, sorted by how far each has been read."
          table={stackTable(triage, "Pages", undefined, "Pages by triage state")}
          defaultView={defaultView}
        >
          <StackedBar name="Pages by triage state" segments={triage} unit="pages" />
        </ChartFrame>
      </div>
    </main>
  )
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ChartsGallery />
  </StrictMode>,
)
