// The speaking check: hear a target sentence, record yourself saying it, and see what the recogniser
// heard against the target. An intelligibility probe, never a pronunciation score: the screen says what a
// match and a miss can and cannot mean, and there is no percentage or grade anywhere (speaking.ts).
import { useState } from "react"
import { Loader2, Mic, Square } from "lucide-react"

import { Eyebrow, Panel } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { languageMeta, type TargetLanguage } from "@/lib/language"
import { errorText } from "./api"
import type { AudioEnv } from "./hooks"
import { appendLog, LOG_KEYS } from "./logs"
import { FIELD, Notice, PlayButton, Segmented } from "./parts"
import { platform } from "./platform"
import { compareSpoken, probeSummary, SPEAKING_TARGETS, type Comparison } from "./speaking"
import { useRecorder, useSay } from "./speech"

type Result = { heard: string; comparison: Comparison }

export function SpeakingCheck({ language, audio }: { language: TargetLanguage; audio: AudioEnv }) {
  const meta = languageMeta(language)
  const targets = SPEAKING_TARGETS[language]
  const [pick, setPick] = useState("0")
  const [custom, setCustom] = useState("")
  const [working, setWorking] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<Result | null>(null)
  const rec = useRecorder()
  const say = useSay(language)

  const target = pick === "custom" ? custom.trim() : targets[Number(pick)] ?? targets[0]

  const toggle = async () => {
    if (rec.state === "idle") {
      setResult(null)
      setError(null)
      say.stop()
      await rec.start()
      return
    }
    const recording = await rec.stop()
    if (!recording || recording.bytes.length === 0) return setError("Nothing was recorded. Try again.")
    setWorking(true)
    try {
      const out = await platform().sttTranscribe(recording.bytes, language)
      const comparison = compareSpoken(target, out.text, language)
      setResult({ heard: out.text, comparison })
      appendLog(LOG_KEYS.speaking, { at: Date.now() / 1000, lang: language, target, heard: out.text, matched: comparison.matched, total: comparison.total })
    } catch (e) {
      setError(errorText(e))
    } finally {
      setWorking(false)
    }
  }

  return (
    <Panel eyebrow="Speaking check" title="Say one sentence and see what the recogniser heard" className="lg-enter p-6">
      {audio.blocker ? (
        <Notice>{audio.blocker} Recording and transcribing run through the desktop app.</Notice>
      ) : (
        <div className="flex flex-col gap-5">
          <div>
            <Eyebrow>Sentence</Eyebrow>
            <Segmented
              label="Sentence to say"
              value={pick}
              onChange={(v) => { setPick(v); setResult(null) }}
              options={[...targets.map((_, i) => ({ id: String(i), label: `${i + 1}` })), { id: "custom", label: "Your own" }]}
              className="mt-2"
              disabled={rec.state === "recording" || working}
            />
            {pick === "custom" ? (
              <input lang={language} className={`${FIELD} mt-3`} value={custom} placeholder={`A sentence in ${meta.english}`} onChange={(e) => { setCustom(e.target.value); setResult(null) }} />
            ) : null}
          </div>

          <p lang={language} className="font-serif text-[26px] leading-snug tracking-[-0.01em] text-balance">{target || "Type a sentence above."}</p>

          <div className="flex flex-wrap items-center gap-3">
            <PlayButton label="Hear it" status={say.status} onPlay={() => void say.say(target)} onStop={say.stop} disabledReason={target ? null : "Choose a sentence first."} />
            <Button onClick={() => void toggle()} disabled={!target || working} variant={rec.state === "recording" ? "destructive" : "default"}>
              {working ? <Loader2 aria-hidden className="animate-spin" /> : rec.state === "recording" ? <Square aria-hidden /> : <Mic aria-hidden />}
              {working ? "Listening" : rec.state === "recording" ? "Stop and check" : "Record"}
            </Button>
            <span className="text-ink-3 text-[13px]" aria-live="polite">
              {rec.state === "recording" ? "Recording. Say the sentence, then stop." : "Listen, repeat, then compare."}
            </span>
          </div>

          {say.error ? <Notice tone="warn">{say.error}</Notice> : null}
          {rec.error ? <Notice tone="warn">{rec.error}</Notice> : null}
          {error ? <Notice tone="warn">{error}</Notice> : null}

          {result ? (
            <div className="lg-enter border-hairline flex flex-col gap-4 border-t pt-5" aria-live="polite">
              <div>
                <Eyebrow>The recogniser heard</Eyebrow>
                <p lang={language} className="mt-1.5 font-serif text-[20px] leading-snug">{result.heard || "Nothing"}</p>
              </div>
              <div>
                <Eyebrow>The target, word by word</Eyebrow>
                <p lang={language} className="mt-1.5 flex flex-wrap gap-x-2 gap-y-1 font-serif text-[20px] leading-snug">
                  {result.comparison.target.map((t, i) => (
                    <span key={i} className="lg-heard inline-flex items-center gap-1" data-heard={t.heard}>
                      {t.text}
                      <span className="sr-only">{t.heard ? " (heard)" : " (not heard)"}</span>
                      <span aria-hidden className="text-ink-3 inline-block font-sans text-[12px]">{t.heard ? "✓" : "✗"}</span>
                    </span>
                  ))}
                </p>
                {result.comparison.extra.length ? (
                  <p className="text-ink-2 mt-2 text-sm">Extra, not in the target: <span lang={language}>{result.comparison.extra.join(" ")}</span></p>
                ) : null}
              </div>
              <p className="text-[15px] leading-relaxed">{probeSummary(result.comparison, result.heard, language)}</p>
              <p className="text-ink-3 text-[13px] leading-relaxed">
                This is an intelligibility probe, not a pronunciation score. A recogniser can quietly fix a slip, so a full match can flatter you, and it can fail on a good sentence. Use it to notice, not to grade.
              </p>
            </div>
          ) : null}
        </div>
      )}
    </Panel>
  )
}
