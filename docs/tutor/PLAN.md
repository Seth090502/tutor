# Fork plan: custom DeepTutor tutor

Base: HKUDS/DeepTutor v1.6.12 (`ef2d9e5`, Apache-2.0). Remote `upstream`, work branch `custom`.
Home: `C:\Users\sethy\Projects\tutor` (outside OneDrive on purpose: node_modules, indexes, corpus).
Layout map: `TUTOR.md`. Local commits only; nothing is pushed anywhere.

## Goals (owner, 2026-09-30)
- A tutor clearly better than upstream DeepTutor, with its own GUI and full ElevenLabs voice.
- LLM: Meta Muse Spark 1.3 (xhigh or max) via OpenCode Go; Claude Opus 5.5 or Sonnet 5.5 as
  alternatives. No local models. The ElevenLabs key arrives after everything is built.
- SOTA RAG over 50-250 GB of lawfully acquired teaching material and papers
  (math, English, science, chemistry).
- Quality is decided by evals: train/test split, one change per round, keep only if
  train AND test improve beyond the noise floor.

## Hard constraints
- No local models; the RTX 5090 is off-limits (Cinference owns it). CPU work only; model work via APIs.
- Disk: C: 141 GB + D: 61 GB free. A 250 GB corpus plus indexes does not fit today.
- RAM: plan for 20-25 GB usable (the WSL VM holds ~33 GB).
- Secrets: never in chat, git, or plaintext settings. Target: Windows Credential Manager via the Tauri shell.
- Corpus privacy: corpus text only goes to providers with no-training / zero-retention terms.
  Voyage trains on customer data unless opted out (opt-out applies only to later data), and the
  Muse Spark "Contributor" tier reportedly trains on prompts, so use the standard tier.

## Done
- Fork, venv (`.venv`, Python 3.14), CI-mirrored baseline on Windows: 7,335 passed, 52 failed,
  65 skipped. All 52 are Windows portability issues (upstream CI runs pytest on Linux only):
  POSIX `sh`/`rm` in sandbox tests (29), venv `bin/` vs `Scripts/`, `/tmp` paths, permission bits,
  a cp1252 decode. Failure IDs are kept outside the repo for regression comparison.
- Security lockdown (tests added first, all passing):
  - Native launches bind 127.0.0.1 (launcher backend + Next `HOSTNAME`, `run_server`, `deeptutor serve`).
    Containers keep their own bind (Dockerfile untouched).
  - CORS is explicit even with auth off (was `https?://.*` + credentials, so any web page could drive
    the local admin). One allowlist: `services/config/origins.py:app_origins`.
  - WebSocket Origin guard in `ws_require_auth` (browsers apply no CORS to WebSockets): foreign
    Origin closes with 4003; no Origin (non-browser clients) is accepted.
  - `sandbox_allow_subprocess` defaults to False (was True: model-chosen code on the host with only
    application-level isolation). Opt back in via system settings.
  - Full suite after lockdown: same 52 Windows failures; one test updated for the new contract
    (`test_unhandled_exception_response_carries_cors_headers` now uses an allowlisted origin).
    `test_turn_repository::test_only_one_process_can_begin_an_active_turn_per_session` is flaky upstream.
- OpenRouter reasoning: no change needed. Top-level `reasoning_effort` is documented as equal to
  `reasoning.effort`; Muse Spark 1.3 supports max/xhigh/high/medium/low/minimal, default medium.
- Content: `content/question_bank.csv` (60 items incl. `misconception` column; 60/60 valid through
  DeepTutor's own import preview) and `content/notes/` (the owner's 4 notes, byte-identical).
- Desktop app `apps/desktop/` (Tauri 2 + Vite + React 19 + Tailwind 4 + shadcn): sidebar with Tutor, Library,
  Evals, Progress, Plan. Tutor shows ElevenLabs' official Orb (MIT registry, unmodified `orb.tsx`) with
  their 3 color presets, Idle/Listening/Thinking/Talking, and a Live mic mode driven by a real mic level.
  Other sections are honest empty states naming their data source. Theme follows Windows light/dark.
  Verified: `npm run build`, headless Edge renders in both themes (software WebGL, GPU untouched).

## Architecture (target)
- **Shell (Rust, Tauri 2.12):** window, sidecar supervision of the Python backend (Windows Job Object so
  the whole process tree dies with the app), secrets via `keyring` 4 + `windows-native-keyring-store`,
  ElevenLabs signed-URL / conversation-token fetch (the xi-api-key never reaches the webview).
- **UI (TypeScript/React, Vite):** new GUI. DeepTutor's Next.js app cannot be statically exported
  (middleware `proxy.ts`, 2 route handlers, `redirects()`, 20 dynamic routes), so it stays as the
  browser UI while the Tauri UI is built. `@elevenlabs/react` 1.16 (MIT) voice teacher whose client
  tools call the backend.
- **Backend (DeepTutor FastAPI):** tutoring loops, practice, mastery path. New RAG pipeline adapter
  calls `rag-engine`. Keep fork changes behind narrow seams so upstream stays mergeable.
- **rag-engine (Rust, new):** ingestion (discovery, BLAKE3 exact dedup, MinHash near-dup, pdfium text
  layer, page triage, OCR API for math/scanned pages, EPUB via `rbook`), chunking (recursive 300-512
  tokens + heading path + page/bbox anchors for citations), indexes (tantivy BM25 + LanceDB vectors),
  query (hybrid RRF, then gated rerank and query rewrite), local HTTP API.
- **Evals:** retrieval graded by code (ranx), answers by an LLM judge from a different model family
  than the tutor, voice by ElevenLabs tests (`evaluation_model` differs from the tutor model).

## RAG decisions (research 2026-09-30; every gated layer must win on the owner's train split)
| Layer | Default | Evidence | Gate |
|---|---|---|---|
| First stage | Hybrid BM25 + dense, RRF | +3.6-5 nDCG@10 over best single retriever (T2-RAGBench), +1.5-4 (SciFact) | always on |
| Dense model | voyage-4 family vs Gemini Embedding 2 vs Cohere Embed 5 | math: voyage-3-large 54.5 vs BM25 32.2 nDCG@10 (MIRB) | retrieval eval |
| Reranker | API cross-encoder (Voyage rerank-2.5 / Cohere Rerank 4 / zerank-2) | +13 nDCG (finance), but general rerankers LOWERED math scores (MIRB) | per-domain eval |
| Query rewrite | LLM reasoning rewrite for hard questions | BRIGHT: BM25 14.5 to ~27 with LLM-written queries | eval |
| Chunk context | Contextual Retrieval or voyage-context-4 | -35 to -49% failures (Anthropic, vendor-run); one small replication showed ~no gain | paid pilot first |
| Parsing | pdfium text layer; OCR API for math/scans (olmOCR 2 via DeepInfra ~$0.3-0.4/1k pages; Mistral OCR 4 batch $2/1k, ZDR default); GROBID metadata for papers | no CPU-only tool turns formulas into LaTeX at usable speed | per-page triage |
| Store | LanceDB embedded (Apache-2.0, Windows-native, disk IVF) + tantivy (MIT); Qdrant native as fallback | int8 1024-d = 1 KB/vector; 10M vectors ~10 GB | size measured in P1 |
| Not doing | GraphRAG/LightRAG at 10M+ chunks, long-context stuffing, local ColBERT | graph: +1-3 on multi-hop only at large indexing cost; long context ~26x cost/query | - |

## Cost (research ranges, NOT measured; P1 replaces them)
- Tokens per GB of PDF varied 4-10x across sources, so every figure below is soft.
- Embeddings: about $6-65 for 50 GB (mid), under ~$250 even for 250 GB (high).
- OCR, if every page needed it: olmOCR ~$70 (50 GB) / ~$350 (250 GB); Mistral batch ~$400 / ~$2,000.
- Contextual chunk context: hundreds to low thousands of dollars, only if the pilot shows a gain.
- Pilots on the sample: ~$20-50. Runtime cost per tutoring session: measured in P3.

## Phases (one in progress at a time)
- **P0** fork, baseline, lockdown. Done; commit pending owner OK.
- **P1** corpus sample (1-2 GB, representative):
  - Done: `rag-engine survey --root <dir> --out report.json` (Rust; blake3, clap, pdfium-render 0.9.4 +
    PDFium chromium/8076 in `vendor/`, rayon, rbook, serde, walkdir; licenses in crates/rag-engine/README.md).
    Dedup, PDF text layer, per-page triage (born_digital / scanned / math_heavy / empty), EPUB/MD/TXT
    sizing, per-GB extrapolation to 50 and 250 GB. 34 unit + 1 integration tests, clippy clean.
  - Waiting on the owner: the sample folder. Then calibrate triage thresholds against hand-labelled pages.
  - Encrypted PDFs, including owner-password-only ones (open without a password but restrict copying),
    are reported as errors, never processed. Owner decision whether restricted-copy PDFs they own may be
    indexed for personal use; the survey will show how many there are first.
- **P2** retrieval eval: synthetic questions from sampled chunks, split by source document (train/test),
  plus human-validated hard cases; ranx recall@k / MRR / nDCG@10; baselines BM25 (tantivy) and
  DeepTutor's current pipeline. ~800-1,000 questions to resolve 3-point gains. About $10.
- **P3** paid pilots on the sample, one change per round: OCR vendors on math pages, embedding models,
  rerankers per domain, query rewrite, chunk context. About $20-50.
- **P4** full ingestion (budget approved from P1 numbers), DeepTutor RAG adapter, OpenRouter profile for
  Muse Spark (verify reasoning effort: `reasoning_params.py:196-208` sends top-level `reasoning_effort`).
- **P5** tutor core:
  - Done: question-bank practice now schedules with FSRS (`fsrs` 6.3.2, MIT; default parameters,
    desired retention 0.9, fuzzing on, 1m/10m learning steps, library max interval). Four nullable
    `fsrs_*` columns added by an idempotent migration; legacy rows rebuild by replaying
    `practice_review_events` on their next review. `ease` is no longer returned (no UI reads it).
    Declared in `pyproject.toml`, `requirements/cli.txt`, `packaging/deeptutor-cli/pyproject.toml`.
    Tests: 59 pass (practice, learning-sources review, packaging metadata).
  - Remaining: FSRS for the mastery-path scheduler (`RetentionScheduler` protocol, 8 construction
    sites); `misconception` field (9 touch points: import, storage, upsert, serializer, list SELECT,
    API models, web API, UI); import `content/question_bank.csv` and `content/notes/`.
- **P6** GUI: shell and Orb done (see Done). Remaining: keyring, Job-Object sidecar, ElevenLabs agent
  (custom LLM pointing at OpenRouter, streaming Chat Completions; client tools call the backend),
  agents-as-code via `@elevenlabs/cli`, and wiring each tracker section to its data source.
- **P7** voice evals (tool-call and simulation tests), latency and cost targets, hillclimb.

## Pending owner decisions
1. Where the 1-2 GB sample is, and where the full library will live (disk space).
2. Provider privacy: no-training / zero-retention only (Voyage opt-out before any upload).
3. Local commits in this fork (no push).
4. Pilot budget ceiling.

## Known issues
- Claude Code subagent defaults to `bypassPermissions` (`services/subagent/config.py:54`). It only runs
  after a user creates a connection; keep it unconfigured.
- Fixed: Muse Spark 1.3 has mandatory reasoning and rejects `effort: none`. `reasoning_params.py` now
  sends `minimal` (with `exclude: true`) for OpenRouter `meta/muse-spark*` when `none` is requested,
  via the existing `_MINIMAL_NOT_OFF_PATTERNS` table; other models unchanged (56 tests pass).
- Voice agent tools must withhold answer keys: practice `/queue` and `/questions/{id}` return
  `correct_answer`; the agent gets the question only and grades through `/questions/{id}/check`.
- `GET /api/mastery-paths/progress/{id}/map` and `/board` create an empty path for an unknown id
  (side effect on GET); the Progress screen must only request ids from `/topics`.
- The Orb loads its Perlin-noise texture from ElevenLabs' public CDN at runtime (not in the MIT repo),
  so it needs network on first load. Tauri CSP is still the template's `null`; set a CSP before shipping.
- ElevenLabs UI blocks reference `ui.elevenlabs.io` registry URLs, which rate-limit (HTTP 429); install
  from `raw.githubusercontent.com/elevenlabs/ui/main/apps/www/public/r/<name>.json`. Best voice-screen
  template: block `voice-chat-03` (Orb + ConversationBar + Conversation + Message + Response).
- The 52 Windows test failures are upstream portability bugs; fix only when touching that code.
- OpenRouter as an ElevenLabs custom LLM is plausible (streaming Chat Completions) but unverified.
