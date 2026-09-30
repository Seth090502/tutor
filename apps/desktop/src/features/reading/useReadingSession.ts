// Hooks that join the flow reducer to the services: the session itself, the section's original text,
// single source chunks, and a paper's abstract.
import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react"

import { toFault, type ReadingApi } from "./api"
import { flowReducer, initFlow, type FlowAction } from "./flow"
import { isPaper } from "./docs"
import { findAbstractEntry, loadChunks } from "./reader"
import { openAndPrepare, sendAnswer, sendAtoms, sendRecall, sendSummary } from "./runner"
import type { Chunk, Confidence, DocDetail, OutlineEntry, SectionRef } from "./types"

export function useGuidedReading(api: ReadingApi, doc: DocDetail, entry: OutlineEntry) {
  const [state, rawDispatch] = useReducer(flowReducer, isPaper(doc) ? "paper" : "section", initFlow)
  // Results that arrive after the learner left are dropped. The flag is re-armed on a StrictMode remount.
  const live = useRef(true)
  useEffect(() => {
    live.current = true
    return () => {
      live.current = false
    }
  }, [])
  const dispatch = useCallback((a: FlowAction) => {
    if (live.current) rawDispatch(a)
  }, [])
  const ctx = useMemo(() => ({ api, dispatch }), [api, dispatch])

  const started = useRef(false)
  useEffect(() => {
    if (started.current) return
    started.current = true
    void openAndPrepare(ctx, doc.doc_id, entry.chunk_id, null)
  }, [ctx, doc.doc_id, entry.chunk_id])

  // Recall and summary are done once the backend has taken them: go on to the next step.
  const settledStep = state.phase === "step" && ((state.step === "recall" && state.recall !== null) || (state.step === "summary" && state.summary !== null))
  useEffect(() => {
    if (settledStep) dispatch({ type: "advance" })
  }, [settledStep, dispatch])

  const sessionId = state.sessionId
  const when = <A extends unknown[]>(f: (id: string, ...a: A) => Promise<void>) => (...a: A) => (sessionId ? f(sessionId, ...a) : Promise.resolve())

  return {
    state,
    retry: () => {
      const resume = state.failure?.stage === "prepare" ? state.sessionId : null
      dispatch({ type: "retry" })
      void openAndPrepare(ctx, doc.doc_id, entry.chunk_id, resume)
    },
    advance: () => dispatch({ type: "advance" }),
    skipRecall: () => dispatch({ type: "recallSkipped" }),
    answer: when((id, qid: string | number, text: string, confidence: Confidence) => sendAnswer(ctx, id, qid, text, confidence)),
    recall: when((id, text: string) => sendRecall(ctx, id, text)),
    summarize: when((id, text: string, judgement: Confidence) => sendSummary(ctx, id, text, judgement)),
    acceptAtoms: when((id, ids: (string | number)[]) => sendAtoms(ctx, id, ids)),
  }
}

export type Loaded<T> = { phase: "loading" } | { phase: "ready"; value: T } | { phase: "error"; message: string }

function useLoaded<T>(load: (() => Promise<T>) | null, key: string): Loaded<T> & { retry: () => void } {
  const [state, setState] = useState<Loaded<T>>({ phase: "loading" })
  const [nonce, setNonce] = useState(0)
  const latest = useRef(load)
  latest.current = load
  useEffect(() => {
    const run = latest.current
    if (!run) return
    let alive = true
    setState({ phase: "loading" })
    run().then(
      (value) => alive && setState({ phase: "ready", value }),
      (e) => alive && setState({ phase: "error", message: toFault("rag-engine", e).message }),
    )
    return () => {
      alive = false
    }
  }, [key, nonce])
  return { ...state, retry: () => setNonce((n) => n + 1) }
}

/** The original text of the section the session opened: its chunks, read from the chunk route (C9). */
export function useSectionText(api: ReadingApi, docId: string, section: SectionRef | null) {
  const ids = section?.chunk_ids ?? []
  return useLoaded<Chunk[]>(
    section
      ? async () => {
          if (ids.length === 0) throw new Error("The section has no chunks.")
          const want = new Set(ids)
          return loadChunks(api, docId, ids[0], (c) => want.has(c.chunk_id), ids.length)
        }
      : null,
    `${docId}|${ids.join(",")}`,
  )
}

/** One chunk by id: from the section text when it is there, otherwise from the chunk route. */
export function useChunk(api: ReadingApi, docId: string, chunkId: string, known: Chunk[] | null) {
  const hit = known?.find((c) => c.chunk_id === chunkId) ?? null
  const fetched = useLoaded<Chunk | null>(hit ? null : async () => (await loadChunks(api, docId, chunkId, () => true, 1))[0] ?? null, `${docId}|${chunkId}|${hit ? "known" : "fetch"}`)
  if (hit) return { phase: "ready" as const, value: hit as Chunk | null, retry: () => {} }
  return fetched
}

/** A paper's abstract: the chunks under its Abstract heading, or null when the outline has none. */
export function useAbstract(api: ReadingApi, doc: DocDetail, enabled: boolean) {
  const entry = enabled ? findAbstractEntry(doc.outline) : null
  const loaded = useLoaded<Chunk[]>(
    entry
      ? () => {
          const path = entry.heading_path.join("\u0000")
          return loadChunks(api, doc.doc_id, entry.chunk_id, (c) => c.heading_path.join("\u0000") === path, 4)
        }
      : null,
    `${doc.doc_id}|${entry?.chunk_id ?? ""}`,
  )
  return entry ? loaded : ({ phase: "ready", value: null, retry: () => {} } as const)
}
