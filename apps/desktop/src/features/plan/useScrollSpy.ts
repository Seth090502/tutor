import { useEffect, useState, type RefObject } from "react"

/** The nearest ancestor that scrolls (the shell's content pane), or null when the window does. */
function scrollParent(el: HTMLElement): HTMLElement | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const overflow = getComputedStyle(p).overflowY
    if (overflow === "auto" || overflow === "scroll") return p
  }
  return null
}

/**
 * Drives the reading-progress hairline (scaleX, written straight to the element) and returns the
 * id of the heading being read: the last one whose top has passed `offset` px below the pane top.
 * Work happens on scroll and resize, one frame at a time; nothing runs while the page is idle.
 */
export function useScrollSpy(
  root: RefObject<HTMLElement | null>,
  bar: RefObject<HTMLElement | null>,
  ids: string[],
  offset = 140,
): string | null {
  const [active, setActive] = useState<string | null>(null)
  const key = ids.join("\n")

  useEffect(() => {
    const el = root.current
    if (!el) return
    const scroller = scrollParent(el)
    const source: HTMLElement | Window = scroller ?? window
    const list = key ? key.split("\n") : []
    let frame = 0

    const update = () => {
      frame = 0
      const top = scroller ? scroller.scrollTop : window.scrollY
      const max = scroller
        ? scroller.scrollHeight - scroller.clientHeight
        : document.documentElement.scrollHeight - window.innerHeight
      const progress = max > 0 ? Math.min(1, Math.max(0, top / max)) : 0
      if (bar.current) bar.current.style.transform = `scaleX(${progress})`

      const line = (scroller ? scroller.getBoundingClientRect().top : 0) + offset
      let current = list[0] ?? null
      for (const id of list) {
        const h = document.getElementById(id)
        if (h && h.getBoundingClientRect().top <= line) current = id
      }
      if (max > 0 && progress > 0.995) current = list[list.length - 1] ?? current
      setActive(current)
    }
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }

    update()
    source.addEventListener("scroll", schedule, { passive: true })
    window.addEventListener("resize", schedule)
    // The document grows when the plan loads or is refreshed.
    const resize = new ResizeObserver(schedule)
    resize.observe(el)
    return () => {
      if (frame) cancelAnimationFrame(frame)
      source.removeEventListener("scroll", schedule)
      window.removeEventListener("resize", schedule)
      resize.disconnect()
    }
  }, [root, bar, key, offset])

  return active
}
