import { useId, type ComponentProps, type ReactNode } from "react"

import { cn } from "@/lib/utils"
import { Eyebrow } from "./Eyebrow"
import { HEADER, SURFACE } from "./surface"
import "./kit.css"

type PanelProps = Omit<ComponentProps<"section">, "title"> & {
  eyebrow?: ReactNode
  title?: ReactNode
  /** Right-aligned header controls. */
  actions?: ReactNode
}

/** Card surface: 1px hairline, 16px radius, inner top highlight. Shares its padding, header and shadow with ChartFrame. */
export function Panel({ eyebrow, title, actions, className, children, ...props }: PanelProps) {
  const titleId = useId()
  const hasHeader = eyebrow || title || actions
  return (
    <section
      data-slot="kit-panel"
      aria-labelledby={title ? titleId : undefined}
      className={cn(SURFACE, className)}
      {...props}
    >
      {hasHeader ? (
        <header className={HEADER}>
          <div className="flex min-w-0 flex-col gap-2.5">
            {eyebrow ? <Eyebrow rule>{eyebrow}</Eyebrow> : null}
            {title ? (
              <h2 id={titleId} className="text-[17px] leading-snug font-medium tracking-[-0.01em]">
                {title}
              </h2>
            ) : null}
          </div>
          {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
        </header>
      ) : null}
      {children}
    </section>
  )
}
