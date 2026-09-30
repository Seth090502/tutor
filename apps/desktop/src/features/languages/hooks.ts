// Screen-level lookups: whether audio can work, the language's agent id and its chosen voices.
import { useCallback, useEffect, useState } from "react"

import type { TargetLanguage } from "./_pending/language"
import { formatVoiceIds, parseVoiceIds } from "./hvpt"
import { platform, type Voice } from "./platform"

export type AudioEnv = {
  loading: boolean
  inTauri: boolean
  hasKey: boolean
  /** Why audio is off, as a sentence; null when it works. */
  blocker: string | null
}

export const NO_SHELL = "Audio plays in the desktop app, which holds your ElevenLabs key. Start it with npm run tauri dev."
export const NO_KEY = "Add your ElevenLabs API key in Settings to hear audio."

export function blockerFor(inTauri: boolean, hasKey: boolean): string | null {
  if (!inTauri) return NO_SHELL
  return hasKey ? null : NO_KEY
}

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))

export function useAudioEnv(): AudioEnv {
  const [env, setEnv] = useState<AudioEnv>(() => {
    const inTauri = platform().inTauri
    return { loading: inTauri, inTauri, hasKey: false, blocker: inTauri ? null : NO_SHELL }
  })

  useEffect(() => {
    const p = platform()
    if (!p.inTauri) return
    let cancelled = false
    p.hasElevenLabsKey().then(
      (hasKey) => {
        if (!cancelled) setEnv({ loading: false, inTauri: true, hasKey, blocker: blockerFor(true, hasKey) })
      },
      () => {
        if (!cancelled) setEnv({ loading: false, inTauri: true, hasKey: false, blocker: "Could not read your secrets. Open Settings." })
      },
    )
    return () => {
      cancelled = true
    }
  }, [])

  return env
}

export type Lookup<T> = { status: "loading" } | { status: "ready"; value: T } | { status: "error"; message: string }

/** The agent id in config key `agent_<lang>`, or null when it is not set. */
export function useAgentId(lang: TargetLanguage) {
  const [state, setState] = useState<Lookup<string | null>>({ status: "loading" })
  const [version, setVersion] = useState(0)

  useEffect(() => {
    let cancelled = false
    setState({ status: "loading" })
    platform()
      .configValue(`agent_${lang}`)
      .then(
        (value) => !cancelled && setState({ status: "ready", value: value?.trim() || null }),
        (e: unknown) => !cancelled && setState({ status: "error", message: errorText(e) }),
      )
    return () => {
      cancelled = true
    }
  }, [lang, version])

  const save = useCallback(
    async (id: string) => {
      await platform().configSet(`agent_${lang}`, id.trim())
      setVersion((v) => v + 1)
    },
    [lang],
  )
  return { state, save }
}

/** The voices the learner chose for this language (`voice_<lang>`), and the catalogue to choose from. */
export function useVoices(lang: TargetLanguage, enabled: boolean) {
  const [chosen, setChosen] = useState<Lookup<string[]>>({ status: "loading" })
  const [catalogue, setCatalogue] = useState<Lookup<Voice[]>>({ status: "loading" })

  useEffect(() => {
    if (!enabled) return
    let cancelled = false
    setChosen({ status: "loading" })
    setCatalogue({ status: "loading" })
    const p = platform()
    p.configValue(`voice_${lang}`).then(
      (v) => !cancelled && setChosen({ status: "ready", value: parseVoiceIds(v) }),
      (e: unknown) => !cancelled && setChosen({ status: "error", message: errorText(e) }),
    )
    p.ttsVoices(lang).then(
      (voices) => !cancelled && setCatalogue({ status: "ready", value: voices }),
      (e: unknown) => !cancelled && setCatalogue({ status: "error", message: errorText(e) }),
    )
    return () => {
      cancelled = true
    }
  }, [lang, enabled])

  const save = useCallback(
    async (ids: string[]) => {
      await platform().configSet(`voice_${lang}`, formatVoiceIds(ids))
      setChosen({ status: "ready", value: parseVoiceIds(formatVoiceIds(ids)) })
    },
    [lang],
  )
  return { chosen, catalogue, save }
}
