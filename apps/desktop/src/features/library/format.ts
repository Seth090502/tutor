// Number and error formatting for the Library screen.
import { HttpError } from "@/lib/backend"

const INT = new Intl.NumberFormat("en-US")
const COMPACT = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 2 })
const finite = (n: number | null | undefined): n is number => typeof n === "number" && Number.isFinite(n)

export const fmtInt = (n: number | null | undefined) => (finite(n) ? INT.format(Math.round(n)) : "—")
export const fmtCompact = (n: number | null | undefined) => (finite(n) ? COMPACT.format(n) : "—")
export const fmtPct = (part: number, whole: number) => (whole > 0 ? `${((100 * part) / whole).toFixed(1)}%` : "—")

/** Decimal units, matching the survey's GB = 1e9 bytes. */
export function fmtBytes(n: number | null | undefined) {
  if (!finite(n)) return "—"
  const units = ["B", "kB", "MB", "GB", "TB"]
  let v = n
  let i = 0
  while (Math.abs(v) >= 1000 && i < units.length - 1) {
    v /= 1000
    i++
  }
  return `${i === 0 ? v : v.toFixed(v < 10 ? 2 : 1)} ${units[i]}`
}

/** Text for any thrown value; prefers the `error`/`detail` field of a service's JSON error body. */
export function errorText(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e)
  if (!(e instanceof HttpError)) return msg
  const at = msg.indexOf("{")
  if (at >= 0) {
    try {
      const body = JSON.parse(msg.slice(at)) as { error?: unknown; detail?: unknown }
      if (typeof body.error === "string") return body.error
      if (typeof body.detail === "string") return body.detail
    } catch {
      // not JSON: fall through to the raw message
    }
  }
  return msg
}
