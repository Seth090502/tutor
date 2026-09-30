import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react"

import { cn } from "@/lib/utils"
import { EASE_SOFT, tween } from "./_motion"
import { useElementWidth } from "./_hooks"
import { ChartTooltip } from "./ChartTooltip"
import {
  arcCentroid,
  arcPath,
  easeInOut,
  focusTarget,
  indexNodes,
  labelFit,
  layoutSunburst,
  lineageOf,
  navigateArcs,
  polar,
  ROOT_ID,
  type ArcNode,
  type NavKey,
  type SunburstDatum,
} from "./geometry"
import { seriesColor, tintOf } from "./palette"
import type { TableData } from "./types"
import "./charts.css"

const TAU = Math.PI * 2
const ZOOM_MS = 760
/** How much of the hue mixes into the card colour when mastery is 100%, by ring. Deeper rings are lighter. */
const DEPTH_TINT = [0, 54, 46, 38]
const LEVELS = ["Subject", "Topic", "Skill"]

export interface SunburstFocus {
  id: string
  label: string
  /** Item count under the focused node. */
  value: number
  /** 0..1 */
  mastery: number
  depth: number
  path: string[]
}

export interface SunburstProps {
  data: SunburstDatum[]
  /** Accessible name. */
  name: string
  /** Largest diameter in px. Shrinks to fit its container. */
  size?: number
  /** One colour per top-level node. Defaults to --chart-1..5. */
  hues?: string[]
  /** What leaf values count. */
  unit?: string
  /** Replaces the default centre readout. */
  centre?: ReactNode | ((focus: SunburstFocus) => ReactNode)
  /** Called when a leaf is clicked or chosen with Enter. */
  onSelect?: (id: string) => void
  className?: string
}

export function sunburstTable(data: SunburstDatum[], unit = "Items", caption?: string): TableData {
  const nodes = layoutSunburst(data)
  return {
    caption,
    columns: ["Name", "Level", unit.charAt(0).toUpperCase() + unit.slice(1), "Mastery"],
    rows: nodes.filter((n) => n.depth >= 1).map((n) => [`${"   ".repeat(n.depth - 1)}${n.label}`, LEVELS[n.depth - 1] ?? `Level ${n.depth}`, n.value, `${Math.round(n.mastery * 100)}%`]),
    align: ["left", "left", "right", "right"],
  }
}

const lerp = (a: number, b: number, t: number) => a + (b - a) * t

export function Sunburst({ data, name, size = 440, hues, unit = "items", centre, onSelect, className }: SunburstProps) {
  const uid = useId()
  const nodes = useMemo(() => layoutSunburst(data), [data])
  const byId = useMemo(() => indexNodes(nodes), [nodes])
  const [outerRef, avail] = useElementWidth<HTMLDivElement>(size)
  const S = Math.max(240, Math.min(size, avail))
  const R = S / 2 - 2
  const hole = R * 0.36

  const [focusId, setFocusId] = useState(ROOT_ID)
  const [hoverId, setHoverId] = useState<string | null>(null)
  const [activeId, setActiveId] = useState<string | null>(null)
  const [kbdId, setKbdId] = useState<string | null>(null)
  const [tip, setTip] = useState<{ id: string; x: number; y: number } | null>(null)
  const [announce, setAnnounce] = useState("")

  // Reset if the data changes under us and the focused node vanished.
  const focus = byId.get(focusId) ?? nodes[0]
  useEffect(() => {
    if (!byId.has(focusId)) setFocusId(ROOT_ID)
  }, [byId, focusId])

  const target = useMemo(() => focusTarget(nodes, focus.id, { hole, radius: R }), [nodes, focus.id, hole, R])
  const visible = (id: string) => target.get(id)?.visible === true
  const rovingId = activeId && visible(activeId) ? activeId : (focus.childIds.find(visible) ?? null)

  // Arc geometry is animated by writing path data straight to the DOM, never through React state.
  const pathRefs = useRef(new Map<string, SVGPathElement>())
  const labelRefs = useRef(new Map<string, SVGTextElement>())
  const groupRefs = useRef(new Map<string, SVGGElement>())
  const ringRef = useRef<SVGPathElement>(null)
  const display = useRef(new Map<string, number[]>())
  const cancelTween = useRef<(() => void) | null>(null)
  const prevFocus = useRef<string | null>(null)
  const pendingFocus = useRef<string | null>(null)
  const kbdRef = useRef<string | null>(null)
  kbdRef.current = kbdId

  const apply = () => {
    for (const n of nodes) {
      if (n.depth < 1) continue
      const st = display.current.get(n.id)
      if (!st) continue
      const [a0, a1, r0, r1, op] = st
      const d = arcPath(r0, r1, a0 * TAU, a1 * TAU)
      pathRefs.current.get(n.id)?.setAttribute("d", d)
      pathRefs.current.get(n.id)?.setAttribute("opacity", String(op))
      const t = labelRefs.current.get(n.id)
      if (t) {
        const fit = labelFit({ a0, a1, r0, r1 }, n.label.length) * op
        if (fit < 0.02) t.setAttribute("opacity", "0")
        else {
          const mid = ((a0 + a1) / 2) * 360
          const p = polar((r0 + r1) / 2, ((a0 + a1) / 2) * TAU)
          t.setAttribute("transform", `translate(${p.x.toFixed(2)} ${p.y.toFixed(2)}) rotate(${(mid > 90 && mid < 270 ? mid + 180 : mid).toFixed(2)})`)
          t.setAttribute("opacity", fit.toFixed(3))
        }
      }
      if (kbdRef.current === n.id) ringRef.current?.setAttribute("d", d)
    }
  }

  useLayoutEffect(() => {
    cancelTween.current?.()
    const disp = display.current
    const animate = disp.size > 0 && prevFocus.current !== null && prevFocus.current !== focus.id
    prevFocus.current = focus.id
    const to = new Map<string, number[]>()
    for (const [id, s] of target) to.set(id, [s.a0, s.a1, s.r0, s.r1, s.opacity])
    if (!animate) {
      display.current = to
      apply()
      return
    }
    const from = new Map<string, number[]>()
    for (const [id, v] of to) from.set(id, disp.get(id) ?? v)
    cancelTween.current = tween(ZOOM_MS, easeInOut, (e) => {
      const cur = new Map<string, number[]>()
      for (const [id, b] of to) {
        const a = from.get(id)!
        cur.set(id, b.map((v, i) => lerp(a[i], v, e)))
      }
      display.current = cur
      apply()
    })
    return () => cancelTween.current?.()
    // apply() reads refs only; the target map is the real dependency.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target, focus.id])

  useLayoutEffect(() => {
    if (kbdId === null) ringRef.current?.setAttribute("d", "")
    else apply()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kbdId])

  useEffect(() => {
    const id = pendingFocus.current
    if (id) {
      pendingFocus.current = null
      groupRefs.current.get(id)?.focus()
    }
  }, [focus.id])

  const pathLabels = (n: ArcNode) => [...n.ancestors.filter((a) => a !== ROOT_ID).map((a) => byId.get(a)?.label ?? ""), n.label]
  const pctOf = (n: ArcNode) => Math.round(n.mastery * 100)

  const zoomTo = (id: string, byKeyboard: boolean) => {
    const n = byId.get(id)
    if (!n) return
    setTip(null)
    setHoverId(null)
    setFocusId(id)
    const first = n.childIds[0] ?? null
    setActiveId(first)
    if (byKeyboard) pendingFocus.current = first
    setAnnounce(id === ROOT_ID ? "Zoomed out to all subjects" : `Zoomed into ${n.label}. ${n.value} ${unit}, ${pctOf(n)}% mastery.`)
  }
  const zoomOut = (byKeyboard: boolean) => {
    if (focus.id === ROOT_ID) return
    const parent = focus.parentId ?? ROOT_ID
    zoomTo(parent, false)
    setActiveId(focus.id)
    if (byKeyboard) pendingFocus.current = focus.id
  }
  const activate = (id: string, byKeyboard: boolean) => {
    const n = byId.get(id)
    if (!n) return
    if (n.childIds.length) zoomTo(id, byKeyboard)
    else onSelect?.(id)
  }

  const onKeyDown = (e: KeyboardEvent<SVGSVGElement>) => {
    const id = (e.target as Element).getAttribute("data-arc")
    if (e.key === "Escape" || e.key === "Backspace") {
      if (focus.id !== ROOT_ID) {
        e.preventDefault()
        zoomOut(true)
      }
      return
    }
    if (!id) return
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault()
      activate(id, true)
      return
    }
    if (e.key.startsWith("Arrow")) {
      e.preventDefault()
      const next = navigateArcs(nodes, visible, id, e.key as NavKey)
      setActiveId(next)
      groupRefs.current.get(next)?.focus()
    }
  }

  const litId = hoverId ?? kbdId
  const lineage = useMemo(() => (litId ? lineageOf(nodes, litId) : null), [nodes, litId])
  const hueOf = (n: ArcNode) => hues?.[n.top % (hues.length || 1)] ?? seriesColor(n.top)
  const fillOf = (n: ArcNode) => tintOf(hueOf(n), (DEPTH_TINT[n.depth] ?? 36) * (0.38 + 0.62 * n.mastery))

  const focusInfo: SunburstFocus = {
    id: focus.id,
    label: focus.id === ROOT_ID ? "All subjects" : focus.label,
    value: focus.value,
    mastery: focus.mastery,
    depth: focus.depth,
    path: focus.id === ROOT_ID ? [] : pathLabels(focus),
  }
  const defaultCentre = (
    <>
      <div className="text-ink-3 max-w-full truncate font-mono text-[11px] tracking-[0.14em] uppercase">{focusInfo.label}</div>
      <div className="text-foreground mt-1.5 text-[38px] leading-none font-extralight tracking-[-0.04em] tabular-nums">
        {pctOf(focus)}
        <span className="text-ink-3 ml-0.5 text-lg tracking-normal">%</span>
      </div>
      <div className="text-ink-3 mt-1.5 font-mono text-[11px]">
        mastery &middot; {focus.value} {unit}
      </div>
    </>
  )

  const tipNode = tip ? byId.get(tip.id) : undefined
  const setTipAt = (id: string, x: number, y: number) => setTip({ id, x, y })
  const box = useRef<HTMLDivElement>(null)

  return (
    <div ref={outerRef} className={cn("min-w-0", className)}>
      <div className="mb-2 flex h-8 items-center gap-3 font-mono text-[11px]">
        <nav aria-label="Zoom path" className="text-ink-3 flex min-w-0 flex-1 items-center gap-2 truncate">
          <span className={focus.id === ROOT_ID ? "text-foreground" : ""}>All subjects</span>
          {focusInfo.path.map((p, i) => (
            <span key={i} className="flex items-center gap-2">
              <span aria-hidden="true">&rsaquo;</span>
              <span className={i === focusInfo.path.length - 1 ? "text-foreground" : ""}>{p}</span>
            </span>
          ))}
        </nav>
        <button
          type="button"
          onClick={() => zoomOut(false)}
          disabled={focus.id === ROOT_ID}
          className="border-hairline-strong text-ink-2 hover:text-foreground focus-visible:outline-world rounded-full border px-3 py-1 tracking-[0.08em] uppercase focus-visible:outline-2 focus-visible:outline-offset-2"
          style={{
            opacity: focus.id === ROOT_ID ? 0 : 1,
            transform: focus.id === ROOT_ID ? "translateX(6px)" : "none",
            pointerEvents: focus.id === ROOT_ID ? "none" : "auto",
            transition: `opacity var(--dur-base) ${EASE_SOFT}, transform var(--dur-base) ${EASE_SOFT}`,
          }}
        >
          &larr; Back
        </button>
      </div>

      <div ref={box} className="relative mx-auto" style={{ width: S, height: S }} onPointerLeave={() => setTip(null)}>
        <svg
          width={S}
          height={S}
          viewBox={`${-S / 2} ${-S / 2} ${S} ${S}`}
          role="group"
          aria-label={`${name}. Arrow keys move between segments, Enter zooms in, Escape zooms out.`}
          onKeyDown={onKeyDown}
          className="block overflow-visible select-none"
        >
          {nodes
            .filter((n) => n.depth >= 1)
            .map((n) => {
              const vis = visible(n.id)
              const dim = lineage !== null && !lineage.has(n.id)
              const hasKids = n.childIds.length > 0
              return (
                <g
                  key={n.id}
                  ref={(el) => {
                    if (el) groupRefs.current.set(n.id, el)
                    else groupRefs.current.delete(n.id)
                  }}
                  data-arc={vis ? n.id : undefined}
                  role={vis ? "button" : undefined}
                  aria-hidden={vis ? undefined : true}
                  tabIndex={vis ? (n.id === rovingId ? 0 : -1) : undefined}
                  aria-label={
                    vis
                      ? `${pathLabels(n).join(" › ")}. ${n.value} ${unit}, ${pctOf(n)}% mastery.${hasKids ? " Press Enter to zoom in." : ""}`
                      : undefined
                  }
                  className="chart-focusable"
                  style={{
                    opacity: dim ? 0.2 : 1,
                    transition: `opacity var(--dur-base) ${EASE_SOFT}`,
                    pointerEvents: vis ? "auto" : "none",
                    cursor: vis ? (hasKids ? "zoom-in" : "pointer") : "default",
                  }}
                  onPointerEnter={() => vis && setHoverId(n.id)}
                  onPointerMove={(e) => {
                    const r = box.current?.getBoundingClientRect()
                    if (vis && r) setTipAt(n.id, e.clientX - r.left, e.clientY - r.top)
                  }}
                  onPointerLeave={() => setHoverId(null)}
                  onFocus={(e) => {
                    setActiveId(n.id)
                    if (e.currentTarget.matches(":focus-visible")) {
                      setKbdId(n.id)
                      const st = target.get(n.id)
                      if (st) {
                        const c = arcCentroid(st)
                        setTipAt(n.id, c.x + S / 2, c.y + S / 2)
                      }
                    }
                  }}
                  onBlur={() => {
                    setKbdId(null)
                    setTip(null)
                  }}
                  onClick={() => vis && activate(n.id, false)}
                >
                  <path ref={(el) => { if (el) pathRefs.current.set(n.id, el); else pathRefs.current.delete(n.id) }} fill={fillOf(n)} stroke="var(--card)" strokeWidth={2} strokeLinejoin="round" />
                  <text
                    ref={(el) => { if (el) labelRefs.current.set(n.id, el); else labelRefs.current.delete(n.id) }}
                    textAnchor="middle"
                    dominantBaseline="central"
                    opacity={0}
                    pointerEvents="none"
                    className="fill-foreground font-mono text-[11px]"
                  >
                    {n.label}
                  </text>
                </g>
              )
            })}
          <path ref={ringRef} fill="none" stroke="var(--world)" strokeWidth={3} strokeLinejoin="round" pointerEvents="none" aria-hidden="true" />
        </svg>

        <div
          key={focus.id}
          className={cn("chart-fade absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center text-center", focus.id !== ROOT_ID && "cursor-zoom-out")}
          style={{ width: hole * 2 - 18, height: hole * 2 - 18, borderRadius: "50%" }}
          onClick={() => zoomOut(false)}
        >
          {typeof centre === "function" ? centre(focusInfo) : (centre ?? defaultCentre)}
        </div>

        <ChartTooltip x={tip?.x ?? 0} y={tip?.y ?? 0} containerW={S} containerH={S} visible={tip !== null && tipNode !== undefined} placement="above">
          {tipNode ? (
            <>
              <div className="text-ink-3 mb-1 tracking-[0.12em] uppercase">{pathLabels(tipNode).join(" › ")}</div>
              <div className="flex items-center gap-4">
                <span className="text-ink-2">
                  {tipNode.value} {unit}
                </span>
                <span className="text-foreground ml-auto tabular-nums">{pctOf(tipNode)}% mastery</span>
              </div>
              <div aria-hidden="true" className="bg-hairline mt-1.5 h-1 overflow-hidden rounded-full">
                <div className="bg-foreground h-full rounded-full" style={{ width: `${pctOf(tipNode)}%` }} />
              </div>
            </>
          ) : null}
        </ChartTooltip>
      </div>
      <p id={`${uid}-live`} className="sr-only" aria-live="polite">
        {announce}
      </p>
    </div>
  )
}
