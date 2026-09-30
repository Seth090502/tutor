# Fork plan: custom DeepTutor tutor

Base: HKUDS/DeepTutor v1.6.12 (`ef2d9e5`, Apache-2.0). Remote `upstream`, work branch `custom`.
Home: `C:\Users\sethy\Projects\tutor` (outside OneDrive on purpose: node_modules, indexes, corpus).
Layout map: `TUTOR.md`. Local commits only; nothing is pushed anywhere.

| Document | What it covers |
|---|---|
| `CONTRACTS.md` | Interfaces C1-C7 |
| `PEDAGOGY.md` | The teaching method |
| `NBACK.md` | Dual N-Back |
| `DESIGN.md` | Visual design |
| `research/` | Verified research syntheses (46-agent study, 2026-09-30) |

## Goals (owner, 2026-09-30)

- A tutor clearly better than upstream DeepTutor: its own GUI, full ElevenLabs voice, and teaching up to
  PhD level in math, English, science and chemistry.
- Teaching grounded in current learning science and the mechanisms behind top-performing school systems
  (`PEDAGOGY.md`).
- A well-built Dual N-Back section (`NBACK.md`).
- Different visual worlds per subject (`DESIGN.md`), polished motion.
- LLM: Muse Spark 1.3 via OpenCode, or Claude Opus/Sonnet 5.5. No local models.
- SOTA RAG over 50-250 GB of lawfully acquired books and papers.
- Quality decided by evals: train/test split, one change per round, keep only if both improve.

## Hard constraints

- **GPU:** no local models, and the RTX 5090 is off-limits (Cinference owns it). The UI adds no WebGL
  besides the Orb.
- **Disk:** C: 141 GB and D: 61 GB free, so a 250 GB corpus plus indexes does not fit today. RAM: plan for
  20-25 GB.
- **Secrets:** Windows Credential Manager through the Tauri shell, handed to services as environment
  variables. Never in chat, git or plaintext settings.
- **Corpus privacy:** the OpenCode Go Muse tier (`muse-spark-1.3-contributor`) trains on prompts, so the
  example catalog defaults to the standard tier on OpenCode Zen.

## Done (merged on `custom`)

- **Baseline and lockdown:**
  - Windows baseline: 52 known portability failures.
  - Loopback binds, explicit CORS, WebSocket Origin guard, host subprocesses off.
- **Practice:**
  - FSRS scheduling (py-fsrs 6.3.2).
  - Per-question `misconception`, end to end: import, DB, API, web UI in 5 locales.
  - The chat tutor's question-bank tool shows the misconception on missed entries.
- **LLM layer:**
  - Correct request shapes for Claude Opus/Sonnet 5.5: effort, thinking-off, forced tool choice, thinking
    replay, refusals.
  - OpenCode provider (Zen and Go, Responses API) with Muse `none` mapped to `minimal`.
  - `${ENV}` key references.
  - Every system message reaches Claude.
  - Example catalog: `docs/tutor/model_catalog.example.json`.
- **rag-engine (C1):**
  - axum service with incremental ingest (BLAKE3, moves, deletions), 400-token chunks with page ranges and
    heading paths, tantivy BM25, LanceDB vectors (voyage or openai-compatible), hybrid RRF, jobs, CORS.
  - 104 tests.
- **DeepTutor adapter:** `tutor_library` connected knowledge base; retrieval goes through rag-engine with
  citations.
- **Evals:** `evals/` harness (split by document, generate, run, paired-bootstrap compare; noise means
  revert). 57 tests.
- **Desktop app:**
  - Tauri shell: Job-Object sidecars, keyring secrets, ElevenLabs token, CSP, microphone permission.
  - Voice tutor: `@elevenlabs/react`, the tools, and the agent config in `apps/desktop/agent/`.
  - Tracker screens (Library, Evals, Progress, Plan) and the Settings screen.
  - Design tokens: paper and lamplight themes, subject worlds, Geist, Geist Mono and Newsreader.

## In progress

Merged since this plan was written:
- Design round 1: shell and motion, component kit, SVG chart kit.
- The teaching-method backend (C7): attempts, hint ladder, server rating policy.
- Voice tools and prompt (C8), and the chat teaching policy.
- Dual N-Back: engine, audio, storage (C6), and the Train section.

Also merged:
- Design round 2: all screens restyled on the kits.
- The Tutor screen's attempt panel, driven by C8.
- The Train screen polish.
- rag-engine C9: remote mode with a token, library browsing, and parallel ingest (about 4.6x faster).
- Research round 3: `research/corpus-use.md` and `CORPUS.md`.

Running, each on its own `wt/*` branch and merged after review:
- Languages (C10): the Rust speech commands and the languages world, the language backend (cards,
  checking, coverage, graded texts, starter decks), and the Languages screen with per-language agents.
- Kit consolidation: the locally built pieces promoted into the kits.
- Library use (C11):
  - Library search with rewrites, the answer-leak guard, citation verification and the eval arms.
  - rag-engine page crops and index quality.
  - The Guided Reading backend and screen.

Next:
- Question generation from the library (CORPUS.md Stage 1).
- Concept graph.
- A visual-QA pass across all screens.
- An external review (`docs/tutor/review/`).

## Phases (remaining)

- **P8 Design polish:** rounds 1-3 above. Train screen styling lands with P10.
- **P9 Teaching method in code (`PEDAGOGY.md`, contract C7):**
  - Server-side attempts, hint ladder, confidence, rating policy, calibration and assistance analytics.
  - Voice tools and prompts.
  - DeepTutor chat tutor prompts.
  - Question schema additions (hints, prerequisites, variant families, citations).
  - Deterministic checkers: SymPy, RDKit and units.
- **P10 Dual N-Back (`NBACK.md`, contract C6):**
  - Engine, audio, Train screen, storage and CSV export.
  - Then the transfer check, and later the opt-in experiment card with probes.
- **P11 Measurement:** delayed unassisted probes (7 and 30 days), calibration chart, assisted vs unassisted
  on Progress, and two personal experiments at most.
- **P12 Corpus:**
  - Licence metadata per source.
  - Open acquisition only for public-domain or openly licensed material (MIT OCW, Gutenberg,
    Standard Ebooks, Wikisource).
  - OpenStax excluded: its licence bans LLM ingestion.
  - A parallel chunker, float16 or int8 vectors, an OCR provider, and the retrieval eval on the owner's
    sample.
- **P13 Question generation:** grounded in cited chunks, blind-solved by a second model family,
  misconception distractors drawn from the owner's own errors.
- **P14 Model audit:** answer-leak rate, sycophancy, and proof, mechanism and numeric errors for Opus 5.5,
  Sonnet 5.5 and Muse Spark 1.3 on the owner's problems. Gates prompt changes.

## Owner decisions (2026-09-30)

- **English:** first language. The English track aims at advanced literature, linguistics, rhetoric and
  writing.
- **Foreign languages:** added as a fifth subject, with ElevenLabs for pronunciation models and speaking
  practice. Research pass pending.
- **Science:** physics, biology, earth and space, and neuroscience and cognitive science, including the
  evidence on what does and does not raise intelligence and cognition (taught honestly; no gain claims).
- **Corpus licences:** CC BY-SA, GFDL and CC BY-NC-SA material is allowed in the private, never-shared
  corpus. OpenStax stays excluded (its terms ban LLM ingestion).
- **Country:** United States. Public domain means published before 1931.
- **Library use (2026-09-30):** indexing stays local.
  - Keyword search runs now; embeddings and OCR run on the owner's second desktop once it exists
    (contract C9).
  - The tutor takes full advantage of the corpus: cited passages in lessons, questions generated from
    the books, extracted concept and prerequisite maps, and guided reading of chapters and papers.
  - The corpus is expected to grow by orders of magnitude.
- **First foreign languages:** Spanish, French, German, Japanese, Mandarin.

## Pending owner decisions

1. **Keys:** ElevenLabs, OpenCode, Anthropic and Voyage, entered in Settings.
2. **Voice model:** the voice agent's LLM is `claude-opus-4-8` at low reasoning, for latency. ElevenLabs'
   API also accepts `claude-opus-5-5`: stronger, but slower.
3. **Muse tier:** Go contributor (trains on prompts, cheaper) or Zen standard (default).
4. **Corpus scope:** where the library lives (disk), and whether restricted-copy PDFs you own may be
   indexed.
5. **Learner profile:** which foreign languages; what "PhD level" means per subject (target programmes or
   qualifying exams). A curriculum research pass proposes defaults.
6. **Vector storage:** float32 needs about 41 GB per 10M vectors; float16 or int8 would cut it.

## Known issues

- `lancedb` is pinned to 0.37.1, since 0.38 and 0.39 need the `remote` feature. The build needs `protoc`,
  vendored under `crates/rag-engine/vendor/protoc` (gitignored; see the crate README).
- The chunker is single-threaded at about 1.7 MB/s (roughly 10 h per 60 GB of text). There is no OCR
  provider, so scanned and math pages stay queued.
- The Orb's Perlin texture loads from ElevenLabs' CDN (allowed by the CSP).
- The CSP covers ElevenLabs' US region only.
- The Settings screen can't pick `xhigh` for OpenCode yet; edit the catalog instead.
- `max` on the Go contributor tier would be rejected with a 400.
- DeepTutor caps some requests at 4,096 max tokens, which may be tight for Opus 5.5 at high effort.
- `tutor_library` has no preflight entry: `deeptutor doctor` reports that preflight could not run.
- Claude Code subagent config defaults to `bypassPermissions` (`services/subagent/config.py:54`). Leave it
  unconfigured.
- The 52 Windows test failures are upstream portability bugs; fix only when touching that code.
