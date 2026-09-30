// World motifs: small line-art SVGs for the section header. Hairline strokes in --hairline-strong
// with --world accents. Each path carries pathLength="1" so shell.css can draw it on once when
// the world changes (the header remounts the motif per world); otherwise they are static.

import type { CSSProperties, ReactNode } from "react"

import { cn } from "@/lib/utils"
import type { World } from "@/lib/world"

const W = 184
const H = 88

const f = (n: number) => Number(n.toFixed(2))
const TAU = Math.PI * 2

/** Times-table chords: point i joins point (multiplier * i) mod n. Multiplier 2 draws a cardioid. */
export function cardioidPath(cx: number, cy: number, r: number, n: number, multiplier = 2) {
  const at = (i: number) => {
    const a = (TAU * i) / n - Math.PI / 2
    return `${f(cx + r * Math.cos(a))} ${f(cy + r * Math.sin(a))}`
  }
  const chords: string[] = []
  for (let i = 1; i < n; i++) {
    const j = (i * multiplier) % n
    if (j !== i) chords.push(`M${at(i)}L${at(j)}`)
  }
  return chords.join("")
}

const circlePath = (cx: number, cy: number, r: number) =>
  `M${f(cx - r)} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`

/** Lit part of a moon disc, `lit` from 0 (new) to 1 (full), waxing so the right side fills first. */
function moonLitPath(cx: number, cy: number, r: number, lit: number) {
  if (lit <= 0) return ""
  const k = 1 - 2 * lit
  const top = `${f(cx)} ${f(cy - r)}`
  const bottom = `${f(cx)} ${f(cy + r)}`
  const terminator =
    Math.abs(k) < 1e-6 ? `L${top}` : `A${f(r * Math.abs(k))} ${r} 0 0 ${k > 0 ? 0 : 1} ${top}`
  return `M${top}A${r} ${r} 0 0 1 ${bottom}${terminator}Z`
}

/** Pointy-top hexagon vertices. */
const hexVertices = (cx: number, cy: number, s: number) =>
  Array.from({ length: 6 }, (_, k) => {
    const a = (Math.PI / 180) * (-90 + 60 * k)
    return [cx + s * Math.cos(a), cy + s * Math.sin(a)] as const
  })

/** A honeycomb patch (a centre hexagon and its six neighbours) with shared edges drawn once. */
function honeycombPath(cx: number, cy: number, s: number) {
  const step = Math.sqrt(3) * s
  const centres: [number, number][] = [[cx, cy]]
  for (let k = 0; k < 6; k++) {
    const a = (Math.PI / 180) * (60 * k)
    centres.push([cx + step * Math.cos(a), cy + step * Math.sin(a)])
  }
  const seen = new Set<string>()
  const edges: string[] = []
  for (const [hx, hy] of centres) {
    const v = hexVertices(hx, hy, s)
    for (let k = 0; k < 6; k++) {
      const [ax, ay] = v[k]
      const [bx, by] = v[(k + 1) % 6]
      const key = [`${f(ax)},${f(ay)}`, `${f(bx)},${f(by)}`].sort().join("|")
      if (seen.has(key)) continue
      seen.add(key)
      edges.push(`M${f(ax)} ${f(ay)}L${f(bx)} ${f(by)}`)
    }
  }
  return edges.join("")
}

/** The alternating inner (double) bonds of a benzene ring. */
function benzeneBonds(cx: number, cy: number, s: number) {
  const inner = hexVertices(cx, cy, s * 0.66)
  return [0, 2, 4]
    .map((k) => `M${f(inner[k][0])} ${f(inner[k][1])}L${f(inner[(k + 1) % 6][0])} ${f(inner[(k + 1) % 6][1])}`)
    .join("")
}

function Frame({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      className={cn("motif overflow-visible", className)}
      fill="none"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  )
}

type PathProps = { d: string; i?: number; accent?: boolean; width?: number; fill?: boolean }

/** `i` staggers the draw-on, in steps set in shell.css. */
function P({ d, i = 0, accent, width, fill }: PathProps) {
  return (
    <path
      d={d}
      pathLength={1}
      strokeWidth={width ?? (accent ? 1.25 : 1)}
      className={cn(accent ? "stroke-world" : "stroke-hairline-strong", fill && "fill-world")}
      style={{ "--i": i } as CSSProperties}
    />
  )
}

// ---- Math: a times-table cardioid, a sine trace and a ruler.
const MATH_CHORDS = cardioidPath(50, 44, 37, 72, 2)
const MATH_SINE = Array.from({ length: 41 }, (_, k) => {
  const x = 108 + k * 1.75
  const y = 30 - 11 * Math.sin((k / 40) * TAU)
  return `${k === 0 ? "M" : "L"}${f(x)} ${f(y)}`
}).join("")
const MATH_RULER = Array.from({ length: 19 }, (_, k) => {
  const x = 108 + k * 4
  const h = k % 6 === 0 ? 11 : k % 2 === 0 ? 7 : 4
  return `M${x} 66v${h}`
}).join("") + "M108 66H176"

function MathMotif({ className }: { className?: string }) {
  return (
    <Frame className={className}>
      <P d={circlePath(50, 44, 37)} />
      <P d={MATH_CHORDS} accent i={1} width={0.6} />
      <P d={MATH_SINE} accent i={2} />
      <P d="M108 30H176" i={2} />
      <P d={MATH_RULER} i={3} />
    </Frame>
  )
}

// ---- English: a row of book spines and a drop cap.
const SPINES = [
  { w: 9, h: 46 },
  { w: 12, h: 58 },
  { w: 8, h: 40 },
  { w: 13, h: 64 },
  { w: 10, h: 50 },
  { w: 9, h: 44 },
  { w: 12, h: 56 },
]
const spineGeometry = (() => {
  let x = 6
  return SPINES.map((s, k) => {
    const left = x
    x += s.w + 2
    return { ...s, left, k }
  })
})()

function EnglishMotif({ className }: { className?: string }) {
  return (
    <Frame className={className}>
      {spineGeometry.map(({ left, w, h, k }) => {
        const top = 76 - h
        const leaning = k === 6
        return (
          <g key={k} transform={leaning ? `rotate(9 ${left + w} 76)` : undefined}>
            <P d={`M${left} 76V${top}H${left + w}V76`} i={k} accent={k === 1 || k === 3} />
            <P d={`M${left} ${top + 8}H${left + w}M${left} ${76 - 8}H${left + w}`} i={k} accent={k === 3} width={0.8} />
          </g>
        )
      })}
      <P d="M6 76H116" i={7} />
      <P d="M128 10H176V58H128Z" i={3} />
      <P d="M138 50L152 18L166 50M143 40H161" accent i={5} width={1.5} />
      <P d="M128 68H176M128 75H160" i={6} />
    </Frame>
  )
}

// ---- Science: orbit rings and the phases of the moon.
const PHASES = [0, 0.25, 0.5, 0.75, 1]

function ScienceMotif({ className }: { className?: string }) {
  return (
    <Frame className={className}>
      {[0, 60, 120].map((deg, k) => (
        <ellipse
          key={deg}
          cx={50}
          cy={44}
          rx={38}
          ry={13}
          pathLength={1}
          transform={`rotate(${deg} 50 44)`}
          strokeWidth={1}
          className="stroke-hairline-strong"
          style={{ "--i": k } as CSSProperties}
        />
      ))}
      <P d={circlePath(50, 44, 4)} accent i={3} />
      <P d={circlePath(88, 44, 2.5)} accent i={4} fill />
      {PHASES.map((lit, k) => {
        const cx = 112 + k * 15.5
        return (
          <g key={lit}>
            <P d={circlePath(cx, 44, 6.5)} i={3 + k} />
            {lit > 0 && <P d={moonLitPath(cx, 44, 6.5, lit)} accent i={4 + k} fill />}
          </g>
        )
      })}
      <P d="M108 64H176" i={8} />
      <P d="M108 64v4M139 64v4M170 64v4" i={8} />
    </Frame>
  )
}

// ---- Chemistry: a honeycomb lattice around a benzene ring, and a short chain.
const HEX_S = 13
const LATTICE = honeycombPath(54, 44, HEX_S)
const BENZENE_RING = `M${hexVertices(54, 44, HEX_S)
  .map(([x, y]) => `${f(x)} ${f(y)}`)
  .join("L")}Z`
const CHAIN = "M112 62L128 50L144 62L160 50L176 62"

function ChemistryMotif({ className }: { className?: string }) {
  return (
    <Frame className={className}>
      <P d={LATTICE} />
      <P d={BENZENE_RING} accent i={2} width={1.5} />
      <P d={benzeneBonds(54, 44, HEX_S)} accent i={3} />
      <P d={CHAIN} i={4} />
      <P d={circlePath(128, 50, 3)} accent i={5} />
      <P d={circlePath(160, 50, 3)} accent i={6} fill />
      <P d="M112 38H176" i={7} />
    </Frame>
  )
}

// ---- Neutral: ruler ticks with a measured span.
const NEUTRAL_TICKS =
  Array.from({ length: 41 }, (_, k) => {
    const x = 8 + k * 4.2
    const h = k % 10 === 0 ? 16 : k % 5 === 0 ? 11 : 6
    return `M${f(x)} 56v${h}`
  }).join("") + "M8 56H176"

function NeutralMotif({ className }: { className?: string }) {
  return (
    <Frame className={className}>
      <P d={NEUTRAL_TICKS} />
      <P d="M8 32H92M8 28v8M92 28v8" accent i={2} />
      <P d="M92 56v26M88 82h8" accent i={3} />
    </Frame>
  )
}

const MOTIFS: Record<World, (props: { className?: string }) => ReactNode> = {
  neutral: NeutralMotif,
  math: MathMotif,
  english: EnglishMotif,
  science: ScienceMotif,
  chemistry: ChemistryMotif,
}

/** The motif for a world. Key it by world so it draws on once when the world changes. */
export function WorldMotif({ world, className }: { world: World; className?: string }) {
  const Motif = MOTIFS[world]
  return <Motif className={className} />
}
