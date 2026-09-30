// The ambient layer behind everything: two world-tinted washes, a vignette and a grain tile.
// It is static in "lite" quality. In "rich" quality (and not under reduced motion) the washes
// drift very slowly with CSS transforms, paused while the page is hidden.

import { useState } from "react"

import { usePageVisible } from "@/lib/motion"

import { grainBackground } from "./grain"
import "./shell.css"

export function Ambient() {
  const visible = usePageVisible()
  const [grain] = useState(grainBackground)

  return (
    <div className="shell-ambient" aria-hidden="true" data-paused={!visible}>
      <div className="shell-wash shell-wash-1" />
      <div className="shell-wash shell-wash-2" />
      <div className="shell-vignette" />
      {grain && <div className="shell-grain" style={{ backgroundImage: grain }} />}
    </div>
  )
}
