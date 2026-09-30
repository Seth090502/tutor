import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react"
import { RotateCw, Save, Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import { inTauri, shell, type SecretName, type ServiceState, type ServicesStatus } from "@/lib/backend"
import { cn } from "@/lib/utils"

const KEYS: { name: SecretName; label: string; use: string }[] = [
  { name: "elevenlabs", label: "ElevenLabs", use: "Voice tutor conversations" },
  { name: "opencode", label: "OpenCode", use: "Backend LLM, as OPENCODE_API_KEY" },
  { name: "anthropic", label: "Anthropic", use: "Backend LLM, as ANTHROPIC_API_KEY" },
  { name: "openrouter", label: "OpenRouter", use: "Backend LLM, as OPENROUTER_API_KEY" },
  { name: "voyage", label: "Voyage", use: "rag-engine embeddings, as VOYAGE_API_KEY" },
]

const SERVICES: { name: keyof ServicesStatus; label: string }[] = [
  { name: "backend", label: "DeepTutor backend" },
  { name: "rag_engine", label: "rag-engine" },
]

const STATE_STYLE: Record<ServiceState["state"], { label: string; dot: string }> = {
  starting: { label: "Starting", dot: "bg-amber-500" },
  running: { label: "Running", dot: "bg-emerald-500" },
  stopped: { label: "Stopped", dot: "bg-muted-foreground" },
  failed: { label: "Failed", dot: "bg-destructive" },
}

const inputClass =
  "border-input bg-background h-8 min-w-0 flex-1 rounded-lg border px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"

function errorText(e: unknown) {
  return e instanceof Error ? e.message : String(e)
}

function Section({ title, note, children }: { title: string; note: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <div>
        <h2 className="text-sm font-semibold">{title}</h2>
        <p className="text-muted-foreground text-xs">{note}</p>
      </div>
      {children}
    </section>
  )
}

function ErrorLine({ text }: { text: string | null }) {
  return text ? (
    <p role="alert" className="text-destructive text-xs whitespace-pre-wrap">
      {text}
    </p>
  ) : null
}

function ServiceCard({
  label,
  service,
  onRestart,
}: {
  label: string
  service: ServiceState | undefined
  onRestart: () => void
}) {
  const style = service ? STATE_STYLE[service.state] : null
  return (
    <div className="bg-card flex min-w-0 flex-col gap-3 rounded-xl border p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="text-sm font-medium">{label}</h3>
          <p className="text-muted-foreground truncate text-xs">{service?.url ?? "…"}</p>
        </div>
        <Button size="sm" variant="outline" onClick={onRestart} disabled={!service}>
          <RotateCw />
          Restart
        </Button>
      </div>
      <p className="flex items-center gap-2 text-sm" aria-live="polite">
        <span className={cn("size-2 rounded-full", style?.dot ?? "bg-muted")} aria-hidden />
        {style?.label ?? "Loading"}
      </p>
      {service?.detail && (
        <pre className="bg-muted max-h-48 overflow-auto rounded-lg p-2 text-xs whitespace-pre-wrap">
          {service.detail}
        </pre>
      )}
    </div>
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

  useEffect(() => {
    refresh()
    const id = setInterval(refresh, 2000)
    return () => clearInterval(id)
  }, [refresh])

  const restart = (name: keyof ServicesStatus) =>
    shell.servicesRestart(name).then(refresh, (e) => setError(errorText(e)))

  return (
    <Section
      title="Services"
      note="Started with the app and stopped when it closes. Keys reach a service when it starts, so restart it after changing its key."
    >
      <ErrorLine text={error} />
      <div className="grid gap-3 md:grid-cols-2">
        {SERVICES.map((s) => (
          <ServiceCard key={s.name} label={s.label} service={status?.[s.name]} onRestart={() => restart(s.name)} />
        ))}
      </div>
    </Section>
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
    <form className="flex flex-col gap-2 p-4" onSubmit={submit}>
      <div className="flex items-center justify-between gap-3">
        <div>
          <label htmlFor={id} className="text-sm font-medium">
            {spec.label}
          </label>
          <p className="text-muted-foreground text-xs">{spec.use}</p>
        </div>
        <span
          className={cn(
            "rounded-full px-2 py-0.5 text-xs",
            saved ? "bg-emerald-500/15 text-emerald-700 dark:text-emerald-400" : "bg-muted text-muted-foreground",
          )}
        >
          {saved === undefined ? "…" : saved ? "Saved" : "Not set"}
        </span>
      </div>
      <div className="flex gap-2">
        <input
          id={id}
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={saved ? "Enter a new key to replace the saved one" : "Paste the key"}
          className={inputClass}
        />
        <Button type="submit" disabled={busy || !value.trim()}>
          <Save />
          Save
        </Button>
        {saved && (
          <Button
            type="button"
            variant="destructive"
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
    <Section
      title="API keys"
      note="Stored in Windows Credential Manager. This window only learns whether a key is saved, never its value."
    >
      <ErrorLine text={error} />
      <div className="bg-card divide-y rounded-xl border">
        {KEYS.map((k) => (
          <KeyRow key={k.name} spec={k} saved={present?.[k.name]} onChange={refresh} />
        ))}
      </div>
    </Section>
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

  return (
    <Section title="ElevenLabs agent" note="The Agents Platform agent the voice tutor talks to. Not a secret.">
      <form className="bg-card flex flex-col gap-2 rounded-xl border p-4" onSubmit={submit}>
        <label htmlFor="elevenlabs-agent-id" className="text-sm font-medium">
          Agent id
        </label>
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
          <Button type="submit" disabled={saved === undefined || value.trim() === (saved ?? "")}>
            <Save />
            Save
          </Button>
        </div>
        <ErrorLine text={error} />
      </form>
    </Section>
  )
}

export function SettingsScreen() {
  if (!inTauri) {
    return (
      <div className="mx-auto flex h-full max-w-2xl flex-col justify-center p-8">
        <div className="bg-card rounded-xl border p-6">
          <h3 className="mb-2 text-sm font-medium">Settings need the desktop app</h3>
          <p className="text-muted-foreground text-sm">
            Keys live in Windows Credential Manager and the services are started by the Tauri shell, so this
            screen works only inside the app. Run <code>npm run tauri dev</code> in <code>apps/desktop</code>.
          </p>
        </div>
      </div>
    )
  }
  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-8 p-8">
      <ServicesSection />
      <KeysSection />
      <AgentSection />
    </div>
  )
}
