import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react"

import { useSpring } from "@/lib/motion"

const PILL_SPRING = { stiffness: 260, damping: 20, precision: 0.05 }

/**
 * Glides an indicator element to the active item with a spring (stiffness 260, damping 20).
 * Items and the indicator must share an offset parent. The indicator is sized to the active item.
 * Returns whether the first measurement is done, so the indicator can stay hidden until then.
 */
export function useGlide(
  axis: "x" | "y",
  activeIndex: number,
  items: RefObject<(HTMLElement | null)[]>,
  indicator: RefObject<HTMLElement | null>,
  container: RefObject<HTMLElement | null>,
) {
  const [target, setTarget] = useState(0)
  const [ready, setReady] = useState(false)

  const { jump } = useSpring(target, PILL_SPRING, (v) => {
    const el = indicator.current
    if (el) el.style.transform = axis === "y" ? `translate3d(0, ${v}px, 0)` : `translate3d(${v}px, 0, 0)`
  })

  const measure = useCallback(() => {
    const item = items.current?.[activeIndex]
    const pill = indicator.current
    if (!item || !pill) return null
    pill.style.width = `${item.offsetWidth}px`
    pill.style.height = `${item.offsetHeight}px`
    return axis === "y" ? item.offsetTop : item.offsetLeft
  }, [axis, activeIndex, items, indicator])

  useLayoutEffect(() => {
    const position = measure()
    if (position === null) return
    if (!ready) {
      jump(position)
      setReady(true)
    }
    setTarget(position)
  }, [measure, jump, ready])

  // Layout changes (window resize, fonts arriving) move the item without a glide. The observer is
  // created once and reads the latest measure, so changing the active item never re-fires it.
  const measureRef = useRef(measure)
  useLayoutEffect(() => {
    measureRef.current = measure
  })
  useLayoutEffect(() => {
    const el = container.current
    if (!el) return
    const observer = new ResizeObserver(() => {
      const position = measureRef.current()
      if (position === null) return
      jump(position)
      setTarget(position)
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [container, jump])

  return ready
}
