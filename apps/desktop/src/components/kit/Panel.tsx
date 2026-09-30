import { useId, type ComponentProps, type ReactNode } from "react"

import { cn } from "@/lib/utils"
import { Eyebrow } from "./Eyebrow"
import "./kit.css"

type PanelProps = Omit<ComponentProps<"section">, "title"> & {
  eyebrow?: ReactNode
  title?: ReactNode
  /** Right-aligned header controls. */
  actions?: ReactNode
}

/** Card surface: 1px hairline, 16px radius, inner top highlight. */
export function Panel({ eyebrow, title, actions, className, children, ...props }: PanelProps) {
  const titleId = useId()
  const hasHeader = eyebrow || title || actions
  return (
    <section
      data-slot="kit-panel"
      aria-labelledby={title ? titleId : undefined}
      className={cn(
        "bg-card text-card-foreground border-hairline rounded-2xl border p-6 [box-shadow:inset_0_1px_0_var(--kit-highlight),var(--kit-shadow)]",
        className,
      )}
      {...props}
    >
      {hasHeader ? (
        <header className="mb-5 flex items-start justify-between gap-4">
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
