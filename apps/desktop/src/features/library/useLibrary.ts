// Data for the Library screen: rag-engine status and survey (contract C1) plus the job this window follows.
import { useCallback, useEffect, useState } from "react"

import { HttpError, RAG_ENGINE_URL, fetchJson } from "@/lib/backend"
import { errorText } from "./format"
import { isActive, type Job, type Status, type Survey } from "./logic"

const POLL_STATUS_MS = 5000
const POLL_JOB_MS = 1000

export type Fault = { offline: boolean; message: string }

export function useLibrary() {
  const [status, setStatus] = useState<Status | null>(null)
  const [fault, setFault] = useState<Fault | null>(null)
  const [survey, setSurvey] = useState<Survey | null | undefined>(undefined) // null: none yet
  const [surveyError, setSurveyError] = useState<string | null>(null)
  const [job, setJob] = useState<Job | null>(null) // the job being followed: started here or found running
  const [starting, setStarting] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)

  const loadStatus = useCallback(async () => {
    try {
      const next = await fetchJson<Status>(RAG_ENGINE_URL, "/status")
      setStatus(next)
      setFault(null)
      const running = next.active_job
      // A job started elsewhere (CLI, another window) is followed too.
      if (isActive(running)) setJob((prev) => (prev?.job_id === running.job_id ? prev : running))
    } catch (e) {
      setFault({ offline: !(e instanceof HttpError), message: errorText(e) })
    }
  }, [])

  const loadSurvey = useCallback(async () => {
    try {
      setSurvey(await fetchJson<Survey>(RAG_ENGINE_URL, "/survey/latest"))
      setSurveyError(null)
    } catch (e) {
      if (e instanceof HttpError && e.status === 404) {
        setSurvey(null)
        setSurveyError(null)
      } else {
        setSurveyError(errorText(e))
      }
    }
  }, [])

  const reload = useCallback(() => {
    void loadStatus()
    void loadSurvey()
  }, [loadStatus, loadSurvey])

  useEffect(() => {
    reload()
    const timer = setInterval(() => {
      if (document.visibilityState !== "hidden") void loadStatus()
    }, POLL_STATUS_MS)
    return () => clearInterval(timer)
  }, [reload, loadStatus])

  const jobId = job?.job_id
  const polling = isActive(job)
  useEffect(() => {
    if (!jobId || !polling) return
    let alive = true
    const timer = setInterval(async () => {
      if (document.visibilityState === "hidden") return
      try {
        const next = await fetchJson<Job>(RAG_ENGINE_URL, `/jobs/${jobId}`)
        if (!alive) return
        setJob(next)
        setNotice(null)
      } catch (e) {
        if (alive) setNotice(`Could not read job ${jobId}: ${errorText(e)}`)
      }
    }, POLL_JOB_MS)
    return () => {
      alive = false
      clearInterval(timer)
    }
  }, [jobId, polling])

  const finishedKind = job?.state === "done" ? job.kind : null
  useEffect(() => {
    if (!finishedKind) return
    void loadStatus()
    if (finishedKind === "survey") void loadSurvey()
  }, [finishedKind, loadStatus, loadSurvey])

  const start = useCallback(async (kind: "survey" | "ingest") => {
    setStarting(true)
    setNotice(null)
    try {
      const { job_id } = await fetchJson<{ job_id: string }>(RAG_ENGINE_URL, `/${kind}`, {
        method: "POST",
        body: JSON.stringify(kind === "ingest" ? { rebuild: false } : {}),
      })
      setJob({ job_id, kind, state: "queued", progress: { done: 0, total: 0 }, started_at: null, finished_at: null, error: null })
    } catch (e) {
      setNotice(e instanceof HttpError && e.status === 409 ? `A job is already running: ${errorText(e)}` : errorText(e))
    } finally {
      setStarting(false)
    }
  }, [])

  return { status, fault, survey, surveyError, job, busy: starting || isActive(job), notice, start, reload }
}
