// The document list for the browser: GET /documents (C9) with debounced search, paging and filter counts.
import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import { toFault, type ReadingApi, type ServiceFault } from "./api"
import type { Filters } from "./docs"
import type { Doc, LibraryCounts } from "./types"

export type DocsState = {
  phase: "loading" | "ready" | "error"
  docs: Doc[]
  total: number
  /** The list is being replaced after a filter change; the old rows stay on screen meanwhile. */
  refreshing: boolean
  loadingMore: boolean
  fault: ServiceFault | null
  /** A failed "show more", shown beside the button while the loaded rows stay. */
  moreError: string | null
  counts: LibraryCounts["documents"] | null
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value)
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms)
    return () => clearTimeout(t)
  }, [value, ms])
  return v
}

export function useDocuments(api: ReadingApi, filters: Filters) {
  const q = useDebounced(filters.q, 250)
  const { domain, sourceType, sort } = filters
  const effective = useMemo<Filters>(() => ({ q, domain, sourceType, sort }), [q, domain, sourceType, sort])
  const [nonce, setNonce] = useState(0)
  const [state, setState] = useState<DocsState>({ phase: "loading", docs: [], total: 0, refreshing: false, loadingMore: false, fault: null, moreError: null, counts: null })
  const seq = useRef(0)

  useEffect(() => {
    const id = ++seq.current
    setState((s) => ({ ...s, refreshing: s.phase === "ready", fault: null, moreError: null }))
    api
      .documents(effective, 0)
      .then((list) => {
        if (id === seq.current) setState((s) => ({ ...s, phase: "ready", docs: list.documents, total: list.total, refreshing: false, loadingMore: false }))
      })
      .catch((e) => {
        if (id === seq.current) setState((s) => ({ ...s, phase: "error", refreshing: false, fault: toFault("rag-engine", e) }))
      })
  }, [api, effective, nonce])

  // Counts for the chips are a nicety: without them the chips show no numbers.
  useEffect(() => {
    let live = true
    api
      .status()
      .then((s) => live && setState((prev) => ({ ...prev, counts: s.documents })))
      .catch(() => {})
    return () => {
      live = false
    }
  }, [api, nonce])

  const more = useCallback(() => {
    const id = seq.current
    setState((s) => ({ ...s, loadingMore: true, moreError: null }))
    api
      .documents(effective, stateRef.current.docs.length)
      .then((list) => {
        if (id === seq.current) setState((s) => ({ ...s, docs: [...s.docs, ...list.documents], total: list.total, loadingMore: false }))
      })
      .catch((e) => {
        if (id === seq.current) setState((s) => ({ ...s, loadingMore: false, moreError: toFault("rag-engine", e).message }))
      })
  }, [api, effective])
  const stateRef = useRef(state)
  stateRef.current = state

  const reload = useCallback(() => {
    setState((s) => ({ ...s, phase: "loading" }))
    setNonce((n) => n + 1)
  }, [])

  return { ...state, more, reload }
}
