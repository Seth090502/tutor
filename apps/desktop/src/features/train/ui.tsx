// Form pieces specific to the Train screen. The general controls (Segmented, Stepper, SwitchRow) live in the kit.
import { useEffect, useState, type ReactNode } from "react"

import { Eyebrow, Kbd } from "@/components/kit"
import { cn } from "@/lib/utils"
import { keyLabel } from "./parts"

const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-world"

/** A labelled control cell: mono eyebrow above the control. */
export function Field({ label, hint, children, className }: { label: string; hint?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <div className={cn("flex min-w-0 flex-col gap-2.5", className)}>
      <Eyebrow>{label}</Eyebrow>
      {children}
      {hint ? <p className="text-ink-3 text-xs leading-snug">{hint}</p> : null}
    </div>
  )
}

/** Click, then press a key to remap. Escape cancels. */
export function KeyPicker({ label, code, onPick }: { label: string; code: string; onPick: (code: string) => void }) {
  const [listening, setListening] = useState(false)
  useEffect(() => {
    if (!listening) return
    const onKey = (e: globalThis.KeyboardEvent) => {
      e.preventDefault()
      setListening(false)
      if (e.code !== "Escape") onPick(e.code)
    }
    window.addEventListener("keydown", onKey, { once: true })
    return () => window.removeEventListener("keydown", onKey)
  }, [listening, onPick])
  return (
    <button
      type="button"
      onClick={() => setListening(true)}
      aria-label={`${label} key: ${keyLabel(code)}. Activate, then press a new key. Escape cancels.`}
      className={cn(
        "border-hairline-strong bg-card flex h-10 items-center gap-3 rounded-xl border pr-1.5 pl-3.5 text-sm transition-colors",
        FOCUS,
        listening && "border-world",
      )}
    >
      <span className="text-ink-2">{label}</span>
      <Kbd className="m-0 min-w-9 px-2 py-1 text-[13px]">{listening ? "press a key" : keyLabel(code)}</Kbd>
    </button>
  )
}
