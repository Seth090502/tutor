// Settings: services, API keys and the ElevenLabs agent id (Tauri commands: contract C3).
import { useCallback, useEffect, useState, type CSSProperties, type FormEvent, type ReactNode } from "react"
import { ChevronRight, RotateCw, Save, Trash2 } from "lucide-react"

import { Eyebrow, LedStatus, ServiceRow, type LedTone } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { DesktopRequired } from "@/features/plan/DesktopRequired"
import { inTauri, shell, type SecretName, type ServiceState, type ServicesStatus } from "@/lib/backend"
import "@/features/plan/rise.css"

const KEYS: { name: SecretName; label: string; use: string; env?: string }[] = [
  { name: "elevenlabs", label: "ElevenLabs", use: "Voice tutor conversations" },
  { name: "opencode", label: "OpenCode", use: "Backend LLM", env: "OPENCODE_API_KEY" },
  { name: "anthropic", label: "Anthropic", use: "Backend LLM", env: "ANTHROPIC_API_KEY" },
  { name: "openrouter", label: "OpenRouter", use: "Backend LLM", env: "OPENROUTER_API_KEY" },
  { name: "voyage", label: "Voyage", use: "rag-engine embeddings", env: "VOYAGE_API_KEY" },
]

const SERVICES: { name: keyof ServicesStatus; label: string }[] = [
  { name: "backend", label: "DeepTutor backend" },
  { name: "rag_engine", label: "rag-engine" },
]

const STATE_STYLE: Record<ServiceState["state"], { label: string; tone: LedTone; pulse?: boolean }> = {
  starting: { label: "Starting", tone: "warn", pulse: true },
  running: { label: "Running", tone: "ok" },
  stopped: { label: "Stopped", tone: "off" },
  failed: { label: "Failed", tone: "bad" },
}

// `::-ms-reveal` hides the WebView's own show-password eye: a typed key is never displayed either.
const inputClass =
  "[&::-ms-reveal]:hidden border-hairline-strong placeholder:text-muted-foreground h-9 min-w-0 flex-1 rounded-lg border bg-transparent px-3 font-mono text-[13px] tracking-[0.02em] outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"

function errorText(e: unknown) {
  return e instanceof Error ? e.message : String(e)
}

function Group({ index, title, note, children }: { index: number; title: string; note: ReactNode; children: ReactNode }) {
  return (
    <section className="screen-rise flex flex-col" style={{ "--i": index } as CSSProperties} aria-labelledby={`settings-${index}`}>
      <header className="mb-4 flex flex-col gap-3">
        <Eyebrow rule className="text-muted-foreground">{String(index + 1).padStart(2, "0")}</Eyebrow>
        <div>
          <h2 id={`settings-${index}`} className="text-[19px] leading-snug font-medium tracking-[-0.015em]">
            {title}
          </h2>
          <p className="text-ink-2 mt-1 max-w-[60ch] text-[14px] leading-relaxed text-pretty">{note}</p>
        </div>
      </header>
      {children}
    </section>
  )
}

function ErrorLine({ text }: { text: string | null }) {
  return text ? (
    <p role="alert" className="border-bad/60 text-bad border-l-2 pl-3 font-mono text-[12px] leading-relaxed whitespace-pre-wrap">
      {text}
    </p>
  ) : null
}

function ServiceItem({
  label,
  service,
  unknown,
  onRestart,
}: {
  label: string
  service: ServiceState | undefined
  /** Status could not be read (and nothing is known yet). */
  unknown: boolean
  onRestart: () => Promise<void>
}) {
  const [restarting, setRestarting] = useState(false)
  const style = service ? STATE_STYLE[service.state] : null
  const firstLine = service?.detail?.trim().split("\n")[0] ?? ""

  const restart = async () => {
    setRestarting(true)
    try {
      await onRestart()
    } finally {
      setRestarting(false)
    }
  }

  return (
    <li className="border-hairline border-b py-1.5 first:border-t">
      <div className="flex items-center gap-4">
        <ServiceRow
          className="min-w-0 flex-1"
          label={label}
          detail={<span className="text-muted-foreground">{service?.url ?? "…"}</span>}
          tone={style?.tone ?? "off"}
          pulse={style?.pulse}
          state={style?.label ?? (unknown ? "Unknown" : "Checking")}
          aria-live="polite"
        />
        <Button size="sm" variant="outline" onClick={() => void restart()} disabled={!service || restarting} aria-label={`Restart ${label}`}>
          <RotateCw />
          Restart
        </Button>
      </div>
      {service?.detail && (
        <details className="group/detail pb-2.5">
          <summary className="text-ink-2 hover:text-foreground flex cursor-pointer list-none items-center gap-2 rounded-sm font-mono text-[11px] tracking-[0.14em] uppercase outline-none focus-visible:ring-2 focus-visible:ring-ring/60 [&::-webkit-details-marker]:hidden">
            <ChevronRight aria-hidden className="size-3.5 shrink-0 transition-transform group-open/detail:rotate-90 motion-reduce:transition-none" />
            <span className="shrink-0">{service.state === "failed" ? "Error detail" : "Detail"}</span>
            <span className="text-muted-foreground min-w-0 truncate tracking-normal normal-case group-open/detail:hidden">{firstLine}</span>
          </summary>
          <pre className="border-hairline-strong text-ink-2 mt-2.5 max-h-56 overflow-auto border-l pl-4 font-mono text-[12px] leading-relaxed break-words whitespace-pre-wrap">
            {service.detail}
          </pre>
        </details>
      )}
    </li>
  )
}

function ServicesSection() {
  const [status, setStatus] = useState<ServicesStatus | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(() => {
    shell.servicesStatus().then(
      (s) => {
        setStatus(s)
        setError(null)
      },
      (e) => setError(errorText(e)),
    )
  }, [])

  // Poll every 2 s, but not while the window is hidden; catch up as soon as it is shown again.
  useEffect(() => {
    refresh()
    const id = setInterval(() => {
      if (!document.hidden) refresh()
    }, 2000)
    const onVisible = () => {
      if (!document.hidden) refresh()
    }
    document.addEventListener("visibilitychange", onVisible)
    return () => {
      clearInterval(id)
      document.removeEventListener("visibilitychange", onVisible)
    }
  }, [refresh])

  const restart = (name: keyof ServicesStatus) => shell.servicesRestart(name).then(refresh, (e) => setError(errorText(e)))

  return (
    <Group
      index={0}
      title="Services"
      note="Started with the app and stopped when it closes. Keys reach a service when it starts, so restart it after changing its key."
    >
      <div className="flex flex-col gap-3">
        <ErrorLine text={error} />
        <ul>
          {SERVICES.map((s) => (
            <ServiceItem key={s.name} label={s.label} service={status?.[s.name]} unknown={error !== null && status === null} onRestart={() => restart(s.name)} />
          ))}
        </ul>
      </div>
    </Group>
  )
}

function KeyRow({
  spec,
  saved,
  onChange,
}: {
  spec: (typeof KEYS)[number]
  saved: boolean | undefined
  onChange: () => void
}) {
  const [value, setValue] = useState("")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const id = `key-${spec.name}`

  const run = async (action: () => Promise<void>) => {
    setBusy(true)
    setError(null)
    try {
      await action()
      setValue("")
      onChange()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const submit = (e: FormEvent) => {
    e.preventDefault()
    if (value.trim()) void run(() => shell.secretSet(spec.name, value))
  }

  return (
    <li className="border-hairline border-b first:border-t">
      <form className="flex flex-col gap-2.5 py-3.5" onSubmit={submit}>
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <label htmlFor={id} className="text-[15px] font-medium">
              {spec.label}
            </label>
            <p className="text-ink-2 mt-0.5 text-[13px]">
              {spec.use}
              {spec.env && (
                <>
                  , as <code className="font-mono text-[12px]">{spec.env}</code>
                </>
              )}
            </p>
          </div>
          <LedStatus
            tone={saved ? "ok" : "off"}
            state={saved === undefined ? "Checking" : saved ? "Saved" : "Missing"}
            className="pt-1"
            aria-live="polite"
          />
        </div>
        <div className="flex gap-2">
          <input
            id={id}
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder={saved ? "Paste a new key to replace the saved one" : "Paste the key"}
            className={inputClass}
          />
          <Button type="submit" variant={value.trim() ? "default" : "outline"} className="h-9" disabled={busy || !value.trim()}>
            <Save />
            Save
          </Button>
          {saved && (
            <Button
              type="button"
              variant="ghost"
              className="text-destructive hover:bg-destructive/10 hover:text-destructive h-9"
              disabled={busy}
              onClick={() => void run(() => shell.secretDelete(spec.name))}
            >
              <Trash2 />
              Delete
            </Button>
          )}
        </div>
        <ErrorLine text={error} />
      </form>
    </li>
  )
}

function KeysSection() {
  const [present, setPresent] = useState<Record<SecretName, boolean> | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(() => {
    shell.secretsStatus().then(
      (p) => {
        setPresent(p)
        setError(null)
      },
      (e) => setError(errorText(e)),
    )
  }, [])

  useEffect(refresh, [refresh])

  return (
    <Group
      index={1}
      title="API keys"
      note="Stored in Windows Credential Manager. This window only learns whether a key is saved, never its value."
    >
      <div className="flex flex-col gap-3">
        <ErrorLine text={error} />
        <ul>
          {KEYS.map((k) => (
            <KeyRow key={k.name} spec={k} saved={present?.[k.name]} onChange={refresh} />
          ))}
        </ul>
      </div>
    </Group>
  )
}

function AgentSection() {
  const [saved, setSaved] = useState<string | null | undefined>(undefined)
  const [value, setValue] = useState("")
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    const config = await shell.configGet()
    setSaved(config.elevenlabs_agent_id)
    setValue(config.elevenlabs_agent_id ?? "")
  }, [])

  useEffect(() => {
    load().catch((e) => setError(errorText(e)))
  }, [load])

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    try {
      await shell.configSet("elevenlabs_agent_id", value)
      await load()
      setError(null)
    } catch (err) {
      setError(errorText(err))
    }
  }

  const dirty = saved !== undefined && value.trim() !== (saved ?? "")

  return (
    <Group index={2} title="ElevenLabs agent" note="The Agents Platform agent the voice tutor talks to. Not a secret.">
      <form className="border-hairline flex flex-col gap-3 border-y py-4" onSubmit={submit}>
        <div className="flex items-start justify-between gap-4">
          <label htmlFor="elevenlabs-agent-id" className="text-[15px] font-medium">
            Agent id
          </label>
          <LedStatus
            tone={saved ? "ok" : "off"}
            state={saved === undefined ? "Checking" : saved ? "Saved" : "Not set"}
            className="pt-1"
            aria-live="polite"
          />
        </div>
        <div className="flex gap-2">
          <input
            id="elevenlabs-agent-id"
            autoComplete="off"
            spellCheck={false}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="agent_…"
            className={inputClass}
          />
          <Button type="submit" variant={dirty ? "default" : "outline"} className="h-9" disabled={!dirty}>
            <Save />
            Save
          </Button>
        </div>
        <ErrorLine text={error} />
      </form>
    </Group>
  )
}

export function SettingsScreen() {
  if (!inTauri) {
    return (
      <DesktopRequired title="Settings need the desktop app">
        Keys live in Windows Credential Manager and the services are started by the Tauri shell, so this screen works only
        inside the app. Run <code className="font-mono text-[13px]">npm run tauri dev</code> in{" "}
        <code className="font-mono text-[13px]">apps/desktop</code>.
      </DesktopRequired>
    )
  }
  return (
    <div className="flex max-w-3xl flex-col gap-12 px-8 pt-8 pb-24">
      <ServicesSection />
      <KeysSection />
      <AgentSection />
    </div>
  )
}
