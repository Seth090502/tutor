// Reading: the document browser, one document's outline, and Guided Reading for a section of it.
// Contracts: C9 (rag-engine library browsing) and C11 (DeepTutor /api/reading). Screen wiring is in App.tsx (C4).
import { useMemo, useState } from "react"

import { createApi } from "./api"
import { DocumentBrowser } from "./DocumentBrowser"
import { DocumentView } from "./DocumentView"
import { nextEntry } from "./docs"
import { ReadingFlow } from "./ReadingFlow"
import type { DocDetail, OutlineEntry } from "./types"

type View = { name: "browse" } | { name: "doc"; docId: string } | { name: "read"; doc: DocDetail; entry: OutlineEntry }

export function ReadingScreen() {
  const api = useMemo(() => createApi(), [])
  const [view, setView] = useState<View>({ name: "browse" })

  if (view.name === "browse") return <DocumentBrowser api={api} onOpen={(d) => setView({ name: "doc", docId: d.doc_id })} />
  if (view.name === "doc")
    return <DocumentView api={api} docId={view.docId} onBack={() => setView({ name: "browse" })} onRead={(doc, entry) => setView({ name: "read", doc, entry })} />
  const { doc, entry } = view
  const next = nextEntry(doc, entry)
  return (
    <ReadingFlow
      key={`${doc.doc_id}|${entry.chunk_id}`}
      api={api}
      doc={doc}
      entry={entry}
      next={next}
      onOutline={() => setView({ name: "doc", docId: doc.doc_id })}
      onNext={() => next && setView({ name: "read", doc, entry: next })}
    />
  )
}
