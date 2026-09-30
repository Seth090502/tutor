import type { ComponentType } from "react"

export type Section = {
  id: string
  label: string
  icon: ComponentType<{ className?: string }>
  subtitle: string
  Screen: ComponentType
}
