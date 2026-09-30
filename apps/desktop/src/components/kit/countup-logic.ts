export type CountPlan = "snap" | "tween"

/**
 * How a CountUp gets to a new value. It snaps under reduced motion, with no duration, or when the
 * value is already shown. In `once` mode only the first run tweens; later values snap.
 */
export function countPlan(o: { once: boolean; ran: boolean; reduced: boolean; duration: number; shown: number; value: number }): CountPlan {
  if (o.reduced || o.duration <= 0 || o.shown === o.value) return "snap"
  if (o.once && o.ran) return "snap"
  return "tween"
}
