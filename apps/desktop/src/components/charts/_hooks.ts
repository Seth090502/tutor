import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react"

const useIsoLayoutEffect = typeof window === "undefined" ? useEffect : useLayoutEffect

/** Width of an element in px, measured before first paint and kept current. Ignores 0 (display: none). */
export function useElementWidth<T extends HTMLElement>(fallback: number): [RefObject<T | null>, number] {
  const ref = useRef<T | null>(null)
  const [width, setWidth] = useState(fallback)
  useIsoLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const read = () => {
      const w = Math.round(el.getBoundingClientRect().width)
      if (w > 0) setWidth((prev) => (prev === w ? prev : w))
    }
    read()
    if (typeof ResizeObserver === "undefined") return
    const ro = new ResizeObserver(read)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])
  return [ref, width]
}

/** true one frame after mount, so a CSS transition has a start state to leave. */
export function useArmed(enabled: boolean): boolean {
  const [armed, setArmed] = useState(!enabled)
  useEffect(() => {
    if (armed) return
    let inner = 0
    const outer = requestAnimationFrame(() => {
      inner = requestAnimationFrame(() => setArmed(true))
    })
    return () => {
      cancelAnimationFrame(outer)
      cancelAnimationFrame(inner)
    }
  }, [armed])
  return armed
}
