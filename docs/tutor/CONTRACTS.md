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
| `config_get` | none | `{"elevenlabs_agent_id": string|null}`, plus `agent_<lang>` and `voice_<lang>` (C10) when set |
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
| Read (guided reading) | `src/features/reading/ReadingScreen.tsx` |
| Languages | `src/features/languages/LanguagesScreen.tsx` |
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
keep working. New clients use C12; reviews recorded here are labelled `legacy_*` (C12).

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
- **Rating policy (server, `review-policy/2`, 2026-09-30):**
  - correct with no hint and not revealed: `good`;
  - anything else (wrong, any hint, revealed): `again`.
  - `hard` is no longer produced (evidence ledger EV-002, EV-014); policy 1 gave `good` after rung 1 and
    `hard` after rungs 2-3.
  - Confidence never changes the rating.
- **`GET /analytics`** adds (C7 attempts only; C12 attempts are reported from Phase 2):
  - `calibration: [{"level": 1-4, "attempts": int, "correct": int}]`;
  - `assistance: {"attempts": int, "hinted": int, "revealed": int}`.

## C8. Voice tutor and the Tutor screen (desktop)

Supersedes the C8 event list of 2026-09-30 (`TutorEvent`, `"My answer: B"` messages), which is removed.

The Tutor screen keeps one C12 study session in practice mode (`apps/desktop/src/features/voice/practice.ts`).
Its id is stored in localStorage under `tutor.study.voice`. On mount the screen resumes it
(`GET /api/study/sessions/{id}`) and forgets the id when the session is closed or unknown. A new session starts
when practice first starts: the first `get_next_question`, "Practise without a call", or Next question.

- **One attempt, one writer.** The voice tools (`X-Tutor-Source: voice_tool`) and the typed controls (`ui`) send
  commands to that session through one `StudySession`, one at a time. The screen renders only the accepted public
  state from `StudySession.subscribe`; nothing is optimistic. A session started by a tool is recorded as
  `voice_tool`.
- **Voice tools** (`tools.ts`, `createTutorTools`):
  - `get_next_question {domain?}`: queue by tag, then `open_item`; refuses while an attempt is live, and says
    nothing is due when the domain has nothing due.
  - `propose_answer {transcript}`: `propose_response` only; it never commits.
  - `set_confidence {level}`: only in `committed`.
  - `get_hint {}`, `check_answer {}`, `give_up {}`.
  - `search_library {query, domain?}`: sends `attempt_id` while an attempt is live.
  - `get_progress {}`: still reads the C7 summary and analytics (C12 attempts join it in Phase 2).
  - There is no `record_review`: the server schedules FSRS at closure. Results carry `correct_answer`,
    `explanation` and `misconception` only from `attempt.released`.
- **Typed controls** (`ui`):
  - A-D or a text field commit a typed answer; "I don't know" commits `dont_know`.
  - A pending proposal shows "Tutor heard: ..." with Confirm and an editable field. Confirm sends a spoken
    commit (or a spoken repair while remediating) with `transcript_confirmation {raw_transcript, confirmed_text}`.
  - In `committed`: confidence 1-4, or Skip with a reason.
  - In `ready`: Check. In `remediating`: hint, typed repair, Give up.
  - In `awaiting_self_assessment`: I got it / I missed it. When closed: Next question.
  - A hint before answering is allowed, and the panel says the item will not count as unaided.
- **Screen to agent:** once the server accepts a typed action and a call is live, the screen sends the agent a
  user message starting with `On screen:`. It carries the same public data the matching tool would return.
- **Showing answers:** answer, explanation and misconception appear only from `attempt.released`. The closed view
  shows the rating with its provenance in words (for example "Again, because a hint was used").
- **Session lifetime:** practice sessions stay active across restarts; nothing closes them yet (assessment
  sessions will close explicitly).

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
  - Per-language voice choices are saved through `config_set` keys `voice_<lang>` (comma-separated voice ids),
    and each language's ElevenLabs agent id under `agent_<lang>`.
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
- **Word glosses:** `POST /api/languages/gloss {"lang", "word", "sentence"}` returns `{"gloss": str | null}`, a
  few English words for the word in that sentence from the lesson LLM. It is shown on screen only; the
  learner checks or rewrites it before adding the card.
- **Adding cards:** `POST /api/languages/cards` takes
  `{"lang", "items": [{"text", "meaning", "example"?}]}` and returns `{"created": int, "entry_ids": [int]}`.
  It creates recognition, production and (with audio) listening cards for words tapped while reading.
- **Starter decks:** `python -m deeptutor.services.languages.build_deck --lang <code> --size <n>` builds a
  starter deck from openly licensed sources (Tatoeba sentence pairs, an open frequency list) into `data/`,
  with licence and source per item, and imports it through the practice import pipeline.

## C11. Library use (spec: `docs/tutor/CORPUS.md`)

**DeepTutor backend** (routes under `/api/library` and `/api/reading`):
- **`POST /api/library/search`** takes
  `{"query", "k"? (default 8), "domains"?, "source_types"?, "attempt_id"?, "depth"?: "normal"|"deep"}` and
  returns `{"pack": [Hit & {"section": [str]}], "queries": [str], "mode_used": str,
  "withheld": int}`.
  - Hits are C1 hits.
  - `withheld` counts the chunks dropped because an attempt is open: the chunk contains the attempt's
    correct answer after normalisation, or sits under a solution or answer-key heading.
  - Rewrites, RRF and rerank follow the settings `library.rewrite`, `library.rerank` and
    `library.agentic` (all off by default).
  - The pack follows CORPUS.md Stage 0 step 3.
- **`POST /api/library/verify`** takes `{"citations": [{"chunk_id", "quote", "page"?}]}` and returns
  `{"results": [{"ok": bool, "reason": null|"no_chunk"|"not_substring"|"page_out_of_range"}]}`.
  Normalisation: whitespace, quotes, dashes, soft hyphens.
- **`POST /api/reading/sessions`** takes `{"doc_id", "from_chunk"}` and returns
  `{"session_id", "section": {"heading_path", "chunk_ids", "page_start", "page_end"}}`.
- **`POST /api/reading/sessions/{id}/prepare`** returns
  `{"prequestions": [{"id", "q"}], "idea_units": int, "atoms": [{"id", "front", "back", "chunk_id"}],
  "warrants": [...]}`. Keys and idea-unit texts stay server-side.
- **`POST .../prequestions/{qid}/answer`** takes `{"answer", "confidence"}` and returns
  `{"correct": bool|null}`. The explanation is revealed only after the section is read.
- **`POST .../recall`** takes `{"text"}`.
- **`POST .../summary`** takes `{"text", "judgement": 1-4}` and returns
  `{"covered": int, "missed": [{"unit", "chunk_id"}]}`. Scoring takes two passes.
- **`POST .../atoms/accept`** takes `{"ids": [...]}` and returns `{"entry_ids": [...]}`. It creates
  practice cards whose source is the chunk.

**rag-engine** (C9 additions):
- **`GET /documents/{doc_id}/pages/{n}.png?w=1200&highlight=<quote>`** returns a pdfium-rendered page as
  PNG. When the quote is found in the page's text, its boxes are drawn with a translucent mark. 404 for
  non-PDF documents.

**Voice tools:** `search_library` calls `/api/library/search` with the open `attempt_id` when one exists,
so the agent never sees withheld chunks.

**C11 amendments (accepted 2026-09-30):**

- **Chunk ids** are `<doc_id>:<ordinal>` (also C1).
- **Library settings:** `GET/PUT /api/library/settings` read and write `{"rewrite", "rerank", "agentic", "model"}`.
  The eval arms A-E use them.
- **Pack hits:** each `section` entry is prefixed with its `[c:<chunk_id>]` tag, and each hit carries
  `ranks.final` (the pack is not in rank order). Agentic search runs inside `/api/library/search`.
- **Guided reading additions:**
  - `GET /api/reading/sessions/{id}` returns the session state for resuming: its step, its answers so
    far, and the section.
  - `GET /api/reading/sessions/{id}/answers` returns
    `{"prequestions": [{"id", "q", "answer", "explanation", "your_answer", "correct"}]}`, only after the
    summary step (409 before).
  - A paper session's `prepare` returns paper prequestions (predict from the abstract), and the summary
    step scores a critique against the paper's claims.
- **Provisional practice entries** (proof warrants, essays) are tagged `provisional`. The practice queue,
  FSRS reviews and mastery exclude them. They can still be practised, with a note that they are unscored.
- **rag-engine page images** report `X-Quote-Matches: <int>`. After a schema change, rag-engine starts a
  re-index from SQLite on its own at startup (no re-extraction).

## C12. Study sessions, attempts and review evidence (DeepTutor backend)

Supersedes the C7 flow for new clients; C7 routes keep working and are labelled legacy. Spec:
`PEDAGOGY.md`. Code: `deeptutor/services/practice/loop.py`, routes `deeptutor/api/routers/learning.py`,
mounted at **`/api/study`** (upstream retired `/api/learning` and guards it).

**One writer.** The practice store writes sessions, attempts, events and FSRS state in the notebook
database. A command's events, its attempt update and, at closure, the FSRS review of the first outcome
commit in one SQLite transaction, so a crash can neither lose nor duplicate a review. Python was chosen
over Rust for this piece because the FSRS writer, the answer keys and the language checkers already live
in that process; any other placement would need two writers or an outbox. Rust projections (skills,
queue, probes) will read `learning_events` (planned C13).

**Envelope.** `POST /api/study/commands` with header `X-Tutor-Source: ui | voice_tool`, set by the calling
code path and never by a model; a missing or other value is 422.

```
Command: {schema_version: 1, command_id: UUID, session_id?, attempt_id?, expected_version: int | null,
          kind, payload}
Result:  {schema_version: 1, command_id, aggregate_version, last_event_seq,
          public_state: {session, attempt | null}, events: [Event]}
Error:   {"error": {code, message, retryable, current_version?}}
         conflict 409, invalid_transition 409, invalid_input 422, not_found 404, unauthorized 403,
         budget_exceeded 402, unavailable 503, unverified 424
Event:   {event_id, seq, kind, attempt_id, recorded_at, source, payload}   (public-safe)
```

- The aggregate is the session. `expected_version` is the version last seen (`start_session` sends null);
  a stale version is `conflict` with `current_version` and changes nothing.
- The server sets `recorded_at` (epoch seconds) and `source`.
- The same `command_id` with the same content returns the stored result; with other content it is
  `invalid_input`. Network retries reuse the id; they are never recorded as answers.
- Events are append-only (database triggers refuse UPDATE and DELETE), with a per-session `seq` and a
  global `event_seq`.
- `GET /api/study/sessions/{id}?after_seq=N` returns `{session, attempts, events after N, last_event_seq}`.
  `GET /api/study/sessions/active` returns the newest active session or null (resume after a restart).
- `GET /api/study/queue?limit&tag` returns due items with public fields only (`entry_id, prompt,
  question_type, options, lang, tags, due_at`). Quarantined and `provisional` items are excluded, and the
  C7 queue excludes them too. `tag` matches a category name case-insensitively (`lang:es`, `Math`).

**Kinds and statuses.** Statuses: `open`, `committed`, `ready`, `remediating`, `awaiting_self_assessment`,
`submitted` (assessment only), `closed`.

| Kind | Allowed from | Effect |
|---|---|---|
| `start_session {mode, curriculum_version?}` | no session | new active session; mode is practice, assessment or instruction |
| `open_item {entry_id}` | no other open item (submitted assessment items do not block) | attempt `open`; assessment items must be auto-checkable |
| `request_assistance {kind: "hint"}` | `open` (an assisted start) or `remediating`; never in assessment | next ladder rung 1-4 with its text, or null text; no rung 5 |
| `request_assistance {kind: gloss, source_open or other, content_ref?}` | `open`, `remediating` | recorded assistance, no text |
| `propose_response {input_mode: spoken or dont_know, raw_transcript}` | `open`, `remediating` | a pending proposal for the learner to confirm |
| `commit_response {response, input_mode, transcript_confirmation?}` | `open`; source `ui` only | first response sealed; spoken needs `{raw_transcript, confirmed_text}` |
| `record_confidence {level: 1-4 or null, skipped_reason?}` | `committed` | `ready`; a skip needs a reason |
| `check {}` | `ready` | see verdicts below |
| `self_assess {judged: correct or incorrect}` | `awaiting_self_assessment`; source `ui` only | closed `self_assessed` |
| `submit_repair {response, input_mode: typed or spoken}` | `remediating`; source `ui` only | appended check; a correct repair closes; two misses add `offer: worked_example_then_fresh_problem` |
| `give_up {}` | `open`, `remediating`; not in assessment | closed `revealed`; before any answer it records `dont_know` |
| `abandon {}` | any live status except `submitted` | closed `abandoned` and nothing released; after hints with no answer it records an assisted `dont_know` |
| `invalidate_item {reason}` | any | item quarantined; a live attempt closes `invalidated` without a review |
| `close_session {}` | an active session | submitted items close and release; the rest close `abandoned` |

Items carry `skill_ids` (from `skill:<id>` category tags) and `family_id` (from a `family:<id>` tag) in
`attempt.item` and in the `item_opened` event, so projections (C13) need only events.

Check verdicts:
- `correct`: in practice, the attempt closes `correct`.
- `incorrect`: in practice, `remediating`. A dont_know response is `incorrect` with checker `dont_know`.
- `unverifiable`: there is no checker, so the reference answer is released and the learner self-assesses.
- `invalid_item`: the key cannot be read, so the item is quarantined and the attempt closes `invalidated`.
- In assessment every checked item waits as `submitted`. Its verdict is hidden in state and events until
  `close_session` publishes `checks_released`.
- An item edited after it was opened makes `check` a conflict.

**Public versus private.** Attempt state carries the prompt, options, type, the learner's responses,
confidence, public checks, the assistance given so far, the pending proposal, the closure and `offer`.
`released: {correct_answer, accepted_answers, explanation, misconception}` appears only after a closure that
releases it (`correct`, `revealed`, `self_assessed`) or for self-assessment. Keys, unrequested hints,
explanations and misconceptions never appear in events, open attempts or the queue. The teaching model
receives exactly what the UI receives.

**Review evidence (policy `review-policy/2`).** Computed from the first outcome and applied once at
closure through `PracticeStore.apply_review` with request id `ev:<attempt_id>`:

| First outcome | Assistance before commit | FSRS rating | Provenance |
|---|---|---|---|
| correct | none | Good | `verified` |
| correct | any | Again | `assisted` |
| incorrect or dont_know | none or any | Again | `verified` or `assisted` |
| unverifiable, then self-assessed | none | the learner's judgement (Good or Again) | `self_report` |
| anything, in instruction mode | - | none | - |
| invalidated, or abandoned before a check | - | none | - |

Hard is never produced. Repairs, reveals and retries never add a review. Every review event records
`provenance` inside `outcome_json` (read it with `COALESCE(json_extract(outcome_json, '$.provenance'),
'legacy')`). C7 reviews are `legacy_attempt`, `legacy_self_report` or `legacy_client_rating`, and rows from
before this change read as `legacy`. Only `verified` reviews may enter verified outcome reports.

**Measurement eligibility** (closed attempts only). An attempt is eligible only for a checked, unassisted
first response, in practice or assessment, on an item not exposed (hint or answer seen) in the previous 24
hours. Reasons: `assisted`, `unverified`, `no_checked_response`, `instruction_mode`, `invalid_item`,
`exposed_within_24h`. A relearning attempt still gets its FSRS review; it is not independent evidence.

**Legacy entry paths** (kept for the upstream web UI and other DeepTutor surfaces, never verified):
- C7 reviews are labelled `legacy_*` (tested in `tests/api/test_practice_attempts.py`).
- `POST /practice/questions/{id}/check` grades without an attempt and writes nothing. Answers probed there
  are not tracked as exposure; the desktop app does not call it.
- Chat, Book focus-check, immersive reading and Mastery Path capture wrong answers as bank mistakes (the
  `practice_capture_*` triggers set a due date) with no review and no study event.
- Imports into "mistakes" set a due date the same way.

**Migration and rollback.** Additive only: new tables (`learning_sessions`, `learning_attempts`,
`learning_events`, `learning_commands`, `practice_item_status`); no existing table, column or row changes.
An older build runs against a migrated database unchanged: it ignores the new tables, its reviews read as
`legacy`, and it does not honour quarantine or the provisional exclusion. Backup before first run: the
notebook database is one SQLite file; copy it with the SQLite backup API (the workspace data-migration
code already does, `services/workspace/data_migration.py`) while the backend is stopped.
