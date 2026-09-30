# Interface contracts

Fixed interfaces between components so they can be built in parallel. Change a contract only
through the lead; implementations must match it exactly. All services bind 127.0.0.1 only.
Times are epoch seconds (float), like DeepTutor.

| Service | URL |
|---|---|
| DeepTutor backend (FastAPI) | `http://127.0.0.1:8001` |
| rag-engine service | `http://127.0.0.1:8811` |
| Desktop dev server (Vite) | `http://127.0.0.1:1420` |

Browser origins the app uses (must be allowed by both services' CORS and DeepTutor's WebSocket
origin guard): `http://localhost:1420`, `http://127.0.0.1:1420`, `http://tauri.localhost`,
`tauri://localhost`.

## C1. rag-engine service

`rag-engine serve --corpus <dir> [--notes <dir>] --data <dir> [--port 8811]`

- Corpus paths decide metadata: `corpus/<domain>/books/**` gives `source_type: "book"`,
  `corpus/<domain>/papers/**` gives `"paper"`, `corpus/inbox/**` gives domain `"unsorted"` and
  `"unknown"`. Files under `--notes <dir>/<domain>/**` get `"note"`.
- Embeddings are optional, configured by env: `RAG_EMBED_PROVIDER` (`voyage` | `openai_compat`),
  `RAG_EMBED_MODEL`, `RAG_EMBED_API_KEY` (or `VOYAGE_API_KEY`), `RAG_EMBED_BASE_URL`. With no provider,
  search runs BM25 only and reports it in `mode_used`.
- OCR is optional, configured by `RAG_OCR_PROVIDER`; without one, scanned and math-heavy pages stay
  queued as `ocr.pending_pages`.

`GET /health` returns `{"ok": true, "version": "<semver>"}`.

`GET /status` returns:
```json
{"corpus_root": "C:/…/corpus", "data_dir": "C:/…/data/rag",
 "documents": {"total": 0, "by_domain": {"math": 0}, "by_source_type": {"book": 0, "paper": 0, "note": 0, "unknown": 0}},
 "pages": {"total": 0, "by_class": {"born_digital": 0, "scanned": 0, "math_heavy": 0, "empty": 0}},
 "chunks": {"total": 0, "embedded": 0},
 "ocr": {"pending_pages": 0, "done_pages": 0},
 "errors": 0,
 "embedding": {"provider": null, "model": null, "dims": null},
 "active_job": null,
 "spend_usd": {"embedding": 0.0, "ocr": 0.0}}
```
`active_job` is null or a job object (below).

`POST /survey {"root": "<optional dir>"}` and `POST /ingest {"root": "<optional>", "rebuild": false}`
return `202 {"job_id": "…"}`, or `409 {"error": "…"}` while another job runs.

`GET /jobs/{job_id}` returns:
```json
{"job_id": "…", "kind": "survey", "state": "queued", "progress": {"done": 0, "total": 0},
 "started_at": null, "finished_at": null, "error": null}
```
`kind` is `survey` | `ingest`; `state` is `queued` | `running` | `done` | `failed`.

`GET /survey/latest` returns the latest survey report (the `rag-engine survey` JSON format) or 404.

`POST /search`:
```json
{"query": "…", "k": 10, "domains": ["math"], "source_types": ["book", "paper", "note"], "mode": "hybrid"}
```
`domains` and `source_types` are optional; `mode` is `hybrid` | `bm25` | `vector` (default `hybrid`).
Response:
```json
{"mode_used": "bm25",
 "hits": [{"chunk_id": "…", "doc_id": "…", "path": "corpus/math/books/x.pdf", "title": "…",
           "domain": "math", "source_type": "book", "page_start": 12, "page_end": 12,
           "heading_path": ["Chapter 3", "3.2 Limits"], "text": "…", "score": 0.0,
           "ranks": {"bm25": 1, "vector": null}}]}
```
Errors are `{"error": "<message>"}` with a 4xx or 5xx status.

## C2. Eval run file

One JSON file per run in `evals/runs/<run_id>.json` (gitignored). Datasets live in `evals/datasets/`.
```json
{"run_id": "20261001T120000Z-bm25-baseline", "created_at": 1790000000.0,
 "eval": "retrieval", "dataset": "evals/datasets/retrieval-v1.jsonl",
 "change": "baseline: BM25 only", "config": {"mode": "bm25", "k": 10},
 "splits": {
   "train": {"n": 0, "metrics": {"recall@5": 0.0, "recall@10": 0.0, "mrr@10": 0.0, "ndcg@10": 0.0},
             "ci95": {"recall@10": [0.0, 0.0]}},
   "test":  {"n": 0, "metrics": {}, "ci95": {}}},
 "per_domain": {"math": {"train": {"n": 0, "metrics": {}}, "test": {"n": 0, "metrics": {}}}},
 "decision": "baseline", "cost_usd": 0.0}
```
`decision` is `baseline` | `keep` | `revert` | `pending`. A compare whose gain is inside the noise
floor records `revert`, since only `keep` keeps a change. Per-question results for the paired
bootstrap sit beside each run in `evals/runs/<run_id>.questions.jsonl`; readers of run files glob
`*.json` only.

## C3. Tauri commands (Rust shell to webview)

Called with `invoke` (`@tauri-apps/api/core`). Rust `snake_case` arguments arrive as `camelCase` in
JS. Typed wrappers live in `apps/desktop/src/lib/backend.ts`.

| Command | Args | Returns |
|---|---|---|
| `services_status` | none | `{"backend": Service, "rag_engine": Service}` where `Service = {"state": "starting"|"running"|"stopped"|"failed", "url": "…", "detail": string|null}` |
| `services_restart` | `name: "backend"|"rag_engine"` | nothing |
| `secrets_status` | none | `{"elevenlabs": bool, "opencode": bool, "anthropic": bool, "openrouter": bool, "voyage": bool}` (presence only) |
| `secret_set` | `name, value` | nothing; stored in Windows Credential Manager, never readable from the webview |
| `secret_delete` | `name` | nothing |
| `elevenlabs_conversation_token` | `agent_id` | `{"token": "…"}`; Rust calls ElevenLabs with the stored key |
| `config_get` | none | `{"elevenlabs_agent_id": string|null}` |
| `config_set` | `key, value` | nothing; non-secret settings in the app config dir |
| `evals_runs` | none | `{"runs": [Run, ...], "warnings": ["…"]}`: every `<root>/evals/runs/*.json` (C2), newest `created_at` first; unreadable files are skipped and named in `warnings` |
| `plan_markdown` | none | `{"markdown": "…"}`: contents of `<root>/docs/tutor/PLAN.md` |

The shell starts both services as child processes in a Windows Job Object (killed with the app),
passes the allowed browser origins to DeepTutor via `CORS_ORIGINS`, and passes stored secrets to the
services as environment variables at launch.

## C4. Desktop module ownership

`apps/desktop/src/App.tsx` and `src/lib/backend.ts` belong to the lead. Screens:

| Screen | File |
|---|---|
| Tutor (voice) | `src/features/voice/VoiceTutorScreen.tsx` |
| Library | `src/features/tracker/LibraryScreen.tsx` |
| Evals | `src/features/tracker/EvalsScreen.tsx` |
| Progress | `src/features/tracker/ProgressScreen.tsx` |
| Plan | `src/features/tracker/PlanScreen.tsx` |
| Settings (keys, services) | `src/features/settings/SettingsScreen.tsx` |

Screens must work in a plain browser too (`npm run dev`): when `inTauri` is false they show that
the desktop shell is required instead of throwing.

## C5. Design system (desktop)

Direction: `docs/tutor/DESIGN.md`. Tokens live in `apps/desktop/src/index.css`.

- **Tailwind colours:** `ink-2`, `ink-3` (secondary and tertiary text), `hairline`, `hairline-strong`,
  `world`, `world-ink` (text on a `world` fill), `world-soft` (14% tint), `ok`, `warn`, `bad`,
  `chart-1`..`chart-5` (`chart-1` is the world accent). shadcn tokens (`background`, `card`,
  `muted-foreground`, `border`, ...) keep their names.
- **Fonts and easing:** `font-sans` (Geist), `font-mono` (Geist Mono), `font-serif` (Newsreader, with
  italic); easings `ease-soft`, `ease-spring`, `ease-inout`.
- **CSS variables:** `--dur-quick|base|slow|world` (1ms under reduced motion), `--wash-1`, `--wash-2`
  (ambient tints).
- **Root state on `<html>`:** class `dark` (follows the OS), `data-world` = `neutral` | `math` |
  `english` | `science` | `chemistry`, and `data-quality` = `lite` | `rich` (lite by default).
- **`src/lib/motion.ts`:**
  - `usePageVisible(): boolean`, `useReducedMotion(): boolean`, `useQuality(): "lite" | "rich"`.
  - `useSpring(target, { stiffness, damping }, apply: (v: number) => void)`: one shared ticker that
    sleeps when every spring is at rest. It snaps to the target under reduced motion or when hidden.
- **`src/lib/world.tsx`:** `type World`, `WORLDS`, `<WorldProvider>`, `useWorld(): { world, setWorld }`.
- **Folders:** `src/components/kit/` holds primitives and interactive pieces. `src/components/charts/` holds
  the SVG charts. `src/components/shell/` holds the app frame, ambient layer and motifs.
- **Rules:**
  - Motion uses transform and opacity only.
  - Every loop pauses when the page is hidden.
  - No `backdrop-filter` over animated content.
  - No new WebGL besides the Orb.
  - Text contrast is at least 4.5:1, and labels are at least 11px.
