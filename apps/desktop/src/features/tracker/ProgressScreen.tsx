// Progress: DeepTutor practice summary, daily analytics and mastery topics.
// Never call /api/mastery-paths/progress/{id}/map or /board: they create paths as a side effect.
import { useCallback, useEffect, useState } from "react"
import { BookOpenCheck, RefreshCw, ServerOff } from "lucide-react"

import { Panel, Rise } from "@/components/kit"
import { Button } from "@/components/ui/button"
import { BACKEND_URL, HttpError, fetchJson } from "@/lib/backend"
import { ActivityChart, ReviewDiscs } from "@/features/progress/Activity"
import { Hero } from "@/features/progress/Hero"
import { AssistancePanel, CalibrationPanel } from "@/features/progress/Learning"
import { PlanSection } from "@/features/progress/Plan"
import { SkillsSection } from "@/features/progress/Skills"
import { MasteryMap, TopicList } from "@/features/progress/Mastery"
import { DAYS, errorText, type Analytics, type Summary, type Topic } from "@/features/progress/model"
import { Notice, Quiet, ScreenBoundary, StateBlock } from "@/features/progress/parts"
import { ProgressSkeleton } from "@/features/progress/Skeleton"

type Loaded = {
  summary: Summary | null
  analytics: Analytics | null
  topics: Topic[] | null
  errors: string[]
  at: Date
}

export function ProgressScreen() {
  return (
    <ScreenBoundary>
      <ProgressView />
    </ScreenBoundary>
  )
}

function ProgressView() {
  const [data, setData] = useState<Loaded | null>(null)
  const [offline, setOffline] = useState(false)
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    setLoading(true)
    const tz = encodeURIComponent(Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC")
    const [summary, analytics, topics] = await Promise.allSettled([
      fetchJson<Summary>(BACKEND_URL, `/api/question-notebook/practice/summary?timezone=${tz}`),
      fetchJson<Analytics>(BACKEND_URL, `/api/question-notebook/practice/analytics?days=${DAYS}&timezone=${tz}`),
      fetchJson<{ topics: Topic[] }>(BACKEND_URL, "/api/mastery-paths/topics"),
    ])
    const failures = [summary, analytics, topics].flatMap((r) => (r.status === "rejected" ? [r.reason as unknown] : []))
    // A rejection that is not an HTTP status means nothing answered on the port.
    setOffline(failures.some((f) => !(f instanceof HttpError)))
    setData({
      summary: summary.status === "fulfilled" ? summary.value : null,
      analytics: analytics.status === "fulfilled" ? analytics.value : null,
      topics: topics.status === "fulfilled" ? (topics.value.topics ?? []) : null,
      errors: failures.map(errorText),
      at: new Date(),
    })
    setLoading(false)
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  return (
    <div className="mx-auto flex max-w-[1120px] flex-col gap-5 px-6 py-7 lg:px-8">
      {!data ? (
        <ProgressSkeleton />
      ) : offline ? (
        <StateBlock
          icon={ServerOff}
          title="The backend is not running"
          detail="Progress is read from the DeepTutor backend on this machine. Start it from Settings, then try again."
          onRetry={() => void load()}
          busy={loading}
        />
      ) : (
        <ProgressBody data={data} loading={loading} onRefresh={() => void load()} />
      )}
    </div>
  )
}

function ProgressBody({ data, loading, onRefresh }: { data: Loaded; loading: boolean; onRefresh: () => void }) {
  const { summary, analytics, topics, errors, at } = data
  const empty =
    summary !== null &&
    analytics !== null &&
    topics !== null &&
    summary.total === 0 &&
    topics.length === 0 &&
    (analytics.daily ?? []).every((d) => !d.questions && !d.mistakes && !d.reviews)

  const days = analytics?.days || DAYS
  const learning = [
    analytics?.calibration ? <CalibrationPanel key="cal" calibration={analytics.calibration} days={days} /> : null,
    analytics?.assistance ? <AssistancePanel key="ast" assistance={analytics.assistance} days={days} /> : null,
  ].filter((p) => p !== null)

  return (
    <>
      <div className="flex items-center justify-end gap-4">
        <span className="text-ink-3 font-mono text-[11px] tracking-[0.08em]">
          Updated {at.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })}
        </span>
        <Button variant="outline" size="sm" onClick={onRefresh} disabled={loading}>
          <RefreshCw className={loading ? "opacity-60" : undefined} />
          Refresh
        </Button>
      </div>

      {errors.length > 0 ? (
        <Notice>
          <p className="font-medium">Some of this screen could not load.</p>
          <ul className="text-ink-2 mt-1 space-y-1">
            {errors.map((e, i) => (
              <li key={i}>{e}</li>
            ))}
          </ul>
        </Notice>
      ) : null}

      {empty ? (
        <StateBlock
          icon={BookOpenCheck}
          title="No practice yet"
          detail="Answer a few questions with the tutor and your reviews, mistakes and mastery will build up here."
        />
      ) : (
        <>
          {summary ? <Hero summary={summary} /> : <Unavailable what="Due and review counts" />}

          {analytics ? (
            <>
              <Rise i={2}>
                <ReviewDiscs analytics={analytics} />
              </Rise>
              <Rise i={3}>
                <ActivityChart analytics={analytics} />
              </Rise>
            </>
          ) : (
            <Unavailable what="The last 30 days" />
          )}

          {learning.length > 0 ? (
            <div className={learning.length > 1 ? "grid items-start gap-5 lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)]" : undefined}>
              {learning.map((panel, i) => (
                <Rise key={panel.key} i={4 + i}>
                  {panel}
                </Rise>
              ))}
            </div>
          ) : null}

          {topics ? (
            <>
              <Rise i={6}>
                <MasteryMap topics={topics} />
              </Rise>
              <Rise i={7}>
                <TopicList topics={topics} />
              </Rise>
            </>
          ) : (
            <Unavailable what="Mastery topics" />
          )}
        </>
      )}

      <Rise i={8}>
        <PlanSection refreshKey={at.getTime()} />
      </Rise>
      <Rise i={9}>
        <SkillsSection refreshKey={at.getTime()} />
      </Rise>
    </>
  )
}

function Unavailable({ what }: { what: string }) {
  return (
    <Panel>
      <Quiet>{what} could not be loaded. Refresh to try again.</Quiet>
    </Panel>
  )
}
