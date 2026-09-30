// Pure chart geometry: scales, ticks, label repel, arc and phase paths, sunburst layout.
// No DOM and no React here, so every function can be unit-tested in plain node.

const TAU = Math.PI * 2

/** Round to 2 decimals and avoid "-0", keeping path strings short and stable. */
const f = (n: number) => String(Math.round(n * 100) / 100 || 0)

/** Geist Mono advance is 0.6em, so this is exact for the 11px mono labels the kit uses. */
export const MONO_CHAR_W = 6.6
export const textWidth = (s: string) => s.length * MONO_CHAR_W
export const truncateLabel = (s: string, max: number) => (s.length > max ? `${s.slice(0, Math.max(1, max - 1))}…` : s)

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v))

// ---------------------------------------------------------------------------
// Scales and ticks
// ---------------------------------------------------------------------------

export interface LinearScale {
  (v: number): number
  invert(px: number): number
}

export function linearScale(d0: number, d1: number, r0: number, r1: number): LinearScale {
  const span = d1 - d0
  const scale = (v: number) => (span === 0 ? (r0 + r1) / 2 : r0 + ((v - d0) / span) * (r1 - r0))
  const invert = (px: number) => (r1 === r0 ? d0 : d0 + ((px - r0) / (r1 - r0)) * span)
  return Object.assign(scale, { invert })
}

/** Heckbert's "nice number": a 1, 2, 5 or 10 multiple near `range`. */
function niceNum(range: number, round: boolean): number {
  const exp = Math.floor(Math.log10(range))
  const frac = range / 10 ** exp
  let nice: number
  if (round) nice = frac < 1.5 ? 1 : frac < 3 ? 2 : frac < 7 ? 5 : 10
  else nice = frac <= 1 ? 1 : frac <= 2 ? 2 : frac <= 5 ? 5 : 10
  return nice * 10 ** exp
}

export interface NiceTicks {
  min: number
  max: number
  step: number
  ticks: number[]
}

/** Round, evenly spaced ticks that cover [min, max]. The returned min/max are the nice domain. */
export function niceTicks(min: number, max: number, count = 5): NiceTicks {
  if (!Number.isFinite(min) || !Number.isFinite(max)) return { min: 0, max: 1, step: 0.2, ticks: [0, 0.2, 0.4, 0.6, 0.8, 1] }
  if (min > max) [min, max] = [max, min]
  if (min === max) {
    const pad = Math.abs(min) * 0.1 || 1
    min -= pad
    max += pad
  }
  const step = niceNum(niceNum(max - min, false) / Math.max(1, count - 1), true)
  const dec = Math.max(0, -Math.floor(Math.log10(step)))
  const lo = Math.floor(min / step + 1e-9) * step
  const hi = Math.ceil(max / step - 1e-9) * step
  const ticks: number[] = []
  for (let v = lo, i = 0; v <= hi + step / 2; i++, v = lo + i * step) ticks.push(Number(v.toFixed(dec)))
  return { min: ticks[0], max: ticks[ticks.length - 1], step, ticks }
}

const HOUR = 3_600_000
const DAY_STEPS = [1, 2, 7, 14, 30, 60, 90, 180, 365]
const HOUR_STEPS = [1, 3, 6, 12]

/** Ticks on local hour or local-midnight boundaries for a time axis (epoch ms). */
export function niceTimeTicks(t0: number, t1: number, maxTicks = 6): { ticks: number[]; unit: "hour" | "day" } {
  const span = t1 - t0
  if (!(span > 0)) return { ticks: [t0], unit: "day" }
  const hourStep = HOUR_STEPS.find((h) => span / (h * HOUR) <= maxTicks)
  if (hourStep && span < 2 * 24 * HOUR) {
    const ms = hourStep * HOUR
    const ticks: number[] = []
    for (let t = Math.ceil(t0 / ms) * ms; t <= t1; t += ms) ticks.push(t)
    return { ticks, unit: "hour" }
  }
  const days = DAY_STEPS.find((d) => span / (d * 24 * HOUR) <= maxTicks) ?? 365
  const d = new Date(t0)
  d.setHours(0, 0, 0, 0)
  if (d.getTime() < t0) d.setDate(d.getDate() + 1)
  const ticks: number[] = []
  while (d.getTime() <= t1) {
    ticks.push(d.getTime())
    d.setDate(d.getDate() + days)
  }
  return { ticks, unit: "day" }
}

/** Index of the entry in an ascending array nearest to v. */
export function nearestIndex(sorted: number[], v: number): number {
  if (sorted.length === 0) return -1
  let lo = 0
  let hi = sorted.length - 1
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1
    if (sorted[mid] < v) lo = mid
    else hi = mid
  }
  return Math.abs(sorted[lo] - v) <= Math.abs(sorted[hi] - v) ? lo : hi
}

/** cubic-bezier easing function, same maths as the CSS timing function of the same name. */
export function cubicBezier(x1: number, y1: number, x2: number, y2: number): (t: number) => number {
  const cx = 3 * x1
  const bx = 3 * (x2 - x1) - cx
  const ax = 1 - cx - bx
  const cy = 3 * y1
  const by = 3 * (y2 - y1) - cy
  const ay = 1 - cy - by
  const X = (s: number) => ((ax * s + bx) * s + cx) * s
  const dX = (s: number) => (3 * ax * s + 2 * bx) * s + cx
  return (t) => {
    if (t <= 0) return 0
    if (t >= 1) return 1
    let s = t
    for (let i = 0; i < 8; i++) {
      const err = X(s) - t
      if (Math.abs(err) < 1e-6) break
      const d = dX(s)
      if (Math.abs(d) < 1e-6) break
      s -= err / d
    }
    if (s < 0 || s > 1) {
      let lo = 0
      let hi = 1
      s = t
      for (let i = 0; i < 40; i++) {
        const x = X(s)
        if (Math.abs(x - t) < 1e-6) break
        if (x < t) lo = s
        else hi = s
        s = (lo + hi) / 2
      }
    }
    return ((ay * s + by) * s + cy) * s
  }
}

/** The kit's ease-inout token, cubic-bezier(.76, 0, .16, 1). */
export const easeInOut = cubicBezier(0.76, 0, 0.16, 1)

// ---------------------------------------------------------------------------
// Line, band and area paths
// ---------------------------------------------------------------------------

export interface XY {
  x: number
  y: number
}

export function linePath(pts: XY[]): string {
  return pts.map((p, i) => `${i === 0 ? "M" : "L"}${f(p.x)} ${f(p.y)}`).join("")
}

export function areaPath(pts: XY[], baseY: number): string {
  if (pts.length === 0) return ""
  const last = pts[pts.length - 1]
  return `${linePath(pts)}L${f(last.x)} ${f(baseY)}L${f(pts[0].x)} ${f(baseY)}Z`
}

/** Closed polygon between an upper and a lower edge that share their x positions. */
export function bandPath(upper: XY[], lower: XY[]): string {
  if (upper.length === 0 || upper.length !== lower.length) return ""
  const back = [...lower].reverse()
  return `${linePath(upper)}${back.map((p) => `L${f(p.x)} ${f(p.y)}`).join("")}Z`
}

/** Split items into runs of consecutive entries for which `keep` holds. */
export function runsOf<T>(items: T[], keep: (t: T) => boolean): T[][] {
  const runs: T[][] = []
  let cur: T[] = []
  for (const it of items) {
    if (keep(it)) cur.push(it)
    else if (cur.length) {
      runs.push(cur)
      cur = []
    }
  }
  if (cur.length) runs.push(cur)
  return runs
}

// ---------------------------------------------------------------------------
// End-of-line label repel
// ---------------------------------------------------------------------------

/**
 * Nudge label centres so none is closer than `minGap` to its neighbour, keeping their order, staying inside
 * [lo, hi] and moving each cluster as little as possible around its targets' mean.
 */
export function repelLabels(targets: number[], minGap: number, lo: number, hi: number): number[] {
  const n = targets.length
  if (n === 0) return []
  const order = targets.map((_, i) => i).sort((a, b) => targets[a] - targets[b] || a - b)
  type Group = { members: number[]; start: number }
  const place = (members: number[]) => {
    const mean = members.reduce((s, m, k) => s + targets[order[m]] - k * minGap, 0) / members.length
    return clamp(mean, lo, Math.max(lo, hi - (members.length - 1) * minGap))
  }
  let groups: Group[] = order.map((_, m) => ({ members: [m], start: place([m]) }))
  for (let guard = 0; guard < n + 2; guard++) {
    const merged: Group[] = []
    let changed = false
    for (const g of groups) {
      const prev = merged[merged.length - 1]
      if (prev && prev.start + (prev.members.length - 1) * minGap + minGap > g.start + 1e-9) {
        const members = [...prev.members, ...g.members]
        merged[merged.length - 1] = { members, start: place(members) }
        changed = true
      } else merged.push(g)
    }
    groups = merged
    if (!changed) break
  }
  const out = new Array<number>(n)
  for (const g of groups) g.members.forEach((m, k) => (out[order[m]] = g.start + k * minGap))
  return out
}

// ---------------------------------------------------------------------------
// Tooltip placement
// ---------------------------------------------------------------------------

/** Top-left for a tooltip anchored at (ax, ay): right of the anchor, flipped left near the edge. */
export function placeTooltip(ax: number, ay: number, w: number, h: number, cw: number, ch: number, gap = 14): XY {
  let x = ax + gap
  if (x + w > cw - 4) x = ax - gap - w
  return { x: clamp(x, 4, Math.max(4, cw - w - 4)), y: clamp(ay, 4, Math.max(4, ch - h - 4)) }
}

/** Top-left for a tooltip centred above the anchor, dropping below it when there is no room. */
export function placeTooltipAbove(ax: number, ay: number, w: number, h: number, cw: number, ch: number, gap = 14): XY {
  const y = ay - gap - h < 4 ? ay + gap : ay - gap - h
  return { x: clamp(ax - w / 2, 4, Math.max(4, cw - w - 4)), y: clamp(y, 4, Math.max(4, ch - h - 4)) }
}

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

export type MarkerShape = "circle" | "diamond" | "cross" | "square" | "triangle"

/** Path centred on the origin, sized by half-extent `s`. Cross is open (stroke only). */
export function markerPath(shape: MarkerShape, s: number): string {
  switch (shape) {
    case "circle":
      return `M${f(-s)} 0A${f(s)} ${f(s)} 0 1 0 ${f(s)} 0A${f(s)} ${f(s)} 0 1 0 ${f(-s)} 0Z`
    case "diamond": {
      const d = s * 1.25
      return `M0 ${f(-d)}L${f(d)} 0L0 ${f(d)}L${f(-d)} 0Z`
    }
    case "square":
      return `M${f(-s)} ${f(-s)}L${f(s)} ${f(-s)}L${f(s)} ${f(s)}L${f(-s)} ${f(s)}Z`
    case "triangle":
      return `M0 ${f(-s * 1.15)}L${f(s * 1.1)} ${f(s * 0.85)}L${f(-s * 1.1)} ${f(s * 0.85)}Z`
    case "cross": {
      const c = s * 0.9
      return `M${f(-c)} ${f(-c)}L${f(c)} ${f(c)}M${f(c)} ${f(-c)}L${f(-c)} ${f(c)}`
    }
  }
}

// ---------------------------------------------------------------------------
// Polar geometry: arcs, gauge ticks, moon phases
// ---------------------------------------------------------------------------

/** Angle 0 is 12 o'clock and grows clockwise (radians). */
export const polar = (r: number, a: number): XY => ({ x: r * Math.sin(a), y: -r * Math.cos(a) })

/** Annular sector centred on the origin. Empty string when the sector has no width. */
export function arcPath(r0: number, r1: number, a0: number, a1: number): string {
  const span = a1 - a0
  if (span <= 1e-4 || r1 - r0 <= 0.01) return ""
  if (span >= TAU - 1e-4) {
    const ring = (r: number, sweep: 0 | 1) =>
      `M0 ${f(-r)}A${f(r)} ${f(r)} 0 1 ${sweep} 0 ${f(r)}A${f(r)} ${f(r)} 0 1 ${sweep} 0 ${f(-r)}Z`
    return r0 > 0.01 ? `${ring(r1, 1)}${ring(r0, 0)}` : ring(r1, 1)
  }
  const large = span > Math.PI ? 1 : 0
  const p0 = polar(r1, a0)
  const p1 = polar(r1, a1)
  const p2 = polar(r0, a1)
  const p3 = polar(r0, a0)
  const inner = r0 > 0.01 ? `A${f(r0)} ${f(r0)} 0 ${large} 0 ${f(p3.x)} ${f(p3.y)}` : "L0 0"
  return `M${f(p0.x)} ${f(p0.y)}A${f(r1)} ${f(r1)} 0 ${large} 1 ${f(p1.x)} ${f(p1.y)}L${f(p2.x)} ${f(p2.y)}${inner}Z`
}

export interface GaugeTick {
  i: number
  major: boolean
  angle: number // degrees from straight up, negative to the left
  x1: number
  y1: number
  x2: number
  y2: number
}

export const GAUGE_TICKS = 21
export const GAUGE_SPAN = 124

/** Ticks on an arc of `span` degrees, centred on straight up, around (cx, cy). */
export function gaugeTicks(cx: number, cy: number, rIn: number, rOut: number, count = GAUGE_TICKS, span = GAUGE_SPAN, majorEvery = 5): GaugeTick[] {
  const out: GaugeTick[] = []
  for (let i = 0; i < count; i++) {
    const angle = count === 1 ? 0 : -span / 2 + (span * i) / (count - 1)
    const rad = (angle * Math.PI) / 180
    const major = i % majorEvery === 0
    const r0 = major ? rIn - 4 : rIn
    out.push({
      i,
      major,
      angle,
      x1: cx + r0 * Math.sin(rad),
      y1: cy - r0 * Math.cos(rad),
      x2: cx + rOut * Math.sin(rad),
      y2: cy - rOut * Math.cos(rad),
    })
  }
  return out
}

export function needleAngle(value: number, min: number, max: number, span = GAUGE_SPAN): number {
  const t = max === min ? 0 : clamp((value - min) / (max - min), 0, 1)
  return -span / 2 + span * t
}

export interface PhaseGeometry {
  /** Terminator ellipse half-width. 0 is a straight terminator (half moon). */
  rx: number
  /** Arc sweep flag for the terminator: 0 bulges to the lit side (crescent), 1 to the dark side (gibbous). */
  sweep: 0 | 1
  empty: boolean
  full: boolean
}

export function phaseGeometry(lit: number, r: number): PhaseGeometry {
  const fr = clamp(lit, 0, 1)
  return { rx: r * Math.abs(1 - 2 * fr), sweep: fr < 0.5 ? 0 : 1, empty: fr < 0.004, full: fr > 0.996 }
}

/**
 * Lit region of a waxing moon (lit on the right) whose lit area is `lit` of the disc: the right limb
 * semicircle closed by a terminator half-ellipse. Centred on the origin.
 */
export function phasePath(lit: number, r: number): string {
  const g = phaseGeometry(lit, r)
  if (g.empty) return ""
  if (g.full) return `M0 ${f(-r)}A${f(r)} ${f(r)} 0 1 1 0 ${f(r)}A${f(r)} ${f(r)} 0 1 1 0 ${f(-r)}Z`
  return `M0 ${f(-r)}A${f(r)} ${f(r)} 0 0 1 0 ${f(r)}A${f(g.rx)} ${f(r)} 0 0 ${g.sweep} 0 ${f(-r)}Z`
}

// ---------------------------------------------------------------------------
// Stacked bar
// ---------------------------------------------------------------------------

/** Segment x/width for a row `width` wide with `gap` between non-empty segments. */
export function stackLayout(values: number[], width: number, gap = 2): { x: number; w: number }[] {
  const pos = values.map((v) => Math.max(0, v))
  const total = pos.reduce((a, b) => a + b, 0)
  const live = pos.filter((v) => v > 0).length
  const usable = Math.max(0, width - gap * Math.max(0, live - 1))
  let x = 0
  return pos.map((v) => {
    const w = total === 0 ? 0 : (v / total) * usable
    const seg = { x, w }
    if (v > 0) x += w + gap
    return seg
  })
}

/** Percentages that sum to exactly 100 (largest remainder), rounded to `decimals`. */
export function percentages(values: number[], decimals = 0): number[] {
  const pos = values.map((v) => Math.max(0, v))
  const total = pos.reduce((a, b) => a + b, 0)
  if (total === 0) return pos.map(() => 0)
  const unit = 10 ** decimals
  const raw = pos.map((v) => (v / total) * 100 * unit)
  const floor = raw.map(Math.floor)
  let left = 100 * unit - floor.reduce((a, b) => a + b, 0)
  const byRemainder = raw.map((r, i) => ({ i, rem: r - floor[i] })).sort((a, b) => b.rem - a.rem)
  for (const { i } of byRemainder) {
    if (left <= 0) break
    floor[i] += 1
    left -= 1
  }
  return floor.map((v) => v / unit)
}

// ---------------------------------------------------------------------------
// Sunburst
// ---------------------------------------------------------------------------

export interface SunburstDatum {
  id: string
  label: string
  /** Item count for a leaf. Defaults to 1. Ignored when the node has children. */
  value?: number
  /** 0..1 for a leaf. A parent's mastery is the value-weighted mean of its children. */
  mastery?: number
  children?: SunburstDatum[]
}

export const ROOT_ID = "__root"

export interface ArcNode {
  id: string
  label: string
  depth: number
  parentId: string | null
  childIds: string[]
  /** Ids from the root down to the parent. */
  ancestors: string[]
  value: number
  mastery: number
  /** Index of the top-level node this one belongs to (-1 for the root). Sets the hue. */
  top: number
  /** Angular extent as a fraction of a turn, for the fully zoomed-out layout. */
  a0: number
  a1: number
}

/** Flatten a forest into pre-ordered arc nodes (root first), trimmed to `maxDepth` rings. */
export function layoutSunburst(data: SunburstDatum[], maxDepth = 3): ArcNode[] {
  const nodes: ArcNode[] = []
  const build = (d: SunburstDatum, depth: number, parentId: string | null, ancestors: string[], top: number): ArcNode => {
    const node: ArcNode = { id: d.id, label: d.label, depth, parentId, childIds: [], ancestors, value: 0, mastery: 0, top, a0: 0, a1: 0 }
    nodes.push(node)
    const kids = depth < maxDepth ? (d.children ?? []) : []
    if (kids.length === 0) {
      node.value = Math.max(0, d.value ?? 1)
      node.mastery = clamp(d.mastery ?? 0, 0, 1)
      return node
    }
    let v = 0
    let m = 0
    kids.forEach((k, i) => {
      const c = build(k, depth + 1, d.id, [...ancestors, d.id], depth === 0 ? i : top)
      node.childIds.push(c.id)
      v += c.value
      m += c.mastery * c.value
    })
    node.value = v
    node.mastery = v > 0 ? m / v : 0
    return node
  }
  const root = build({ id: ROOT_ID, label: "All", children: data }, 0, null, [], -1)
  const byId = new Map(nodes.map((n) => [n.id, n]))
  const assign = (n: ArcNode, a0: number, a1: number) => {
    n.a0 = a0
    n.a1 = a1
    let cur = a0
    for (const id of n.childIds) {
      const c = byId.get(id)!
      const next = n.value > 0 ? cur + ((a1 - a0) * c.value) / n.value : cur
      assign(c, cur, next)
      cur = next
    }
  }
  assign(root, 0, 1)
  return nodes
}

export interface ArcState {
  /** Fractions of a turn. */
  a0: number
  a1: number
  r0: number
  r1: number
  opacity: number
  visible: boolean
}

export interface FocusConfig {
  hole: number
  radius: number
  maxRings?: number
}

export const indexNodes = (nodes: ArcNode[]) => new Map(nodes.map((n) => [n.id, n]))

/** Display state of every node when `focusId` is the zoom root. Focus and its ancestors collapse to the hole. */
export function focusTarget(nodes: ArcNode[], focusId: string, cfg: FocusConfig): Map<string, ArcState> {
  const byId = indexNodes(nodes)
  const focus = byId.get(focusId) ?? nodes[0]
  const maxRings = cfg.maxRings ?? 3
  const inSubtree = (n: ArcNode) => n.id === focus.id || n.ancestors.includes(focus.id)
  let deepest = focus.depth + 1
  for (const n of nodes) if (inSubtree(n)) deepest = Math.max(deepest, n.depth)
  const rings = clamp(deepest - focus.depth, 1, maxRings)
  const th = (cfg.radius - cfg.hole) / rings
  const span = focus.a1 - focus.a0 || 1
  const out = new Map<string, ArcState>()
  for (const n of nodes) {
    const isCentre = n.id === focus.id || focus.ancestors.includes(n.id)
    if (isCentre) {
      out.set(n.id, { a0: 0, a1: 1, r0: cfg.hole, r1: cfg.hole, opacity: 0, visible: false })
      continue
    }
    const k = Math.max(1, n.depth - focus.depth)
    const inside = inSubtree(n) && k <= maxRings
    out.set(n.id, {
      a0: clamp((n.a0 - focus.a0) / span, 0, 1),
      a1: clamp((n.a1 - focus.a0) / span, 0, 1),
      r0: cfg.hole + (Math.min(k, maxRings) - 1) * th,
      r1: cfg.hole + Math.min(k, maxRings) * th,
      opacity: inside ? 1 : 0,
      visible: inside,
    })
  }
  return out
}

/** The node itself, all its ancestors (except the synthetic root) and all its descendants. */
export function lineageOf(nodes: ArcNode[], id: string): Set<string> {
  const byId = indexNodes(nodes)
  const n = byId.get(id)
  const out = new Set<string>()
  if (!n) return out
  out.add(id)
  for (const a of n.ancestors) if (a !== ROOT_ID) out.add(a)
  for (const d of nodes) if (d.ancestors.includes(id)) out.add(d.id)
  return out
}

export type NavKey = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"

/** Roving keyboard move between visible arcs: left/right walk siblings, up goes to the parent, down to the first child. */
export function navigateArcs(nodes: ArcNode[], visible: (id: string) => boolean, id: string, key: NavKey): string {
  const byId = indexNodes(nodes)
  const n = byId.get(id)
  if (!n) return id
  if (key === "ArrowDown") return n.childIds.find(visible) ?? id
  if (key === "ArrowUp") return n.parentId && visible(n.parentId) ? n.parentId : id
  const sibs = (n.parentId ? byId.get(n.parentId)?.childIds : undefined)?.filter(visible) ?? []
  const i = sibs.indexOf(id)
  if (i < 0 || sibs.length < 2) return id
  return sibs[(i + (key === "ArrowRight" ? 1 : -1) + sibs.length) % sibs.length]
}

/** Centre of an arc in px, for tooltips and labels. Angles are fractions of a turn. */
export function arcCentroid(s: Pick<ArcState, "a0" | "a1" | "r0" | "r1">): XY {
  return polar((s.r0 + s.r1) / 2, ((s.a0 + s.a1) / 2) * TAU)
}

/** Whether a tangential label of `chars` mono characters fits inside an arc. */
export function labelFit(s: Pick<ArcState, "a0" | "a1" | "r0" | "r1">, chars: number): number {
  const arcLen = (s.a1 - s.a0) * TAU * ((s.r0 + s.r1) / 2)
  const need = textWidth("x".repeat(chars)) + 10
  if (s.r1 - s.r0 < 16) return 0
  return clamp((arcLen / need - 1) / 0.08, 0, 1)
}

// ---------------------------------------------------------------------------
// Sparkline
// ---------------------------------------------------------------------------

export function sparklinePoints(values: number[], w: number, h: number, pad = 3): XY[] {
  if (values.length === 0) return []
  const lo = Math.min(...values)
  const hi = Math.max(...values)
  const x = linearScale(0, Math.max(1, values.length - 1), pad, w - pad)
  const y = linearScale(lo, hi === lo ? lo + 1 : hi, h - pad, pad)
  return values.map((v, i) => ({ x: x(i), y: y(v) }))
}
