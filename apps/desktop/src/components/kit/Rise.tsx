import type { ComponentProps, CSSProperties, ElementType } from "react"

import { cn } from "@/lib/utils"
import "./kit.css"

/** Style that sets the stagger step; use it with the `kit-rise` class on an element that is already there. */
export function riseStyle(i: number, style?: CSSProperties): CSSProperties {
  return { ...style, "--i": i } as CSSProperties
}

type RiseProps = ComponentProps<"div"> & {
  /** Place in the stagger: each step waits 70 ms more, capped at step 9 (the CSS does the capping). */
  i?: number
  /** False renders the block in place with no entrance (for a screen that has played its intro already). */
  enter?: boolean
  as?: Extract<ElementType, "div" | "section" | "li" | "header" | "footer" | "aside">
}

/**
 * One-shot entrance: rises 8px and fades in when it mounts, then never again. Transform and opacity
 * only; removed under reduced motion. The class is `kit-rise` for elements that cannot be wrapped.
 */
export function Rise({ i = 0, enter = true, as = "div", className, style, children, ...props }: RiseProps) {
  const Tag = as as "div"
  return (
    <Tag data-slot="kit-rise" className={cn("min-w-0", enter && "kit-rise", className)} style={riseStyle(i, style)} {...props}>
      {children}
    </Tag>
  )
}
