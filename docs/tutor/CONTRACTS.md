# Interface contracts

Fixed interfaces between components so they can be built in parallel. Change a contract only
through the lead; implementations must match it exactly. All services bind 127.0.0.1 only, except rag-engine's token-protected remote mode (C9).
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
| Train (Dual N-Back) | `src/features/train/TrainScreen.tsx` |
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

## C6. Dual N-Back storage (Tauri commands)

Spec: `docs/tutor/NBACK.md`. Stored in SQLite at `<app data dir>/nback.sqlite`, written by Rust.

| Command | Args | Returns |
|---|---|---|
| `nback_save_session` | `session: NBackSession` | `{"id": "…"}`; idempotent on `id` |
| `nback_sessions` | `limit?: number` (default 200) | `{"sessions": [NBackSummary, ...]}`, newest first |
| `nback_session` | `id` | `NBackSession` |
| `nback_export_csv` | none | `{"path": "…"}`: one row per trial, written to `<app data dir>/exports/` |

```ts
type Metrics = { hits: number; misses: number; false_alarms: number; correct_rejections: number;
                 d_prime: number; c: number }
type Trial = { i: number; position: number | null; letter: string | null;
               pos_match: boolean; let_match: boolean; lure: boolean;
               pos_response: boolean; let_response: boolean;
               pos_rt_ms: number | null; let_rt_ms: number | null;
               intended_onset: number; actual_onset: number }   // ms on the session clock
type Block = { index: number; n: number; void: boolean; void_reason: string | null;
               trials: Trial[]; metrics: { position: Metrics | null; letter: Metrics | null };
               next_n: number }
type NBackSession = {
  id: string                      // UUID generated by the client
  started_at: number; ended_at: number   // epoch seconds
  mode: "dual" | "position" | "letter" | "transfer"
  seed: number
  settings: { trial_ms: number; stimulus_ms: number; blocks: number; up_max_errors: number;
              down_min_errors: number; lures: boolean; in_trial_feedback: boolean;
              keys: { position: string; letter: string } }
  blocks: Block[]
  comparable: boolean             // false if in-trial feedback or settings differ from defaults
  timing: { output_latency: number | null; base_latency: number | null; calibrated: boolean }
  checkin: { expectation: number | null; sleep_hours: number | null; exercise: boolean | null;
             fatigue: number | null; strategy: string | null }
}
type NBackSummary = { id: string; started_at: number; mode: string; blocks: number; void_blocks: number;
                      start_n: number; max_n: number; mean_n: number;
                      d_prime: { position: number | null; letter: number | null }; comparable: boolean }
```

## C7. Practice attempts, hint ladder and rating policy (DeepTutor backend)

Spec: `docs/tutor/PEDAGOGY.md`. The routes live under `/api/question-notebook/practice`. The existing routes
keep working unchanged.

- **Questions:** questions gain `hints: list[str]`, ordered rungs 1-4 (nudge, hint, key step, worked
  example). They are imported from a `hints` column, `|`-separated, and returned on the entry.
- **`POST /questions/{id}/attempts`** returns `{"attempt_id": "…", "rungs_available": 0-4}` and opens an
  attempt.
- **`POST /attempts/{attempt_id}/confidence {"level": 1-4}`** returns `{"ok": true}`. It is only allowed
  before the check; otherwise 409.
- **`POST /attempts/{attempt_id}/hint`** reveals the next rung and returns
  `{"rung": 1-5, "text": str | null, "is_answer": bool}`.
  - `text` is null when the question has no pre-written text for that rung.
  - Rung 5 returns the correct answer and marks the attempt revealed.
  - Returns 409 after the check.
- **`POST /attempts/{attempt_id}/check {"answer": "…"}`** closes the attempt and returns
  `{"correct": bool | null, "rating": "good" | "hard" | "again" | null, "correct_answer": str,
  "explanation": str, "misconception": str | null}`.
  - `correct` is null for questions that cannot be auto-checked.
- **`POST /questions/{id}/review`** accepts
  `{"attempt_id", "request_id", "version", "answer", "judged_correct"?}`.
  - With `attempt_id`, the server computes the rating, and a client `rating` is rejected with 422.
  - `judged_correct` is required only when the check returned `correct: null`; it is recorded as
    self-reported.
  - Without `attempt_id`, the existing behaviour applies.
- **Rating policy (server):**
  - not correct: `again`;
  - revealed or highest rung 4 or more: `again`;
  - highest rung 2-3: `hard`;
  - otherwise `good`.
  - Confidence never changes the rating.
- **`GET /analytics`** adds (see also C8 for how the voice screen shows attempts):
  - `calibration: [{"level": 1-4, "attempts": int, "correct": int}]`;
  - `assistance: {"attempts": int, "hinted": int, "revealed": int}`.

## C8. Voice tutor events (desktop)

`apps/desktop/src/features/voice/tools.ts` owns the client tools and publishes what they did, so the Tutor
screen can show the current question without the answer key leaking to the model.

```ts
export type TutorEvent =
  | { type: "question"; attempt_id: string; question_id: number; question: string;
      question_type: string; options: Record<string, string> }
  | { type: "confidence"; attempt_id: string; level: 1 | 2 | 3 | 4 }
  | { type: "hint"; attempt_id: string; rung: number; text: string | null; is_answer: boolean }
  | { type: "checked"; attempt_id: string; correct: boolean | null; correct_answer: string;
      explanation: string; misconception: string | null }
  | { type: "reviewed"; attempt_id: string; rating: string; next_due: string | null }
export function onTutorEvent(listener: (e: TutorEvent) => void): () => void
```

- **Showing the attempt:** the screen renders the question card, the A-D keycaps, the confidence keys 1-4
  and the hint rung from these events.
- **Answers and confidence from the screen:** they are sent to the agent as user messages
  (`"My answer: B"`, `"Confidence: 3"`), so the agent still calls the tools. The screen never calls the
  practice API directly.
- **After the check:** the card flips to show the explanation and misconception only after a `checked`
  event.

## C9. rag-engine: remote host, library browsing, scale

For the owner's second desktop, which will hold the library and do embedding and OCR locally, and for a
corpus that will grow by orders of magnitude.

- **Bind and token:**
  - `rag-engine serve ... [--bind <ip>]`, default `127.0.0.1`.
  - With `RAG_ENGINE_TOKEN` set, every route except `GET /health` requires
    `Authorization: Bearer <token>`, or it returns 401 `{"error": "…"}`.
  - A non-loopback `--bind` without a token refuses to start.
  - CORS keeps the four app origins and allows the `Authorization` header.
- **`GET /documents?domain=&source_type=&q=&sort=title|added|pages&limit=50&offset=0`** returns
  `{"total": int, "documents": [Doc]}`.
  - `Doc = {doc_id, path, title, domain, source_type, pages, chunks, bytes, added_at, ocr_pending}`.
  - `q` filters by title or path substring.
- **`GET /documents/{doc_id}`** returns `Doc` plus
  `"outline": [{"heading_path": [...], "page_start", "page_end", "chunk_id"}]`: distinct heading paths in
  reading order.
- **`GET /documents/{doc_id}/chunks?from=<ordinal>&limit=20`** returns
  `{"chunks": [{"chunk_id", "ordinal", "text", "page_start", "page_end", "heading_path"}], "next": int | null}`,
  in reading order (guided reading).
- **Clients:** the DeepTutor adapter sends the token from `RAG_ENGINE_TOKEN` when it is set. The desktop
  gets a configurable engine URL and token when the second desktop exists. Until then it uses
  `127.0.0.1:8811`.
- **Scale:** ingest chunks and tokenises documents in parallel with bounded memory, and search latency
  stays interactive (p95 under 300 ms for BM25 at 10M chunks is the target).

## C10. Languages (spec: `docs/tutor/LANGUAGES.md`)

- **World and language:**
  - `data-world="languages"` joins C5, with accent `--acc-languages` (light `#7a3b7d`, dark `#d99ad6`).
  - `src/lib/language.tsx` exports `type TargetLanguage = "es" | "fr" | "de" | "ja" | "zh"`, `LANGUAGES`
    metadata (English name, native name, script, BCP-47 tag, Scribe language code), `<LanguageProvider>`
    and `useLanguage(): { language, setLanguage }`, persisted like the world.
- **Speech through the Rust shell** (the key never reaches the webview):

| Command | Args | Returns |
|---|---|---|
| `tts_audio` | `text, lang, voice_id?, model? (default "eleven_v4"), speed?` | raw MP3 bytes (`ArrayBuffer`). Cached at `<app data>/tts-cache/<sha256(model, voice, lang, speed, text)>.mp3`, so each sentence is paid for once |
| `tts_voices` | `lang` | `{"voices": [{"voice_id", "name", "labels", "preview_url"}]}` for that language |
| `stt_transcribe` | `audio: bytes (webm/opus or wav), lang` | `{"text": "…", "words": [{"text", "start", "end"}]}` via Scribe v2 with the language pinned |

  - The CSP adds `media-src 'self' blob:`.
  - Errors are readable sentences that never contain the key.
  - Per-language voice choices are saved through `config_set` keys `voice_<lang>` (comma-separated voice ids).
- **Language cards (DeepTutor practice, so FSRS, attempts and analytics apply unchanged):**
  - Each word or sentence yields up to three practice questions, tagged `lang:<code>` and
    `card:recognition|production|listening`.
  - The prompt is in `question`, the target in `correct_answer`, and accepted variants in
    `accepted_answers: list[str]`.
  - Source and licence go in `explanation` metadata.
- **Language answer checking:**
  - Deterministic normalisation (case, whitespace, punctuation, full-width forms for ja/zh).
  - Accent or diacritic-only differences return `correct: false` with `diagnostic: "accent"`, so the
    tutor can name the slip.
  - Any accepted variant counts as correct.
  - Free-response items return `correct: null` (rubric grading).
- **Graded reading:** `POST /api/languages/coverage {"lang", "text"}` returns
  `{"tokens": int, "known": int, "coverage": float, "unknown": [str]}`. Known means the word has a
  production or recognition card with FSRS retrievability ≥ 0.9.

- **Graded texts:** `POST /api/languages/graded-text` takes
  `{"lang", "band": "A1"|"A2"|"B1"|"B2"|"C1"|"C2", "topic"?, "target_coverage"? (default 0.97)}` and
  returns
  `{"text", "sentences": [str], "coverage": float, "unknown": [str], "questions": [{"q", "a"}]}`.
  - The text is generated by the lesson LLM and regenerated at most twice when coverage misses the target.
  - The questions check comprehension, in the target language at B1 and above and in English below.
- **Adding cards:** `POST /api/languages/cards` takes
  `{"lang", "items": [{"text", "meaning", "example"?}]}` and returns `{"created": int, "entry_ids": [int]}`.
  It creates recognition, production and (with audio) listening cards for words tapped while reading.
- **Starter decks:** `python -m deeptutor.services.languages.build_deck --lang <code> --size <n>` builds a
  starter deck from openly licensed sources (Tatoeba sentence pairs, an open frequency list) into `data/`,
  with licence and source per item, and imports it through the practice import pipeline.
