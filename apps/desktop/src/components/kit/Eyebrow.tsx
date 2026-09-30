import type { ComponentProps, ElementType } from "react"

import { cn } from "@/lib/utils"

type EyebrowProps = ComponentProps<"span"> & {
  /** Leading 24px hairline. */
  rule?: boolean
  as?: Extract<ElementType, "span" | "div" | "p" | "h2" | "h3">
}

/** Mono, uppercase, 11px, tracked: the small label above numbers and titles. */
export function Eyebrow({ rule = false, as = "span", className, children, ...props }: EyebrowProps) {
  const Tag = as as "span"
  return (
    <Tag
      data-slot="kit-eyebrow"
      className={cn(
        "text-ink-3 inline-flex items-center gap-3 font-mono text-[11px] leading-none font-normal tracking-[0.18em] uppercase",
        className,
      )}
      {...props}
    >
      {rule ? <span aria-hidden className="bg-hairline-strong h-px w-6 shrink-0" /> : null}
      {children}
    </Tag>
  )
}
