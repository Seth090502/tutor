/**
 * CSS linear() easing that follows a damped spring. The curve is normalised to the unit
 * interval, so it only depends on the damping ratio: the element's duration comes from the
 * --dur-* tokens (and collapses under reduced motion). `settle` is the residual amplitude at
 * t = 1; the last point is forced to exactly 1.
 */
export function springLinear(zeta: number, settle = 0.01, samples = 40): string {
  if (!(zeta > 0 && zeta < 1)) throw new RangeError("zeta must be in (0, 1)")
  const decay = Math.log(1 / settle) / zeta // w0 * T
  const wd = Math.sqrt(1 - zeta * zeta) * decay
  const k = zeta / Math.sqrt(1 - zeta * zeta)
  const pts: string[] = []
  for (let i = 0; i <= samples; i++) {
    const t = i / samples
    const x =
      i === samples
        ? 1
        : 1 - Math.exp(-zeta * decay * t) * (Math.cos(wd * t) + k * Math.sin(wd * t))
    const r = Math.round(x * 1000) / 1000
    pts.push(String(r).replace(/^0\./, ".").replace(/^-0\./, "-."))
  }
  pts[0] = "0"
  return `linear(${pts.join(", ")})`
}
