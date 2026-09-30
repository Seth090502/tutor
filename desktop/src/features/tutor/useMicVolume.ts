import { useEffect, useRef, useState } from "react"

/** Live microphone level (0-1) in a ref that the Orb reads every frame. */
export function useMicVolume(enabled: boolean) {
  const level = useRef(0)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!enabled) return
    setError(null)
    let stopped = false
    let frame = 0
    let stream: MediaStream | undefined
    let ctx: AudioContext | undefined

    navigator.mediaDevices.getUserMedia({ audio: true }).then(
      (s) => {
        if (stopped) {
          s.getTracks().forEach((t) => t.stop())
          return
        }
        stream = s
        ctx = new AudioContext()
        const analyser = ctx.createAnalyser()
        analyser.fftSize = 512
        ctx.createMediaStreamSource(s).connect(analyser)
        const buf = new Float32Array(analyser.fftSize)
        const tick = () => {
          analyser.getFloatTimeDomainData(buf)
          let sum = 0
          for (const v of buf) sum += v * v
          // RMS of speech sits around 0.02-0.2; scale so normal talking fills the range.
          level.current = Math.min(1, Math.sqrt(sum / buf.length) * 5)
          frame = requestAnimationFrame(tick)
        }
        tick()
      },
      (e: unknown) => setError(e instanceof Error ? e.message : String(e)),
    )

    return () => {
      stopped = true
      cancelAnimationFrame(frame)
      stream?.getTracks().forEach((t) => t.stop())
      void ctx?.close()
      level.current = 0
    }
  }, [enabled])

  return { level, error }
}
