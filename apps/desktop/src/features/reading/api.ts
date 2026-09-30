// Clients for the two services the Reading screens talk to: rag-engine (C9 library browsing, page crops)
// and the DeepTutor backend (C11 /api/reading). Every failure is a ServiceFault naming the service, so the
// screens can say which one is offline and keep "offline" apart from "answered with an error".
import { BACKEND_URL, HttpError, RAG_ENGINE_URL, fetchJson } from "@/lib/backend"
import { errorText } from "@/features/library/format"

import { documentsQuery, type Filters } from "./docs"
import type {
  AcceptResult,
  AnswerResult,
  ChunkPage,
  Citation,
  Confidence,
  DocDetail,
  DocList,
  LibraryCounts,
  OpenedSession,
  Prepared,
  SummaryResult,
  VerifyResult,
} from "./types"

export type ServiceName = "rag-engine" | "backend"

export class ServiceFault extends Error {
  constructor(
    readonly service: ServiceName,
    /** No answer at all: the service is not running or not reachable. */
    readonly offline: boolean,
    readonly status: number | null,
    message: string,
  ) {
    super(message)
  }
}

export function toFault(service: ServiceName, e: unknown): ServiceFault {
  if (e instanceof ServiceFault) return e
  if (e instanceof HttpError) return new ServiceFault(service, false, e.status, errorText(e))
  return new ServiceFault(service, true, null, e instanceof Error ? e.message : String(e))
}

const enc = encodeURIComponent
const post = (body: unknown): RequestInit => ({ method: "POST", body: JSON.stringify(body) })

/** GET /documents/{id}/pages/{n}.png (C11): a pdfium render, with the quote's boxes marked when it is found. */
export function pageImageUrl(docId: string, page: number, opts: { highlight?: string; width?: number } = {}, base = RAG_ENGINE_URL): string {
  const p = new URLSearchParams({ w: String(opts.width ?? 1200) })
  const quote = opts.highlight?.trim().slice(0, 240)
  if (quote) p.set("highlight", quote)
  return `${base}/documents/${enc(docId)}/pages/${page}.png?${p}`
}

export function createApi(bases: { engine?: string; backend?: string } = {}) {
  const engine = bases.engine ?? RAG_ENGINE_URL
  const backend = bases.backend ?? BACKEND_URL
  const call = async <T>(service: ServiceName, path: string, init?: RequestInit): Promise<T> => {
    try {
      return await fetchJson<T>(service === "rag-engine" ? engine : backend, path, init)
    } catch (e) {
      throw toFault(service, e)
    }
  }
  const reading = (id: string, rest: string) => `/api/reading/sessions/${enc(id)}${rest}`

  return {
    status: () => call<LibraryCounts>("rag-engine", "/status"),
    documents: (filters: Filters, offset: number) => call<DocList>("rag-engine", `/documents?${documentsQuery(filters, offset)}`),
    document: (docId: string) => call<DocDetail>("rag-engine", `/documents/${enc(docId)}`),
    chunks: (docId: string, from: number, limit: number) =>
      call<ChunkPage>("rag-engine", `/documents/${enc(docId)}/chunks?from=${from}&limit=${limit}`),

    openSession: (docId: string, fromChunk: string) =>
      call<OpenedSession>("backend", "/api/reading/sessions", post({ doc_id: docId, from_chunk: fromChunk })),
    prepare: (sessionId: string) => call<Prepared>("backend", reading(sessionId, "/prepare"), { method: "POST" }),
    answer: (sessionId: string, questionId: string | number, answer: string, confidence: Confidence) =>
      call<AnswerResult>("backend", reading(sessionId, `/prequestions/${enc(String(questionId))}/answer`), post({ answer, confidence })),
    recall: (sessionId: string, text: string) => call<unknown>("backend", reading(sessionId, "/recall"), post({ text })),
    summary: (sessionId: string, text: string, judgement: Confidence) =>
      call<SummaryResult>("backend", reading(sessionId, "/summary"), post({ text, judgement })),
    acceptAtoms: (sessionId: string, ids: (string | number)[]) =>
      call<AcceptResult>("backend", reading(sessionId, "/atoms/accept"), post({ ids })),

    verify: (citations: Citation[]) => call<{ results: VerifyResult[] }>("backend", "/api/library/verify", post({ citations })),
  }
}

export type ReadingApi = ReturnType<typeof createApi>
