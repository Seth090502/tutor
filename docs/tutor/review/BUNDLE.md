# Tutor: review bundle

Generated 2026-09-30 from branch custom at 9b32ed32. Fork of HKUDS/DeepTutor v1.6.12 (Apache-2.0, base ef2d9e5).

## Fork commit log (43 commits)

```
9b32ed32 Integrate screens: calibration reference 25/50/75/95 per PEDAGOGY, ink-3 above 4.5:1, remove dead tracker ui
ab959e40 Plan and Settings: editorial reading surface and hairline service/key lists
d58124da Progress screen: due gauge, 30-day phase discs and area chart, calibration and assistance panels, mastery sunburst and topic list
31ac8037 Library screen: kit stat row, charts, typed job log, calm offline state
9ff85ab0 Plan: in-progress status
b27be720 Contract C10: languages (world, speech commands through the shell, language cards, checking, coverage)
9588c495 Contract C9 (remote rag-engine, library browsing, scale); corpus policy and first languages in the plan
42591605 Evals screen: latest-run hero, trajectory with CI bands and decision markers, expandable run list
d82fb95c Integrate: Train section, voice status labels for the new tools, track n-back clips, 45-minute voice calls
4568e379 Dual N-Back: commit the SAPI-rendered letter clips (force-added past the root *.wav ignore)
adf27721 Dual N-Back: engine, audio, SQLite storage (C6) and Train screen
6ba956d7 Languages spec, round-2 research syntheses, pedagogy resolutions (mastery, prerequisites, confidence, workload, verification, grounding)
8a4ad7f0 Tokens: per-world chart palettes from the other subjects' accents; darker light-theme science teal
1c2b6276 Component kit: eyebrow, panel, stats, keycaps, holo question card, LEDs, idle dim
203bc2d1 Contract C7: practice attempts, hint ladder, confidence and server rating
6c528e24 Voice tutor on practice attempts (C7/C8) and chat teaching policy
6ce7bc81 Desktop shell: motion runtime, world provider, sidebar, header, ambient layer and motifs
62fc7da6 Chart kit: SVG line, area, range bars, stacked bar, sunburst, phase discs, gauge, sparkline
a88a9f6e Plan: owner decisions (languages subject, science scope, corpus licences, US public domain)
19840458 Contract C8: voice tutor events for the Tutor screen
f43a010f Teaching method, Dual N-Back spec, contracts C6-C7, research syntheses, updated plan
831eb9df rag-engine: retrieval service (contract C1) with incremental ingest and hybrid search
69e8102b Design tokens: warm paper/lamplight neutrals, subject worlds, bundled Geist Mono and Newsreader; contract C5
c1fd2231 Design direction draft from the 100-piece gallery study
07527f9f Anthropic provider: keep every system message, in order
d6e49b93 Question bank tool: show the misconception on missed entries
a2d17ba7 LLM layer: correct requests for Claude Opus/Sonnet 5.5 and Muse Spark on OpenCode
8fab2fe1 Tauri shell: sidecar supervisor, keys, C3 commands, CSP, Settings screen
b8c7644e Add a per-question misconception to the question notebook and practice
830ea926 Evals: record a noise verdict as revert; document the per-question sidecar in C2
f01a8b26 Add retrieval eval harness with train/test split, runner and compare
26f013f4 Voice tutor: ElevenLabs conversation, client tools, agent config
760488b6 Add tutor_library RAG pipeline that retrieves through rag-engine
fcf0b6d0 Tracker screens: Library, Evals, Progress, Plan
ad73bbc5 Contract: evals_runs and plan_markdown shell commands for the tracker
ed7fb48d Add interface contracts, shared service client, and screen stubs for parallel build
efee6c89 Reorganize into a monorepo layout with a corpus folder
c66a81f3 Add Tauri + React desktop app with the ElevenLabs Orb, and the fork plan
28690df0 Add rag-engine: Rust corpus survey (dedup, PDF triage, sizing)
e78f1c62 Add the owner's question bank and notes in DeepTutor import format
517b8747 Send minimal instead of none to Muse Spark on OpenRouter
e9c4ea19 Schedule question-bank practice with FSRS (py-fsrs 6.3.2)
c95ee62c Harden local deployment: loopback binds, explicit CORS, WebSocket origin guard, subprocess exec off
```

## Code written for this fork (paths)

```
apps/desktop/agent/README.md
apps/desktop/agent/agent_configs/tutor.json
apps/desktop/agent/agents.json
apps/desktop/agent/tool_configs/check_answer.json
apps/desktop/agent/tool_configs/get_hint.json
apps/desktop/agent/tool_configs/get_next_question.json
apps/desktop/agent/tool_configs/get_progress.json
apps/desktop/agent/tool_configs/record_review.json
apps/desktop/agent/tool_configs/search_library.json
apps/desktop/agent/tool_configs/set_confidence.json
apps/desktop/agent/tools.json
apps/desktop/src-tauri/src/config.rs
apps/desktop/src-tauri/src/elevenlabs.rs
apps/desktop/src-tauri/src/job.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src-tauri/src/main.rs
apps/desktop/src-tauri/src/nback.rs
apps/desktop/src-tauri/src/root.rs
apps/desktop/src-tauri/src/secrets.rs
apps/desktop/src-tauri/src/services.rs
apps/desktop/src-tauri/src/tracker.rs
apps/desktop/src/App.tsx
apps/desktop/src/components/charts/ChartFrame.tsx
apps/desktop/src/components/charts/ChartTooltip.tsx
apps/desktop/src/components/charts/Gauge.tsx
apps/desktop/src/components/charts/LineChart.tsx
apps/desktop/src/components/charts/PhaseDiscs.tsx
apps/desktop/src/components/charts/RangeBars.tsx
apps/desktop/src/components/charts/Sparkline.tsx
apps/desktop/src/components/charts/StackedBar.tsx
apps/desktop/src/components/charts/Sunburst.tsx
apps/desktop/src/components/charts/_hooks.ts
apps/desktop/src/components/charts/_motion.ts
apps/desktop/src/components/charts/charts.css
apps/desktop/src/components/charts/geometry.test.ts
apps/desktop/src/components/charts/geometry.ts
apps/desktop/src/components/charts/index.ts
apps/desktop/src/components/charts/palette.ts
apps/desktop/src/components/charts/tables.test.ts
apps/desktop/src/components/charts/types.ts
apps/desktop/src/components/kit/CountUp.tsx
apps/desktop/src/components/kit/Eyebrow.tsx
apps/desktop/src/components/kit/IdleDim.tsx
apps/desktop/src/components/kit/KbdHint.tsx
apps/desktop/src/components/kit/Keycap.tsx
apps/desktop/src/components/kit/Led.tsx
apps/desktop/src/components/kit/Panel.tsx
apps/desktop/src/components/kit/QuestionCard.tsx
apps/desktop/src/components/kit/Scrim.tsx
apps/desktop/src/components/kit/Stat.tsx
apps/desktop/src/components/kit/StateChip.tsx
apps/desktop/src/components/kit/_motion.test.ts
apps/desktop/src/components/kit/_motion.ts
apps/desktop/src/components/kit/_spring.test.ts
apps/desktop/src/components/kit/_spring.ts
apps/desktop/src/components/kit/holo.test.ts
apps/desktop/src/components/kit/holo.ts
apps/desktop/src/components/kit/idle.test.ts
apps/desktop/src/components/kit/index.ts
apps/desktop/src/components/kit/keycap-logic.ts
apps/desktop/src/components/kit/keycap.css
apps/desktop/src/components/kit/keycap.test.ts
apps/desktop/src/components/kit/kit.css
apps/desktop/src/components/kit/question-card.css
apps/desktop/src/components/shell/Ambient.tsx
apps/desktop/src/components/shell/AppShell.tsx
apps/desktop/src/components/shell/SectionHeader.tsx
apps/desktop/src/components/shell/Sidebar.tsx
apps/desktop/src/components/shell/WorldSwitcher.tsx
apps/desktop/src/components/shell/grain.test.ts
apps/desktop/src/components/shell/grain.ts
apps/desktop/src/components/shell/motifs.test.tsx
apps/desktop/src/components/shell/motifs.tsx
apps/desktop/src/components/shell/shell.css
apps/desktop/src/components/shell/types.ts
apps/desktop/src/components/shell/useGlide.ts
apps/desktop/src/components/shell/useSectionPhase.ts
apps/desktop/src/components/ui/avatar.tsx
apps/desktop/src/components/ui/button.tsx
apps/desktop/src/components/ui/card.tsx
apps/desktop/src/components/ui/conversation-bar.tsx
apps/desktop/src/components/ui/conversation.tsx
apps/desktop/src/components/ui/live-waveform.tsx
apps/desktop/src/components/ui/message.tsx
apps/desktop/src/components/ui/orb.tsx
apps/desktop/src/components/ui/response.tsx
apps/desktop/src/components/ui/separator.tsx
apps/desktop/src/components/ui/shimmering-text.tsx
apps/desktop/src/components/ui/textarea.tsx
apps/desktop/src/dev/ChartsGallery.tsx
apps/desktop/src/dev/KitGallery.tsx
apps/desktop/src/features/evals/LatestRun.tsx
apps/desktop/src/features/evals/RunList.tsx
apps/desktop/src/features/evals/Trajectory.tsx
apps/desktop/src/features/evals/evals.css
apps/desktop/src/features/evals/fixtures.ts
apps/desktop/src/features/evals/model.test.ts
apps/desktop/src/features/evals/model.ts
apps/desktop/src/features/evals/parts.tsx
apps/desktop/src/features/library/BarRows.tsx
apps/desktop/src/features/library/JobsPanel.tsx
apps/desktop/src/features/library/LibraryStates.tsx
apps/desktop/src/features/library/SurveyPanel.tsx
apps/desktop/src/features/library/format.ts
apps/desktop/src/features/library/library.css
apps/desktop/src/features/library/logic.test.ts
apps/desktop/src/features/library/logic.ts
apps/desktop/src/features/library/useLibrary.ts
apps/desktop/src/features/plan/DesktopRequired.tsx
apps/desktop/src/features/plan/MarkdownView.tsx
apps/desktop/src/features/plan/markdown.test.ts
apps/desktop/src/features/plan/markdown.ts
apps/desktop/src/features/plan/rise.css
apps/desktop/src/features/plan/useScrollSpy.ts
apps/desktop/src/features/progress/Activity.tsx
apps/desktop/src/features/progress/CalibrationPlot.tsx
apps/desktop/src/features/progress/Hero.tsx
apps/desktop/src/features/progress/Learning.tsx
apps/desktop/src/features/progress/Mastery.tsx
apps/desktop/src/features/progress/Skeleton.tsx
apps/desktop/src/features/progress/model.test.ts
apps/desktop/src/features/progress/model.ts
apps/desktop/src/features/progress/parts.tsx
apps/desktop/src/features/progress/progress.css
apps/desktop/src/features/settings/SettingsScreen.tsx
apps/desktop/src/features/tracker/EvalsScreen.tsx
apps/desktop/src/features/tracker/LibraryScreen.tsx
apps/desktop/src/features/tracker/PlanScreen.tsx
apps/desktop/src/features/tracker/ProgressScreen.tsx
apps/desktop/src/features/train/Calibration.tsx
apps/desktop/src/features/train/Checkin.tsx
apps/desktop/src/features/train/History.tsx
apps/desktop/src/features/train/Run.tsx
apps/desktop/src/features/train/Setup.tsx
apps/desktop/src/features/train/TrainScreen.tsx
apps/desktop/src/features/train/audio.ts
apps/desktop/src/features/train/audio/c.wav
apps/desktop/src/features/train/audio/h.wav
apps/desktop/src/features/train/audio/k.wav
apps/desktop/src/features/train/audio/l.wav
apps/desktop/src/features/train/audio/q.wav
apps/desktop/src/features/train/audio/r.wav
apps/desktop/src/features/train/audio/s.wav
apps/desktop/src/features/train/audio/t.wav
apps/desktop/src/features/train/engine/adapt.ts
apps/desktop/src/features/train/engine/dsp.test.ts
apps/desktop/src/features/train/engine/dsp.ts
apps/desktop/src/features/train/engine/engine.test.ts
apps/desktop/src/features/train/engine/generate.ts
apps/desktop/src/features/train/engine/history.test.ts
apps/desktop/src/features/train/engine/history.ts
apps/desktop/src/features/train/engine/prng.ts
apps/desktop/src/features/train/engine/scheduler.test.ts
apps/desktop/src/features/train/engine/scheduler.ts
apps/desktop/src/features/train/engine/scoring.ts
apps/desktop/src/features/train/engine/session.ts
apps/desktop/src/features/train/parts.tsx
apps/desktop/src/features/train/runner.ts
apps/desktop/src/features/train/storage.ts
apps/desktop/src/features/train/types.ts
apps/desktop/src/features/tutor/TutorScreen.tsx
apps/desktop/src/features/tutor/useMicVolume.ts
apps/desktop/src/features/voice/VoiceTutorScreen.tsx
apps/desktop/src/features/voice/tools.test.ts
apps/desktop/src/features/voice/tools.ts
apps/desktop/src/index.css
apps/desktop/src/lib/backend.ts
apps/desktop/src/lib/motion.test.ts
apps/desktop/src/lib/motion.ts
apps/desktop/src/lib/utils.ts
apps/desktop/src/lib/world.test.ts
apps/desktop/src/lib/world.tsx
apps/desktop/src/main.tsx
apps/desktop/src/vite-env.d.ts
crates/rag-engine/src/bm25.rs
crates/rag-engine/src/chunk.rs
crates/rag-engine/src/dedup.rs
crates/rag-engine/src/discover.rs
crates/rag-engine/src/embed.rs
crates/rag-engine/src/engine.rs
crates/rag-engine/src/error.rs
crates/rag-engine/src/extract.rs
crates/rag-engine/src/headings.rs
crates/rag-engine/src/ingest.rs
crates/rag-engine/src/jobs.rs
crates/rag-engine/src/main.rs
crates/rag-engine/src/meta.rs
crates/rag-engine/src/ocr.rs
crates/rag-engine/src/report.rs
crates/rag-engine/src/search.rs
crates/rag-engine/src/server.rs
crates/rag-engine/src/store.rs
crates/rag-engine/src/survey.rs
crates/rag-engine/src/text.rs
crates/rag-engine/src/triage.rs
crates/rag-engine/src/vector.rs
evals/tutor_evals/__init__.py
evals/tutor_evals/__main__.py
evals/tutor_evals/cli.py
evals/tutor_evals/compare.py
evals/tutor_evals/dataset.py
evals/tutor_evals/generate.py
evals/tutor_evals/httpjson.py
evals/tutor_evals/metrics.py
evals/tutor_evals/paths.py
evals/tutor_evals/run_retrieval.py
evals/tutor_evals/runfile.py
evals/tutor_evals/split.py
evals/tutor_evals/stats.py
```

## DeepTutor files changed by the fork (diff stat)

```
 deeptutor/agents/chat/agentic_pipeline.py                      |  17 +++-
 deeptutor/agents/chat/prompts/en/teaching_policy.yaml          |  39 ++++++++
 deeptutor/agents/chat/prompts/zh/teaching_policy.yaml          |  28 ++++++
 deeptutor/api/main.py                                          |  38 ++-----
 deeptutor/api/routers/auth.py                                  |  10 +-
 deeptutor/api/routers/knowledge.py                             |  35 +++++++
 deeptutor/api/routers/practice.py                              | 151 ++++++++++++++++++++++++++--
 deeptutor/api/routers/question_notebook.py                     |   6 ++
 deeptutor/api/run_server.py                                    |   2 +-
 deeptutor/knowledge/kb_types.py                                |   9 ++
 deeptutor/knowledge/manager.py                                 |  38 ++++++-
 deeptutor/knowledge/manifest.py                                |   2 +
 deeptutor/runtime/agentic/client.py                            |   1 +
 deeptutor/runtime/launcher.py                                  |   4 +-
 deeptutor/services/config/model_catalog.py                     |  25 +++++
 deeptutor/services/config/origins.py                           |  18 ++++
 deeptutor/services/config/provider_runtime.py                  |  12 ++-
 deeptutor/services/config/runtime_settings.py                  |   6 +-
 deeptutor/services/llm/provider_core/anthropic_provider.py     | 198 ++++++++++++++++++++++++++++++++++---
 deeptutor/services/llm/provider_core/openai_compat_provider.py |   5 +
 deeptutor/services/llm/provider_factory.py                     |   1 +
 deeptutor/services/llm/reasoning_params.py                     |  30 +++---
 deeptutor/services/practice/analytics.py                       |  20 ++++
 deeptutor/services/practice/attempts.py                        |  21 ++++
 deeptutor/services/practice/importing.py                       |   9 ++
 deeptutor/services/practice/scheduler.py                       |  96 ++++++++++++------
 deeptutor/services/practice/storage.py                         | 208 +++++++++++++++++++++++++++++++++++----
 deeptutor/services/provider_registry.py                        |  21 ++++
 deeptutor/services/rag/factory.py                              |  12 +++
 deeptutor/services/rag/pipelines/tutor_library/__init__.py     |   1 +
 deeptutor/services/rag/pipelines/tutor_library/pipeline.py     | 252 +++++++++++++++++++++++++++++++++++++++++++++++
 deeptutor/services/session/sqlite_store.py                     |  59 ++++++++++-
 deeptutor/tools/question_bank.py                               |   3 +
 tests/agents/chat/test_teaching_policy.py                      |  72 ++++++++++++++
 tests/api/test_cors_settings.py                                |   7 +-
 tests/api/test_knowledge_tutor_library.py                      | 143 +++++++++++++++++++++++++++
 tests/api/test_main_exception_handler.py                       |   3 +-
 tests/api/test_practice.py                                     | 244 ++++++++++++++++++++++++++++++++++++++++++++-
 tests/api/test_practice_attempts.py                            | 440 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 tests/api/test_ws_origin_guard.py                              |  54 ++++++++++
 tests/runtime/test_uvicorn_launch_flags.py                     |  10 ++
 tests/services/config/test_catalog_env_keys.py                 | 157 +++++++++++++++++++++++++++++
 tests/services/config/test_model_catalog_example.py            | 111 +++++++++++++++++++++
 tests/services/config/test_runtime_settings.py                 |   7 ++
 tests/services/llm/test_anthropic_claude_5_5_requests.py       | 478 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 tests/services/llm/test_anthropic_provider_fixes.py            |  25 +++++
 tests/services/llm/test_opencode_provider.py                   | 371 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 tests/services/llm/test_reasoning_params.py                    |  29 ++++++
 tests/services/rag/test_tutor_library_pipeline.py              | 416 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 tests/services/session/test_sqlite_store.py                    | 118 ++++++++++++++++++++++
 tests/tools/test_question_bank_tool.py                         |  32 ++++++
 web/components/learning/practice/PracticeSession.tsx           |   8 ++
 web/components/space/question-bank/QuestionCard.tsx            |  18 ++++
 web/lib/notebook-api.ts                                        |   4 +
 54 files changed, 3998 insertions(+), 126 deletions(-)
```


---

# FILE: TUTOR.md

# Tutor

A personal tutor built on a fork of [HKUDS/DeepTutor](https://github.com/HKUDS/DeepTutor)
(Apache-2.0), with its own desktop app, a Rust retrieval engine for a large library, and an
ElevenLabs voice teacher. Upstream's own docs are in `README.md` and `docs-for-user/`.

## Layout

| Path | What |
|---|---|
| `corpus/` | **Your books and papers.** Never committed. See `corpus/README.md`. |
| `content/` | Your own notes and question bank (tracked). |
| `apps/desktop/` | Desktop app: Tauri 2 (Rust) shell + React 19 / TypeScript / Tailwind 4 UI. |
| `crates/rag-engine/` | Rust engine for surveying, ingesting and searching the corpus. |
| `Cargo.toml` | Rust workspace for `apps/desktop/src-tauri` and `crates/*` (one lockfile, one `target/`). |
| `docs/tutor/PLAN.md` | Build tracker: phases, decisions, known issues. |
| `deeptutor/`, `deeptutor_cli/`, `web/`, `tests/`, ... | Upstream DeepTutor backend (FastAPI), CLI, web UI and tests, with fork changes. |
| `data/` | Runtime data and indexes (never committed). |

## Run

```powershell
# Python backend (first time: python -m venv .venv; .venv\Scripts\python -m pip install -e ".[dev]")
.venv\Scripts\deeptutor serve                 # API on http://127.0.0.1:8001 (localhost only)

# Desktop app (first time: npm install in apps/desktop)
cd apps\desktop; npm run tauri dev

# Measure the corpus
cargo run -p rag-engine --release -- survey --root corpus --out data/rag/survey.json
```

## Checks

- Python: `.venv\Scripts\python -m pytest -q tests deeptutor/learning/tests` (52 known Windows-only
  upstream failures, listed in `docs/tutor/PLAN.md`), `ruff check`, `ruff format --check`, `lint-imports`.
- Rust: `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
- Desktop UI: `cd apps\desktop; npm run build`.


---

# FILE: docs/tutor/PLAN.md

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

Running, each on its own `wt/*` branch and merged after review:
- Design round 2: the Library, Evals, Progress, and Plan with Settings screens.
- The Tutor and Train screen polish.
- rag-engine remote mode, library browsing and parallel ingest (C9).
- Research round 3: corpus exploitation.

Next:
- Languages (C10): the world and language picker, shell TTS/STT with a cache, language cards and
  checking, the Languages screen, and per-language role-play agents.
- A corpus-exploitation build from research round 3: guided reading, concept maps, question generation
  from the library, and LLM query rewrite and rerank.
- A visual-QA pass.

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


---

# FILE: docs/tutor/PEDAGOGY.md

# Teaching method

The rules the tutor follows. Evidence: the verified syntheses in `docs/tutor/research/` (46-agent study,
2026-09-30). Where the areas disagreed, the decision below settles it. Effect sizes are priors from
children, undergraduates and lab samples. No study covers a PhD-level adult learning alone, so Tutor measures
its own effect on its one learner (Measurement below).

## Core loop

1. **Attempt first.** There is no ask-anything answer mode. The learner attempts before any explanation,
   hint or reveal. Unguarded AI help raised practice scores and cut later unaided performance, and
   guardrails mostly removed the harm (Bastani et al. 2025; erratum 10.1073/pnas.2518204122 still unread,
   so the "17%" figure stays out of product copy).
2. **Confidence before feedback.** For retrieval items the learner rates confidence first: 1 guess,
   2 unsure, 3 fairly sure, 4 certain. Voice asks "how sure, one to four?". Confidence routes the feedback
   and feeds the calibration chart. It never changes FSRS intervals.
3. **Graded hint ladder.** The rungs are: 1 nudge, 2 hint, 3 key step, 4 worked example, 5 answer.
   - Rungs come from the question's pre-written `hints` when it has them. Otherwise the tutor writes a
     conceptual nudge without the answer.
   - Rung ceilings are enforced by the server (contract C7), not by the prompt.
   - After three failed rungs the tutor gives a worked example plus a fresh isomorphic problem, not the
     original answer.
4. **Elaborated feedback that names the misconception.** Never right/wrong only: elaborated feedback
   g 0.49 vs right/wrong 0.05 (Van der Kleij 2015).
   - Use refutation form: state the wrong idea, why it fails, and the correct idea (g 0.41, Schroeder &
     Kucera 2022).
   - One idea and one question per turn.
   - Hold the correction under pushback (anti-sycophancy).
5. **High-confidence errors** (confidence 3-4, wrong): immediate re-study, then a near-transfer item later
   in the same session. Hypercorrection is shown for facts, so this stays a routing rule, not a promise.
6. **The tutor never sees the answer mid-attempt.** During a graded attempt the model gets the verdict
   from the check tool and the permitted rung text only. The solution, explanation and misconception enter
   its context only after the attempt closes. Answer checking is deterministic: key, CAS, unit or
   structure checker. The LLM grades free text only against a rubric; low-confidence grades go to the
   learner, and the tutor says "I cannot verify this step" rather than bluff.

## Two practice paths

| Material | Path | Scheduling |
|---|---|---|
| Atoms: facts, definitions, vocabulary, notation, named results | Retrieval with feedback, successive relearning: about 3 correct recalls in the first session, then one per FSRS review | FSRS, desired retention 0.9 (a tunable default; no study validates 0.9 itself) |
| Procedures, derivations, proofs, multi-step problems | Worked example, then completion (faded steps), then full problems with fresh variants of the same family | By mastery per skill; variants of one seed are **not** independent FSRS evidence |
| Writing (English) | Draft, then feedback on 2-3 global priorities, then revision. The tutor never writes the learner's prose; model text only after a revision | Deliberate practice sessions, not FSRS; a monthly timed AI-off writing sample |

**FSRS learns only from unassisted retrieval.** Rating per attempt, computed by the server:

- correct, no hint or rung 1 only: **Good**;
- correct after rung 2-3: **Hard**;
- rung 4-5 or wrong: **Again** (relearning).

## Stage policy per topic (resolves "novice first" vs "problem first")

- Entering a topic runs a 3-5 item diagnostic, and the topic's stage is set per skill from it. With no
  diagnostic, the stage is **novice**.
  - **Novice:** worked example first; no productive failure.
  - **Developing:** completion problems with fading.
  - **Proficient:** problem-first. A productive-failure opener is allowed (g 0.36, contested), then
    consolidation.
- Fading follows unassisted accuracy (expertise reversal: Tetzlaff 2025, weaker in humanities). The
  English track keeps its own fading rule and tests it.

## Self-explanation (resolves the conflict)

- **Required** on proofs and derivations: proof comprehension d 0.95 in Hodds 2014 Exp 1 (verified abstract
  claim); self-explanation g 0.55 (Bisra 2018). Also required after an error.
- **Not prompted** while studying a math worked example, where prompts lowered the worked-example effect
  (Barbieri 2023).
- Predict-then-reveal for demonstrations and simulations.

## Interleaving

A new procedure gets one short blocked introduction. After that, reviews of problem families mix
confusable types, starting with about 30% of problem reviews mixed (Brunmair 2019 math g 0.34; Rohrer 2020
d 0.83, grade 7). Atoms are never deliberately interleaved: for vocabulary-like pairs interleaving hurt
(g -0.39).

## What we take from top-performing systems

The data are K-12 and mostly confounded, so we take mechanisms, not national labels (see `research/intl.md`):

- **Comparison before rule.** 2-3 varied cases plus a near-miss non-example; the learner names the
  invariant, then the rule is stated (variation teaching; Alfieri 2013 d 0.50). Only after the
  prerequisite check passes.
- **One rich problem, then summary.** One rich problem per unit. The learner writes the summary before the
  tutor's (Japanese structured problem solving, adapted). Summaries and insights become FSRS atoms.
- **Coherence and mastery.** Prerequisite maps, move on at mastery, and cumulative mixed review
  (Shanghai/Singapore-style mastery; the English RCTs found +0.06 to +0.10 SD, so small).
- **Concrete-pictorial-abstract and bar models** for novices only. At PhD level the default is symbols
  first.
- **Paper path** (print, read, then a next-day closed-book quiz) as a measured option, not a mandate
  (Delgado 2018 g -0.21 for screens).
- **Honest claims.** The Nordics are not a top-performing block and Finland has declined. The tutor makes no
  "how Japan or China teaches" claim without a retrieved document.

## Sessions

- 45-minute blocks with a suggested break. Soft stop at 90 minutes a day, configurable.
- No points, streaks or XP.
- Short exposition, then a question. Interpolated questions double as the attention device (Szpunar 2013).
- An optional sleep and fatigue check-in adjusts load only.

## Voice

Voice is for retrieval, teach-back and short Socratic turns. Long derivations go to the screen. Spoken
maths may be mis-transcribed, so answers that need notation are typed or confirmed.

## Measurement (the headline metric)

- **Headline metric:** delayed (7 and 30 days) **unassisted** accuracy on parallel items the tutor never
  used for teaching.
- **Always separate:** assisted and unassisted results, hint dependence, reveal rate, calibration
  (confidence vs correctness) and minutes to criterion. Never satisfaction (Deslauriers 2019).
- **Experiments:** at most two or three personal (n=1) experiments at a time, each with a stated minimum
  detectable change. Differences under about 0.2 SD are noise. The first two are the n-back
  experiment (opt-in, `NBACK.md`) and interleaved vs blocked review for one math family.
- **Tutor behaviour evals:** answer-leak rate, sycophancy, and proof, mechanism and numeric error rates on
  the learner's own problems for Opus 5.5, Sonnet 5.5 and Muse Spark 1.3. These gate prompt changes.

## Question bank

- **Schema target:**
  - atomic skill; explanation; misconception; `hints` (ladder);
  - prerequisites; `confusable_with`; variant family (seed, varied dimension, invariant);
  - reference solution (verified); source citation (chunk id).
- **Format:** short answer by default. Multiple choice only with misconception-based distractors, taken
  from the learner's own wrong answers when available.
- **Generated items** must cite a source chunk and be blind-solved by a different model family. LLM
  difficulty labels are ignored (rho 0.06).

## Subject worlds

Theme the app chrome only: accent, icons, header motif. Learning panes (question, explanation, reading,
writing) stay neutral and identical across subjects. There is no decorative imagery in learning panes
(seductive details g -0.16), and themes are never presented as memory aids. Context-dependent memory
effects are small and fragile (Smith & Vela 2001, d about 0.25, secondary source).

## Round 2 resolutions (research round 2, 2026-09-30)

Sources: `research/cognition.md`, `curricula.md`, `engineering.md`, `languages.md`, `critique-2.md`.

- **Mastery (one definition).** Per skill, a decayed Beta posterior over **unassisted** attempts on fresh
  variants.
  - A skill is **mastered** when P(p ≥ 0.7) ≥ 0.8 **and** at least one unassisted success came 7 or more
    days after the last exposure. That is about 15-20 attempts over several days.
  - Assisted attempts never count.
  - Periodic batteries are measurement, not mastery.
- **Prerequisites are advisory.** They are shown as "ready" or "not yet" with the reason, never as hard
  gates; the learner can override. The graph is small, hand-edited data with typed edges.
- **Confidence scale.** 1-4 everywhere. For calibration and Brier scores it maps to probabilities
  0.25 / 0.5 / 0.75 / 0.95.
- **First exposure vs practice.** At the novice stage, first exposure to a procedure is a labelled worked
  example. That is instruction, not practice. Attempt-first governs every practice and retrieval item.
- **Variant families are testlets.** A family is one FSRS card, with at most one fresh variant per family
  per day.
- **FSRS.** FSRS-6 defaults at retention 0.90, capped at 0.95. Parameters are personalised only after
  about 1,000 unassisted reviews, and only if a chronological holdout beats the defaults; both parameter
  sets are stored.
- **Daily workload cap.**
  - A configurable daily review budget (default 45 minutes or 150 reviews). Overflow is served by lowest
    retrievability first.
  - New items are capped per day (default 15 atoms, and 10 new words, which is 20 cards, per language).
  - Without the caps, a language's cards alone would exceed 500 reviews a day within a year.
- **Experiment register.** At most two running at a time, each with a preregistered minimum detectable
  effect. Small effects are labelled underpowered.
- **Verification ladder.**
  1. Deterministic checkers first: SymPy and Pint for maths and units, RDKit for structures, Lean
     optional for proofs.
  2. Then a rubric with at least two independent LLM passes, calibrated against the learner's own grading.
  - Never a single-pass LLM verdict on proofs or mechanisms.
  - Otherwise: "I cannot verify this."
- **Grounding.** Formulas, quantities, quotes, dates, constants and citations are stated only from a chunk
  retrieved this turn or a checker result. After every cited answer, a deterministic span check confirms
  that each number and equation appears in the cited text. On a miss: "not found in the library".
- **Cognition track.**
  - Every claim about intelligence or cognition carries an evidence tag (strong / moderate / weak /
    contested, plus design and population).
  - Approved line: schooling-type learning is the best-supported route, and gains carry mainly to what
    you practise (Ritchie & Tucker-Drob 2018: 1-5 IQ points per year of schooling, quasi-experimental,
    mostly youth).
  - No IQ or brain scores.
  - Supplements and stimulants get an evidence-tiered answer with no doses, and a referral to a
    clinician.
  - A statistics and probability family uses natural frequencies and diagrams first, then notation.
  - Later: an argument-map editor (Cullen 2018: d 0.71, quasi-experimental).
- **Sleep.** After a short night, the tutor suggests favouring retrieval over new material. This is a
  weak inference, shipped as an optional suggestion; the outcome is logged, and scheduling is never
  enforced by it.
- **Mock qualifying exams (later).** Per-subject exam simulations modelled on published formats (e.g.
  Berkeley math: two 3-hour sittings), plus a voice oral-examiner mode. There is no pass probability or
  time-to-PhD score.

## Not building

- An open answer chat, answer-reveal review credit, points, streaks or XP.
- Learning-style profiles, 2-sigma or "brain" claims.
- Free-explore simulation rooms.
- Themes as memory aids.
- n-back as a study strategy or a gate on anything.


---

# FILE: docs/tutor/DESIGN.md

# Tutor design direction

Status: from the gallery study and the pedagogy research (2026-09-30). The research
(`research/tools.md`) limits subject theming to the app chrome. Learning panes (question, explanation,
reading, writing) stay neutral and identical across subjects. Decoration in learning panes hurts learning
(seductive details g -0.16), so there is none. Colour signals function, never decoration.

Source: the 100 Sonnet 5.5 HTML pieces at miaai-lab/Sonnet-5.5-100-HTML-Files, studied by seven agents. That
repository has no licence, so we take design ideas only and write our own code. Piece numbers are cited so
each decision can be traced back.

## Principles

1. **A calm instrument.** Long study sessions come first. Typography and hairlines carry the look; effects
   stay at the edges and never sit behind text at full strength (034 scrims, 017 reading type).
2. **One accent per world.** Each subject has its own accent and ambient tint. Everything else is neutral
   (002, 005, 100).
3. **Numbers are the heroes.** Large light tabular numerals over small tracked mono labels, divided by
   hairlines (024, 005, 072, 048).
4. **Motion is earned.** Motion responds to an action or a state change. Nothing loops behind reading text.
   Reduced motion stops loops rather than slowing them (002 and 014 got this wrong).
5. **The GPU is busy.** The 5090 runs other workloads, so the only live WebGL is the Orb. Everything else is
   CSS, SVG or Canvas2D that repaints only when something changes and pauses when hidden (053, 010, 098).

## Tokens

Neutral base, warm rather than grey (017, 005):

| Token | Light ("paper") | Dark ("lamplight") |
|---|---|---|
| background | `#f5f0e6` | `#1a1714` |
| card | `#fbf7ee` | `#221d18` |
| ink (text 1) | `#1a1714` | `#f1eadb` |
| text 2 / text 3 | ink at .72 / .58 | ink at .72 / .58 |
| hairline / strong hairline | ink at .12 / .28 | ink at .15 / .32 |

- Text 3 must stay at or above 4.5:1. The gallery's .46 tiers fail.
- Labels are never smaller than 11px.
- Colour never carries meaning alone: pair it with a word or a mark.

### Subject worlds

`data-world` on the root swaps a small token set. `@property`-registered colours cross-fade over 0.9 to
1.2 s (057, 005, 061). With no subject selected, the world is neutral.

| World | Accent, dark / light | Ambient | Motif (static SVG, drawn once) | Draws on |
|---|---|---|---|---|
| Math | periwinkle `#9aa6f0` / ink blue `#2f3f8f` | slate washes | times-table cardioid chords, ruler ticks | 074, 098, 025 |
| English | ember `#e4573a` / vermilion `#c8402a` | paper-and-ink washes | book spines, drop cap | 017, 081, 064 |
| Science | cyan `#56d4e0` / teal `#1a7f86` | indigo washes | orbit rings, moon phases | 014, 091, 045 |
| Chemistry | brass `#dcb86e` / bronze `#8c6218` | cream/walnut washes | hexagon lattice, molecule sprite | 084, 040 |

- The four hues are spaced around the colour wheel.
- Before shipping, check equal OKLCH lightness and AA contrast for each accent, using a 060-style check
  that stays out of the app.

### Type

- **Geist** (UI), **Geist Mono** (labels and numerals), plus one bundled OFL serif for display and reading.
  The candidate is **Newsreader** (variable, optical sizes, true italic).
- The gallery's Iowan, Palatino and Bodoni stacks don't exist on Windows, so every font is bundled.
- Display numerals: Geist 200, `tabular-nums`, tight tracking (-0.03 to -0.05em).
- Eyebrows: Geist Mono 11px, uppercase, +0.18em, paired with a 24px hairline (066).
- Reading surfaces (Plan, explanations, misconceptions): serif 17 to 18px, line-height 1.6, about 68ch,
  with `text-wrap: pretty` (017, 064).
- Math symbols and emphasis use serif italic (026).

## Components

| Component | Decision | From |
|---|---|---|
| Sidebar | Icon-and-label rail; the active item is a spring-driven pill (stiffness 260, damping 20); a subject pill switches world | 048, 065, 053 |
| Stat row | Hairline-divided cells: big light numeral, mono caps label, small delta chip | 005, 024, 048, 072 |
| Charts | Hand-built SVG kit: 2px lines, 10% area wash, labels at line ends, dim others on hover, keyboard scrubber, chart/table toggle, draw-on once (under 1.2 s). Flat fills with a darker same-hue outline. Train/test whiskers as range bars | 012, 048, 010, 053 |
| Answer keys A-D | Keycaps: contact shadow, skirt, top face, about 4px travel with a spring press. Bound to the A-D and 1-4 keys. The chosen answer stays latched with an LED dot. The result is shown by a tint plus a mark | 061, 020 |
| Question card | Hover-only holo sheen and glare: transform-only layers in static blend containers, 6 to 8 degree tilt, glare opacity at most .4. Text sits outside the blend stack. Flips to explanation and misconception. The frame ring tints by mastery | 029 |
| Orb stage | Ambient gradient blobs (transform drift, 60 to 120 s, static under reduced motion). A local scrim behind the transcript. Dark-glass state chips with a state word. The page tint follows the Orb state | 014, 002, 034, 051 |
| Progress | Due/overdue gauge (21-tick arc, spring needle), 30-day moon-phase discs (lit fraction = share of the daily goal), mastery sunburst (subject > topic > skill) with a table view | 005, 091, 040 |
| Library | Book-spine shelf (height = pages, colour = subject, 2D lean on hover). Filter chips with counts. The ingest log is typed mono lines with a spring check-stamp | 081, 071 |
| Plan | Editorial reading column, progress hairline, sticky outline with scroll-spy | 017, 064 |
| Settings | Dot-leader service rows with an LED plus a state word | 051 |
| Train (Dual N-Back) | Neutral stage: 3x3 grid of soft cells, the active cell lit in the world accent with shape redundancy, keycap response keys (A position, L letter), level and rule shown, block-end summary, separate training-curve and transfer panels, no streaks. Spec: `NBACK.md` | 061, 025 |
| Session chrome | Session timer. After 5 s idle, chrome dims to 40% over 1.8 s and returns on hover or focus | 079 |

## Motion and performance

- One shared spring ticker that sleeps when idle (065).
- Transforms and opacity only.
- Easing `cubic-bezier(.2,.8,.2,1)` for entrances and `cubic-bezier(.3,1.35,.4,1)` for toggles.
- A "lite by default" governor: richer ambients turn on only after 20 frames under 24 ms, and step back above
  52 ms. It is exposed as `data-quality` on the root (053).
- No `backdrop-filter` over anything that animates. No full-screen per-frame canvases or shaders besides
  the Orb. Grain is a pre-generated tile, not a filter.
- Every loop pauses on `visibilitychange`. `prefers-reduced-motion` removes ambient loops and keeps state
  transitions (0.2 to 0.5 s).
- Sound is off by default. Nothing plays during a voice call, since it would leak into the microphone
  (020, 056).

## Not adopting

- Live shaders: 002, 014, 022, 066.
- Full-screen simulations: 026, 034 live, 090.
- Custom cursors, scroll-jacking, text effects that blur or rotate, and difference-blend type.
- Skeuomorphic chrome beyond the answer keys.


---

# FILE: docs/tutor/NBACK.md

# Dual N-Back

Evidence: `docs/tutor/research/nback.md`.
- Training reliably improves the n-back task and similar n-back tasks (g 0.62 on untrained n-back, Soveri
  2017).
- Transfer to other working-memory tests is small (g 0.24).
- Transfer to reasoning or academic outcomes is not established once active controls and bias are
  handled.

The module is therefore a well-built, optional, honestly labelled skill practice with a personal experiment
that tests any broader benefit on this learner.

## Rules

- Copy says: "This trains the n-back task. Transfer to other skills is small or unproven; the Experiment
  tab tests it on you."
- It never claims IQ, "brain power", working-memory age, learning speed or grades.
- n-back never drives FSRS, question difficulty, lesson gating or curriculum. The tutor may mention it only
  as an optional warm-up after due reviews.
- Dose default: 15-20 minutes, up to 3 sessions a week, a soft 60 min/week reminder. Adjustable. No streaks,
  XP, badges, leaderboards or multipliers.
- Adaptation is always visible: the level and rule are shown on screen and logged per session.

## Task

- **Modes:** dual (default), single position, single letter. Start at n = 1.
- **Stimuli:**
  - Position: 8 cells of a 3x3 grid (centre unused).
  - Letters: 8 non-rhyming consonants C H K L Q R S T, as pre-rendered clips.
- **Timing:** 500 ms stimulus, 3,000 ms trial, 20 + n trials per block.
- **Targets:** about 25% matches per modality (about 6 per stream), including dual matches.
- **Lures:** off by default; optional, and tagged in the log.
- **Generation:** a seeded generator; the seed is stored.
- **Adaptation (Jaeggi/Thompson, per block):** n goes up when each stream has 2 or fewer errors (misses
  plus false alarms) and down when either stream has more than 5. Thresholds are configurable.
- **Session:** 12-20 blocks (default 15) with 10 s rests.
- **Keys:** A = position match, L = letter match. Remappable. Buttons are shown too.
- **Feedback:** a block-end summary by default. In-trial feedback is opt-in and marks the session
  non-comparable.
- **Void rule:** a block is void on focus loss, the window hiding, or an audio device change.
- **After the session:** one question, "Which strategy did you use?".

## Scoring

- **Per modality:** hits, misses, false alarms, correct rejections.
- **d'** and criterion **c**, with the log-linear correction (add 0.5 to each count, 1 to each total).
- **Raw counts** are always shown next to d' and c.
- **Response time** is logged as a within-person trend only.

## Timing engineering

- One AudioContext, created on the Start click.
- Letter clips are pre-rendered offline, committed as small WAVs, decoded at startup, trimmed of silence
  under 450 ms and loudness-normalised. They are generated once with Windows SAPI (offline, free) or later
  with ElevenLabs. There are no live TTS calls and no `speechSynthesis` in the trial loop.
- Lookahead scheduling on the audio clock; visual onset on the frame nearest the scheduled time.
- Each trial logs intended vs actual onsets, plus `outputLatency` and `baseLatency`.
- A calibration screen (audio/visual sync check) is shown; timing is labelled "unmeasured" until it has run.

## Measurement

- **Training curve:** trained task only. Each session is a point; rolling median with an uncertainty band;
  level, hit and false-alarm rates, and d' per modality. It carries the line: "n-back scores are noisy and
  do not measure intelligence."
- **Transfer check:** every 4 weeks, labelled "measurement". A single n-back with different stimuli
  (shapes instead of grid positions, tones instead of letters), fixed n (the current level minus 1) and
  fixed settings. Ungamified.
- **Experiment card (opt-in, later phase):**
  - 3-5 baseline probe sessions: backward digit span, a short operation span, and open reasoning items once
    their licences are checked (ICAR, MaRs-IB).
  - A locked plan, an active control (adaptive visual search), and a reliable-change analysis.
  - The minimum detectable change is shown first (about 18 IQ points for one matrix probe), with a
    statement that a null result is expected.
- **Log:** expectation (one rating per session), sleep hours, exercise and fatigue.

## Architecture

- `apps/desktop/src/features/train/`:
  - React screen: task, settings, calibration, history.
  - TypeScript engine: generator, scoring, adaptation, audio scheduler. All pure functions are unit-tested.
- Storage in the Rust shell (contract C6): SQLite in the app data directory, per-trial rows, CSV export.
- Sidebar section "Train", between Progress and Plan.

## Not building

- IQ, brain-age or composite scores.
- Triple, quad or arithmetic modes.
- Streaks.
- Stimulation.
- n-back-styled subject content.
- Any code or assets from Brain Workshop (GPL-2.0, ideas only).


---

# FILE: docs/tutor/LANGUAGES.md

# Foreign languages

The fifth subject, with ElevenLabs as the voice. Evidence: `docs/tutor/research/languages.md`. All
teaching rules in `PEDAGOGY.md` apply. This file adds what is specific to second-language learning.

## Method

1. **Attempt first in the target language.** No translation or gloss before an attempt. The learner may
   ask for help through an on-screen gloss tool, which is logged as assistance.
2. **Brief explicit rule in English, then production.** Instruction works (Kang et al. 2018: g 1.06); the
   explicit/implicit difference is minor, so keep rules short and move to output fast.
3. **Spaced retrieval with FSRS.** Spacing is strong in L2 vocabulary (Kim & Webb 2022, 48 experiments).
   - Each word or phrase gets two cards: recognition (L2 to meaning) and production (meaning to L2, typed
     or spoken). A listening card (audio to meaning) is added for words met in audio input.
   - Retention target 0.9.
   - A spoken card completed with a hint or gloss earns no Good rating.
4. **Production formats by default:** L1-to-L2 translation, sentence cloze, dictation, rubric-scored
   free response. Every item is a full sentence with audio and provenance (source, licence, id), or
   carries an "unverified" flag. It gets a second-pass check before it enters FSRS.
5. **Corrective feedback order** (Lyster & Saito 2010: prompts beat recasts):
   - **Accuracy mode:** clarification request, then repetition, then a metalinguistic clue, then
     elicitation, then explicit correction. At most two corrections per turn.
   - **Fluency mode:** no interruptions. An end-of-session digest of 1-3 errors, each turned into
     production cards. Recasts are used only when a slip breaks comprehension.
6. **Perception-first pronunciation.** A minimal-pair trainer using high-variability phonetic training
   (HVPT: g 0.92 pre-post, 0.67 vs control; Uchihara et al. 2025): several voices, real words, immediate
   feedback, and held-out voices and words at test.
   - Production transfer is smaller (d 0.54), so it is trained and tested separately: listen, repeat,
     compare.
   - ElevenLabs voices stand in for natural talkers, which is untested. They are native-checked and
     labelled until a personal test (3 voices vs 1 on one contrast) runs.
7. **Comprehensible input lane.** Graded texts and audio stories whose known-word coverage is computed
   from the learner's own FSRS state. The target is 95-98% for reading and about 95% for audio. Each text
   ends with a short comprehension check. Reading is extra exposure, not the main source of new cards.
8. **Honest speech claims.**
   - The tutor sees recogniser text, not sound. It says "the recogniser heard X", never "your accent is
     good", and the raw transcript is shown beside any corrected form.
   - Scribe is a machine-listener intelligibility probe, never a pronunciation score.
   - Phoneme-level scoring (Azure Pronunciation Assessment, paid, prosody en-US only) is used only for
     scripted drills in supported locales, as phoneme colouring. There is no single 0-100 score.

## ElevenLabs use

| Use | Model | Notes |
|---|---|---|
| Pre-generated drill, sentence and story audio | Eleven v4 | 90+ languages; the only model documented for non-English IPA pronunciation-dictionary rules. $0.022 per 1k characters during the discount to Oct 12, list $0.08 |
| Long narrative with numbers | Multilingual v2 | 29 languages |
| Live role-play agent | Flash v2.5 or v3 Conversational | Only after testing which model an agent actually uses |
| Speech to text | Scribe v2 | $0.22/h, word timestamps. WER tiers: 34 languages at 5% or less (Spanish, French, German, Italian, Portuguese, Russian, Japanese, and others); Mandarin 5-10%; Arabic, Korean, Hebrew, Thai 10-20% |

- **Per-language config table:** `{lang, variety, tts_model, voice_ids[], speed, dictionary_id, stt_lang}`.
- **QA gate per language:** a 50-sentence native-checked gold set. An "audio unverified" badge shows until
  it passes. A TTS-to-Scribe round trip catches gross errors cheaply.
- **Role-play agent:**
  - One agent per target language, with the language pinned and the language-detection tool off (an
    accent can trigger an unwanted switch).
  - Scenario and level come in through dynamic variables. English help is an on-screen client tool, not a
    spoken switch.
  - Retention is set explicitly to short, and audio storage is off.
- **Audio cache:** generated audio is cached locally (in `data/`), so each sentence is paid for once.

## Measurement

- **Monthly no-AI production hold-out:** speak and write unaided from a fixed prompt, scored on a fixed
  rubric (accuracy, complexity, fluency in words per minute and pauses, intelligibility). This is the
  test for the crutch effect.
- **Delayed retests:** at 7 and 30 days, on new sentences and new voices.
- **Level:** per-skill CEFR-style bands labelled "Tutor estimate, not an official level". Never inferred
  from card accuracy or chat alone.
- **Placement (about 10 minutes):** yes/no vocabulary with pseudowords by frequency band, a short
  read-and-answer block, and a 3-5 minute voice interview.

## Languages

- **First candidates** (top STT tier and deepest open data): Spanish, French, German, Italian.
- **Available but more hours:** Russian, Japanese.
- **Weaker speech tooling:** Mandarin, Arabic, Korean, Hebrew.
- **Reading-only classical languages:** Latin, Ancient Greek, Biblical Hebrew, with no pronunciation
  scoring.
- The owner picks.

## Sources

- **Grounded, licence-tracked corpora per language:** Tatoeba (filtered by rating), Wiktionary via
  Kaikki, Universal Dependencies, Wikisource and Gutenberg for reading, plus the owner's notes.
- **Metadata only:** frequency lists.
- **Not used:** copyrighted textbooks, subtitles or commercial graded readers.
- **Graded text:** open A2-C1 graded readers were not found, so graded text is generated and
  coverage-validated.

## Not building

- Streaks, XP or leagues.
- Certified or chat-inferred CEFR scores; fluency percentages; hours-to-fluency countdowns.
- Pronunciation scores from transcripts.
- Always-on language detection.
- Open free chat without attempt-first.
- Shadowing as a headline feature.
- Local phoneme models (no GPU).

## Open tests before build

1. Scribe and the agent's ASR on the owner's accented beginner speech: does it silently fix errors?
2. Which TTS model an agent uses per language; whether dictionary rules apply inside agents; the real
   per-minute agent cost.
3. The native-checked gold set per language, and an Azure pilot with deliberate mispronunciations.


---

# FILE: docs/tutor/CONTRACTS.md

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


---

# FILE: docs/tutor/research/cognition.md

# Cognition area synthesis: what raises intelligence and cognition, and how Tutor should teach the science

Verifier/synthesizer pass, 2026-09-30. Inputs: I1 (raising intelligence), I2 (cognitive-science curriculum), I3 (expertise and thinking). I re-opened 24 of the most load-bearing sources via Europe PMC, Crossref, arXiv, PMC full text and openstax.org. Fetch output came from a small summarizing model, so numbers are as that model quoted the abstract; licence readings carry moderate confidence. Web search was not needed. I read abstracts (and Kestin's full text), not full papers.

## 1. Bottom line

1. The honest product claim: Tutor teaches content and skills. It cannot raise g, and it must not imply that it can. Schooling-type learning is the best-supported lever for measured cognitive ability (about 1-5 IQ points per extra year, quasi-experimental, mostly youth), and gains carry mainly to what is practised.
2. Nearly every "brain booster" (exercise, bilingualism, music, games, supplements, stimulants, tDCS, brain training) is small, contested or null once bias is corrected. None justifies a feature.
3. The strongest design levers are the ones Tutor already has: retrieval practice with feedback, spacing, attempt-first, hint-guarded AI. The main new work is measurement honesty (delayed, unassisted, fresh parallel items, calibration, evidence tags) and a few question families with real evidence (statistics and probability, argument analysis, evidence critique).
4. No verified evidence covers adult self-learners using an AI tutor for months at PhD depth. Every transfer claim to Tutor is an extrapolation and should be logged as a default to be checked against the learner's own data.

## 2. Verified citation table

Status key: V = I re-opened the record and the figures match. C = corrected or narrowed. W = worker-verified only, not re-checked by me (treat as moderate confidence). No citation failed to exist.

| # | Source | Claim checked | Status | Note |
|---|---|---|---|---|
| 1 | Ritchie & Tucker-Drob 2018, Psychol Sci | 142 effects, 42 datasets, >600,000 people; about 1-5 IQ points per extra year; persists across lifespan | V | Abstract calls education "the most consistent, robust, and durable method yet to be identified for raising intelligence". Quasi-experimental; mostly schooling in youth. |
| 2 | Pietschnig & Voracek 2015, Perspect Psychol Sci | 271 samples, nearly 4M people, 1909-2013; 0.41/0.30/0.28/0.21 points/yr (fluid/spatial/full-scale/crystallized) | V | Gains declined in recent decades. Population trend, not an individual lever. |
| 3 | Bratsberg & Rogeberg 2018, PNAS | Flynn effect and reversal recoverable from within-family variation | V | Norwegian conscripts only. |
| 4 | Bratsberg et al. 2026, PNAS | 579,379 Norwegian men, 25 cohorts (1967-1991); reversal first among high-income sons; education expansion delayed it elsewhere | V | I1's "masked an earlier decline" is fair. Authors call upper-secondary completion a "potential lever". |
| 5 | Ciria et al. 2023, Nat Hum Behav | Exercise d 0.22, then 0.13, then 0.05 (CI -0.09 to 0.14) after publication-bias correction | V | Healthy population RCTs. Good worked example for a myths course. |
| 6 | Newbury et al. 2021, Psychol Bull | Deprivation before learning g 0.621 (CI 0.473-0.769); after learning g 0.277; publication bias in both | V | Total acute deprivation (lab), not ordinary short sleep. See conflict C3. |
| 7 | Lehtonen et al. 2018, Psychol Bull | 152 studies, 891 effects; "no evidence for a bilingual advantage remained after correcting for bias" | V | Adults. I1's Nichols 2020 (n=11,041) not re-checked (W). |
| 8 | Sala & Gobet 2020, Mem Cogn | Music training N=6,984, k=254; overall effect null once study quality controlled | V | Children only. |
| 9 | Gobet & Sala 2023, Perspect Psychol Sci | "Overall effect of far transfer is null" | V | |
| 10 | Simons et al. 2016, PSPI | "Little evidence" of transfer to distant tasks or everyday cognition | V | |
| 11 | Scharfen et al. 2018, Psychon Bull Rev | Retest g = 0.28 (first to second administration), 95 samples, 68 studies, ages 12-70 | V | Working-memory tests specifically. I1 generalises to "any repeated test"; that is an extrapolation. |
| 12 | Xu et al. 2024, Front Nutr | Creatine: 16 RCTs, n=492; memory SMD 0.31 (0.18-0.44), moderate certainty; no global cognition or executive effect | V | Erratum (2025-02-17) and a commentary (2026-04-10) exist and were not read. Ages 20.8-76.4. |
| 13 | Yang et al. 2021, Psychol Bull | Practice testing g = 0.499, 222 studies, 48,478 students | V | Classroom; larger with feedback. |
| 14 | Freeman et al. 2014, PNAS | Active learning 0.47 SD (158 studies); odds of failing under lecture 1.95 (67 studies); larger in classes of 50 or fewer | V | Undergraduate STEM. |
| 15 | Abrami et al. 2015, Rev Educ Res | CT instruction g+ = 0.30, 341 effect sizes; dialogue, authentic problems, mentoring helped | V | Moderators by age not read. |
| 16 | Cullen et al. 2018, npj Sci Learn | 12-week argument-visualization seminar: LSAT logical reasoning d = 0.71 (CI 0.37-1.04); essays d = 0.87 (CI 0.26-1.48) | V | Quasi-experimental; wide CIs. I3's "essay" outcome is essay scoring. |
| 17 | Macnamara et al. 2014, Psychol Sci | Deliberate practice explains 26% games, 21% music, 18% sports, 4% education, <1% professions | V | I2's "Ericsson 29-61% vs 14%" was not verified; see C5. |
| 18 | McDowell & Jacobs 2017, Psychol Bull | 226 estimates, 35 articles; short-menu formats and visual aids strongest moderators | V | No headline effect size in abstract. |
| 19 | Graham et al. 2020, Rev Educ Res | Writing about content raises learning, effect size 0.30, 56 experiments, grades 1-12 | V | School-age only. |
| 20 | Sinha & Kapur 2021, Rev Educ Res | Problem-solving-then-instruction g 0.36 (0.20-0.51), 53 studies, 166 comparisons; 0.37-0.58 with high-fidelity Productive Failure | C | I3 said "larger for older learners". Abstract: younger learners (grades 2-5) and domain-general skills trended toward instruction first. Keep attempt-first for conceptual learning only. |
| 21 | Bastani et al. 2025, PNAS | About 1,000 high-school maths students; GPT Base +48% in practice, -17% after removal; GPT Tutor +127% in practice, harm largely mitigated | C | The -17% is for unguarded GPT Base only; guarded tutor did not show it. Adolescents, not adults. |
| 22 | Kestin et al. 2025, Sci Rep | N=194 eligible; crossover of two lessons in consecutive weeks (9-10); Harvard Physical Sciences 2; quantile 0.73-1.3 SD, regression 0.63; median 49 min (AI) vs about 60 (class) | C | The abstract has no numbers; they come from full text (Europe PMC). Preregistration not mentioned. Single site, two lessons, tutor seeded with expert step-by-step solutions. |
| 23 | Kosmyna et al. 2025, arXiv 2506.08872 | 54 in sessions 1-3, 18 in session 4; EEG; LLM users weakest connectivity | C | Page shows revision v2 (31 Dec 2025) and no peer-review indication. Still not citeable as proof of harm. |
| 24 | OpenStax Psychology 2e | Licence | C | openstax.org details page and book page both returned CC BY-NC-SA; published 22 Apr 2020. Two small-model reads agree, but confirm on the book's copyright page before any ingest decision. |

W (not re-checked by me, carried from the workers, moderate confidence): Sedlmeier & Gigerenzer 2001; Mellers 2014; Sellier 2019 (19% fewer confirmation-biased choices, 290 grad students; a corrigendum exists); Heijltjes 2014; Hubbard & Dunbar 2017; CREATE (Stevens & Hoskins 2014); Melby-Lervag 2016; Sala & Gobet 2017; Dunlosky 2013; OSC 2015; Pashler 2008; Nichols 2020; Sala 2018 and Bediou 2018 (video games); Lim & Dinges 2010; Horvath 2015; Alex 2020 (omega-3); Ilieva 2015; Repantis 2021; Whitfield 2022; Barnes 2023 and Sachs 2023; Plomin & von Stumm 2018; Pietschnig 2022 (brain size); the 2025-2026 supplement and caffeine papers in I1 (checked only by Europe PMC records by the worker).

Dropped as unsupported (recall only in the worker reports, not usable as facts): Twardy 2004 (about 0.8) and other older argument-mapping figures; Bisra 2018 (about 0.55), Schwichow 2016 (about 0.6), Bangert-Drowns 2004 (about 0.2), Kalyuga expertise-reversal numbers; the "87-91%" neuromyth-belief figures (I2 attributes them to 2026 papers not opened by the worker); the EFSA 400 mg caffeine limit; Dworak et al. The claims these supported are kept only as hypotheses to measure, or removed.

## 3. Conflicts and resolutions

- C1. "Ban neuroplasticity claims" (I1) vs teaching LTP/LTD and plasticity in the neuroscience track (I2). Resolution: the ban covers claims about the learner's own brain ("Tutor rewires your brain", "builds connectivity"). Curriculum content about plasticity mechanisms is normal teaching and carries an evidence tag.
- C2. Attempt-first always (I1/I2/I3) vs Sinha & Kapur caveat (domain-general skills and young novices favoured instruction first). Resolution: attempt-first default for conceptual material at the learner's level; a labelled worked-example mode for true novices and procedural skills. Log which mode was used per item so the owner can compare delayed accuracy.
- C3. Sleep rule strength. I1 rests "short sleep means review, not new encoding" on Newbury's g 0.62, but that is total acute deprivation in lab studies with publication bias, not a night of 5-6 hours. Resolution: ship as a soft default ("lighter encoding load, more retrieval"), tagged weak inference, and never claim it improves learning.
- C4. No n=1 experiments (I1) vs within-person alternating designs (I3). Resolution: no n=1 tests of caffeine, creatine, exercise or any effect under about 0.3 SD. Within-person alternation is allowed only for pedagogical choices expected to be large and measurable over many items (for example worked-example vs attempt-first), reported with intervals and labelled exploratory.
- C5. Expertise numbers. I3 gives Macnamara's domain variances (verified); I2 cites Ericsson-and-Harwell 29-61% vs 14% (not verified). Resolution: cite only Macnamara as verified, mark the dispute "contested", and teach it as an open debate. Do not use "10,000 hours" or any practice-quota claim.
- C6. Retest effect generality. I1 says every repeated test inflates scores (g 0.28). Verified only for working-memory capacity tests. Resolution: still use fresh parallel items for delayed checks (cheap, conservative), but do not quote 0.28 as a general figure.
- C7. Strength grades. I1 rates education-raises-IQ "moderate to strong", I2 "moderate to strong", I3 "moderate-strong". I agree with "moderate": quasi-experimental designs, youth samples, gains may be test-specific. Bastani is "moderate" (one field experiment, adolescents), Kestin "moderate-weak" (one site, two lessons).
- C8. Licence of OpenStax Psychology 2e (I2 flagged). Two fetches say NC-SA; if true it follows the same rule as MIT OCW: metadata and own notes unless the owner explicitly relaxes AGENTS.md.

## 4. Ranked recommendations

Rank = evidence strength times impact on Tutor. Evidence tag: S strong, M moderate, W weak/inference.

| Rank | Recommendation | Tag | Evidence | Area |
|---|---|---|---|---|
| 1 | Retrieval practice with feedback and spacing for facts and skills (already built); keep FSRS as the main schedule. | prompt, scheduling | S: Yang 2021 g 0.50; Dunlosky 2013 (W) | scheduling |
| 2 | Keep the attempt-first hint ladder; require a first attempt or prediction before worked answers; no unguarded answer-on-demand mode; labelled worked-example mode for novices (C2). | prompt | M: Bastani 2025, Sinha & Kapur 2021 (g 0.36), Kestin 2025 | prompt |
| 3 | Learning signal is delayed, unassisted (no hint, no AI), held-out item accuracy per domain. Report it separately from in-session performance, never as a composite "brain" or IQ score. | measurement | M-S: Bastani (practice up, later down), Scharfen retest effect | measurement |
| 4 | Every delayed check uses fresh parallel items generated from the same skill, never the items used in teaching; keep a small stable non-generated battery (Bayes/base-rate, control of variables, argument analysis, calibration) with forms A/B/C every 6-8 weeks; claim only item-level within-person change. | question generation, measurement | M: Scharfen, design logic (n=1 has no control) | measurement |
| 5 | Evidence tag (strong/moderate/weak/contested, design, population) on every cognition claim in tutor prompts and concept cards; say "not checked" when unsure. | prompt, corpus | M: replication and neuromyth literature (OSC 2015, Pashler 2008; W) | prompt |
| 6 | Hard-coded claims policy: no IQ-boost, brain-boost, neuroplasticity, bilingual-brain or music-IQ claims about the learner. Approved line: "Schooling-type learning is the best-supported way to improve measured cognitive skills; gains carry over mainly to what you practise." | prompt | S: Ritchie 2018, Lehtonen 2018, Sala & Gobet 2020, Gobet & Sala 2023 | prompt |
| 7 | Statistics and probability question family: natural frequencies plus a diagram first, then notation; Bayes, base rates, CI and p-value misreading, regression to the mean, selection bias; fresh surface variants on the FSRS track. | question generation | M: McDowell & Jacobs 2017; Sedlmeier & Gigerenzer 2001 (W); Mellers 2014 (W) | question gen |
| 8 | Confidence rating (0-100) before feedback on critical items; per-domain calibration (Brier/plot) dashboard. No claim that it transfers. | UI, measurement | W: motivated, no verified transfer effect | UI |
| 9 | Argument-map editor (claim, reason, objection, rebuttal) with structure feedback from the tutor. | feature | M: Cullen 2018 d 0.71/0.87 (quasi-experimental, single domain) | feature |
| 10 | Evidence-critique and paper-reading items: given a real abstract, name design, sample, effect, one threat to validity; staged reading mode (predict conclusion from Methods/Results, critique, propose follow-up). | question generation, feature | W-M: Hubbard & Dunbar 2017, CREATE (W); Abrami 2015 g+ 0.30 for dialogue and authentic problems | feature |
| 11 | Derivation and simulation items in Python for neuroscience (adapt Neuromatch notebooks, CC BY 4.0); maths prerequisite gating; strand-by-level progress map. | feature, question generation | W: curriculum synthesis, not a program requirement | feature |
| 12 | Sleep check-in default: after short or poor sleep, prefer review and retrieval over heavy new encoding; log the outcome (C3). | scheduling | W: inference from Newbury 2021 | scheduling |
| 13 | Supplement, stimulant or modafinil questions: short tiered answer from evidence cards; no doses, no prescription recommendations, refer to a clinician. | prompt | M-W: small domain-specific effects, publication bias (Xu 2024, Ilieva 2015; W) | prompt |
| 14 | Cognition evidence-cards shelf and myths mini-course (original notes plus DOIs/links; Ciria 0.22 to 0.05 as a worked example of bias correction). | corpus | M: verified sources in section 2 | corpus |
| 15 | Log sleep, caffeine, exercise as descriptive covariates only. | measurement | W: effects of 0.05-0.3 are below the detectable change | measurement |
| 16 | "Write it in your own words" step with a metacognitive prompt; the tutor does not write the summary. | UI | W: Graham 2020 (g 0.30, school samples), Bastani | UI |
| 17 | Periodic no-AI, no-hint check cadence shown as "independent performance". | UI, scheduling | M: Bastani | UI |

## 5. Corpus guidance

- Full-text ingest allowed: Computational Cognitive Neuroscience book (CC-BY-4.0 per repo; I2 verified GitHub licence) and Neuromatch content (CC BY 4.0; code BSD-3). I did not re-verify these licences.
- Metadata, links and own notes only: OpenStax Psychology 2e and MIT OCW 9.01/9.40/9.66J/9.85 (NC-SA under current rule; owner decision needed), Kandel 6th ed. and Neuronal Dynamics (proprietary).
- Evidence cards at `corpus/books/cognition/` (I1's suggestion): one original note per source with DOI, design, population, effect size, evidence tag. Do not store full text of Frontiers or Sci Rep papers without checking each licence.
- Seed cards from the verified table first; add W sources only after the owner opens them or a later pass confirms them.

## 6. What not to build

- IQ scoring, "brain age", "cognitive age", or any composite brain or critical-thinking score (Retest inflation, measurement error, small gains).
- Brain-training games, n-back, working-memory training, cognitive-training marketing (Simons 2016, Gobet & Sala 2023).
- Supplement, stimulant or nootropic recommender, dosing calculator, pharmacy integration.
- tDCS, neurofeedback, binaural-beat or "focus audio" features sold as cognition boosters.
- Bilingual-brain or music-IQ copy; exercise-for-IQ gamification.
- Learning-style or "cognitive profile" personalization.
- A generic critical-thinking course detached from content (Abrami: dialogue and authentic problems helped).
- An AI that writes the learner's summaries, essays or arguments; an unguarded answer-on-demand mode.
- A "brain region = function" quiz that teaches blob-mapping as settled.
- Dunning-Kruger shaming; cite Kosmyna "cognitive debt" or Lee 2025 survey as proof of harm.
- Automated grading of experimental-design skill without a human-quality rubric.
- Streaks, points or XP tied to cognition claims.

## 7. Open questions (need the owner or a later pass)

1. Adult self-directed AI-tutored study: does any of the 1-5 IQ points per year appear? No study; do not promise.
2. Does Kestin's result replicate at adult self-study, months-long, PhD depth? Unknown. Bastani's harm result is high-school maths over a short period.
3. OpenStax Psychology 2e licence: confirm on the book's copyright page.
4. Does the owner accept NC-SA full text for personal use, or does the AGENTS.md rule stand?
5. Xu 2024 creatine erratum and commentary: read before any tutor text cites the 0.31 figure.
6. Effect sizes for Bisra, Donker, Schwichow, Huber & Kuncel, and Abrami moderators for adults were never verified; self-explanation and scaffold-fading stay hypotheses.
7. PhD qualifying-level expectations are my synthesis; the owner should name a target program so a later pass can read its handbook.
8. PEDAGOGY.md's 0.2 SD minimum detectable change was quoted by the workers; I did not read that file.


---

# FILE: docs/tutor/research/corpus.md

# Synthesis: open sources for the Tutor corpus

Date: 2026-09-30. Inputs: F1 to F5. I re-opened 28 primary pages or API records. Fetch results are small-model summaries, so quotes are secondhand. Nothing was downloaded.

## Answer to the owner's question

No. Chinese institutional papers and teaching material are not all open source. The same holds for Japan, Korea, Singapore, Finland and Estonia. Free to read is not the same as openly licensed. What I verified:

- **Open:** MEXT (Japan) site content, reusable with credit and a modification note, and stated compatible with CC BY. Also ChinaXiv preprints, but only those the author licensed CC BY, BY-SA, CC0 or BY-NC-SA. ChinaXiv's bulk rules are silent, so email first.
- **Non-commercial:** Korea's KOGL Type 2 items.
- **All rights reserved, free to read:** PEP textbooks, Singapore MOE, NIER, EDUFI, most J-STAGE articles.
- **Unverified:** OECD/PISA, Estonia, Finland's English documents, smartedu.cn terms.

The usable free corpus is therefore mostly Western and mostly undergraduate-level. PhD-level depth has to come from the owner's own lawfully acquired library. Curricula from East Asia and the Nordics serve as scope-and-sequence maps, not content.

## Source register (acquisition recommendation)

"Auto" means a script may fetch it, with the per-item licence recorded in `corpus/manifest.json`. "Owner" means the owner adds a lawful copy, or the manifest holds metadata and a link only.

| Source | Licence (text link) | Bulk route | Format, size, language | Recommendation |
|---|---|---|---|---|
| MIT OpenCourseWare | CC BY-NC-SA 4.0, AI use explicitly allowed within the licence ([terms](https://ocw.mit.edu/terms/)) | robots `Allow: /` plus sitemap ([robots](https://ocw.mit.edu/robots.txt)). Terms do not address bulk. Use per-course downloads, politely | PDF, HTML, transcripts. A few MB per course. English | **Auto**, private corpus only. Skip video, keep transcripts. Third-party items inside courses have their own terms |
| GitHub-source math texts: Judson (GFDL), Lebl (CC BY-NC-SA / CC BY-SA), OpenIntro Statistics (CC BY-SA 3.0), Stacks (GFDL), APEX (CC BY-NC-SA) | per repo LICENSE ([GFDL](https://www.gnu.org/licenses/fdl-1.3.html); [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/legalcode)) | git clone | LaTeX, PreTeXt. Tens of MB. English | **Auto** after reading each repo's LICENSE. Only some were checked (moderate confidence) |
| Project Gutenberg | Public domain in the US only ([permission](https://www.gutenberg.org/policy/permission.html)) | Sanctioned: `wget -w 2 -m -H "https://www.gutenberg.org/robot/harvest?filetypes[]=txt"` or the feeds directory. The site is "intended for human users only" ([robot policy](https://www.gutenberg.org/policy/robot_access.html)) | `txt-files.tar.zip` 11 GB, `pg_catalog.csv` 20 MB ([feeds](https://www.gutenberg.org/cache/epub/feeds/)). Multilingual | **Auto** from feeds. Index only a curated list of about 300 to 1,000 works |
| Standard Ebooks | CC0 production, texts believed US public domain ([bulk page](https://standardebooks.org/bulk-downloads)) | Bulk zips are a Patrons Circle benefit. Per-book downloads and public git repos are not gated | EPUB, git. About 1,500 titles (inferred). Literature and philosophy | **Auto** per book or repo. Never bypass the gate |
| Wikisource, Wikibooks | CC BY-SA 4.0 + GFDL ([policy](https://en.wikisource.org/wiki/Wikisource:Copyright_policy)) | dumps.wikimedia.org dumps. Do not crawl | bz2 wikitext. Wikisource English about 3.6 GB (F5, not re-checked). Wikibooks size unverified | **Auto**, filtered. Wikibooks quality is uneven and Wikisource OCR fidelity varies |
| MEXT (Japan) | Site terms: reproduction, translation, adaptation and commercial use with credit and a modification note. The terms state compatibility with CC BY ([terms](https://www.mext.go.jp/b_menu/1351168.htm)). Third-party content is excluded | No bulk terms read | 16 or so English PDFs, 19 to 215 KB each. Japanese originals | **Auto**, low value. K-12 standards only. No lower- or upper-secondary maths or science in English. The PDFs carry a separate "Copyright (C) MEXT" footer |
| ChinaXiv | Author-chosen CC BY 4.0, BY-SA 4.0, BY-NC-SA 4.0 or CC0, per paper ([licences](https://chinaxiv.org/user/license.htm)) | Terms silent on mining and crawling | Preprints, not peer reviewed. About 47,900 records. Chinese and English | **Owner decision.** Email eprint@mail.las.ac.cn before any harvest. Until then, metadata and links |
| OpenAlex | Metadata CC0 ([pricing](https://help.openalex.org/access/pricing/)) | API with free key ($1/day) | JSON | **Auto** for the metadata index only. Skip the roughly 745 GB snapshot |
| arXiv | Metadata CC0. Papers carry a per-paper licence ([TOU](https://info.arxiv.org/help/api/tou.html)) | One request per 3 s, one connection | TeX, PDF | **Auto** only for CC BY, BY-SA or CC0 papers, using the stricter rate |
| PMC Open Access | Per article CC licence | Only Cloud (S3/HTTPS), OAI-PMH, E-Utilities, BioC. Other systematic retrieval is prohibited ([list](https://pmc.ncbi.nlm.nih.gov/tools/openftlist/)). Legacy FTP is deprecated August 2026 | JATS XML | **Auto** for CC BY/CC0 items from a curated seed list |
| LibreTexts | Per page. Shelf pages often "not declared" | robots: Crawl-delay 5, `/@*` and `action=*` disallowed ([robots](https://chem.libretexts.org/robots.txt)) | Per-book PDF. English | **Owner-selected**, one book at a time, licence logged per book. No crawler |
| Open Textbook Library / AIM list | Catalogue metadata CC BY 4.0 ([license.xml](https://open.umn.edu/opentextbooks/license.xml)). Each book keeps its own licence | Crawl-delay 120 (F1, not re-checked) | Links | Use as a **selection aid**. CC BY-ND books (Book of Proof) are read-only |
| OpenStax | CC BY-NC-SA 4.0 **plus** a ban on LLM training or ingestion without written permission ([book page](https://openstax.org/books/chemistry-2e/pages/1-introduction)). robots.txt blocks GPTBot on `/books/` | n/a | About 60 books. English | **Do not automate.** Owner may file OpenStax's reuse request. Human reading only until permission arrives |
| Korea NCIC/KICE | KOGL Type 2: attribution, non-commercial, derivatives allowed ([KOGL](https://www.kogl.or.kr/info/licenseTypeEng.do)). Unmarked items need permission | not assessed | Korean. English availability unverified | **Owner**, personal use only, never exported |
| J-STAGE | Publisher or author copyright. CC tag on open-access articles ([terms](https://www.jstage.jst.go.jp/static/pages/TermsAndPolicies/ForIndividuals/-char/en)) | **Forbidden:** downloading a large amount by mechanical means. Translation and derivatives need holder consent | Mostly Japanese PDF/HTML | **Owner**, one article at a time. Metadata and links only otherwise |
| Singapore MOE, NIER, EDUFI, Estonia, IEA/TIMSS, OECD | MOE: fair dealing only, no modification, no commercial reuse ([terms](https://www.moe.gov.sg/terms-of-use)). NIER: "All Rights Reserved". IEA: click-through licence. EDUFI, Estonia, OECD: not verified | none | Mostly PDFs and datasets | **Owner** or link only. Use as syllabus maps |
| CNKI, PEP textbooks, smartedu.cn, Chinese MOOCs, Khan Academy | Proprietary or user agreement | Not permitted or unread | n/a | **Owner** copies only. No scrapers |
| Journals in the learning-science canon (Dunlosky, Adesope, Yang, Murray, Melby-Lervåg) | Publisher TDM licences or none | none | PDF | **Owner** copies. Write own-words evidence notes |
| Internet Archive, HathiTrust, Darwin Online, Newton Project | Terms unread or per item | `ia download` exists | OCR scans, noisy | **Owner** or read-only links until the terms are read |

## Verified evidence table

"Verified" means I opened the source and it says what the report claims. "Corrected" means the claim needed a change.

| Claim | Status | What the source says |
|---|---|---|
| OpenStax is CC BY-NC-SA 4.0 and bars LLM ingestion without written permission | Verified | Chemistry 2e page quotes the clause. robots.txt has `User-agent: GPTBot / Disallow: /books/`. Six books were sampled by F1 and one re-checked here |
| MIT OCW is CC BY-NC-SA 4.0 and permits AI training within its terms | Verified | "OCW content may be used to train, develop, and improve artificial intelligence and machine learning models", provided attribution, non-commercial use and ShareAlike. **Corrected:** the terms do not address bulk download, so the F1 "Download Course" route is unconfirmed here |
| MEXT permits reuse with credit and a modification note, compatible with CC BY | Verified | Japanese text of the terms. The English PDF page has a separate MEXT copyright footer, and the coverage question stays open |
| J-STAGE bans mechanical bulk download | Verified | Article 3, Paragraph 1, Item 5 |
| PG bulk access only by sanctioned routes | Verified | `-w 2` harvest command, human-only statement, feeds sizes 11 GB, 20 MB and 121 MB |
| Standard Ebooks bulk downloads are members-only and its work is CC0 | Verified | Quoted |
| ChinaXiv offers four author-chosen licences and is silent on mining and crawling | Verified | Via a fetch summary |
| PMC forbids other systematic retrieval. Legacy packages are removed | Verified, corrected | The page lists Cloud, OAI-PMH, E-Utilities and BioC. The announcement gives August 2026. The exact day "26 Aug" was not confirmed |
| arXiv: one request per 3 s, metadata CC0 | Verified | The conflicting bulk-page limit was not re-opened, so apply the stricter limit |
| OpenAlex is CC0 with a free key | Verified | $1 a day. The 429 experience was not retested |
| KOGL Type 2 is non-commercial and allows derivatives | Verified | Quoted |
| OTL catalogue metadata is CC BY 4.0 | Verified | Via a fetch summary of the XML |
| US public domain covers works published before 1931 | Verified, corrected | The fetch summary said 1931 works entered in 2026. That is wrong. A 95-year term puts 1930 works in on 1 Jan 2026 and 1931 works in on 1 Jan 2027, so the rule holds for today's date. US only |
| Yang 2021: quizzing g = 0.499, 222 studies | Verified | 48,478 students. Crossref licence is the publisher PDF link, not an open licence |
| Pan & Rickard 2018: transfer d = 0.40, CI [0.31, 0.50] | Verified | 122 experiments, 67 articles, 10,382 participants. Crossref lists an accepted-manuscript route under APA policy after 310 days. That is not a CC licence |
| Melby-Lervåg 2016: no convincing far transfer | Verified | 87 publications, 145 comparisons, compared with treated controls. SAGE TDM licence only |
| Kestin 2025: designed AI tutor beat in-class active learning | Verified | CC BY-NC-ND 4.0. The abstract gives no effect size, so quote none |
| Bastani 2025: unguarded GPT-4 harms unaided performance | Verified | Practice grades +48% (base) and +127% (tutor). After access removal, -17% for GPT Base. Guardrails largely mitigate it. CC BY-NC-ND 4.0 |
| Bego 2024: spacing significant in 2 of 9 STEM courses | Verified, corrected | CC BY. The two courses were Calculus I for Engineers and Chemistry for Health Professionals. The pooled effect disappeared without calculus. F4's reading that spacing is weaker in maths is not supported by this paper alone |
| Murray 2025: smaller spacing effect in maths, testing alone uncertain | **Partly verified** | The title and the TDM-only licence are confirmed. The publisher elided the abstract, so the findings rest on F4 alone. Flagged |
| LibreTexts crawl limits, Wikisource CC BY-SA + GFDL, MOE terms, NIER footer | Verified | Crawl-delay 5, Request-rate 1/5. Copyright 2015 NIER |

## Conflicts between reports and how I resolved them

1. **Which licences count as open for auto-fetch.** F1 puts NC-SA (OCW) in Tier A. F2 and F4 limit Tier A to CC0, CC BY and CC BY-SA. The project's AGENTS.md says full text only for public-domain, CC-BY or the owner's own notes. Resolution: the strongest evidence is the licence text, and OCW's terms allow AI use within NC-SA. But the stated project rule is stricter than either report. **The owner must amend the AGENTS.md rule or lose OCW, BY-SA, GFDL and Wikisource.** My recommendation is to amend it to allow CC BY-SA, GFDL and NC-SA material for a private, single-user, never-distributed corpus, with `ai_use` gating per item. Until then, only PD, CC0 and CC BY are auto.
2. **OpenStax.** Common memory says CC BY. The page says CC BY-NC-SA plus a no-LLM clause. Page text wins. Excluded.
3. **"Only MEXT is open" (F3) versus a larger open set (F1, F5).** F3 covered government curricula only. MEXT is the one verified permissive government source.
4. **ChinaXiv as Tier A (F2).** F2's own text says the terms are silent on crawling. I downgraded it to owner decision pending the email.
5. **Spacing in maths.** F4 implies weaker effects in maths. Bego shows the positive results sat in calculus and chemistry, and the pooled effect depended on calculus. The two fit only under "heterogeneous, do not promise a fixed gain", which is what the rule below says.
6. **Sizes.** F1's Wikibooks dump figure (about 207 GB) looks wrong, and I dropped it. Other sizes are report figures.
7. **arXiv rate limits** conflict between pages. Use one request per 3 s.

## Rules for Tutor, ranked by evidence strength times impact

Corpus rules rest on licence text I re-read. Learning rules rest on effect sizes I confirmed.

1. **[corpus, UI] A licence gate on every item.** Add `license_spdx`, `license_url`, `ai_use` (allowed/restricted/unknown), `nd_flag`, `acquisition` (auto / owner / link-only) and `source_checked_date` to `corpus/manifest.json`. The indexer refuses to embed or send to an LLM any item whose `ai_use` is not allowed, unless the owner overrides. The Library shows a badge (Open, Free to read, Owner copy). Strength: strong. Impact: highest.
2. **[corpus] Exclude OpenStax until written permission.** Also exclude Khan Academy and any free PDF with no licence. The owner may submit OpenStax's reuse request. Strength: strong (quoted clause).
3. **[corpus] Automate only open material, politely.** Sources: OCW per course, GitHub-source texts, PG feeds, Standard Ebooks repos, Wikisource and Wikibooks dumps, OpenAlex metadata with a key. Honour robots, Crawl-delay and 429s, and record SHA-256 and licence text per item. Never crawl J-STAGE, LibreTexts, gutenberg.org pages or CNKI.
4. **[corpus] Keep the corpus private.** NC-SA and GFDL are copyleft and PG has trademark limits. Do not commit fetched texts to a repo or ship them in the app. Keep attribution in a NOTICE file.
5. **[voice/chat tutor prompt] Attempt first, then hints, then a worked solution. Never answer first.** Bastani 2025: access raised practice grades 48% (127% with guardrails) but cut later unaided grades 17%, and guardrails largely removed that harm. Strength: moderate (one RCT, high-school maths, abstract numbers only). Impact: high.
6. **[measurement] Add unassisted delayed retention probes** days after study, scored separately from in-session accuracy, plus a strength tag (strong/moderate/weak/contested) on every evidence card. Strength: moderate. Impact: high, because it detects the Bastani failure.
7. **[question generation] Retrieval items with corrective feedback and varied formats.** Yang 2021: g = 0.499 over 222 classroom studies. Pan & Rickard 2018: transfer d = 0.40, strongest for different formats and application items, weakest for untested material. Keep the atomic item with explanation and misconception. Strength: strong for classroom testing. Impact: high.
8. **[scheduling] Keep FSRS at 0.9, but promise no fixed spacing gain in maths.** Bego 2024 found effects in 2 of 9 courses. Murray 2025 is unverified by me. Measure the owner's own retention instead. Strength: moderate. Impact: medium.
9. **[tutor prompt, corpus] Provenance.** Cite source, licence class and original title. Quote only short passages from non-open items. Tag Gutenberg and similar items `historical-primary`, never the current state of a field. Label English renderings of foreign texts unofficial unless the source provides them. Make no "how Japan, Singapore or China teaches" claims without a retrieved document. Strength: moderate (design rule). Impact: medium.
10. **[question generation] Items carry `source_id` and licence.** ND sources (Book of Proof) are not seeds. Items from non-open sources are original, with no long quotation.
11. **[corpus, measurement] Curate, then measure.** Start with roughly 300 to 1,000 public-domain works, not 79,509. Check `eval-search` on the held-out test split before and after each new source, and prune if the hit rate drops. Use curricula as topic maps only.
12. **[feature] If Dual N-Back ships, make no transfer claims.** Melby-Lervåg 2016: 87 publications, 145 comparisons, no convincing far transfer against treated controls. Training improves the trained task. Strength: strong against far-transfer claims. Impact: medium.
13. **[corpus] Evidence cards use original notes** for NC/ND and TDM-only papers (Kestin, Bastani, Yang): link plus own-words notes with metric, N and boundary conditions.
14. **[UI] "Add my copy" action** for metadata-only entries, plus an attribution export for open items.

## What not to build

- An OpenStax ingester, or anything that sends OpenStax text to an LLM.
- Scrapers for J-STAGE, CNKI, smartedu.cn, PEP, NIER, MOE, EDUFI, OECD, LibreTexts, Khan Academy, gutenberg.org pages, or the Standard Ebooks bulk zips.
- Mirrors of PMC, arXiv, S2ORC or the OpenAlex snapshot (745 GB, no gain over API queries).
- A full 79k-book Gutenberg index.
- A machine-translation pipeline republishing translated Japanese or Korean material.
- An IEA/TIMSS ingester or a national curriculum benchmark score.
- Any pipeline relying on PRC Copyright Law Article 5 for curriculum standards (scope unresolved).
- A default raw-LLM answer mode, or a dual n-back feature sold as raising intelligence.

## Open items for the owner

1. Decide whether to amend the AGENTS.md corpus rule (conflict 1).
2. Decide whether to file the OpenStax reuse request, and whether to email ChinaXiv.
3. State your country. Gutenberg and Cornell rules are US law, and personal-use copying elsewhere differs.
4. Confirm in a browser: OECD/PISA licence, Finland and Estonia English curricula, smartedu.cn terms, MEXT coverage of the English PDFs.
5. Read Murray 2025 before quoting it.

## Citation list with verification status

Verified (opened by me on 2026-09-30):
- openstax.org/books/chemistry-2e/pages/1-introduction (CC BY-NC-SA 4.0 and LLM clause); openstax.org/robots.txt (GPTBot)
- ocw.mit.edu/terms/ (AI use allowed; bulk not addressed); ocw.mit.edu/robots.txt
- mext.go.jp/b_menu/1351168.htm (reuse terms); mext.go.jp/en/policy/education/elsec/title02/detail02/1373859.htm (document list, sizes, footer)
- jstage.jst.go.jp terms (Art. 3 Para. 1 Item 5; Art. 2 Para. 2)
- gutenberg.org/policy/robot_access.html; gutenberg.org/cache/epub/feeds/
- standardebooks.org/bulk-downloads
- chinaxiv.org/user/license.htm
- info.arxiv.org/help/api/tou.html
- help.openalex.org/access/pricing/
- pmc.ncbi.nlm.nih.gov/tools/openftlist/; ncbiinsights.ncbi.nlm.nih.gov/2026/02/12/pmc-article-dataset-distribution-services/
- kogl.or.kr/info/licenseTypeEng.do; ncic policy (F3 only, not re-opened)
- open.umn.edu/opentextbooks/license.xml
- guides.library.cornell.edu/copyright/publicdomain (corrected)
- en.wikisource.org/wiki/Wikisource:Copyright_policy; chem.libretexts.org/robots.txt; moe.gov.sg/terms-of-use; nier.go.jp/English/index.html
- Yang et al. 2021, doi 10.1037/bul0000309 (g = 0.499)
- Pan & Rickard 2018, doi 10.1037/bul0000151 (d = 0.40)
- Melby-Lervåg et al. 2016, doi 10.1177/1745691616635612
- Kestin et al. 2025, doi 10.1038/s41598-025-97652-6
- Bastani et al. 2025, doi 10.1073/pnas.2422633122
- Bego et al. 2024, doi 10.1186/s40594-024-00468-5 (corrected)

Partly verified: Murray, Horner & Göbel 2025, doi 10.1007/s10648-025-10035-1 (title and licence only).

Not re-checked, taken from reports (moderate or unverified): GitHub text licences; OTL Crawl-delay; dump sizes; Standard Ebooks repo count; DOAJ counts; Latimier 2021, Freeman 2014, Tabibian 2019, Diamond & Ling 2016; Unpaywall, Europe PMC, CORE terms; OECD, Finland, Estonia, IEA licences; PRC Copyright Law Art. 5.


---

# FILE: docs/tutor/research/critique-2.md

# Completeness critique, research round 2

Basis: I read all four round-2 SYNTHESIS.md files in full, the round-1 critique.md, and grepped PEDAGOGY.md, corpus.md, domains.md and expert.md. I did not re-check primary sources. The arithmetic in gap 1 is mine.

## 1. Gaps, most serious first

1. **No owner time budget across areas.** Round 2 adds 7/30-day retests on every domain, a monthly no-AI language hold-out, forms A/B/C batteries every 6-8 weeks, calibration, mock quals, coverage-meter reading, and three card types per language word. Nobody sums it. Using engineering's simulated 7.9 reviews per card-year at 0.90 retention (workload, assuming FSRS is the true model), an 8,000-word vocabulary at three cards per word is about 190,000 reviews a year, roughly 520 a day, before any math or science. Split cards per skill is the languages area's own design choice, not a cited result. A workload cap and a per-word rather than per-skill default need deciding.
2. **The owner's own library vs the corpus rules.** Engineering plans to parse and embed 50-250 GB with hosted engines. Curricula puts Rudin, Hatcher, Dummit-Foote and similar at "metadata and own notes only". AGENTS.md forbids vendoring copyrighted books. The round-1 gap (may owned copies go to Anthropic, ElevenLabs or a parser, and on what terms) is only half answered: engineering covers Mathpix's 30-day retention and says other vendors are unverified. Nobody checked Anthropic or ElevenLabs terms for owned copies, or how the OpenStax LLM ban interacts with a user's own copy. Owner decision plus a terms check, not more literature.
3. **"PhD level" is still undefined outside math.** Only Berkeley and Harvard math formats were opened. Physics, chemistry, biology, earth and space science, and English ladders are worker syntheses. Also absent: research skills (reading and writing papers, experimental design, data analysis, scientific programming), lab and field work, and first-round gap 6 (tutorial, seminar, oral-defence practice). Coursework mastery is not research readiness.
4. **English and linguistics remain thin even though the owner is an L1 speaker (which closes one round-1 question).** The only English evidence is Graham & Perin (grades 4-12), Dodell-Feder (g about 0.15) and Steiss. Nothing on doctoral writing, rhetoric or formal linguistics pedagogy. Literary corpus is restricted to pre-1930 public domain, so modern criticism is reachable only through the owner's copies (see gap 2).
5. **Language track has no curriculum or goal.** Language order follows speech-recognition tiers, not the owner's purpose. Missing: reading-only classical languages (Latin, Greek, Hebrew; the languages area itself found no pronunciation API for them), scripts and morphology-heavy languages, time-to-target ranges (FSI figures failed verification), and whether any language is tied to the English PhD track (curricula open question 5).
6. **Voice for technical subjects is untested.** Round 2 tested ElevenLabs only for language. Spoken math and chemistry transcription, latency, real per-minute agent cost (only "Speech Engine $0.08/min" seen), and spoken vs typed retrieval are still open from round 1.
7. **Model reliability is still unmeasured.** Curricula's 36.9% (PHYBench) and 18-55% fabrication figures are 2023-2025 models. Every area defers to an in-house eval on current Opus and Sonnet. It has not been built, and it gates every relaxation of the verifier ladder.
8. **Initial reading of dense chapters and papers** (round-1 gap 7) got no round-2 coverage. Cognition's staged paper reading is tagged weak.
9. **Adherence for a solo adult** (round-1 gap 11): streaks are ruled out everywhere; nothing replaces them.
10. **Question-bank authoring cost** (round-1 gap 12) is worse: each item now needs CAS key, two-family blind solve, span check, second-pass validation. No per-item owner-time or API cost estimate.
11. **Chemistry structures, and equation-heavy retrieval through pdfium plus 400-token chunks,** are untested. The gold set is designed but not built.

## 2. Contradictions

1. **Prerequisite gating.** Curricula ranks "stage opens on prerequisite mastery, with owner override" and math nodes "gate" physics and chemistry. Engineering lists hard gating under "do not build" and makes edges advisory. Pick advisory with a diagnostic (engineering's version, since no validated dependency model exists).
2. **Two mastery definitions.** Curricula: 70% delayed, unaided, novel-item accuracy at 7 and 30 days. Engineering: Beta posterior with about 15-20 unassisted attempts over several days, assisted attempts never counted. Cognition adds stable batteries every 6-8 weeks. These can disagree for the same skill.
3. **Testing effect in math.** Cognition calls retrieval practice strong (Yang g 0.499, classroom, all subjects) and makes it the main schedule. Engineering cites Murray 2025 (preprint): math testing vs restudy g 0.22 with a confidence interval crossing zero, spacing g 0.26, "may be smaller than other domains". Math is the largest domain; the plan should use the smaller figure.
4. **Experiment budget.** Engineering caps personal experiments at 2-3, about 160-320 items per arm, effects under about 6 points unresolvable. Languages proposes several (3 voices vs 1, alternating-week recasts vs prompts), cognition allows worked-example vs attempt-first, PEDAGOGY has n-back and interleaving. Total exceeds the cap and overlaps skills.
5. **Confidence scale.** PEDAGOGY uses 1-4; cognition says 0-100 with a Brier dashboard. Round-1 critique flagged this; round 2 adds a second voice rather than resolving it.
6. **Worked examples vs "no answer before attempt".** Cognition allows a labelled worked-example mode; languages make attempt-first a hard rule; curricula says no full solution before a written attempt. Needs a per-item-type rule and a log field.
7. **Interleaving parameters.** Curricula: after one blocked intro, confusable families. Engineering: about 30% mixed-family queue. Round-1 said about 20%. Languages: separate card states. FSRS for vocabulary (interleaving g = -0.39) must stay blocked-neutral.
8. **Bastani extrapolation.** Three areas rely on the -17% (unguarded arm only, high-school math). The languages synthesis applies it to language and the correction is unread in all four areas. Keep as design direction, labelled.
9. **FSRS-6 vs FSRS-7 and FSRS for families/vocabulary.** Languages build on FSRS for vocabulary; engineering says calibration for family-level skills and language vocabulary is untested, and FSRS evidence is recall prediction only.
10. **Sleep rule.** Cognition ships a weak-inference default that changes scheduling while also saying covariates are descriptive only.

## 3. Follow-ups, most valuable first

1. Owner-decision memo: owned-library processing (vendors, terms, retention), NC-SA in the private index, OpenStax, Lean, target programs, first languages.
2. Build the in-house eval: current models on the owner's real problems (proof, mechanism, numeric, leak, sycophancy), including spoken-math transcription.
3. Corpus inventory plus 150-200 page gold set, then the parser bake-off (measured before any spend).
4. One integrated design doc: mastery definition, single confidence scale, hint and review schema (engineering C4 fields), workload cap, experiment register limited to 2-3.
5. Open real target syllabi (the owner names programs) and write one prerequisite-graph pilot for math; add a research-skills track.
6. Small recorded test of ElevenLabs Scribe and agent behavior on the owner's accented beginner speech, plus a native-checked 50-sentence gold set for the first language.
7. Read the unread items: Bastani correction, Murray full text, Xu 2024 erratum, OpenStax Psychology copyright page.
8. Adherence design and the English/linguistics doctoral-writing literature sweep.


---

# FILE: docs/tutor/research/critique.md

# Completeness critique of the Tutor research round

Basis: I read the intl, corpus and expert SYNTHESIS.md files in full and the other four through the structured summaries supplied in the brief. Checks were against the brief and the summaries, not against primary sources.

## 1. Gaps (most serious first)

1. **Owner-copy licence gate vs the main use case.** The corpus area says the indexer must refuse to embed or send to an LLM anything whose `ai_use` is not allowed. The PhD-level content will be the owner's 50-250 GB of all-rights-reserved textbooks and papers, and RAG sends chunks of them to Claude or Muse Spark. As written, the gate blocks the core feature. Nobody researched provider data terms, retention, or the personal-use basis for sending owned copies to a hosted API, or the local-model alternative. This needs an owner decision, not more literature.
2. **RAG engineering for technical libraries is not researched.** The corpus area covers only licensing. Missing: parsing scanned or LaTeX-heavy PDFs, equations, chemical structures and tables; chunking; embeddings and reranking; OCR cost at 250 GB; and a citation-span check (only the 2023 Liu et al. result is cited). Every area assumes quote-level grounding works.
3. **No integrated scheduler design.** The areas add FSRS atoms, problem families whose variants are not independent evidence, per-skill mastery (BKT/Rasch), cumulative interleaved review (about 20%), hint-capped grades, a load cap, 2-3 new skills per session and randomized holdouts. No area specifies how these compose. FSRS cold start on a single user with few reviews is also unaddressed.
4. **Experiment budget is infeasible for n = 1.** Proposed within-learner tests: retrieval vs restudy, interactive-first vs symbol-first, theme vs no theme, paper vs screen, tutor- vs learner-initiated feedback, self-explanation required vs not, plus an n-back n-of-1. No area gives a power calculation or considers carryover. The n-back area itself says the minimum detectable change is about 18 IQ points. Most of these cannot be resolved by one learner.
5. **"PhD level" is undefined.** No area maps the target: qualifying-exam syllabi, prerequisite graphs per subject, placement diagnostics, time-to-goal, or an external yardstick. The Plan screen has no research behind it. "Science" is not specified (domains lists biology, earth science, upper-division physics and physical chemistry as gaps).
6. **International pedagogy is K-12 only.** The owner is an adult working toward doctoral level. Nothing covers doctoral-level practices (Oxbridge tutorials, Russian problem-solving circles, qualifying-exam preparation, research apprenticeship, seminar and oral-defence formats). The intl area's own rule 15 lists a Danish oral-defence mode as unverified. Its findings are mostly null or confounded.
7. **Initial exposure to new material from textbooks.** The areas cover retrieval, feedback and problem solving. How the owner should first read and understand a dense chapter is thin: reading strategy, note-taking, elaboration, mnemonics and imagery. Only a "paper mode" labelled as hypothesis appears.
8. **Neuro-cognitive content is thin.** The brief asks for it. Only sleep (g about 0.3) is covered. Exercise, attention, consolidation, circadian timing and caffeine are not. No deliberate-practice literature (Ericsson; the intl area cites Macnamara's 4% only in passing).
9. **English is thin.** The rules cover AI writing guardrails and a timed monthly sample. Missing: advanced reading, rhetoric, literary analysis and vocabulary depth. It is unclear whether the owner is an L1 speaker or an ESL learner; that changes the whole track. Tetzlaff 2025 says expertise-reversal evidence is weaker for language learning.
10. **Model and voice reliability in the owner's subjects.** No area measured Opus 5.5, Sonnet 5.5 or Muse Spark 1.3 on proofs, mechanisms, numerics, answer leakage or sycophancy; all failure figures come from 2023-2025 models. Spoken-maths speech-to-text errors, ElevenLabs latency and cost, and spoken vs typed retrieval are untested.
11. **Adherence for a solo adult over years.** Gamification is ruled out, but nothing positive replaces it (autonomy support, habit design, plateau handling). The owner's load, sleep and burnout covariates are planned with no evidence on what to do with them.
12. **Question-bank authoring cost.** Every item needs a verified solution, blind solve by a different model family and a source citation. Nobody estimated the cost or owner time for thousands of PhD-level items.

## 2. Contradictions and inconsistent statuses

1. **Self-explanation.** Learning says selective (after errors or at proficiency). Expert says required on proofs and derivations. Domains says before answering. Intl cites Barbieri 2023 (prompts were a negative moderator) against "require it". One rule is needed per item type, and it should be marked as a hypothesis under test.
2. **Novice default vs diagnostic routing.** Learning says treat the owner as a novice in each new topic (worked example first, gate productive failure). Intl and expert say run a short diagnostic, and use problem-first when prerequisites exist. Tools defaults to symbols-first at PhD level and also proposes interactive-first tests. Decide which default applies before the diagnostic exists.
3. **Interleaving definitions differ.** Learning: after a brief block, confusable problem types only. Intl: cumulative mixed review at about 20% of reviews. Domains: after first blocked exposure. All also say FSRS handles vocabulary-like items, where interleaving hurts (g = -0.39). They agree in spirit but not in parameters.
4. **Hint handling in FSRS.** Learning says log every review. Expert caps hinted or revealed items at Again or Hard, and revealed items reschedule as relearning (D5 said new). Intl says variants of one seed are not independent evidence. No area says what the stored review record looks like.
5. **Confidence scale and timing.** Learning: four levels before feedback. Intl: 0-100 before reveal. Domains: 1-5 with predict-then-reveal. Expert: a rating before each answer. Pick one scale and one moment.
6. **Session length.** Learning: 40-50 minutes with pre-set breaks. Intl: soft stop at about 90 minutes. Bastani's sessions were 90 minutes. Both are labelled defaults; settle on one for the build.
7. **Whether the tutor model sees the answer** is resolved only by an untested tiered design (expert). Tools and domains say "hide until tried" for the learner, which is a different question from the model's context.
8. **Status of the same citations differs by area.**
   - Bastani 2025: verified in corpus and expert. Learning and domains say the erratum was not read. The -17% figure is the single most repeated number in all seven areas.
   - Kestin 2025: intl and expert say 0.63 SD verified from full text. Corpus says the abstract has no effect size, so quote none. Learning excluded it.
   - Murray 2025: learning reports g = 0.18 and 0.28 from the abstract. Corpus says findings rest on one report and are unverified.
   - Tetzlaff 2025 and Sinha & Kapur 0.87 are cited with different confidence.
   Nothing should reach prompts or UI copy until one evidence ledger gives each number a single status.
9. **Chinese open-source premise.** Corpus and intl say licences do not support it. Tools, domains and the intl open questions still carry it as open, although smartedu.cn and OECD terms are genuinely unread. The answer is "no" for what was checked and "unknown" for what was not; the areas should say the same thing.
10. **Inquiry vs explicit instruction.** Tools supports guided discovery with sims (d = 0.30 enhanced) and learning supports explicit first for novices. They are reconciled only if the prediction-gate and "never free-explore" rules are written into the sim specs.
11. **N-back scope.** Every area agrees on no transfer claims. The tension left is effort: an optional n-back, four-weekly transfer probes and an n-of-1 card cost owner time against a 60-minute weekly cap, for a result the n-back area expects to be null.

## 3. Follow-ups worth a second round (most valuable first)

1. **Owner-decision memo on corpus licensing and LLM use** (gap 1, corpus conflict 1): provider terms, local-model option, AGENTS.md amendment, country of residence.
2. **Single evidence ledger.** Read the Bastani erratum, Kestin, Murray and Tetzlaff full text, then fix each number's status (contradiction 8). Cheap and unblocks prompt and UI copy.
3. **Empirical model audit on the owner's real problems**: leak rate, sycophancy, proof, mechanism and numeric error for Opus, Sonnet and Muse Spark, plus spoken-maths transcription. Code and evals, not literature.
4. **Scheduler and measurement design spike**: FSRS cold start, problem families, mastery model, review schema, plus a power analysis that limits the experiments to two or three that one learner can resolve.
5. **RAG for technical PDFs**: parsers for math and chemistry, chunking, rerank, citation-span verification, evaluated on a sample of the owner's own books using `eval-search` with the held-out split.
6. **Define the target**: doctoral-level syllabus and prerequisite maps per subject, placement diagnostics, English-track definition (ask the owner), external benchmarks.
7. **Doctoral-level pedagogy and reading strategy**: tutorial, seminar and qualifying-exam practice; textbook and paper reading methods; deliberate practice.
8. **Neuro-cognitive and adherence sweep**: sleep, exercise, attention, autonomy-supportive design for solo adults.
9. **Low priority**: K-12 Chinese variation teaching, PISA tables, Nordic details. The intl area itself finds little transferable beyond mechanisms already covered.


---

# FILE: docs/tutor/research/curricula.md

# SYNTHESIS: PhD-level curricula and prerequisite maps

Date: 2026-09-30. Inputs: P1-math, P2-physics, P3-biology-earth, P4-chemistry, P5-english. Verifier: Sonnet 5.5. Method: 26 sources re-opened by direct fetch (WebSearch was out of budget). The fetch tool returns a model summary of each page, so figures are as the fetched page states them.

## 1. Bottom line

1. **What is solid.** The teaching method Tutor already uses (attempt-first, hint ladder, delayed unaided retesting, grounded answers) is supported by the best verified evidence in all five reports. The evidence is mostly school or classroom studies, so for one adult self-learner it supports design direction, not predicted effect sizes.
2. **What is not available.** No report could open a non-math PhD qualifying syllabus. Only math has verified official exam pages (Berkeley, Harvard; MIT as reported). Physics, chemistry, biology, earth science and English "PhD level" ladders are the workers' own syntheses. Tutor should ship them as editable, versioned data labelled "proposal", never as a standard.
3. **The biggest practical finding is about the corpus.** OpenStax (the whole library, verified on the Chemistry 2e and Biology 2e prefaces) is CC BY-NC-SA 4.0 and forbids LLM ingestion without written permission. That removes OpenStax from every RAG plan in the reports. MIT OCW, by contrast, explicitly allows AI use with conditions.
4. **Two owner decisions block the corpus work** (Section 6): whether NC-SA material may enter the local index, and which qual target to use per domain.

## 2. Verified table

Status: V = verified as claimed; C = corrected (claim changed); U = could not verify, not relied on.

| # | Claim (source) | What the fetched page says | Status |
|---|---|---|---|
| 1 | Bastani et al. 2025, PNAS, doi 10.1073/pnas.2422633122 | "nearly a thousand high school math students"; GPT-4 access raised grades 48% (GPT Base) and 127% (GPT Tutor); after access removed, GPT Base 17% lower than never-had-access; GPT Tutor's safeguards "largely mitigated" the harm; "crutch" quote | V. Reports' "Turkey, one school" was not on the page |
| 2 | Freeman et al. 2014, PNAS | 225 studies; +0.47 SD on exams (158 studies); odds of failing under lecture 1.95 (67 studies); about 6% exam gain; stronger in classes of 50 or fewer | V. The "failure 32% to 21%" figure in P3 was not on the page; dropped |
| 3 | Kestin et al. 2025, Sci Rep, doi 10.1038/s41598-025-97652-6 | Abstract: learn "significantly more in less time", no numbers. Full text (Europe PMC): 194 of 233 students eligible; effect size 0.73 to 1.3 SD (quantile model), 0.63 (linear); AI median 49 min vs an **assumed** 60 min in class; first-exposure content, no retention test | C. P4 said no effect size: it is in the full text. The in-class time is assumed, not measured |
| 4 | Brunmair and Richter 2019, Psych Bull | Overall g = 0.42; math tasks g = 0.34 ("small"); words g = -0.39 (blocking better); paintings 0.67; expository texts ambiguous | V |
| 5 | Rohrer et al. 2020, J Educ Psychol | Preregistered cluster RCT, 54 grade-7 classes, 4 months, test one month later, 61% vs 38%, d = 0.83 | V |
| 6 | Berkeley math prelim | Two consecutive 3-hour mornings; 5 of 8 questions; 2 per area (real analysis/calculus, complex, linear algebra, abstract algebra); pass = "roughly the level of an A grade in an honors undergraduate course"; passing score "usually between 60% and 70%", about two-thirds pass, "not official policy" | V |
| 7 | Harvard math quals syllabus | "aim to test your ability to solve concrete problems by identifying and applying important theorems"; six areas with named books (Dummit-Foote, Serre, Fulton-Harris; Shafarevich, Harris; Ahlfors; Hatcher, Fulton, Spanier, Greenberg-Harper; Taubes, Lee, Kobayashi-Nomizu; Rudin, Stein-Shakarchi, Lieb-Loss, Evans, plus probability texts) | V. The Real Analysis area also lists PDE and probability texts |
| 8 | MIT OCW terms | CC BY-NC-SA 4.0; "OCW content may be used to train, develop, and improve artificial intelligence and machine learning models" provided attribution, non-commercial use, and ShareAlike on derivative models | V |
| 9 | OpenStax Chemistry 2e | CC BY-NC-SA; "may not be used in the training of large language models or otherwise be ingested into large language models or generative AI offerings without OpenStax's prior written permission" | V |
| 10 | OpenStax Biology 2e | Same licence and same LLM clause | V. Corrects P3 (CC BY from memory) and P2/P3 plans to ingest OpenStax. Astronomy 2e and University Physics not re-opened; assume the same |
| 11 | Graham and Perin 2007 | Grades 4-12; 123 documents, 154 effect sizes; strategy instruction 0.82, summarization 0.82, peer assistance 0.75, product goals 0.70, sentence combining 0.50, process writing 0.32, grammar instruction -0.32 | V |
| 12 | Walters and Wilder 2023, Sci Rep | 636 references, 42 topics; 55% of GPT-3.5 and 18% of GPT-4 citations fabricated; 43% and 24% of real citations had substantive errors | V. 2023 models, no retrieval grounding tested |
| 13 | Miller et al. 2019, Sci Adv | About 1 in 8 US physics PhD students 2000-2010; GPA consistently associated with completion; GRE Physics and Verbal not significant; GRE Quantitative significant in some models; completion differs under 10% between 10th and 90th percentile | V. Nuance: GRE-Q did predict weakly in two of four models |
| 14 | Sinha and Kapur 2021 (productive failure) | 53 studies, 166 comparisons; g = 0.36; 0.37-0.58 at high fidelity; bias-adjusted 0.87 as reported; younger learners favour instruction-first | V as reported; the 0.87 is unusual and is not used |
| 15 | PHYBench (arXiv 2504.16074) | Gemini 2.5 Pro 36.9% vs human experts 61.9% | V. Dated (2025); re-measure on the current model |
| 16 | DeepMind IMO 2025 | Advanced Gemini Deep Think: 35/42, five of six problems, officially graded and certified by IMO coordinators; natural-language end to end | V |
| 17 | Petrov et al. (arXiv 2503.21934) | Best model (Gemini 2.5 Pro) 25% on USAMO 2025; all others under 5% | V |
| 18 | Dodell-Feder and Tamir 2018 | 53 effect sizes from 14 studies; g = .15-.16; "small"; no publication-bias sign | V |
| 19 | Steiss et al. 2024 | 200 human vs 200 ChatGPT feedback comments on secondary essays; humans better on all categories other than criteria-based | V. P5's headline matches its body ("4 of 5") |
| 20 | Yang et al. 2021, Psych Bull | 222 studies, 48,478 students, g = 0.499 for classroom testing | V |
| 21 | Rahman and Lewis 2020 | 99 chemistry studies; trim-and-fill range 0.29 to 0.62; publication bias noted | V |
| 22 | Hodds, Alcock, Inglis (JRME) | Three experiments; a booklet of self-explanation training significantly improved proof comprehension; no effect size | V. Undergraduates, short-term |
| 23 | Lebl, Basic Analysis | v6.3, 15 May 2026; dual CC BY-NC-SA 4.0 and CC BY-SA 4.0 | V |
| 24 | Axler LADR 4e | "Creative Commons BY-NC license" | V |
| 25 | Judson AATA | GFDL | V |
| 26 | Libarkin and Anderson 2005 GCI | Title and year confirmed | V (existence only) |

**Not verified, not relied on:** the 2026 arXiv LLM-proof-grading preprints (P1; never opened); Bisra 2018 g about 0.55 and Barbieri 2023 g about 0.5 (no numbers in the abstracts available); Adesope 2017 g about 0.6; Deslauriers 2019 and Von Korff 2016 (abstract level only, from P2); ChemBench and MaCBench (P4 read the abstracts; not re-opened); RDKit/Ketcher/Open Babel licences (P4 via GitHub API; not re-opened); Latimier 2021 and the "2 of 9 courses" chemistry spacing note; Theobald 2020 figures; TCP and NACLO licences; Princeton's exam format (P1 only).

## 3. Design recommendations, ranked by evidence strength times impact

Evidence grade: A = verified, directly on point; B = verified but by analogy (school or classroom, other subject); C = expert synthesis or analogy only, log and test locally.

| Rank | Tag | Recommendation | Evidence | Why it matters |
|---|---|---|---|---|
| 1 | prompt | **Attempt first, then hint ladder; log every answer reveal and hint level.** No full solution before a written attempt. | B (rows 1, 3) | The only verified harm result: unguarded GPT cut later unaided grades 17%. Already Tutor's method; this confirms it and adds the logging. |
| 2 | prompt | **Ground every fact.** The tutor may not state a quote, date, constant, citation, pKa, mechanism outcome or stereochemical result unless it came from a chunk retrieved this turn or a checker. Otherwise say "I cannot verify this." Label claims textbook-consensus, active-research or contested. | A/B (rows 12, 15) | Fabricated citations 18-55% in 2023 models; best 2025 model 36.9% on hard physics. Models are better now, but unverified. |
| 3 | feature | **Verifier ladder instead of LLM grading.** Tier 1: deterministic (SymPy and Pint for numerics and derivations, RDKit for structures and mechanism steps, Lean optional for math). Tier 2: rubric with at least two independent LLM passes, disagreements flagged to the owner. Tier 3: never a single-pass LLM verdict. The tutor states which tier checked an answer. | B/C (rows 15, 17, 19) | LLM proof and mechanism judgement is the highest-risk failure. The specific 2026 grading preprints are unverified; the rule stands on verified evidence of LLM error and on caution. |
| 4 | measurement | **Delayed, unaided, novel-item retest is the mastery signal.** Retest at 7 and 30 days with fresh parallel items, hints off, plus a transfer set. Report per node: delayed accuracy, transfer accuracy, hint level used. Label results n=1. | B (rows 4, 5, 20) | Rohrer used a one-month delayed test; the four domain reports each proposed 7 days to 1 month. The thresholds are design choices, not validated cut scores. |
| 5 | scheduling | **FSRS only for atomic items** (definitions, theorem statements, "which theorem applies", IR/NMR ranges, paradigms, dates, figures of speech, proof-outline cards). Problem solving is scheduled as schemas with fresh variants. Do not schedule "prove X", "interpret this poem" or a mechanism as a fixed-answer card. | B (row 20) | Testing g = 0.499 in classrooms. Recognising a remembered answer is not skill. |
| 6 | scheduling | **Interleave confusable problem families after one blocked introduction** (SN1/SN2/E1/E2, which-theorem-applies, period or figure identification, ensemble choice). Do not interleave vocabulary or word pairs. | B (rows 4, 5) | Math g = 0.34 (plan for this, not 0.83); words g = -0.39. |
| 7 | question generation | **Computed answers, LLM only for wording and hints.** Parametrised numeric and symbolic families with CAS-computed keys; every generated item blind-solved by a second model family and machine-checked where possible; distractors from documented misconceptions; original items, not copies of licensed inventories (FCI, BEMA, GCA and similar). | B/C | Needed because of the LLM error evidence; concept-inventory reuse terms are unchecked. |
| 8 | corpus | **Licence-tiered manifest** (see Section 5). Record licence text, URL, edition and translator per file. | A (rows 8-10, 23-25) | Avoids an OpenStax terms violation and keeps NC/SA items out of any shareable bundle. |
| 9 | feature | **Prerequisite graph stored as data**: typed edges (hard or soft), per-node evidence state (never attempted, attempted with hints, passed unaided after delay, passed mixed-timed), cross-links so math nodes gate physics, chemistry and astro nodes. Stage opens on prerequisite mastery, with owner override. On a failed physical-chemistry or physics item, run a math-prerequisite diagnostic first. | C | Every report reached the same structure independently, but none is a validated dependency model. Keep it small and hand-edited. |
| 10 | prompt | **Self-explanation before confirmation**: the learner says why each proof inference or mechanism arrow is valid (source, sink, why) before the tutor confirms. | B (row 22) | Undergraduate proof-reading experiments; Bisra's effect size is unverified. |
| 11 | feature | **Mock-qual modes.** Math: Berkeley format (two 3-hour sittings, 5 of 8, four areas), then a Harvard-style theorem-selection set. Oral: voice tutor as MIT-style examiner. Other domains: timed closed-book mixed sets built from the department syllabi the owner chooses. | A for the Berkeley and Harvard format (rows 6, 7); C elsewhere | Only math has a verified exam format. |
| 12 | measurement | **Calibrate any LLM rubric grader against owner-graded work** and report agreement. Blind re-score old drafts to detect drift. | B (row 19) | Humans beat ChatGPT feedback on four of five criteria in a 2023-era study. |
| 13 | prompt | **Confidence rating before feedback; show delayed-retest results, not satisfaction.** | C (Deslauriers 2019 abstract-level, unverified) | Learners rate effortful practice as worse; cheap to implement. |
| 14 | UI | **Structure and curved-arrow editor (Ketcher) wired to the RDKit checker with commit-then-reveal**; Mol* only for biochemistry. | C | Highest-value chemistry build, but it is substantial work; schedule after the checker exists. |
| 15 | feature / UI | **English and linguistics:** reading ledger per field with coverage map against an owner-chosen template list; quote-check button that re-runs retrieval; draft, criterion feedback, owner revision, diff loop with all drafts stored; NACLO-style unseen-language data problems as the linguistics mastery test. | B/C (rows 11, 19) | Writing strategy instruction has school-level ES 0.5-0.8; adult transfer is inference. |
| 16 | prompt | **Predict-then-reveal** for mechanism questions in biology and earth science; faded worked examples as competence grows. | C | By analogy; log and compare with plain practice. |

## 4. Conflicts and resolutions

1. **OpenStax licence (P3 memory: CC BY; pages: NC-SA).** Resolved by fetch: CC BY-NC-SA 4.0 on Chemistry 2e and Biology 2e, both with an LLM-ingestion ban. P2 and P3 proposed ingesting OpenStax; that is withdrawn. A RAG pipeline sending OpenStax text to a cloud LLM is the prohibited use; human reading only until OpenStax grants written permission.
2. **OCW and the project's corpus rule.** AGENTS.md allows full text only for public domain, CC BY or the owner's notes. OCW is CC BY-NC-SA with an explicit AI-use clause (attribution, non-commercial, ShareAlike on derivative models). Resolution: the AGENTS.md rule is stricter than OCW's terms; ingesting OCW for private local use is consistent with OCW's terms but needs the owner to amend the rule.
3. **Interleaving effect size (d = 0.83 vs g = 0.34).** Different designs and populations; 0.83 is one school RCT, 0.34 is the math-specific meta-analytic estimate. Plan and report against 0.34 or lower for an adult learner.
4. **Kestin effect size (P2: 0.73-1.3 SD; P4: none).** Both true: abstract has none, full text has the range and 0.63 linear. The in-class comparison time is assumed. Design direction only.
5. **Mastery thresholds (P1: 60-70%; P2: 70% owner pick; P4: tunable).** Resolution: one owner-set threshold per domain, default 70% on delayed, unaided, novel items, recorded as a design choice. Berkeley's 60-70% is an observed pass band for an exam, not a mastery cut score.
6. **Retest intervals (P1: one month; P2: 1-2 weeks then longer; P3: 2-4 weeks; P4: 7 and 30 days).** Resolution: 7 and 30 days for every domain; matches the verified Rohrer design at the long end.
7. **Proof and answer grading (P1: two LLM passes; P4/P2: deterministic only; P5: calibrate).** Resolution: the verifier ladder in Section 3, rank 3.
8. **Qualifying-exam targets.** P2-P5 found none; unresolved until the owner names departments (Section 6).

## 5. Corpus licence tiers (per source, verified where marked)

| Tier | Rule | Sources |
|---|---|---|
| Ingest freely | Public domain (US, published 1930 or earlier; check edition and translator) and CC BY | Gutenberg via documented mirror, Wikisource (CC BY-SA: attribute, keep SA), Mathematics in Lean (CC BY 4.0, per P1), Earle Physical Geology 2e (CC BY 4.0, per P3), Müller Grammatical Theory (CC BY 4.0, per P5), Lebl under its CC BY-SA option (verified dual licence) |
| Private local index only, owner approval needed | CC BY-NC-SA or NC: attribution stored per chunk, never in a distributed bundle | MIT OCW (AI use verified allowed with conditions), Lebl under NC-SA, Axler LADR (CC BY-NC, verified), Judson (GFDL, verified; GFDL has its own copyleft terms), LibreTexts books chosen one at a time with the per-book licence logged (per P4, not re-verified) |
| Excluded from LLM pipeline | Explicit ban | OpenStax, whole library (verified on Chemistry 2e and Biology 2e) |
| Owner library, metadata and own notes only | In copyright | Hatcher (free download, copyright retained), Lee, Rudin, Ahlfors, Dummit-Foote, Jackson, Sakurai, Kardar, Alberts, Norton Theory and similar |
| Unconfirmed, do not ingest yet | Licence not stated | TCP texts, NACLO problems, Feynman Lectures, Khan Academy, IPCC, PhysPort items, concept-inventory items |

Pipeline note (P2, P4; untested): PDF extraction likely mangles equations and structures; test retrieval on equation-heavy queries and keep SMILES and LaTeX in owner notes.

## 6. Owner decisions needed

1. May CC BY-NC-SA and NC material (OCW, Lebl, Axler, LibreTexts books) live in the private local index? AGENTS.md currently says no.
2. Qual target per domain: math Berkeley-level (Tier 2) or Harvard-level (Tier 3); for physics, chemistry, biology, earth science and English, which departments' published syllabi or reading lists to use as templates.
3. Is Lean acceptable as an optional verifier for math proofs?
4. Will the owner request written permission from OpenStax (their form is on the book page) or drop OpenStax?
5. Is foreign-language reading (Old English, Latin, French) in scope for the English plan?

## 7. What not to build

- A "PhD in N months" pace, a qualifying-exam pass probability, a "PhD-ready" certificate, or a GRE-style score proxy. No validated data exists; GRE Physics did not predict completion (Miller 2019).
- An LLM-only grader for proofs, mechanisms, stereochemistry, spectra or derivations; a single-pass grader of any kind without calibration and owner override.
- A solver that returns final answers first, or LLM-authored answer keys with no CAS, checker or human check.
- Any OpenStax ingester or pipeline that sends OpenStax text to an LLM; a LibreTexts crawler; scraping Gutenberg, Wikisource or NACLO.
- Redistribution of any NC/SA material in releases.
- A home-made concept inventory presented as validated, or copies of licensed inventory items.
- A large custom ontology or an LLM-built knowledge graph; hand-edit the small graph.
- A fixed "PhD reading list" presented as authoritative.
- Streaks, XP or badges as mastery evidence; learning-style personalisation; empathy or "reading makes you better" claims (g about 0.15); "Your Brain on ChatGPT" as evidence (n=54, not peer-reviewed as seen).
- Local reaction-prediction models, heavy ab initio compute, or linking Open Babel (GPL-2.0).
- VLM reading of drawn structures, spectra or figures as authoritative.

## 8. Prerequisite maps

Proposals only; full node lists are in the five reports. Math: five tiers tied to OCW 18.xx courses. Physics S0-S5 and chemistry C1-C7 block on math nodes first. Biology, earth and astro route through chemistry, statistics and the physics map. English uses a per-skill graph.

## 9. Citation list with status

V (verified this pass): every row marked V in Section 2, with DOIs as in the source reports, plus Sinha and Kapur 2021 (doi 10.3102/00346543211019105), Hodds et al. JRME 45(1) (doi 10.5951/jresematheduc.45.1.0062), Libarkin and Anderson 2005 (doi 10.5408/1089-9995-53.4.394).

- U (unverified, not relied on): the 2026 arXiv LLM-grading preprints; Bisra, Barbieri and Adesope effect sizes; Deslauriers 2019, Von Korff 2016, Theobald 2020; ChemBench and MaCBench findings; Latimier 2021; the chemistry and English education papers marked T in P4 and P5; Noy and Zhang 2023; Kosmyna 2025 (do not cite); Princeton and MIT exam formats; canonical textbook lists; tool licences (RDKit, Ketcher, Open Babel); PhysPort, TCP, NACLO, Khan, IPCC reuse terms.


---

# FILE: docs/tutor/research/domains.md

# SYNTHESIS: Discipline-based pedagogy for advanced math, English, science and chemistry

Scope: reports E1 (math), E2 (English), E3 (science), E4 (chemistry), E5 (concept inventories and misconceptions). Verified 2026-09-30 by looking up abstracts and records on Crossref, Europe PMC, ERIC, Semantic Scholar and arXiv. No full texts were read, so "verified" means the abstract or record says what the report claims.

## 1. Bottom line

1. All five reports converge on one design principle: the tutor helps only after the learner attempts, and success is judged by delayed, unaided performance on fresh problems. The strongest direct evidence is Bastani et al. 2025 (unguarded GPT-4 lowered later grades 17%; a guardrailed tutor largely removed the harm) and Kestin et al. 2025 (a pedagogically built AI tutor beat in-class active learning).
2. Nearly all the evidence is school-age or introductory undergraduate. For PhD-level adult self-study, no trial in any of the four subjects was found. Every rule below is an inference from mechanisms plus intro-level data, and Tutor should test each rule on the owner's own delayed-retention data (single-learner alternating comparisons).
3. FSRS at 0.9 is well supported for discrete facts (Yang 2021, Latimier 2021) and is not a measure of skill. For problem solving, schedule by skill and serve a new variant each time.
4. Current LLMs are unsafe as sole authority for proofs, calculations and chemical mechanisms. Published failure numbers are for 2023-2025 models, so Tutor needs its own evals on Opus 5.5 and Muse Spark 1.3 before relaxing any verifier rule.

## 2. Verified evidence table

Status: V = abstract or record read this session and figure matches; C = corrected; U = exists but figure not checked here (taken from the report).

| # | Claim | Source | Checked figure | Status |
|---|---|---|---|---|
| 1 | Unguarded GPT-4 harms later unaided grades; guardrails mitigate | Bastani et al. 2025, PNAS 122(26), 10.1073/pnas.2422633122 | About 1,000 high-school students; "17% reduction in grades for GPT Base"; "largely mitigated" in GPT Tutor | V. Caveat: a PNAS erratum exists (10.1073/pnas.2518204122, Aug 2025); its content could not be read. Treat the 17% as provisional |
| 2 | AI tutor beats in-class active learning | Kestin et al. 2025, Sci Rep 15, 10.1038/s41598-025-97652-6 | Crossover RCT, N = 194 of 233; 0.63 SD linear, 0.73-1.3 SD quantile; AI median 49 min vs about 60 min class | V. Limits: first-exposure content, two lessons, no delayed test, authors caution about higher-order synthesis |
| 3 | Active learning beats lecture in STEM | Freeman et al. 2014, PNAS, 10.1073/pnas.1319030111 | 0.47 SD on exams and concept inventories (158 studies); failure odds ratio 1.95 (67 studies); 225 studies total | C. E1 attached 0.47 to "225 studies"; correct is 158 |
| 4 | Active learning feels worse but works | Deslauriers et al. 2019, PNAS, 10.1073/pnas.1821936116 | Active students learned more but perceived less learning (direction only in abstract) | V |
| 5 | Interleaving improves math delayed tests | Brunmair and Richter 2019, Psych Bull, 10.1037/bul0000209 | 59 studies, 238 effects; g = 0.42 overall, g = 0.34 math; blocking better for word-based material | V |
| 6 | Interleaving RCT in math | Rohrer et al. 2020, J Educ Psych 112(1), 10.1037/edu0000367 | 54 seventh-grade classes; 61% vs 38%, d = 0.83, unannounced test 1 month later | V |
| 7 | Interleaved physics homework | Samani and Pan 2021, npj Sci Learn, 10.1038/s41539-021-00110-x | Median +50% and +125% on two surprise tests; students rated it harder and thought they learned less | V. One course, 8 weeks |
| 8 | Self-explanation helps proof comprehension | Hodds, Alcock and Inglis 2014, JRME, 10.5951/jresematheduc.45.1.0062 | Three experiments; Exp 1 d = 0.95 on comprehension; Exp 3 a 15-minute intervention with lasting benefit | V. New figure: d = 0.95, small samples, inflated-effect risk |
| 9 | Self-explanation prompts in general | Bisra et al. 2018, Educ Psych Rev, 10.1007/s10648-018-9434-x | 64 reports, 69 effects, g = 0.55 | V (was recalled [R] in E1) |
| 10 | Productive failure | Sinha and Kapur 2021, Rev Educ Res, 10.3102/00346543211019105 | 53 studies, 166 comparisons, g = 0.36 [0.20, 0.51]; 0.37-0.58 at high fidelity; 0.87 bias-adjusted; grades 2-5 better with instruction first | V. Headline 0.36 only |
| 11 | Retrieval (quizzing) in classrooms | Yang et al. 2021, Psych Bull, 10.1037/bul0000309 | g = 0.499, 222 studies, 48,478 students | V |
| 12 | Spaced retrieval beats massed | Latimier, Peyre and Ramus 2021, Educ Psych Rev | 29 studies; spaced vs massed g = 0.74; expanding vs uniform g = 0.034, not significant | V |
| 13 | Feedback effectiveness depends on information content | Wisniewski, Zierer and Hattie 2020, Front Psychol, 10.3389/fpsyg.2019.03087 | d = 0.48, 435 studies, 994 effects, I2 = 83.4%; about 17% negative; high-information 0.99; corrective 0.46; reward/punishment 0.24 | V |
| 14 | Automated writing feedback | Fleckenstein, Liebenow and Meyer 2023, Front AI, 10.3389/frai.2023.1162454 | 20 studies, k = 84, N = 2,828; g = 0.55; heterogeneous | V. CI not rechecked |
| 15 | Writing instruction, adolescents | Graham and Perin 2007, J Educ Psych, 10.1037/0022-0663.99.3.445 | 123 documents, 154 effects; strategy 0.82, summarization 0.82, grammar -0.32 | V |
| 16 | Writing instruction, grades 6-12, update | Graham et al. 2025, Reading and Writing | 148 comparisons, 22,838 students; overall 0.38; process 0.75; strategy 0.59; transcription 0.54; SRSD 0.84 | V |
| 17 | Refutation text | Schroeder and Kucera 2022, Educ Psych Rev, 10.1007/s10648-021-09656-z | 44 experiments, g = 0.41 | V |
| 18 | Spatial training is durable and transfers to other spatial tasks | Uttal et al. 2013, Psych Bull, 10.1037/a0028446 | 217 studies, g = 0.47; stable over delays; transfer to untrained spatial tasks | V. Transfer to chemistry or geology not shown |
| 19 | LLMs fail at olympiad proofs (time-sensitive) | Petrov et al. 2025, arXiv 2503.21934 | USAMO 2025: best 25% (Gemini 2.5 Pro), all others under 5%; revised Sep 2025 | V. March 2025 models |
| 20 | Calculus Concept Inventory is weak | Gleason et al. 2019, 10.1080/0020739X.2018.1538466 | About 1,800 students, four institutions; "lacks sufficiently strong validity and reliability for its intended use" | V |
| 21 | Testing effect (retrieval vs restudy) | Rowland 2014, Psych Bull, 10.1037/a0037559 | Paper exists; abstract carries no effect size; g of about 0.50 is recalled | U |

Not checked (taken from reports as stated): Kalyuga expertise reversal, Rau 2018, Stieff, Padalkar and Hegarty, Cooper 2012, Houchlei 2021, Danielson 2025, Hake 1998, Madsen 2015, Kortemeyer 2023, Steiss 2024, Thoma and Iannone 2021, Potvin, Sadler, Traxler, Nissen, Melby-Lervag, Bego 2024, ChemBench, VanLehn 2011.

Failed citations: none. Corrections: Freeman study count (row 3); the Bastani erratum flag (row 1). Two items remain unverified and are flagged wherever used: Rowland g and every figure marked U above.

## 3. Rules for Tutor, ranked by evidence strength times impact

Tags: [P] voice/chat tutor prompt, [S] scheduling, [Q] question generation, [F] feature, [UI] interface, [M] measurement, [C] corpus.

1. **Attempt first, hint ladder, no answer dumping.** Order: clarifying question, goal or strategy cue, next-step hint, worked step, full solution only after an attempt or an explicit "show me" that is logged and costed. [P][UI][M] Evidence: Bastani (moderate, erratum caveat), Kestin. Applies to all four subjects. Impact: highest.
2. **Success metric is delayed (7+ days), unaided accuracy on new variants; log assisted and unassisted separately; never use satisfaction, streaks or XP as quality signals.** [M][UI] Evidence: Deslauriers 2019, Samani and Pan 2021, Bastani. Strong on direction.
3. **Retrieval and spacing for discrete items via FSRS at 0.9:** definitions, theorem statements, formulas, reaction facts, vocabulary, rhetorical terms. Production or cued recall, not recognition. [S][Q] Evidence: Yang 0.499, Latimier 0.74 (strong). Expanding versus uniform spacing does not matter. Domain caveat: chemistry spaced-retrieval gains were significant in only 2 of 9 courses (Bego 2024, unchecked), so measure locally.
4. **Predict then reveal with a 1-5 confidence rating; show mismatches; track calibration.** [P][UI][M] Evidence: active learning 0.47 SD (Freeman), hypercorrection (Metcalfe, pattern only). Medium-strong.
5. **Self-explanation aimed at logical or causal links** before the answer: why is this proof line valid, which electron source and sink, why does this paragraph follow. Reject paraphrase. [P] Evidence: Bisra g = 0.55; Hodds d = 0.95 (small). Chemistry-specific evidence is mixed (weak). Moderate.
6. **High-information, criterion-referenced feedback, 2-3 priorities, global issues first**, never praise-only. [P] Evidence: high-information 0.99 versus 0.24, but feedback overall is heterogeneous with 17% negative effects. Moderate.
7. **Interleave confusable categories after the first blocked exposure; always use fresh variants; schedule problem skills by skill or problem family, not by item.** [S][Q] Evidence: g = 0.34 math, d = 0.83 RCT, physics homework. Moderate. Do not interleave vocabulary definitions (blocking was better for word material in Brunmair). Unknown for graduate proof-technique selection.
8. **Verify with deterministic tools, never with the LLM alone:** CAS/Python for numbers and symbols, RDKit-style checks for atom balance, charge and arrow validity, optional Lean for proofs; the tutor states "I cannot verify this step" instead of bluffing. [F][P] Evidence: Petrov, Kortemeyer, ChemBench and Watts (unchecked). Direction moderate, figures dated; impact high.
9. **Misconception catalogue and refutation-form correction.** Name the wrong idea, why it is tempting, why it fails, end on the correct model; say "consistent with misconception X", never "validated". [P][Q][C][UI] Evidence: g = 0.41 (text-reading studies only; not shown for voice or PhD level). Moderate.
10. **Fade support by prior knowledge.** Worked example plus self-explanation for new topics; problem-first for familiar ones; productive-failure opener (5-10 minutes, no help) only when the learner has prerequisites. [P] Evidence: expertise reversal and Rau (unchecked), productive failure g = 0.36 (contested). Moderate-weak.
11. **English: never write the learner's prose.** Require a draft and revision before any model text; exemplars only after a draft; contextual error correction logged to a recurring-errors list; no isolated grammar drills. [P][F] Evidence: grammar -0.32, process 0.75, strategy 0.59 (school-age), Bastani by analogy. Moderate for adults.
12. **Writing is deliberate practice, not FSRS:** fixed weekly drafts, required revision, and a monthly timed AI-off sample scored on a fixed rubric, with revision uptake logged. [S][M][F] Evidence: theory (Kellogg), Farrokhnia (quality of feedback did not improve revisions). Weak-moderate.
13. **Proof workspace:** state goal and proof type first, then line-by-line steps with justifications; proof-critique items with one flawed step; Library proofs become prove-first or fill-the-gap tasks. [F][Q] Evidence: Hodds, Selden and Selden, Weber (qualitative). Moderate-weak. Lean is opt-in (weak evidence).
14. **Chemistry structure and curved-arrow editor** with deterministic checks; withhold the product on early attempts; commit-then-reveal 3D viewer. [F][P] Evidence: quasi-experimental and qualitative; no test of an LLM-plus-tool tutor. Reasoned, weak-moderate, high impact because LLMs fail here.
15. **Question-bank schema:** skill, misconception_id, representation, variant_family, verified solution, a transfer context; two-tier answer-plus-reason for conceptual items; parameterised variants computed in code; 3-4 options; at least one catalogue-mapped distractor; independent solve check; unmapped distractors labelled "generic". [Q] Evidence: LLM distractors diverge from real student choices (Feng 2024, unchecked). Moderate.
16. **Optional spatial drills**, tracked separately. [F] Evidence: g = 0.47 on spatial tasks; chemistry transfer unproven. Low-priority.
17. **Corpus and licensing.** Catalogue misconceptions as concepts with citations and own-words text; write original items; do not ingest FCI, CCI, QMCS or FMCE item text or scrape PhysPort. Record licence per source. [C] Basis: PhysPort access terms and 17 USC 102(b) as reported by E5, not legal advice.

## 4. Conflicts and how they were resolved

1. **Freeman study count** (E1 says 0.47 SD over 225 studies; E3 and E5 say 158). Verified: 0.47 SD comes from 158 exam studies; 225 is the total including failure-rate studies. Use 158.
2. **Interleaving everywhere (E1, E3) versus null or reversed effects for words and expository text (E2, from Brunmair).** Both true. Benefit scales with similarity between categories and with complexity. Interleave confusable problem families in math, physics and chemistry; block vocabulary and definitions.
3. **Attempt-first (productive failure) versus worked examples for novices (E1, E3, E4).** Resolved by prior knowledge: novices on a topic get worked examples plus self-explanation; learners with prerequisites get a capped problem-first opener. Productive failure is g = 0.36, contested, adult samples were a minority, and the 0.87 bias-adjusted figure (verified in the abstract, unusual direction) is not used as evidence.
4. **FSRS scope.** E1 and E3 say FSRS only for discrete items; E4 says one FSRS card per skill. Compatible: a card may represent a skill node, but what is shown on review must be a fresh variant, never the identical problem. Strong general retrieval evidence (Yang, Latimier) outweighs the weak chemistry-specific finding (Bego, 2 of 9 courses), but it should be measured on the owner's data.
4b. **Retest timing.** E5 proposes a retest at about one day after a high-confidence wrong answer; E1 and E3 propose delayed tests at 7 days and 4 weeks. Resolved: the 1-day retest is remediation (a design hypothesis, flagged experimental); the 7-day and 4-week tests are the measurement.
5. **Hint ladder thresholds.** E1 allows a logged "show me"; E3 requires two attempts before a full solution. Resolved: default two attempts, with a logged escape.
6. **Spatial training strength.** E3 calls it strong; E4 calls it moderate for chemistry. Both fit: strong for spatial tasks (verified g = 0.47, durable, transfers to other spatial tasks), unproven for chemistry or geology outcomes. Keep it optional. Note g = 0.47 coincides with Freeman's 0.47 but measures a different thing.
7. **Feedback quality versus learning (E2).** AI feedback can rate as good as teachers' yet not improve revisions (Farrokhnia), while aggregate AWE g = 0.55 (verified) is heterogeneous and likely inflated by short, researcher-made outcomes. Stronger evidence: high-information feedback (0.99) plus the revision gate; do not rely on feedback quality ratings as the metric.
8. **Intro-level inventories versus a PhD-track learner (E5 versus E3).** Inventories are group-level, introductory and ceiling-limited; they serve as floor checks only. Use upper-division instruments' misconceptions as concepts, and never use an FCI score as the learner's level.
9. **Dated LLM-failure evidence.** Petrov (March 2025 models), Kortemeyer (2023) and the JCE chemistry studies (GPT-3.5/4 era) are out of date for Opus 5.5 and Muse Spark 1.3. The stable finding is overconfidence and weak electron-movement reasoning. Keep the verifier rules until Tutor's own evals show low error rates.

## 5. What not to build

- A default answer-giving chat, a "rewrite my essay" button, or an LLM that writes the learner's prose (Bastani; Fan 2025 reported in E2).
- A Lean-first curriculum (weak, self-selected evidence), or LLM proofs, arrow diagrams or stereochemistry presented as authoritative.
- FSRS on whole multi-step problems with repeated identical items.
- Streaks, XP, satisfaction or Hake normalized-gain dashboards (Nissen 2018, unchecked; gain is biased toward strong pretests).
- Learning-styles personalization, a generic critical-thinking module, empathy-from-fiction claims, or EEG "cognitive debt" messaging (n = 54 preprint).
- An essay score shown to the learner as a grade (AWE g ranges 0.43-0.86); keep scores internal for trend tracking.
- Isolated grammar drills (-0.32).
- A Johnstone-triangle mode (no intervention evidence); use it as a prompt pattern only.
- Copies of FCI, CCI, QMCS or FMCE items, a PhysPort scraper, an English multiple-choice concept inventory with invented authority, an LLM-only distractor generator, single-answer automatic misconception diagnosis.
- A large VR module, an unguided simulation sandbox, simulated peers, or Dual N-Back claims for subject learning (no far transfer, Melby-Lervag 2016, unchecked); keep N-Back optional and separate.

## 6. Corpus and ingestion notes

- Seed the misconception catalogue from published literature (concept plus citation, own words). Otero 2024 (arXiv 2412.03765) math-misconception benchmark is reported CC BY-NC-SA 4.0, unchecked here.
- Licences to confirm before ingesting full text: CBE-LSE articles, J Chem Educ, Project 2061, the Eedi dataset, PhET terms. All unverified.
- The owner's question about open Chinese institutional materials was outside E1-E5 and is not answered here. No licences were checked. It needs its own research pass before anything is added to `corpus/manifest.json`.

## 7. Open questions

1. No evidence at PhD level or for adult solo learners in any of the four subjects; run single-learner alternating comparisons (for example predictions on versus off, by topic) on delayed retention.
2. Does refutation correction work in spoken dialogue and with advanced adults? Only text-reading studies found.
3. Does a verifier-backed LLM tutor reproduce human-feedback gains in chemistry? No study.
4. Current Opus 5.5 and Muse Spark 1.3 error rates on proof grading, mechanisms and numerics: needs Tutor's own held-out evals.
5. What did the Bastani erratum change? Read it before quoting the 17% in any product copy.
6. Is the owner a native English speaker? It decides how much EFL corrective-feedback evidence applies.
7. Coverage gaps: biology, earth science, upper-division and graduate physics, physical and quantum chemistry, East Asian curricula, Polya and Schoenfeld (books unopened), bianshi variation teaching.
8. Verify before quoting: Rowland g, VanLehn 2011 (human tutoring about 0.79, ITS about 0.76), Alfieri 2011, Rau, Danielson 2025 pooled effect.

## 8. Citation list with verification status

V = verified this session (abstract or record read). U = exists per report, not checked by me. Full DOIs are in section 2 for V items.

V: Bastani 2025 (plus erratum flag); Kestin 2025; Freeman 2014 (corrected); Deslauriers 2019; Brunmair and Richter 2019; Rohrer 2020; Samani and Pan 2021; Hodds 2014; Bisra 2018; Sinha and Kapur 2021; Yang 2021; Latimier 2021; Wisniewski 2020; Fleckenstein 2023; Graham and Perin 2007; Graham 2025; Schroeder and Kucera 2022; Uttal 2013; Petrov 2025; Gleason 2019.

U (report-only, not checked by me): Rowland 2014 (figure recalled), plus every source in the "Not checked" list in section 2 and all other works cited only in E1-E5 (for example Dunlosky 2013, Kellogg 2007/2009, Steiss 2024, Farrokhnia 2026, Fan 2025, Kosmyna, Hake 1998, Chernikova 2020, Lazonder and Harmsen 2016, Mirza 2025, Watts 2023, Otero 2024). Consult the source reports for their full details.


---

# FILE: docs/tutor/research/engineering.md

# Engineering synthesis: technical RAG and scheduling design for Tutor

Verified 2026-09-30 by re-opening primary pages (arXiv, GitHub READMEs, vendor pricing pages, Crossref, Europe PMC, Anthropic docs). Inputs: X1 (technical RAG) and X2 (scheduler and mastery). "[sim]" means a simulation by the report author, not literature; I did not re-run any simulation.

## 1. Bottom line

1. Parsing quality and citation discipline matter more than embedding choice. Tier the parse, ground every quantitative claim in a stored span, and show the page image next to any quoted formula.
2. Schedule flashcard-like atoms with FSRS-6 at its published defaults and 0.90 desired retention. Model procedures and proofs with a per-skill decayed Beta posterior plus a sequential mastery test, not FSRS and not "k in a row".
3. Nearly all numbers that are specific to Tutor (cost, pages per GB, power, workload) rest on assumptions or simulations. Build a corpus inventory and a hand-checked gold set first so these become measured.
4. No cited study involves PhD-level adults, and no source shows FSRS improves learning; its evidence is recall prediction on Anki data.

## 2. Verified table

Status: **verified** = page opened, claim matches; **corrected** = claim needed a fix; **partial** = exists, but the specific figure was not confirmed; **failed** = could not be confirmed.

| # | Claim | Source checked | Status | Note |
|---|---|---|---|---|
| 1 | olmOCR-Bench math scores: olmOCR v0.4.0 83.0 (ArXiv) / 82.3 (old scans); Chandra 0.1.0 82.2 / 80.3; Marker 1.10.1 83.8 / 66.8; Mistral OCR API 77.2 / 67.5 | allenai/olmocr README table | **verified** | Overall: Chandra 83.1, olmOCR 82.4, Marker 76.1, Mistral 72.0. Ai2 authors the benchmark. |
| 2 | olmOCR is Apache 2.0, claims under $200 per million pages | same README | **verified** | The cost claim is for self-hosting, which the owner cannot do. |
| 3 | olmOCR 2 paper: Poznanski, Soldaini, Lo, arXiv 2510.19817 | arXiv abstract | **verified** | |
| 4 | Hosted olmOCR at $0.07-0.10 per 1M input tokens, and the per-page estimate of about $0.25-0.45 per 1k | DeepInfra page returned 404 | **failed** | Price and token counts remain unverified. The "$185 OCR everything" cost is dropped from recommendations. |
| 5 | MIRB: Ju and Dong, 12 datasets, 13 models, voyage-3-large 54.54, text-embedding-3-large 49.93 | arXiv abstract and PDF text | **verified** | |
| 6 | MIRB reranking on top 10: voyage-3-large 54.54 becomes 48.36 (bge-reranker-v2-m3) or 50.42 (jina-reranker-v2); text-embedding-3-large 49.93 becomes 44.67 or 46.55 | PDF Table 10 text | **verified** | The paper says reranking "generally leads to a decline", with a few slight gains. It also says reasoning and premise retrieval are hard for all models. Only two older general rerankers were tested. |
| 7 | ALCE (Gao, Yen, Yu, Chen, EMNLP 2023): best models lack complete citation support 50% of the time on ELI5 | arXiv abstract | **verified** | Models of that date. |
| 8 | Claude Citations: `cited_text` not billed as output; citations on `document` or `search_result` blocks plus structured outputs returns HTTP 400; pointers parsed by the API | platform docs (markdown) | **verified** | Pointers are valid by construction; nothing says the passage supports the claim. |
| 9 | Mathpix: $0.005/page (v3/pdf), $0.0015/page (Files API batch), $0.002/image, $19.99 setup fee, 30-day default retention, 24 h opt-out | mathpix.com/pricing/api | **verified** | Accuracy on math is still unbenchmarked independently. |
| 10 | Mistral OCR 4.1 at $4 per 1,000 pages | mistral.ai/pricing/api | **verified** | A general half-price Batch tier is listed; whether it covers OCR is not confirmed. |
| 11 | Datalab convert $4 per 1k (fast/balanced), $10 (accurate) | datalab.to/pricing | **verified** | The "25% off for retention" claim was not on the page. |
| 12 | voyage-4 $0.06 per 1M tokens, first 200M free; voyage-context-4 $0.12; batch 33% off | docs.voyageai.com pricing | **verified** | voyage-3-large is $0.18, with no free tokens. |
| 13 | Anthropic contextual retrieval: failures down 35% (5.7% to 3.7%), 49% with contextual BM25, 67% with reranking (to 1.9%); $1.02 per million document tokens with caching | anthropic.com/news | **verified** | Vendor-reported; not textbook math; 2024 pricing. |
| 14 | FSRS benchmark: about 10k users, about 350M reviews; FSRS-6 log loss 0.3460; FSRS-5 0.3561; FSRS-4.5 0.3625 | srs-benchmark README | **verified** | 9,999 collections, 349,923,850 reviews. |
| 15 | "FSRS-7 0.3370" | same README | **corrected** | 0.3370 is the "FSRS-7 recency" variant. Plain FSRS-7 is 0.3401. Both beat FSRS-6. |
| 16 | FSRS-6 has 21 parameters; R=(1+factor·t/S)^(-w20) with trainable decay w20 | awesome-fsrs wiki, The Algorithm | **verified** | The wiki does not mention FSRS-7. |
| 17 | Licences: fsrs-rs (crate `fsrs`) BSD-3-Clause; py-fsrs MIT | GitHub repos | **verified** | New: fsrs-rs reports that FSRS-6 and FSRS-7 are selected from the parameter count, so both are available in Rust. |
| 18 | Brunmair & Richter 2019: g = 0.42 overall, maths 0.34, words -0.39; 59 studies, 238 effects | Europe PMC abstract | **verified** | Abstract calls the maths effect "small". |
| 19 | Yang et al. 2021: g = 0.499, 222 studies, 48,478 students | Europe PMC abstract | **verified** | Classroom students. |
| 20 | Lindsey et al. 2014: +16.5% vs massed, +10.0% vs one-size spaced | Crossref abstract | **verified** | Middle-school foreign-language course. |
| 21 | Cepeda et al. 2008: more than 1,350 people; optimal gap about 20-40% of a 1-week delay, 5-10% of a 1-year delay | Crossref abstract | **verified** | Fact recall. |
| 22 | Murray, Goebel, Horner 2025: maths spacing g 0.26 (25 studies, 49 effects); testing vs restudy g 0.22 with CI crossing zero (6 studies, 20 effects) | Crossref abstract | **verified** | Crossref type is posted-content, so a preprint. The abstract says the maths effect "may be smaller than in other domains". |
| 23 | Gervet et al. 2020, JEDM 12(3): logistic regression with good features leads on moderate datasets; 9 datasets | Zenodo abstract | **verified** | Abstract adds that BKT-style models underperformed. |
| 24 | Mettler, Massey, Kellman 2016: adaptive spacing beat fixed schedules on immediate and delayed tests | Europe PMC abstract | **verified** | Effect sizes not read. |
| 25 | Beck & Chang 2007, identifiability problem for student modelling | Crossref | **verified (title and authors)** | Content of the argument not re-read. |
| 26 | MolScribe 76-93% accuracy on public benchmarks | arXiv abstract | **verified** | It is a local model. See conflict C5. |
| 27 | Latimier et al. 2021 meta-analysis of spacing retrieval | Crossref | **partial** | Exists; effect sizes unread, so none are quoted here. |
| 28 | Azure Read $1.50 per 1k; formula add-on and Google Document AI prices | not re-checked | **unverified** | Left as unknowns; both are candidates for the bake-off only. |

No claim that a recommendation depends on failed verification. The only failure (row 4) removes an optimistic cost figure, not a design decision.

## 3. Recommendations, ranked by evidence strength times impact

Tags: prompt, scheduling, question generation, feature, UI, measurement, corpus. "Evidence" is how well the source supports the rule; "impact" is how much it changes Tutor.

1. **[prompt] State a formula or quantitative claim only if it sits in a cited span.** Otherwise label it as reasoning beyond the source. On a retrieval miss say "not found in the library" instead of filling from memory. Say when a formula came from OCR and offer the page image. Evidence: strong for the mechanism (Anthropic docs), moderate for the need (ALCE). Impact: highest, because it is the honesty guarantee.
2. **[corpus] Add a deterministic span check after every cited answer.** Cited text must be a normalized substring of a retrieved chunk, and every number and equation in the answer must appear in cited text. Citations only guarantee a valid pointer, not support. Evidence: strong docs, moderate ALCE.
3. **[corpus, measurement] Before any bulk spend, run a library inventory and build a hand-verified gold set of about 150-200 pages.** Classify pages as text-layer or scan, score math density, and count real pages per GB. Every cost figure scales with pages per GB (assumed 10,000, about 3x uncertain either way). Evidence: the need is strong because no independent math benchmark exists for Mathpix, Azure or Google.
4. **[corpus] Tier the parse.** Keep pdfium for clean prose. Send math-dense and scanned pages to a hosted parser, and run a bake-off on the gold set between at least olmOCR 2 (hosted, through a provider whose price I confirm), Mathpix batch, Mistral OCR 4.1 and Datalab. Escalate any page whose extracted LaTeX fails KaTeX rendering to a second engine. Evidence: moderate; the claim that pdfium garbles equations is X1's untested judgement.
5. **[scheduling] Atoms use FSRS-6 with published defaults and desired retention 0.90; never above 0.95.** Use the Rust `fsrs` crate (BSD-3) or py-fsrs (MIT).
6. **[measurement] Mastery for procedures and proofs is a band, not a number.** Keep a time-decayed Beta posterior per skill and a sequential probability ratio test. Assisted attempts never update it. "Mastered" needs roughly 15-20 independent unassisted attempts over several days. Evidence: moderate, rests on Gervet 2020, Beck & Chang 2007 and the author's simulations [sim]. Impact: high.
7. **[question generation] Generate from cited spans and store the evidence.** Each item stores the span, page, parse engine and version, `family_id`, the varied dimension and the invariant. Reject an item if its answer key does not validate against the cited text, and compute numeric keys independently. Citations cannot be combined with structured outputs (HTTP 400), so use a citation-enabled call for grounded text, then a separate schema-constrained extraction call, then the span check.
8. **[scheduling] Variant families are testlets.** Schedule the family as one FSRS card whose review is a fresh unassisted variant; serve at most one fresh variant per family per day; never call variants of one seed independent practice. The FSRS-for-families approximation is unvalidated; monitor calibration.
9. **[corpus] Hybrid retrieval with no default reranker.** tantivy BM25 with a tokenizer that keeps LaTeX commands, hosted dense vectors chosen by bake-off (voyage-4, voyage-3-large, Gemini Embedding 2, text-embedding-3-large), reciprocal rank fusion. Add a reranker only if it wins on the owner's queries. Evidence: moderate and narrow (see C1).
10. **[corpus] Structure-aware chunking.** Split by heading, definition, theorem, proof, example; never inside `$$...$$` or a LaTeX environment; keep parent-child links; keep the 400-token cap but expect fewer words. Add short contextual prefixes only for the current curriculum, where the cost is small.
11. **[UI] "Verify source" page-crop button, KaTeX everywhere, per-page "OCR suspect" flag.** Never show raw OCR LaTeX strings as final.
12. **[scheduling] Personalise FSRS weights only after about 1,000 unassisted graded reviews**, at most monthly, and accept the fitted set only if a chronological holdout beats the defaults by a margin fixed beforehand. Store both parameter sets. The 1,000 figure is X2's judgement; the Anki manual only says that "less than a few hundred" reviews is a common cause of poor fit.
13. **[scheduling] Daily queue:** due atoms, then due families, about 30% mixed-family problems with confusables adjacent, cumulative review of earlier units, 90-minute soft cap. Evidence: interleaving g 0.34 in maths, small.
14. **[scheduling] Prerequisite edges are advisory**, labelled author, LLM or data, used to route the entry diagnostic and suggest review, never to block. Flag edges the data contradict.
15. **[UI, measurement] Show binned predicted-vs-observed recall and a retention-workload table.** [sim] Reviews per card in year 1 at FSRS-6 defaults: 5.5 (R 0.80), 6.7 (0.85), 7.9 (0.90), 13.2 (0.95), 18.3 (0.97). (Assumes FSRS is the true model: workload, not validity.) Headline outcome stays delayed unassisted parallel items.
16. **[measurement] Retrieval evaluation.** nDCG@10 and recall@20 on about 200 queries split into semantic, formula and premise types, using the existing `--split train|test --seed N` discipline and paired bootstrap. Add citation precision and recall in the ALCE sense.
17. **[measurement] At most two or three personal experiments**, each with a preregistered minimum detectable effect, randomised over items or days, never periods. [sim] A retention-setting contrast (0.85 vs 0.92) needs about 320 reviews per arm; a logit-0.8 method effect needs about 160-320 items per arm; effects under about 6 points are unresolvable. Label small-effect tests underpowered and report intervals.
18. **[corpus] Provenance.** Store raw engine output, engine and version, the page image and original PDF, so pages can be re-parsed selectively and cards flagged when a source changes.

## 4. Conflicts and resolutions

- **C1. Rerankers: MIRB says they hurt; Anthropic says they help (67% fewer failures).** Different data (math benchmark with two older general rerankers vs mixed, mostly non-math data) and different rerankers. Resolution: default off, test a modern reranker on the gold set. Do not cite either number as settled for Tutor.
- **C2. FSRS-6 vs FSRS-7.** X2 recommends FSRS-6 and lists FSRS-7 as unchecked. I confirmed FSRS-7 beats FSRS-6 on the benchmark (0.3401, or 0.3370 for the recency variant), and that fsrs-rs supports both. But the wiki page I read documents only FSRS-6, and its 21 parameters have published defaults. Resolution: start with FSRS-6 and revisit FSRS-7 when its algorithm page and default parameters are documented; log enough review data (`fsrs_log`) that switching needs no data loss.
- **C3. olmOCR-Bench ranking.** X1's scores match the README exactly. The one thing to keep in mind is that Chandra's overall score (83.1) is above olmOCR's (82.4) and Marker's ArXiv math (83.8) is above both; olmOCR's case rests on price and open weights, not on leading the table.
- **C4. Schema gap between the two reports.** X2's SQLite schema has `item.chunk_id` but nothing for the cited span, page or parse engine that X1 says each item and card must carry. Resolution: add `source_span`, `page`, `parse_id` (engine and version) and `span_verified` to `item`, and a `parse` table. Keep X2's append-only `attempt` log unchanged.
- **C5. "No local models" vs on-demand MolScribe.** MolScribe and DECIMER are local neural models, and the GPU is unavailable. Resolution: defer chemistry-structure recognition. If needed later, try a cloud vision model on the crop plus an RDKit round-trip check, or confirm Mathpix's SMILES output. Do not bulk-parse.
- **C6. Cost range.** X1's "OCR everything" range ($185 to $5,000 at 50 GB) mixes a verified price list with an unverified olmOCR estimate and an assumed page density. After dropping the olmOCR figure, the verified floor is Mathpix batch at $750 per 500k pages, and the tiered plan ($150-400 at 50 GB) is an estimate, not a quote. Treat all of it as a planning range until item 3 is done.
- **C7. Copyright and privacy.** The project rules forbid vendoring copyrighted books into the corpus; parsed text of purchased books is derived local data and must stay out of git. Sending purchased books to a hosted parser is a separate owner decision. Mathpix retains data 30 days unless you opt out (24 hours); retention for Azure, Google, Datalab and hosted olmOCR is unverified. Check terms before any bulk upload.
- **C8. Power numbers** assume independent items with item-difficulty SD 1.0 logit; generated variants may differ. Treat them as optimistic until the owner's variance is measured. Other open unknowns: Mathpix/Azure/Google math accuracy on the owner's pages, modern reranker and voyage-4 behaviour on math, FSRS calibration for family cards and for foreign-language vocabulary.

## 5. What not to build

- Whole-library OCR with the priciest engine, or any bulk OCR before the inventory and bake-off.
- Self-hosted OCR, embedding or reranking models.
- Bulk chemical-structure extraction.
- A default cross-encoder reranker.
- Reliance on any engine's confidence score (Azure's is hard-coded) or on Mistral's March 2025 94% marketing figure. Third-party olmOCR-Bench puts that Mistral model at 72.0 overall.
- Deep knowledge tracing, transformer or RWKV schedulers, per-skill BKT fitting at n=1.
- "Three correct in a row = mastered" (passes a 0.7-accuracy learner about 34% of the time [sim]).
- Hand-tuned FSRS weights, and a retention dial above 0.95 (X2 forbids only above 0.97; I cap it lower because workload nearly doubles from 0.90 to 0.95 [sim]).
- Hard prerequisite gating, and any efficiency claim for it.
- Crediting assisted attempts or same-session variants as independent evidence.
- Marketing copy citing "20-30% fewer reviews than SM-2" (community-wiki simulation), or claiming FSRS improves learning.
- Several simultaneous experiments on overlapping skills, or p-value verdicts on effects under about 0.3 logit.

## 6. Citation list with verification status

Verified unless stated.
- Poznanski, Soldaini, Lo 2025, olmOCR 2, arXiv 2510.19817; github.com/allenai/olmocr README (benchmark table, licence). Verified.
- Ju & Dong 2025, MIRB, arXiv 2505.15585 (Tables 9 and 10 read from PDF text). Verified.
- Gao, Yen, Yu, Chen 2023, ALCE, EMNLP 2023, arXiv 2305.14627. Verified.
- Anthropic, Citations documentation, platform.claude.com/docs/en/build-with-claude/citations. Verified.
- Anthropic 2024, Contextual Retrieval, anthropic.com/news/contextual-retrieval. Verified.
- mathpix.com/pricing/api; mistral.ai/pricing/api; datalab.to/pricing; docs.voyageai.com/docs/pricing. Verified (details in section 2).
- open-spaced-repetition/srs-benchmark README; awesome-fsrs wiki "The Algorithm"; fsrs-rs; py-fsrs. Verified (FSRS-7 figure corrected).
- Brunmair & Richter 2019, Psych Bull 145(11), 10.1037/bul0000209. Verified.
- Yang, Luo, Vadillo, Yu, Shanks 2021, Psych Bull 147(4), 10.1037/bul0000309. Verified.
- Lindsey, Shroyer, Pashler, Mozer 2014, Psych Sci, 10.1177/0956797613504302. Verified.
- Cepeda, Vul, Rohrer, Wixted, Pashler 2008, Psych Sci, 10.1111/j.1467-9280.2008.02209.x. Verified.
- Murray, Goebel, Horner 2025, OSF preprint, 10.31219/osf.io/wdahf_v1. Verified; preprint.
- Gervet, Koedinger, Schneider, Mitchell 2020, JEDM 12(3), 31-54. Verified.
- Mettler, Massey, Kellman 2016, JEP: General, 10.1037/xge0000170. Verified (direction only).
- Beck & Chang 2007, "Identifiability: A Fundamental Problem of Student Modeling", UM 2007. Verified (title and authors).
- Qian et al. 2023, MolScribe, arXiv 2205.14311. Verified.
- Latimier, Peyre, Ramus 2021, Educ Psych Rev. Partial (existence only).
- Not re-checked this round (quote none of their numbers without opening them): Khajah et al. 2016; Pavlik et al. 2009; Pelánek 2016 and 2017; Choffin et al. 2019; Pelánek & Řihák 2017; Zhang et al. 2025; Wainer et al. 2007; Doignon & Falmagne 1999; Kravitz & Duan 2022; Ye et al. 2022; Anki manual; Azure, Google and Gemini pricing.


---

# FILE: docs/tutor/research/expert.md

# Expert-level learning and AI tutoring: verified synthesis

Scope: five reports (D1 AI tutor trials, D2 tutoring and ITS, D3 graduate learning, D4 LLM tutor design, D5 assessment), all read in full. I re-checked 23 load-bearing citations on 2026-09-30 by direct fetch (web search was out of quota). No citation failed outright; three were corrected and several carry caveats.

## Bottom line

1. The only robust, replicated finding is a negative one: unguarded LLM chat raises practice scores and lowers later unassisted performance, and learners do not notice. Guardrails remove the harm. They do not, by themselves, produce a gain.
2. Gains show up when the tutor's pedagogy is the intervention (attempt-first, expert solutions, scaffolding). The evidence is one immediate-test undergraduate physics crossover and one small MBA trial. Nothing is tested at PhD level or on delayed retention.
3. So build Tutor as a measured system: enforce the guard, log assisted and unassisted performance separately, and let the owner's delayed closed-book results decide what stays.

## Verified evidence table

Status key: V = verified as claimed; V* = verified with a caveat; C = corrected; U = not verified by me (taken from a report).

| # | Claim | Source | Status and what I saw |
|---|---|---|---|
| 1 | Unguarded GPT-4: practice +48%, later unassisted exam -17% (-0.054, p<.05). Hint-only tutor: practice +127%, exam -0.4% (-0.004, n.s.). | Bastani et al., PNAS 2025, doi:10.1073/pnas.2422633122 | V (Europe PMC full text). About 1,000 students in grades 9-11, 4 x 90 min, 3 arms. The tutor prompt held correct solutions and common mistakes. GPT-4 answered correctly 51% of the time. GPT Base users did not notice the loss; GPT Tutor users thought they did better. A correction exists and was not read. |
| 2 | Designed AI tutor beat in-class active learning, 0.63 SD (linear), 0.73-1.3 SD (quantile) | Kestin et al., Sci Rep 2025, doi:10.1038/s41598-025-97652-6 | V (full text). N=194 crossover, two intro-physics lessons, immediate post-test, no delayed test, median 49 min. Authors caution it may not transfer to complex synthesis. |
| 3 | Structured AI tutor, graduate adults: +6.63 points (p=.007), voice = text for mastery, voice 2.8x cost | Yang, Van Alstyne, Dellarocas, arXiv 2609.23958 | V* (abstract). Preregistered, N=86 MBA finance. Instructor-graded gain about 2.6 points; gains concentrated in written reasoning. The d=0.53 figure was not seen. |
| 4 | Substituting LLM for exercises lowers understanding per topic; LLMs widen the prior-knowledge gap | Lehmann, Cornelius, Sting, arXiv 2409.09047 | V* (abstract). No aggregate effect. Sample sizes not in the abstract. |
| 5 | Human-AI copilot: +4 pp mastery, +9 pp for weaker tutors | Wang et al., Tutor CoPilot, arXiv 2410.03017 | V. Preregistered, 900 tutors, 1,800 K-12 students. |
| 6 | Expert-supervised LearnLM: transfer 66.2% vs 60.7% | LearnLM Team and Eedi, arXiv 2512.23633 | C. Numbers confirmed, N=165. The abstract says 76.4% of drafts had zero or minimal edits. D1 said 74.4% unedited. Use 76.4% minimal edits. The study is exploratory and every message was human-approved. |
| 7 | GPT-4 reveals the answer about 47% of the time; Claude 3.5-era Sonnet about 5%; expert human tutors about 9% | Maurya et al., MRBench, NAACL 2025, arXiv 2412.09416 | V* (full text). Table 3 lists the share that does not reveal (GPT-4 53.12, Llama-3.1-405B 80.73, Sonnet 94.79, expert 90.62), so D4's readings hold. Older models. The dataset has 192 conversations and 1,596 responses. |
| 8 | Pooled tutoring effect 0.37 SD | Nickow, Oreopoulos, Quan, NBER w27476 | V for 0.37 SD, PreK-12 only. The "96 RCTs" count is not on the abstract page (U). |
| 9 | Human tutoring d=0.79, ITS d=0.76, versus a believed 2.0 and 1.0 | VanLehn 2011, Educ Psychol | V* . The two numbers were confirmed, but the tool paraphrase swapped the labels. The original abstract gives human 0.79 and ITS 0.76. The step/substep breakdown stayed unverified (U). |
| 10 | ITS median 0.66 SD over 50 evaluations; smaller with standardized tests and flawed implementations | Kulik and Fletcher 2016, RER | V (abstract). |
| 11 | Self-explanation prompting g=0.55 (69 effects, 64 studies, 20 moderators) | Bisra et al. 2018, Educ Psychol Rev | V (ERIC record). |
| 12 | Self-explanation training on proof line-to-line logic d=0.95; effect persisted | Hodds, Alcock, Inglis 2014, JRME | V (verbatim abstract). UK undergraduates, transition-to-proof level. |
| 13 | Active learning beats lecture: 0.47 SD over 225 studies | Freeman et al. 2014, PNAS | C. 0.47 SD and 225 studies confirmed. The abstract says lecture students were 1.5 times as likely to fail. D3's "odds ratio 1.95" was not confirmed. Use 1.5x. |
| 14 | Unassisted discovery d=-0.38 versus explicit instruction; enhanced discovery d=+0.30 | Alfieri et al. 2011, J Educ Psychol | V (abstract, 164 studies). Mostly school-age samples. |
| 15 | Problem-first-then-instruction g=0.36 [0.20, 0.51]; reverses in grades 2-5 and for domain-general skills; bias-adjusted 0.87 is not a planning number | Sinha and Kapur 2021, RER | V (abstract, 53 studies). |
| 16 | LLM distractors are valid but miss real student errors | Feng et al., NAACL Findings 2024, arXiv 2404.02124 | V (abstract). K-12 maths. |
| 17 | LLM difficulty labels vs empirical difficulty rho=0.06 (378 items, 7,888 responses) | An and Wang, arXiv 2609.27063 | V. Single data-science class, preprint. |
| 18 | LLM proof judge: human double-grading agreement 90.4%, GPT-5 89.3% single, 90.8% with 5 votes; only 59.5% of o3's correct answers had valid proofs | Dekoninck et al., Open Proof Corpus, arXiv 2506.21621 | V (HTML full text). Competition-style proofs. |
| 19 | Generative search engines: 51.5% of sentences fully supported, 74.5% citation precision | Liu, Zhang, Liang, EMNLP Findings 2023, arXiv 2304.09848 | V. Older systems, not textbook RAG. |
| 20 | Active learning feels worse and works better | Deslauriers et al. 2019, PNAS | V (abstract). Intro physics. |
| 21 | Andragogy research "inconclusive" | Rachal 2002, Adult Educ Q | V (abstract). |
| 22 | Pooled ChatGPT effect g=0.867 vanishes after publication-bias adjustment | Bartoš, Martinková, Wagenmakers 2025, PsyArXiv | V for the preprint and its abstract. D2's claim that the original Wang and Fan meta-analysis was retracted is U (OpenAlex rate-limited me). |
| 23 | "Cognitive debt" from LLM essay writing | Kosmyna et al. 2025, arXiv 2506.08872 | V for existence: N=54, 18 in session 4, EEG, preprint. Contested. Design must not rest on it. |

Citations I did not re-check are listed at the end as U. Rules resting only on them are tagged "reported."

## Conflicts between reports and how I resolved them

1. **Does the tutor model see the solution?** D1, D2 and D4 put the reference solution and misconception text into every tutor turn (Bastani's protective condition). D5 says the tutor model must never see the key, and that a separate check-answer tool returns only a verdict, error category and hint level. Bastani's tutor had solutions in the prompt, the only trial evidence. D5's argument is robustness: a prompt-only "do not reveal" rule can be broken, and the owner can jailbreak their own tutor. Its supporting evidence (Zhao 2026) is weak.
   - Resolution (design inference, untested): during a graded attempt, the tutor model gets the question, the learner's attempt, and a verdict from the deterministic check tool. It also gets pre-written, pre-validated hint text for each rung. It does not get the final answer. After the attempt is closed (correct, given up, or revealed), the full solution and misconception text enter context for explanation. This keeps Bastani's ingredients without giving the model the ability to leak in the middle of an attempt. Run the leak-rate eval on the real models to see whether the stricter split is needed.
2. **Guarded tutors: gain or no gain?** Kestin (+0.63 SD and up) versus Bastani (exam null). Both are valid. Kestin measured an immediate test after a first-exposure lesson. Bastani measured an unassisted exam later. Stronger evidence for "no harm after removal" is Bastani. Stronger evidence for "can speed up first learning" is Kestin. Neither shows a delayed gain. Plan: treat the guard as a floor and get the gain evidence from the owner's own delayed tests.
3. **Voice.** D1 found equal mastery and 2.8x cost in one small preregistered trial; D4 found no outcome evidence. I use D1 for the claim and D4 only for spoken-turn rules, marked as inference.
4. **When a hint is used, how is FSRS graded?** D1: hint above rung 1, explanation or reveal gives Again or Hard only. D4: not Easy after hint 3. D5: revealed items reschedule as new. No study tested any of this. I adopt D1's strict rule because it keeps FSRS inputs to unassisted retrieval, which is what the algorithm assumes.
5. **Mastery thresholds.** D2: advance at about 80-90% on a parallel retest. D5: select items at 60-75% predicted success, and label skills with under about 10 scored items unverified. Different jobs (gate versus selection), so both stand. All numbers are assumptions.
6. **Self-explanation versus worked examples.** Self-explanation prompts have g=0.55 (verified), but D3 also reports that adding prompts to worked examples can reduce the effect (Barbieri 2023, U). Use self-explanation on proofs and derivations the learner works through, not as a mandatory layer on every worked example.
7. **Meta-analytic hype.** D1 says "reportedly" erased, D2 says "retracted." I can confirm only the Bartoš preprint. Cite no pooled AI effect.
8. **LearnLM edit rate.** See table row 6.
9. **Expertise reversal.** D2 verified Kalyuga 2003 and D4 could not open it. Lehmann's prior-knowledge gap points the same way. Treated as moderate, with a within-learner check in Evals.

## Rules for Tutor, ranked by evidence strength times impact

Tags: [P] voice/chat tutor prompt, [S] scheduling, [Q] question generation, [F] feature, [U] UI, [M] measurement, [C] corpus.

1. **No open "ask anything" mode as the main surface. Attempt first, always.** Require a learner attempt or prediction before any hint. Offer free chat only as a labeled lookup mode that never feeds mastery. [P][F][U] Evidence: strong for the harm (row 1), moderate for the benefit (rows 2, 3). Impact: highest.
2. **Headline metric is delayed, unassisted, AI-off performance on parallel items (3 days, 2-4 weeks).** Show assisted and unassisted scores separately everywhere, plus hint-dependency rate and reveal rate. Do not use chat time, satisfaction or self-rated learning as outcomes. [M][U] Evidence: rows 1, 20, strong. Impact: highest, because every other rule is tested by it.
3. **FSRS learns only from unassisted retrieval.** Hint above rung 1, an explanation, or a reveal caps the grade at Again or Hard. A revealed item reschedules as relearning. Keep FSRS (0.9) for item memory, and a separate per-skill mastery estimate for "ready to move on." [S] Evidence: design inference from row 1 (practice inflation). Impact: high, cheap.
4. **Every item ships a verified reference solution, steps and a misconception tag. Answer checking is deterministic where possible** (stored key, CAS, unit checker). An LLM grades free text only against a rubric and low-confidence grades go to the owner. [Q][P][M] Evidence: row 1 (51% base correctness, older model), rows 16, 18. Impact: high.
5. **Server-side hint ladder.** Nudge, hint, step, worked example, answer, with the maximum rung set by policy code, not the LLM. Add a deterministic leak filter on every tutor turn and log the leak rate. After about 3 failed rungs, show a worked example and immediately give an isomorphic problem. [P][F][M] Evidence: rows 1, 7; ladder details are design inference. See conflict 1 for what the model may see.
6. **Self-explanation as a required move on proofs and derivations** ("what justifies this line?"), with predict-then-reveal for results. [P][F] Evidence: rows 11, 12 (strong for g=0.55; moderate for proofs; not tested at graduate level).
7. **Feedback is task-level, specific, and correction-first.** Name the error in one sentence and require a redo. Give no praise unless the check says correct. Hold the correction under pushback. One idea and one question per turn. [P] Evidence: Wisniewski and Kluger and DeNisi (reported), Sharma on sycophancy (reported). Moderate.
8. **Confidence rating before each answer; calibration chart.** [U][M] Evidence: rows 1 and 20 (felt learning misleads). Moderate; cheap.
9. **Adapt to prior knowledge.** If prerequisites are present, pose the problem first and teach after. If they are absent, use a worked example first. Fade scaffolds as unassisted accuracy rises. Default to guided discovery. [P][S] Evidence: rows 14, 15 (moderate, mostly school-age), expertise reversal (moderate). Validate within-learner.
10. **Question-generation pipeline.** Generate from a cited source chunk. Blind-solve with a different model family and quarantine key disagreements. Take misconceptions and distractors from the owner's own wrong answers, not from the model's imagination. Do not use LLM difficulty labels. Prefer short-answer, worked and proof items at advanced level. Add a twin item and a transfer item per skill. [Q] Evidence: rows 16, 17 (moderate); key-error rate (Gema, reported).
11. **Grade proofs with multiple judge passes.** Use 3-5 passes with majority vote, step labels (valid, gap, error), an owner override, and a gold set of the owner's adjudicated answers. Track judge-versus-owner agreement per domain and stop using the judge where agreement is poor (the 0.7 floor is an assumption). [M][Q] Evidence: row 18 (moderate; competition proofs).
12. **Weekly closed-book, AI-off checkpoints per domain** on the Plan screen. [F][M] Evidence: follows from rows 1 and 20.
13. **Grounding with quote-level citations.** Show the exact span, verify programmatically that it exists in the retrieved text, retrieve few passages with the best first and last, and have the tutor say when retrieval is weak and label model-memory statements. [P][C][U] Evidence: row 19 (strong, but older, non-textbook systems); "lost in the middle" (reported).
14. **Voice.** Use it for oral explanation, teach-back and low-friction retrieval. Expect equal mastery to text and about 2.8x cost per engaged minute. Keep about two sentences and one question per turn, put equations on screen, acknowledge quickly while tools run, and allow interruption. [P][F] Evidence: row 3 (moderate-weak) for cost and equivalence; spoken-turn rules are inference.
15. **Knowledge tracing: start simple.** Use BKT or a PFA/IRT-style logistic model with hint and time logging. Adopt a deep model only if it beats the baseline on time-ordered splits of the owner's own data with no leakage. A Bayesian Rasch estimate per skill for item selection is fine, with credible intervals and no single global "mastery %." [S] Evidence: D2 (verified by D2, reported here); D5 (inference).
16. **Paper mode and Proof mode in Library.** Three passes: skeleton and claim/evidence, figures and methods, reconstruct the argument and name a weakness. Proof mode shows one line at a time. [F][C] Evidence: weak (Keshav, apprenticeship, CREATE are frameworks without effect sizes). Label as hypotheses and measure them.
17. **Evals screen.** Per-turn rubric (MRBench's 8 dimensions, LearnLM's 25 items, reported), headline answer-leak and false-praise rates, a regression set of the owner's real mistakes plus pushback prompts, LLM judge as a screen only. Run it on the actual models (Opus, Sonnet, Muse Spark) before trusting any 2023-2025 leak or sycophancy rate. [M] Evidence: rows 7, 18; moderate.
18. **Within-learner crossovers** (guarded tutor versus free chat, or AI versus AI-off blocks, by topic) scored on delayed tests. [M] Design only; with n=1 this is monitoring, not proof.
19. **Corpus: every generated item carries `source_chunk_id`; worked solutions come from the owner's notes or public-domain/CC-BY text.** [C][Q] Serves rule 4 and the project's licensing rule.

## What not to build

- Promises of "2 sigma" or "PhD in X months." Bloom's figure was bundled with mastery learning on narrow tests; modern pooled tutoring is about 0.37 SD (row 8), and I found no adult graduate tutoring trial.
- A reveal-answer button that counts as a successful review.
- Unrestricted chat as the default mode (Bastani's harmful arm).
- LLM-only grading or LLM-only solution checking in maths or chemistry. No proof grading without an owner override.
- An LLM difficulty labeler feeding the scheduler (row 17).
- Same-model generate, answer and grade. A single global mastery percentage.
- Deep-learning knowledge tracing as a core dependency.
- Designs justified by pooled "AI boosts learning" numbers or by "cognitive debt" (rows 22, 23).
- EEG or neuro-adaptive features. NeuroChat had N=24 and measured engagement only (reported).
- Andragogy-based personalization, learning-styles adaptation, a practice-hours mastery proxy, a threshold-concept detector (row 21; Pashler, Macnamara, Barradell reported).
- Engagement, streak and satisfaction as primary outcomes (rows 1, 20).
- Decorative citations, a custom fine-tuned tutor, "sounds human" as a goal, claims that Tutor replaces research apprenticeship.

## Open questions

- No adult or PhD-level RCT of LLM tutoring appears in these reports; the owner's target extrapolates from undergraduate physics, high-school maths and one MBA course.
- No delayed-retention data for the two strongest positive trials.
- Whether current frontier models still leak or flatter at 2023-2025 rates is unmeasured.
- Voice transcription errors on spoken mathematics could confound voice-graded answers.
- No ablation shows which guardrail component carries the effect.

## Citation list with verification status

V = verified as claimed, V* = verified with caveat, C = corrected, U = not verified by me.

- Bastani et al. 2025, PNAS, doi:10.1073/pnas.2422633122 (V)
- Kestin et al. 2025, Sci Rep, doi:10.1038/s41598-025-97652-6 (V)
- Yang, Van Alstyne, Dellarocas, arXiv 2609.23958 (V*)
- Lehmann, Cornelius, Sting, arXiv 2409.09047 (V*)
- Wang et al., Tutor CoPilot, arXiv 2410.03017 (V)
- LearnLM Team and Eedi, arXiv 2512.23633 (C)
- Maurya et al., arXiv 2412.09416 (V*)
- Nickow, Oreopoulos, Quan, NBER w27476 (V, study count U)
- VanLehn 2011, doi:10.1080/00461520.2011.611369 (V*)
- Kulik and Fletcher 2016, doi:10.3102/0034654315581420 (V)
- Bisra et al. 2018, doi:10.1007/s10648-018-9434-x (V)
- Hodds, Alcock, Inglis 2014, doi:10.5951/jresematheduc.45.1.0062 (V)
- Freeman et al. 2014, doi:10.1073/pnas.1319030111 (C)
- Alfieri et al. 2011, doi:10.1037/a0021017 (V)
- Sinha and Kapur 2021, doi:10.3102/00346543211019105 (V)
- Feng et al. 2024, arXiv 2404.02124 (V)
- An and Wang, arXiv 2609.27063 (V)
- Dekoninck et al., arXiv 2506.21621 (V)
- Liu, Zhang, Liang 2023, arXiv 2304.09848 (V)
- Deslauriers et al. 2019, doi:10.1073/pnas.1821936116 (V)
- Rachal 2002, doi:10.1177/0741713602052003004 (V)
- Bartoš, Martinková, Wagenmakers 2025, doi:10.31234/osf.io/8vs32_v1 (V)
- Kosmyna et al. 2025, arXiv 2506.08872 (V, contested)
- Reported only (U): Henkel 2024 (arXiv 2402.09809); World Bank WP 11125; Nie et al. (2407.09975); Chung et al. (2608.16907); Pisan (2608.12292); Wisniewski 2019; Kluger and DeNisi 1996; Chi 2001; Chi and Wylie 2014; Kalyuga 2003; Khajah (1604.02416); pyKT (2206.11460); Jurenka (2407.12687); LearnLM arena (2505.24477); Bridge (2310.10648); Wallat (2412.18004); Magesh (2405.20362); Gema (2406.04127); Shimmei (2505.05815); Zheng (2306.05685); Panickssery (2404.13076); Hofmann (2509.11106); Macnamara 2014; Barbieri 2023; Wang and Fan 2025 retraction claim.


---

# FILE: docs/tutor/research/intl.md

# Pedagogy of top-performing school systems: synthesis for Tutor

Verified 2026-09-30. Reports C1 to C8 read in full; about 30 citations checked against Crossref, OpenAlex, Semantic Scholar, ERIC, Europe PMC, NBER, Education Next, the NCES PDF and Wikipedia.

## 1. Bottom line

1. The data do not show that East Asian or Nordic classroom pedagogy caused their scores. Culture, selection, shadow education and sampling are confounded with method. Jerrim (2015) finds second-generation East Asian children in Australia outscoring natives by about 100 PISA points. Exported "mastery" programmes gave about +0.06 to +0.15 SD in children.
2. The transferable part is mechanism, not institution. The mechanisms with experimental support (case comparison, interleaving, expertise-faded worked examples, problem-first for learners with prior knowledge, informative feedback, retrieval and spacing) come from trials run in many countries, so "take little from the US" cannot be applied cleanly.
3. Nothing reviewed tested PhD-level or self-directed adult learners. Every rule below is a design hypothesis. The one adult RCT (Kestin 2025) is one lesson at one university.
4. Realistic target: about 0.2 to 0.4 SD on broad delayed measures. Tutoring meta-analyses give 0.37 SD for preK-12 (Nickow 2020), and most education RCTs give 0.06 SD (Lortie-Forgues and Inglis 2019). Bloom's "2 sigma" is not a target.
5. Finland declined, Estonia did well, and Denmark, Sweden, Norway and Iceland are mid-table. "Nordic method" is not a valid label.

## 2. Verification result

Of 29 load-bearing items checked, 24 are verified as stated, 4 corrected and 1 (PISA figures) could not be verified. Search quota was exhausted, so I used abstract APIs.

Corrections and flags:

- **von Hippel 2024**: C8 attributed "0.84 SD narrow vs 0.27 SD broad" to von Hippel. It is from Cohen, Kulik and Kulik 1982, cited by him. His own point: mastery learning (feedback plus testing, no tutoring) scored about 1.1 SD above whole-class, roughly half of Bloom's 2 sigma.
- **Barbieri 2023** (worked examples, g = 0.48): the abstract says self-explanation prompts moderated the effect negatively. This contradicts the C5 and C8 rule "require self-explanation". See section 5.
- **Karaman 2021**: Turkish studies only. C6 used it to argue learner-initiated feedback beats tutor-initiated. The abstract gives student-initiated 1.16, mixed 0.83, adult-initiated 0.69, computer-initiated 0.42. The study cannot carry a causal design rule.
- **Kestin 2025**: C8 left the effect size unverified. Full text gives N = 194 analysed, effect 0.63 SD (linear, underestimated by ceiling) and 0.73 to 1.3 SD (quantile). One 60-minute lesson, immediate post-test, one Harvard physics course. No delayed test.
- **PISA figures**: not verifiable. OECD returned 403. My Wikipedia extraction put Finland at 469 and Singapore at 563 in a "2022" row, which matches C6's "provisional 2025" figures and disagrees with my recall of official 2022 values (Singapore about 575, Finland about 484). Treat every PISA number in C1, C5, C6 and C7 as unverified. TIMSS 2023 grade 8 maths (Singapore 605, Taipei 602, Korea 596, Japan 595, Sweden 517, Finland 504) was confirmed by two independent Wikipedia extractions (C1 and mine). It is secondary but consistent.
- **Leong 2011** (Polya "look back", C4): unconfirmed; my DOI attempt resolved to a different paper.

## 3. Verified evidence table

| Claim | Source (status) | Figure | Strength, applies to |
|---|---|---|---|
| Unguarded LLM access harms later unaided performance | Bastani 2025 PNAS (V) | about 1,000 high-school students. Practice +48% (Base), +127% (Tutor). Later exam -17% (Base), mostly mitigated by Tutor | Moderate. One field experiment, teenagers, short horizon |
| AI tutor beat in-class active learning | Kestin 2025 Sci Rep (V) | 0.63 SD linear, 0.73 to 1.3 SD quantile, N = 194 | Moderate to weak: one lesson, immediate test. Adults, physics |
| Case comparison aids learning | Alfieri 2013 (V) | d = 0.50 [0.44, 0.56], 57 experiments, 336 tests. Larger when learners find similarities and the principle comes after comparison | Strong. Mostly short studies. Needs prior knowledge (Rittle-Johnson 2009, abstract not re-read) |
| Interleaved maths practice | Rohrer 2020 (V) | 61% vs 38% at one month, d = 0.83, 54 grade-7 classes, preregistered | Moderate to strong. Grade 7 only. Both groups took an interleaved review first |
| Problem-first then instruction | Sinha and Kapur 2021 (V) | g = 0.36 [0.20, 0.51]. Reversed for grades 2 to 5 and domain-general skills. Bias-adjusted 0.87 | Moderate, contested. Do not cite 0.87 |
| Worked examples, maths | Barbieri 2023 (V) | g = 0.48. Self-explanation prompts: negative moderator | Moderate. Grades K to postsecondary |
| Expertise reversal | Kalyuga 2003 (V); Tetzlaff 2025 (exists, numbers not read) | Interaction effect, no single ES | Strong as principle. Short studies, induced expertise |
| Unassisted discovery loses, guided wins | Alfieri 2011 (V) | d = -0.38 (unassisted vs explicit), d = +0.30 [0.23, 0.36] (enhanced discovery), 164 studies | Moderate to strong. Mostly school age |
| Formative assessment is about 0.2 SD, not 0.4 to 0.7 | Kingston and Nash 2011 (V) | Weighted 0.20, median 0.25. Only 13 of over 300 studies usable. ELA 0.32, maths 0.17, science 0.09. Computer-based 0.28 | Moderate. K-12 |
| Feedback content matters | Wisniewski 2020 (V) | d = 0.48, 435 studies, strongly heterogeneous | Moderate. Information content is the moderator |
| Tutoring meta-analysis | Nickow 2020 (V) | 0.37 SD pooled, preK-12 RCTs | Strong for school age |
| Bloom's 2 sigma is narrow-test inflated | von Hippel 2024 (V, corrected) | 0.84 narrow vs 0.27 broad (Cohen 1982) | Moderate |
| Singapore-style curriculum, US | Jaciw 2016 (V) | 0.11 to 0.15 SD, 12 schools, grades 3 to 5 | Moderate, small. Curriculum bundle |
| Singapore-inspired mastery, England | Jerrim and Vignoles 2016 (V for "modest positive"; numbers from C2's opened PDF) | +0.10 primary, +0.06 secondary (n.s.), pooled about +0.077 | Moderate, small |
| Word-problem interventions, LD/MD | Lein 2020 (V) | g = 0.56 [0.40, 0.72], larger at elementary than secondary, larger with researcher measures | Moderate. Not typical learners |
| Japanese lessons, TIMSS video | Hiebert 2003, NCES 2003-013 (V, PDF text) | Japan: 54% connection problem statements, 27% problems summarized. Hong Kong: 84% procedure statements. Authors: "no direct inferences" to achievement | Descriptive. 1995 sample, 50 schools |
| Lesson study has no pupil effect at scale | Murphy 2017 EEF (V design; null from C3's opened PDF) | 181 schools, 12,747 pupils, MDES 0.108, null | Strong null. It was the Edge Hill "Talk for Learning" variant |
| Typical education RCT effect | Lortie-Forgues and Inglis 2019 (V) | 0.06 SD mean, CI width 0.30, 141 RCTs | Strong calibration |
| Paper vs screen | Delgado 2018 (V) | g = -0.21, 54 studies, 171,055 participants. Larger under time pressure and for informational text. Null for narrative | Moderate. Trend disputed (Kong 2018, g about -0.25 in Clinton 2019). Not tested on long PhD-level reading |
| Finland's decline partly explained by boys' gap and SES gradient | Ahonen 2020 (V abstract) | "explain it only partly" | Moderate. Cause contested |
| Wellbeing cost | Choi 2026, Jeong 2026, Hu 2026, Zhu 2026 (exist, per Crossref; findings from C5 abstracts not re-read) | Ecological or cross-sectional | Weak to moderate for causation |

## 4. Rules for Tutor, ranked by evidence times impact

Tags: [P] voice/chat prompt, [S] scheduling, [Q] question generation, [F] feature, [U] UI, [M] measurement, [C] corpus.

1. **Attempt first, hint ladder, never hand over the solution on the first request.** Answers and worked solutions stay hidden until the learner commits an answer. Evals run AI-off and closed-book. (Bastani; Kestin's tutor also withheld answers.) [P][U][M] Strength moderate, impact highest.
2. **FSRS at 0.9 stays the retention engine. Add cumulative mixed review after initial learning (start about 20% of reviews) and measure at 7 and 30 days.** Japanese lessons supply no spaced retrieval, so FSRS covers what they lack. (Rohrer; Dunlosky exists, not re-read.) Keep 0.9 as a tunable per-domain default. No source compared 0.9 with other targets. [S][M] Strength moderate to strong, impact high.
3. **Route each concept by prior knowledge.** A 1 to 3 question ungraded diagnostic picks the mode: worked and faded examples for novices, guided problems for intermediates, problem-first then consolidation for advanced. Fade per skill, not per person. Log the mode. (Kalyuga; Sinha and Kapur; Barbieri.) [P][U Plan][M] Strength moderate, impact high. The adult-specific evidence is missing.
4. **Comparison before rule.** Show 2 to 3 varied cases, including a non-example and a flawed solution. Ask what changed, what stayed, and why, then state the rule. Use only when the prior-knowledge check passes; otherwise study cases sequentially first. (Alfieri 2013; Rittle-Johnson.) [P][Q][F Compare card] Strength strong, impact high.
5. **Feedback names the misconception and the next step.** Ask for a 0 to 100 confidence rating before the reveal and show a calibration chart and misconception counts, not a single score. (Wisniewski; Kingston; calibration rationale from screen-reading studies is weaker.) [P][U][M] Strength moderate, impact high.
6. **Question bank as variant families.** Fields: `variation_dimension`, `invariant`, `near_miss_of`, `representation_type`, `relation_type`, `scaffold_level`, item type (recall, near transfer, far transfer, procedural vs application), and an optional 3-level `hint_ladder`. Variants of one seed must not each count as independent FSRS mastery evidence. An LLM-generated variant needs an independent solver pass before it enters the deck. [Q][S] Strength design-derived (Alfieri, Rohrer, Paas), impact high because it protects correctness.
7. **Measure what matters.** Delayed retention, transfer to unseen variants and hint dependence, not minutes, streaks or hours. Run within-learner crossovers on matched topics (explanation-first vs problem-first, paper vs screen, variation-rich vs plain). Treat differences under about 0.2 SD as noise and report intervals. Include an adult-baseline numeracy and literacy track. [M] Strength strong (Lortie-Forgues), impact high.
8. **Load cap and recovery.** Configurable daily cap on new items and reviews, soft stop after about 90 minutes, late-night quiet period, optional weekly wellbeing check-in (sleep, anxiety) used only to adjust load. Korean intensity and sleep data are ecological, so this rests on precaution. [S][U] Strength weak to moderate, cost very low.
9. **One rich problem per unit, then a learner-written summary before the tutor's.** Turn each summary and comparison insight into an FSRS card. Do not use this as the only mode; retrieval and proof-reading need other modes. (TIMSS video is descriptive only; Hong Kong shows it is not necessary.) [P][Q][S] Strength weak, impact medium.
10. **Structure-first representation with adaptive fading.** Diagram or structural sketch before symbols for new structures (part-whole, comparison, change, or a domain analogue such as free-body or reaction-coordinate diagrams). Drop it after 2 to 3 correct unaided attempts and restore it when errors look structural. Evidence is children and learners with difficulties (Lein 2020 g = 0.56; Carbonneau 2013, V-abs, manipulatives help retention more than transfer). Build a drawable canvas only after rules 1 to 7. [P][F] Strength weak for adults, impact medium.
11. **Paper path.** Print-and-read export with a closed-book quiz a day later, self-paced by default, screen reading left available. The average gap is about 0.2 SD and its trend is disputed, so run the crossover (rule 7) before making a claim to the owner. [F][M] Strength small, moderate.
12. **Guided inquiry only.** If a learner is stuck, escalate hints to a worked step. Never leave them in pure discovery. (Alfieri 2011.) [P] Strength moderate to strong, largely covered by rules 1 and 3.
13. **Corpus.** Store citable notes (title, licence, link, own summary), not full reports, for PISA, TIMSS, PIRLS and PIAAC material. No number enters the corpus until a worker verifies it against the OECD or IEA table. Nothing in these reports verifies that Chinese textbooks or papers are openly licensed, so add each through `corpus/manifest.json` with an explicit licence, and do not bulk-ingest. [C]
14. **Solo "lesson study" loop.** Log stalls, hint depth and surfaced misconceptions; revise prompts and item probes weekly. Content QA only. The EEF null says the ritual alone does not raise outcomes. [M][Q] Weak, cheap.
15. **Optional, untested.** A session "board" note saved to the Library, a Danish-style oral-defence voice mode (format unverified), a primer mode before new chapters, a notification-muting focus mode.

## 5. Conflicts and how I resolved them

- **Self-explanation.** C5 and C8 say require it. Barbieri 2023 found prompts lowered the worked-example effect in maths. Barbieri is a meta-analytic moderator, C5 and C8 rely on general principle. Resolution: do not mandate it on every step. Ask "explain why that step works" after an error, at a mode change, or on a transfer item, and test with a crossover.
- **Problem-first vs explicit instruction.** C3 and C8 favour problem-first for learners with prior knowledge; Alfieri 2011 and Rittle-Johnson show novices need guidance; Sinha and Kapur reverse for younger learners. All agree prior knowledge decides, so the diagnostic (rule 3) routes it. The 0.87 bias-adjusted figure is out.
- **Mastery gates.** C2 says no hard gates. C4 wants prerequisite gating. C8 wants mastery with delayed checks. Kulik 1990 reports self-paced college programmes sometimes cut completion (from C8, not re-verified). Resolution: soft gates with a skip, a cold-recall basics check, prerequisite stability before scheduling new items, and at most 2 to 3 new skills per session.
- **Learner-initiated feedback.** C6's claim rests on Turkish K-12 data where adult-initiated feedback (0.69) also beat computer-initiated (0.42). Downgraded: offer hints on request and run the ladder, and test tutor-initiated vs learner-initiated in the crossover. Do not cite Karaman as design evidence.
- **Formative assessment magnitude.** B&W 0.4 to 0.7 against Kingston and Nash 0.20. Kingston's abstract confirms the lower figure and shows only 13 usable studies. Use 0.2 and do not treat either as precise.
- **Practice volume.** C1, C5 and C8 agree hours are a weak lever. This does not conflict with spacing: Macnamara's 4% (V-abs) concerns deliberate-practice hours, not retrieval. C8's 80% first-attempt success target and FSRS 0.9 retrievability are different quantities.
- **Screen vs paper.** Delgado says the gap grows, Kong says it shrinks, and neither tests long PhD-level reading. Kept as a measured option.
- **PISA values differ across C1, C5, C6, C7.** No report has an official table. All set aside. TIMSS 2023 and PIAAC ranks are used only as qualitative context.

## 6. What not to build

- A "Chinese, Japanese, Singapore, Finnish or Nordic method" mode or "proven" copy. The mastery RCTs show +0.06 to +0.15 SD, the Shanghai exchange showed no attainment effect, lesson study showed no pupil effect at scale, and phenomenon-based learning has no controlled evidence.
- Country rankings, leaderboards, percentile competition, streak-shame gamification or norms from B-S-J-Z data (sampling disputed; mainland China absent from PISA 2022).
- Hours-driven drilling, hagwon-style volume targets, Suneung-style timed multiple choice as the main assessment.
- Hard mastery gates, a fixed four-step script for every interaction, a discovery-only default or an always-on productive-failure mode.
- Streaming or exam-gate simulation, a MEXT engine, a teacher network.
- Physical or virtual manipulative modules, a fixed CPA sequence for every topic, or a standalone metacognition module.
- A tutor that hands out solutions on request, or a score-free experience with no calibration signal.
- "2 sigma" or "0.4 to 0.7 formative assessment" claims anywhere in the app. Hard-coded PISA, KI or SEK figures. "Digital native" assumptions. A screen-only or paper-only mandate.
- Scaling AI features ahead of evidence (Korea's 2025 AI-textbook downgrade: headlines only, reasons unverified).

## 7. Citation list and verification status

V = existence and headline figure checked this session. V-abs = checked at abstract level. E = exists (Crossref or OpenAlex), findings not re-read. U = unverified. C = corrected.

- Bastani et al. 2025, PNAS, 10.1073/pnas.2422633122: V-abs
- Kestin et al. 2025, Sci Rep, 10.1038/s41598-025-97652-6: V (full text, Europe PMC)
- Alfieri, Nokes-Malach, Schunn 2013, Educ Psychol, 10.1080/00461520.2013.775712: V-abs
- Alfieri, Brooks, Aldrich, Tenenbaum 2011, J Educ Psychol, 10.1037/a0021017: V-abs (abstract prints CI garbled)
- Rohrer et al. 2020, J Educ Psychol, 10.1037/edu0000367: V-abs
- Sinha and Kapur 2021, Rev Educ Res, 10.3102/00346543211019105: V-abs
- Barbieri et al. 2023, Educ Psychol Rev, 10.1007/s10648-023-09745-1: V-abs, C (self-explanation negative)
- Kalyuga, Ayres, Chandler, Sweller 2003, Educ Psychol, 10.1207/s15326985ep3801_4: V-abs
- Tetzlaff et al. 2025, Learn Instr, 10.1016/j.learninstruc.2025.102142: E
- Jerrim and Vignoles 2016, Econ Educ Rev, 10.1016/j.econedurev.2015.11.003: V-abs ("modest, positive"); SD figures U (C2 preprint)
- Jerrim 2015, Oxf Rev Educ, 10.1080/03054985.2015.1028525: V-abs
- Jaciw et al. 2016, JREE, 10.1080/19345747.2016.1164777: V-abs
- Lein, Jitendra, Harwell 2020, J Educ Psychol, 10.1037/edu0000453: V-abs
- Peng et al. 2025, Educ Psychol Rev, 10.1007/s10648-025-10057-9: E (title: network meta-analysis)
- Hiebert et al. 2003, NCES 2003-013: V (PDF text)
- Murphy, Weinhardt, Wyness 2017, EEF, ERIC ED581145: V (design); null result from C3
- Lewis and Perry 2017, JRME, 10.5951/jresematheduc.48.3.0261: V-abs (no effect sizes)
- Lortie-Forgues and Inglis 2019, Educ Res, 10.3102/0013189X19832850: V-abs
- Kingston and Nash 2011, EM:IP, 10.1111/j.1745-3992.2011.00220.x: V-abs
- Karaman 2021, IJATE, 10.21449/ijate.870300: V-abs, C (Turkey only)
- Wisniewski, Zierer, Hattie 2020, Front Psychol, 10.3389/fpsyg.2019.03087: V-abs
- VanLehn 2011, Educ Psychol, 10.1080/00461520.2011.611369: V-abs
- Nickow, Oreopoulos, Quan 2020, NBER w27476: V-abs
- von Hippel 2024, Education Next: V, C (attribution)
- Delgado et al. 2018, Educ Res Rev, 10.1016/j.edurev.2018.09.003: V-abs
- Clinton 2019, J Res Reading, 10.1111/1467-9817.12269: E
- Kessel, Hardardottir, Tyrefors 2020, Econ Educ Rev, 10.1016/j.econedurev.2020.102009: V-abs
- Abrahamsson 2026, J Hum Resour, 10.3368/jhr.0224-13403r2: E
- Dunlosky et al. 2013, PSPI, 10.1177/1529100612453266: E
- Ahonen 2020, 10.1007/978-3-030-59031-4_6: V-abs
- Choi et al. 2026 (Lancet Reg Health West Pac, 10.1016/j.lanwpc.2026.101924); Jeong et al. 2026 (Medicine, 10.1097/md.0000000000050516); Hu, Lin, Tang 2026 (Front Psychol, 10.3389/fpsyg.2026.1891921); Zhu, Liu, Jia 2026 (Front Psychol, 10.3389/fpsyg.2026.1795875): E
- TIMSS 2023 maths (Wikipedia, two extractions agree): V-secondary
- PISA 2022 and 2025 figures, PIAAC Cycle 2 figures, Loveless 2013 sample counts, hagwon and juku participation: U
- Leong et al. 2011 (Polya look-back): U (DOI mismatch)
- Not re-checked (taken from reports): Gu et al. 2004, Kullberg 2017, Marton, Stigler and Hiebert 1999, Ma 1999, Rosenshine 2012, Black and Wiliam 1998, Kulik 1990, Slavin 1987.

## 8. Before building

1. Have a worker pull PISA 2022 and 2025, TIMSS 2023, PIRLS 2021 and PIAAC Cycle 2 tables from OECD or IEA sources before any figure enters the corpus or UI.
2. Read full texts for Tetzlaff 2025 and the EEF Mathematics Mastery reports.
3. Audit LLM-generated variant families for invariant preservation on the owner's subjects before scaling question generation (rule 6).


---

# FILE: docs/tutor/research/languages.md

# Foreign-language learning with ElevenLabs: verified synthesis for Tutor

Date: 2026-09-30. Inputs: reports L1 to L5. Method: I re-checked 26 load-bearing items this session. Bibliographic records and abstracts came from the Crossref API (and OUP for one abstract). Product facts came from ElevenLabs and Microsoft Learn pages fetched through a summarising tool (second-hand, so quoted figures should be re-read on the live page before a budget or build decision). Web search was exhausted, so nothing was discovered beyond the reports. Status words: **Verified** (exists, says what is claimed), **Corrected** (exists, a claim in a report was wrong or needed a qualifier), **Failed/unverified** (could not be confirmed; claims resting on it are dropped or flagged).

## 1. Verified table

| # | Item | Status | What I saw / what changed |
|---|---|---|---|
| 1 | Kim & Webb 2022, Language Learning, doi:10.1111/lang.12479 | Verified | 48 experiments, 98 effect sizes, N = 3,411. Medium-to-large spacing effect. Shorter gaps equal longer gaps on immediate tests, worse on delayed. Equal = expanding. |
| 2 | Lyster & Saito 2010, SSLA, doi:10.1017/S0272263109990520 | Verified | 15 classroom studies, N = 827. CF effects significant and durable. Larger for prompts than recasts, and on free-response measures. Younger learners benefit more than older. |
| 3 | Li 2010, Language Learning, doi:10.1111/j.1467-9922.2010.00561.x | Verified | 33 studies, medium overall effect, maintained over time. Lab > classroom. |
| 4 | Kang, Sok & Han (online 2018, LTR), doi:10.1177/1362168818776671 | Verified | 54 studies, 5,051 learners, g = 1.06 (CI 0.84-1.29). Minor explicit/implicit difference; moderated by outcome mode, proficiency, setting, intensity. |
| 5 | HVPT meta-analysis, SSLA 2025, doi:10.1017/s0272263125100879 | Verified, authors added | Uchihara, Karas & Thomson. 79 studies. g = 0.92 pre-post (k = 96), g = 0.67 vs control (k = 32). Retention confirmed; generalisation to novel stimuli "to some extent". |
| 6 | Sakai & Moorman, Applied Psycholinguistics (online 2017), doi:10.1017/s0142716417000418 | Verified | Perception d = 0.92, "medium-sized"; production d = 0.54, "small". Perception-production gain correlation small-to-medium and not significant. Obstruents gain more. |
| 7 | Lee, Jang & Plonsky 2015, Applied Linguistics, doi:10.1093/applin/amu040 | Verified | 86 reports; d = 0.89 within, 0.80 between. Larger with longer duration, feedback, more controlled outcome measures. |
| 8 | Bastani et al., PNAS 2025, doi:10.1073/pnas.2422633122 | Verified; correction unread | ~1,000 high-school math students. GPT Base +48% practice grades, then -17% unassisted; GPT Tutor +127% practice, harm largely mitigated. A correction (doi:10.1073/pnas.2518204122) exists; PNAS returned 403, so its content is unknown. Quote the numbers only with that caveat. |
| 9 | Li, Wang & Yang 2025, JCAL, doi:10.1111/jcal.70060 | Verified | 41 studies, 48 effect sizes, N = 3,515, ES = 0.576 (CI 0.385-0.768). Largest moderators: vocabulary and 1-7 day interventions. Not speaking-specific. |
| 10 | Hartshorne, Tenenbaum & Pinker 2018, Cognition, doi:10.1016/j.cognition.2018.04.007 | Corrected | n = 669,498; grammar-learning ability preserved to about 17.4 then "declines steadily". The authors frame it as a sharply defined critical period with a late offset. L1's "gradual" and "says nothing against adult learning" were softer than the abstract. Correct use: adults should expect to rely on explicit learning, not child-like intuition; it does not show adults cannot reach high proficiency. |
| 11 | Nakata 2016, SSLA, doi:10.1017/s0272263116000280 | Corrected | 98 Japanese learners, 16 word pairs. Five and seven retrievals gave higher scores than one and three at every posttest; only when time-on-task was controlled was one retrieval best. So "cap repeats" is a time-efficiency argument from one study, not a finding that repeats hurt. |
| 12 | Sutton & Webb 2026, SSLA, doi:10.1017/s0272263126101612 | Verified | 75 effect sizes, 56 experiments, n = 1,954, within-group g = .89 (the abstract itself calls it "small"; within-group values are inflated). Entertainment video yields lower effects than educational video. |
| 13 | Jeon & Day 2016, RFL, doi:10.64152/10125/66901 | Verified | 71 samples, 49 studies, 5,919 participants. Small-to-medium. Adults > children; EFL > ESL settings. |
| 14 | Schmitt, Jiang & Grabe 2011, MLJ, doi:10.1111/j.1540-4781.2011.01146.x | Verified | 661 participants, 8 countries. Roughly linear coverage-comprehension relation, no threshold; 98% the more reasonable target for academic texts. English only. |
| 15 | Alghamdi & Alghizzi 2026, Front. Res. Metrics Analytics, doi:10.3389/frma.2026.1908592 | Corrected | It is an LLM-assessment validity review (52 studies). g = 0.42 (CI 0.27-0.57, I2 = 69.8%) is for LLM formative feedback on revision quality and short-term writing outcomes across 14 studies, not L2 speaking or long-run proficiency. Human-LLM score agreement r = 0.66 comes from only 3 L2 studies. The paper warns of cognitive offloading. |
| 16 | Wu & Yu 2023, BJET, doi:10.1111/bjet.13334 | Verified | 24 randomized studies, "large" effect, short interventions stronger; authors themselves attribute this to novelty. Not language-specific. |
| 17 | Simsek et al. 2026, Front. AI, doi:10.3389/frai.2026.1806700 | Verified, distrust | g = 1.309, 18 studies, 1,823 participants. Treat as implausible. |
| 18 | Teimouri, Goetz & Plonsky 2019, SSLA, doi:10.1017/s0272263118000311 | Verified | r = -.36, 97 reports, 105 samples, N = 19,933. |
| 19 | Jiang et al. 2021 (FLA) and 2024 (CALICO), Duolingo | Verified; vendor link confirmed | Crossref affiliations: Duolingo for 4 of 5 authors (2021) and 3 of 4 (2024). No RCT; receptive skills only (2021: n = 225). |
| 20 | ElevenLabs models page | Corrected | v4 and v4 Turbo 90+ languages (v4 10,000-char limit, Turbo ~100 ms); v3 70+ (5,000 chars); v3 Conversational ~280 ms; Multilingual v2 29 languages (10,000); Flash v2.5 32 languages (40,000, ~75 ms); Flash v2 English only. `turbo` IDs are aliases of Flash. No pricing on that page. |
| 21 | ElevenLabs pronunciation-dictionary page | Verified | Phoneme (IPA/CMU) rules work with eleven_v4, eleven_flash_v2, eleven_v3; other models use alias rules only. "For IPA and CMU in languages other than English, switch to eleven_v4." The separate best-practices page says inline phoneme tags work only on eleven_flash_v2. |
| 22 | ElevenLabs agent retention page | Verified; one claim dropped | Default 2 years; settable to N days, -1 (unlimited), 0 (scheduled deletion). The page never mentions Zero Retention Mode or plan limits, so "Enterprise only" (L3) is unverified. |
| 23 | ElevenLabs Scribe v2 language tiers | Corrected | 90+ languages. Tiers: 34 languages at 5% WER or lower (includes English, Spanish, French, German, Portuguese, Russian, Italian, Japanese, Turkish, Greek, Polish, Dutch); 20 at 5-10% (Hindi, Mandarin, Cantonese, Persian, Swahili); 18 at 10-20% (Arabic, Korean, Thai, Hebrew); 19 at 25-50%. L3's "36" is wrong; L4 wrongly put Mandarin in the top tier and Arabic and Korean at 5-10%. Word-level timestamps confirmed. |
| 24 | ElevenLabs language-detection system tool | Verified, contradicts L3 | The tool lets an agent "switch its output language" mid-conversation; `only_at_conversation_start` limits switching to the first two user turns; docs warn an unusual accent can trigger an unintended switch. The "language settings" page now returns 404, so "fixed per call" and "31 languages" cannot be re-checked. |
| 25 | ElevenLabs API pricing | Verified | Per 1k characters: v4 $0.022 (72% discount through Oct 12; list $0.08 per L3), v3 $0.08, Multilingual v2 $0.08, Flash/Turbo $0.04. Scribe v2 $0.22/h, Realtime $0.39/h. "Speech Engine" $0.08/min beyond included minutes; all-in agent cost unknown. |
| 26 | Azure Pronunciation Assessment (Learn pages updated 2026) | Verified | Prosody (and content) en-US only; prosody is a paid add-on above baseline STT price; audio over 30 s needs continuous mode without miscue; Microsoft reports Pearson > 0.5 with human judges "in our evaluations" (internal, no dataset); advises 16 kHz audio, one speaker, repeat prompts below threshold. Locale count and data-retention statement not re-checked. |
| 27 | FSI difficulty tiers and hours | Failed | state.gov 403; Wikipedia gives only "8-44 weeks". Four-vs-five-tier conflict unresolved. All hour figures dropped except as order-of-magnitude. |

Not re-verified (from reports, flagged where used): see section 6.

## 2. Recommendations, ranked by evidence strength x impact

Evidence grade: A = replicated meta-analysis or field experiment read this session; B = meta-analysis in a nearby population or single strong study; C = design reasoning or untested.

1. **[prompt] Attempt first, guarded hints, no answer-dumping (A/B, high impact).** Require a target-language attempt before any hint or gloss; never hand over the translation first. Supported by Bastani (verified; math, not language) and the LLM-feedback synthesis warning about cognitive offloading (item 15). Already in PEDAGOGY.md; make it a hard rule in language mode.
2. **[measurement] Monthly no-AI production hold-out (B, high).** Speak or write unaided from a fixed prompt, scored on a fixed rubric (accuracy, complexity, fluency as words per minute and pauses, intelligibility). This is the direct test for the crutch effect and is separate from card accuracy. Report within-learner change, never literature d benchmarks (effects in this field are inflated; items 12, 7).
3. **[scheduling] FSRS with spaced retrieval, separate card states per skill (A for spacing, C for the split).** Keep FSRS (item 1; expanding = equal, so algorithm details matter little; do not compress gaps). Keep recognition, production and listening as separate cards per word; this split is a design choice, not a cited result. Start desired retention near 0.9 and expose it. Do not credit a spoken card completed with a hint or gloss.
4. **[question generation] Production formats by default (B).** L1-to-L2 translation, sentence cloze, dictation, rubric free response; every item is a full sentence with audio and source provenance (source, licence, URL, ID), or carries an "unverified" flag. Second-pass validation before an item enters FSRS (wrong cards are costly once scheduled); require Tatoeba or Wiktionary grounding for less-resourced languages.
5. **[feature] Perception-first minimal-pair trainer (A for perception, B for production).** HVPT g = 0.92 pre-post, 0.67 vs control (item 5); transfer to production d = 0.54 is smaller and not tied to perception gain (item 6), so train and test production separately. Multi-talker, real words, immediate feedback, held-out talkers and words at test; consider a single-talker start for very hard contrasts (weak, single study). Evidence used natural talkers; ElevenLabs voices are an untested substitute, so native-check them and label untested. One n=1 test: 3 voices vs 1 voice on one contrast.
6. **[prompt] Feedback order: prompts before correction (B).** Clarification request, repetition, metalinguistic clue, elicitation, then explicit correction (item 2). Recast only off-target errors or for comprehension-breaking slips. Evidence is classroom-based and favours younger learners, so run an n=1 alternating-weeks comparison on delayed error recurrence.
7. **[prompt] Brief explicit rule in English, then production (B/A for instruction, moderate for explicit-over-implicit).** Kang et al. (g = 1.06) show instruction works; the explicit/implicit difference is minor. So keep rules short and move quickly to output.
8. **[feature] Fluency mode vs accuracy mode (B for delayed CF, C for the rest).** Fluency mode: no interruptions, at most 1-3 errors in an end-of-session digest, each converted to production cards. Accuracy mode: the prompt ladder. One or two corrections per turn at most. This reconciles L1 (delay) with L5 (recast in fluency).
9. **[feature] Comprehensible-input lane with coverage meter (B).** Extensive reading is small-to-medium (item 13); audiovisual input gains are within-group and entertainment video underperforms educational (item 12). Compute known-word coverage per text from the learner's own FSRS state. Target 95-98% for reading (98% better for academic text, linear relation, English-only: item 14) and about 95% for graded audio (van Zeeland & Schmitt, unverified here). Add a short comprehension check after each text; new words enter FSRS from context sentences, but count reading as extra exposure, not the main source of new cards.
10. **[prompt/UI] Honest speech claims (A for the limits).** The agent sees ASR text, not acoustics: say "the recogniser heard X", never "your accent is good". Show the raw transcript beside any corrected form. Scribe is a machine-listener intelligibility probe (transcript match), not a pronunciation scorer. For scripted reading drills in supported locales, Azure gives phoneme and word accuracy (item 26); show phoneme colouring, repeat-below-threshold prompts, and "unavailable for this language" honestly; no single 0-100 headline score.
11. **[feature] Per-language ElevenLabs config and QA (C, high impact).** Table `{lang, variety, tts_model, voice_id, speed, dictionary_id, stt_lang}`. v4 for pre-generated drill and story audio (only v4 is documented for non-English IPA: item 21); Multilingual v2 for long narrative with numbers; Flash v2.5 or v3 Conversational inside live agents only after testing which model an agent really uses. Gate each language on a 50-sentence native-checked gold set and show an "audio unverified" badge until it passes. Cheap round trip: TTS, Scribe, diff against source text; catches gross errors only.
12. **[feature/voice] Live role-play agent (C).** One agent per target language; scenario via dynamic variables; learner-level prompt; target vocabulary from the corpus. Pin the language. Do not enable the language-detection tool (accents can trigger a switch: item 24). L1 help is an on-screen client tool (gloss, translate), not a spoken switch. Log transcripts locally; set retention explicitly (item 22) and audio storage off.
13. **[measurement] Level claims (C).** Show per-skill CEFR-style estimates as bands labelled "Tutor estimate, not an official level", re-estimated from attempt logs and periodically checked against an external test (DIALANG status unverified, or a practice exam). Never infer a level from card accuracy or chat alone. Log correctness by skill, item source, frequency band; retention per band. Delayed retests at 7 and 30 days on new sentences and new talkers.
14. **[corpus] Grounded, licence-tracked sources (B for need, licences second-hand).** Per language: Tatoeba (filtered by rating), Kaikki/Wiktionary, UD treebanks (drop NC if ever distributed), wordfreq/FrequencyWords as metadata only, Wikisource/Gutenberg for reading, plus the owner's own notes (L1-English interference errors, contrast notes). Record licence, URL and retrieval date in `corpus/manifest.json`. No copyrighted textbooks, subtitle text or commercial graded readers. Open A2-C1 graded readers were not found, so generate graded text and validate coverage.
15. **[feature] Onboarding placement (C).** About 10 minutes: yes/no vocabulary with pseudowords by frequency band, a short read-and-answer block, a 3-5 minute voice interview. Output a band; LexTALE-style validity unverified.
16. **[UI] Time-cost panel (C, optional).** Ranges only, labelled rough and unverified (item 27).

Language order (design judgement from L4, corrected): Spanish, French, German, Italian first (top Scribe tier, deepest open data). Russian and Japanese are top-tier STT but cost more hours. Mandarin sits at 5-10% WER; Arabic, Korean, Hebrew, Thai at 10-20%. No pronunciation-scoring API found for Latin, Greek or Hebrew.

## 3. Conflicts and resolutions

| Conflict | Resolution |
|---|---|
| Is agent language fixed per call (L3) or switchable (L5)? | Both are in the docs. A `language_detection` system tool can switch mid-call (verified); it is opt-in, carries an accent warning, and can be limited to conversation start. The "fixed" page is now a 404. Rule: pin the language and leave the tool off; test actual behaviour before designing bilingual flows. |
| Which models take phoneme/IPA rules? | Dictionary page: v4, v3, flash_v2 (verified). Best-practices page: inline phoneme tags flash_v2 only. Different features, likely different pages' ages. Rule: use dictionary rules on v4 for non-English; test before relying on v3 or on dictionary rules inside agents. |
| Scribe tiers: 36 (L3), Mandarin top tier and Arabic/Korean 5-10% (L4) | Current docs: 34 / 20 / 18 / 19; Mandarin 5-10%; Arabic, Korean, Hebrew, Thai 10-20%. L4's language feasibility ranking is adjusted. |
| Agents: 70+ or 31 languages; TTS model unclear | Unresolved: language page 404. Measure via API and a test call. Budget assumes nothing. |
| Recasts (L5 fluency) vs prompts/delay (L1, L2) | Fluency mode: no interruption, brief end digest, recasts only for comprehension breaks; accuracy mode: prompts ladder. Compare by n=1 on delayed error recurrence. |
| "Cap within-session repeats" (L1) | Nakata shows more retrievals score higher per session; one is best only per unit time. Rule: prefer spreading reviews; repeat after a failure; do not claim repeats are harmful. |
| Generative-AI effect sizes (0.39 to 1.3) | Trust only the pattern (short, vocabulary, proximal tests, novelty); plan around none. |
| Zero Retention Mode "Enterprise only" (L3) | Not on the retention page; unverified. Use configurable retention (0 or short) plus audio-saving off; assume TTS/STT requests are logged. |
| Sutton & Webb "small" vs g = .89 | The abstract says both. Within-group pre-post; not comparable to between-group d. |
| FSI four vs five tiers | Unresolved; hours dropped from product copy. |

## 4. What not to build

- Streaks, XP, leagues, loss-aversion notifications as the core loop. No verified evidence links them to proficiency; the Duolingo outcome studies are vendor-authored, non-randomized and receptive-only.
- A CEFR or ACTFL score certified or inferred from chat or card accuracy; "fluency %"; hours-to-fluency countdowns.
- Pronunciation scoring from Scribe transcripts, `logprob`, or WER alone; a single headline pronunciation score; "native-like accent" goals. ASR repairs errors (L2 citation not re-verified).
- Always-on language detection for an accented beginner; open free chat with no attempt-first rule or hold-out check (matches the harmful arm in Bastani).
- Shadowing as a headline feature (no verified meta-analysis); keep at most a labelled "experimental" clip mode.
- Learning-styles modes, immersion-only modes, recast-only feedback, correcting every error live.
- A local phoneme model (no GPU); reliance on Zero Retention Mode; cost promises based on the v4 discount or unmeasured agent minutes.
- Many languages with shallow data; copies of commercial readers or subtitles.

## 5. Open tests before build

1. Scribe and the agent's ASR on the owner's accented, error-laden beginner speech, per language: does it normalise errors?
2. Which TTS model an agent uses per language, whether dictionary rules apply, and real per-minute agent cost.
3. A native-checked 50-sentence gold set per language; an Azure pilot with deliberate mispronunciations.
4. Read the Bastani correction before quoting its numbers.

## 6. Citation list with verification status

Verified (record and abstract read): table rows 1-9, 12-14, 16-19 (Kim & Webb; Lyster & Saito; Li 2010; Kang et al.; Uchihara et al. 2025; Sakai & Moorman; Lee et al.; Bastani et al.; Li, Wang & Yang; Sutton & Webb; Jeon & Day; Schmitt et al.; Wu & Yu; Simsek et al.; Teimouri et al.; Jiang et al. 2021, 2024), and vendor pages in rows 21, 22, 24-26.

Corrected: rows 10, 11, 15, 20, 23 (Hartshorne framing; Nakata interpretation; Alghamdi & Alghizzi scope; ElevenLabs model and Scribe-tier figures).

Failed or unverified, dropped or flagged: FSI tiers (state.gov 403); ElevenLabs agent language page (404: "fixed per call", "31 languages"); "Enterprise-only Zero Retention"; Chua & Pan 2026; Sangers et al. 2025; Webb et al. 2020/2023; Nation 2006; Nakanishi 2015; Saito & Plonsky 2019; PeerJ 2017; van Zeeland & Schmitt 2013 abstract; Tatoeba, Wiktionary, wordfreq, UD licences and counts; Speechace and SpeechSuper details; Duolingo Max claims; LexTALE duration; Azure locale count and data-retention statement.


---

# FILE: docs/tutor/research/learning.md

# Learning science and cognitive neuroscience: synthesis for Tutor

Scope: eight reports (A1 to A8). I read all of them in full, then re-checked 33 load-bearing sources against abstracts retrieved this session from Europe PMC, Crossref, OpenAlex, Semantic Scholar, publisher pages and the srs-benchmark README. Full texts were not read.

## 0. Bottom line

1. The strongest, best-replicated levers are retrieval with feedback, spacing with relearning, and feedback that explains. Together they fit Tutor's current design (FSRS, question bank with explanation and misconception).
2. Retrieval practice is not the engine for multi-step problem solving. The 2025 math meta-analysis (corrected below) found the testing effect not robust in math. Procedural skill needs its own worked-example, fading and interleaved path.
3. Scaffolding must depend on the learner's per-topic knowledge (expertise reversal is robust), not on a global pedagogy.
4. An LLM that answers on demand harms later unassisted performance; a guardrailed one largely does not. This is the single most important prompt constraint.
5. Neuroscience supplies explanations, not new methods. The planned Dual N-Back section has no support as a learning aid.
6. Nearly all evidence is from children, undergraduates or lab samples. No verified PhD-level evidence exists for any rule below. Treat effect sizes as priors and have Tutor measure the owner's own data.

The owner's East Asian and Nordic pedagogy idea is outside this area. A4 could not verify a primary source for Chinese variation teaching, so nothing here relies on it.

## 1. Verified evidence table

Status: V = abstract matched this session; C = corrected versus the report; R = report-level only (not re-checked by me).

| Claim | Figure (as verified) | Status | Strength and limits |
|---|---|---|---|
| Classroom testing effect | g = 0.499, 222 studies, N = 48,478; moderated by feedback, repeats, format (Yang 2021) | V | Strong. School and college. |
| Testing transfers | d = 0.40 [0.31, 0.50], 122 experiments; weakest for rearranged items, untested material, worked-example problems; bias-corrected intercepts often near zero without moderators (Pan & Rickard 2018) | V | Moderate. Transfer is conditional. |
| Testing vs restudy at delay | 5-min test favoured restudy; 2-day and 1-week favoured testing; students felt restudy was better (Roediger & Karpicke 2006) | V | Strong. Prose passages. |
| Retrieval in math | Testing vs restudy g = 0.18, CI crosses zero, 7 studies; spacing g = 0.28, 27 studies (course-embedded 0.24) (Murray, Horner & Goebel 2025) | C | Report A1 quoted preprint values 0.22 and 0.26; published abstract differs. Weak for testing. |
| Spacing ridgeline | Optimal gap about 20-40% of a 1-week delay, about 5-10% of a 1-year delay; N > 1,350 (Cepeda 2008) | V | Strong. Facts, up to 1 year. |
| Successive relearning | Relearning vs single session ds = 1.52 to 4.19; initial-lag effects ds = 1.07-1.87 fall to -0.20-0.38 after relearning (Rawson 2018). "3 correct recalls then relearn" prescription, 533 students (Rawson & Dunlosky 2011) | V | Strong in lab. Vocabulary and concept definitions. |
| FSRS prediction | 9,999 Anki users, 349.9M evaluated reviews; log loss FSRS-6 0.346, FSRS-7 about 0.336-0.340, DASH 0.368, ACT-R 0.403, HLR 0.469 | C | A2 said "350M reviews" loosely; README says 727M raw, 349.9M evaluated. Measures prediction, not learning. Community-run. |
| Interleaving | g = 0.42 overall; math g = 0.34 ("small"); paintings 0.67; words -0.39 (blocking better); expository text n.s. (Brunmair & Richter 2019) | V | Moderate. Needs confusable, similar-between categories. |
| Interleaved math RCT | 61% vs 38%, d = 0.83, 54 classes, unannounced test 1 month later (Rohrer et al., J Educ Psychol, online 2019 / issue 2020) | V | Moderate. 7th grade, near transfer. |
| Elaborated feedback | EF 0.49, KCR 0.32, KR 0.05; delayed timing negatively affected effects but no significant interaction with outcome level (Van der Kleij 2015) | V | Moderate. Computer-based item feedback. Resolves A3's garbled timing note. |
| Unguarded LLM harm | GPT Base +48% practice, then -17% when access removed; GPT Tutor +127%, harm largely mitigated; about 1,000 students (Bastani 2025) | V | Moderate. One field study, high-school math. A3 reports a correction notice; I did not read it. |
| Structured AI tutor | Students learned significantly more in less time than active-learning class (Kestin 2025) | V | Moderate. No effect size in abstract; A5's "twice as much" is from a preprint and is not used. |
| Worked examples, math | g = 0.48, 55 studies, 181 effect sizes; adding self-explanation prompts moderated the effect negatively; correct examples alone best (Barbieri 2023) | V | Strong for novices. Math only. |
| Expertise reversal | Low knowledge d = 0.505 for high assistance; high knowledge d = -0.428; weaker for humanities and language (Tetzlaff 2025) | V | Strong. Asymmetric. Matters for the English track. |
| Productive failure | g = 0.36 [0.20, 0.51], 53 studies; 0.37-0.58 when faithful; grades 2-5 and domain-general skills favoured instruction first; abstract also states bias-adjusted g = 0.87 (Sinha & Kapur 2021) | V | Moderate, contested. The 0.87 is unexplained; do not use. |
| Self-explanation prompts | g = 0.55, 69 effect sizes (Bisra 2018) | V | Moderate. Lab-heavy; shrinks in digital and delayed settings (Tan 2025, R). |
| Learning by teaching | g = 0.27 overall; 0.48 with teaching expectancy; -0.02 without (Kobayashi 2024) | V | Moderate. Needs preparing-to-teach expectancy. |
| Case comparison | d = 0.50 [0.44, 0.56], 57 experiments; best when learners find similarities first and principle comes after (Alfieri 2013) | V | Strong. |
| Pretest vs posttest | d = 0.35 vs 0.74, transfer only after posttesting (Latimier 2019); pretest >= posttest, 5 experiments, n = 1,573 (Pan & Sana 2021) | V both | Contested (see section 3). |
| Sleep and memory | g = 0.44; g = 0.28 after selective-reporting correction; larger with multiple study trials (Berres & Erdfelder 2021). Sleeping between sessions halved relearning practice, n = 40 (Mazza 2016) | V | Moderate; vocabulary only. |
| Cognitive-training far transfer | No convincing far transfer vs treated controls (Melby-Lervåg 2016; 87 publications). n-back: medium on untrained n-back, very small elsewhere, single vs dual and dose did not moderate (Soveri 2017; 33 RCTs) | V | Strong (null). |

## 2. Rules for Tutor, ranked by evidence strength times impact

Tags: [PROMPT] voice or chat tutor prompt; [SCHED] scheduling; [QGEN] question generation; [FEAT] feature; [UI]; [MEAS] measurement; [CORPUS].

1. **Attempt before reveal, then elaborated feedback.** For every scored item, the tutor collects the learner's committed answer before any hint or explanation, then gives feedback that explains (names the misconception), never right/wrong alone. Escalate hints: nudge, partial step, worked step. Log hint level. [PROMPT] [UI] [FEAT]. Basis: Yang 2021, Van der Kleij 2015, Bastani 2025. The "check answer before explain" ordering is an inference from these, not a tested protocol.
2. **Successive relearning on FSRS for declarative atoms.** New item: retrieve to about 3 correct recalls in session 1, then one correct recall per later FSRS-scheduled session. Do not over-drill session 1. Log item, time, grade, latency, mode (voice or chat) on every review. Always show the answer after a miss. [SCHED] [MEAS]. Basis: Rawson 2011 and 2018, Cepeda 2008. Lab vocabulary evidence; FSRS itself has no learning-outcome RCT.
3. **Two separate paths: declarative and procedural.** Atoms (definitions, theorem statements, mechanisms, vocabulary) go through FSRS recall. Derivations and multi-step problems go through worked example, completion problem, then full problem, with fresh isomorphic variants each time. Report the two separately. [SCHED] [QGEN] [UI]. Basis: Barbieri 2023, Murray 2025, Pan & Rickard (worked-example problems weak).
4. **Per-topic stage policy (novice, developing, proficient).** Novice on a high-interactivity item: example first, then fade backward. Proficient: problem first, no hints by default, example only after a committed attempt. Recompute after each attempt from stability, correct rate, latency and hint use. Treat the owner as a novice in every new topic. [PROMPT] [SCHED]. Basis: Tetzlaff 2025. Stage thresholds (for example 80% accuracy) are assumptions to tune. English track: do not port math fading rules untested.
5. **Guardrail the LLM.** No answer-on-demand in practice flows; hints graded and logged; track an AI-reliance gap (hint-free accuracy vs guided accuracy). [PROMPT] [MEAS]. Basis: Bastani 2025.
6. **Question-bank schema.** Keep one skill per item, explanation, misconception. Add prereqs, element_interactivity (1-5, an LLM-seeded estimate to calibrate), confusable_with, typical_confident_error, near-transfer and structural-transfer variants tagged separately, and a corpus passage citation for each explanation. Validate generated worked solutions with a second pass or a computer algebra system. Default format: cued recall or short answer; multiple choice only for discrimination, with misconception-based distractors and per-option feedback. [QGEN] [CORPUS]. Basis: Pan & Rickard, Little et al. 2012 and Greving & Richter 2018 (R), Chen, Paas & Sweller 2023 (R).
7. **Confidence before feedback.** Four levels (guess, unsure, fairly sure, certain), asked after the answer and before feedback, on retrieval attempts only. Route feedback: high-confidence error gets a misconception-naming explanation, a same-session retest and a retest at the next FSRS interval; low-confidence correct gets a short reason. Confidence does not drive FSRS parameters. [PROMPT] [UI]. Basis: Butler 2008, Butler 2011, Metcalfe 2017 (all R; hypercorrection is established for facts, unproven for problems). Design inference.
8. **Interleave confusable problem types.** Block briefly when a procedure is new, then mix confusable types (integration techniques, equilibrium vs kinetics, similar mechanisms). Do not interleave vocabulary-like pairs or long expository reading. [SCHED]. Basis: Brunmair 2019, Rohrer 2019/20; part of the benefit may be spacing (Foster 2019, R).
9. **Selective generative prompts.** Self-explanation after errors or for proficient learners, including "why is this misconception wrong". Comparison of two same-structure cases, principle stated afterwards, for chemistry and physics. Fade prompts as unprompted quality rises. [PROMPT] [QGEN]. Basis: Bisra 2018, Alfieri 2013, Barbieri 2023.
10. **Teach-back with announced expectancy.** Announce before study that the owner will explain it; the tutor plays a naive student; owner explains from memory, then may open sources for a second pass. Schedule after first successful recall. [FEAT] [PROMPT]. Basis: Kobayashi 2024; open-book finding is R (Sibley 2022).
11. **Measure the owner, not the population.** Randomize about 20% of eligible items to restudy vs retrieval, hold out about 10% from extra review, test at 7-30 days, keep a separate transfer set. Report Brier score, calibration curve, delayed accuracy, minutes to criterion, reliance gap. State that one learner is low-power. Never use satisfaction as the outcome. [MEAS] [UI]. Basis: Deslauriers 2019 (felt vs actual diverge); Roediger & Karpicke 2006.
12. **Short exposition, retrieval as the attention device.** At most 2-3 minutes of talk before an answerable question. Default 40-50 minute sessions with a pre-set, overridable 5-10 minute break, labelled an A/B default. Voice never interrupts mid-derivation. Feedback informational, not praise. Offer 2-4 choices; no points or streaks. [PROMPT] [UI]. Basis: Szpunar 2013; break values are not from a verified trial.
13. **Desired retention as a tunable default.** Start at 0.90; offer tiers (0.95 near exam dates, 0.85 bulk, 0.80 breadth) with daily workload shown. No source validates 0.90 as optimal; tiers are design judgment. [SCHED] [UI].
14. **Sleep as a logged covariate.** First relearn naturally falls next day under FSRS; log sleep window and test a sleep-crossing effect on own data; expect g near 0.3. [SCHED] [MEAS]. Basis: Berres 2021, Mazza 2016. Weak.
15. **Progressive multimedia rules for new material only.** Signal key terms, keep text beside figures, segment long derivations, no decorative images; offer expert mode that turns them off. [UI]. Basis: Noetel 2022 (R), expertise reversal.
16. **Neuromyth guard.** Fixed adversarial prompt set confirms the tutor never asserts learning styles, 10% of the brain, dopamine-hit or brain-training claims. [MEAS] [PROMPT].

## 3. Conflicts and how I resolved them

- **Retrieval practice as universal engine (A1 headline) vs complex material (A1, A4, Murray).** Resolved toward conditional use. The general effect is strong (g = 0.499), but the math-specific effect is g = 0.18 with a CI crossing zero, and van Gog & Sweller 2015 is an editorial hypothesis (not a pooled effect). Use retrieval for atoms; step back one stage on a lapse of a high-interactivity item.
- **Attempt-first everywhere (A1, A5, A8) vs worked examples first for novices (A4).** These are two different rules. "Commit an answer before the tutor reveals it" is a guardrail that applies to every scored item (Bastani). "Problem before instruction" (productive failure) is a sequencing choice with g = 0.36, reversed in children and contested at high element interactivity (Ashman 2020, R). Resolved: the guardrail is universal; productive failure is gated by prerequisites and a conceptual target and always followed by consolidation. Stronger evidence: expertise reversal (d = 0.505 and -0.428, 60 studies) over productive failure (mostly non-adult).
- **Self-explanation prompts (A5 recommend, A4 caution).** Bisra g = 0.55 compares prompts vs none broadly; Barbieri found prompts added to worked examples reduced the effect in math. Resolved: prompts are selective (after errors, proficient stage), not always on. Barbieri is the more specific evidence for Tutor's math use.
- **Pretesting (A1, A6 positive vs Latimier).** Both verified. Pan & Sana found pretest >= posttest on expository text; Latimier found post 0.74 vs pre 0.35 with transfer only after posttesting. Materials and formats differ. Resolved: optional short prequestions only when the target is taught immediately, never core loop.
- **Confidence format (A1 0-100, A3 four-level, A6 1-5).** All agree it comes before feedback. Resolved: four levels (works by voice, avoids spurious precision). A3's warning against per-item JOL sliders on study material stands (reactivity g = 0.054, R).
- **Feedback timing (A1 immediate implied; A3 adaptive).** Verified Van der Kleij: delayed timing had a negative effect overall, without a significant interaction. Classroom-versus-lab mixed evidence and Mullet 2014 (record verified only) favour delay for transfer. Resolved: default immediate elaborated feedback; randomize delay for proficient topics and read the owner's own 7-day retest.
- **Interleave by default (A2) vs block first (A8).** Math g = 0.34 is small. Resolved: brief block for new procedures, then interleave confusable types (design inference).
- **Scheduling after sleep (A6) vs FSRS-decided intervals (A2).** Compatible: first FSRS interval is at least a day. Keep criterion learning in session 1 and log sleep. Mazza is one n = 40 vocabulary study.
- **FSRS "20-30% fewer reviews" (docs claim).** Not independently verified; do not display.
- **Stale or unverifiable numbers.** Murray values corrected; Sinha & Kapur 0.87 flagged; Kestin "twice as much" dropped.

## 4. What not to build

- Dual N-Back as a learning aid, or any score feeding Plan or scheduling. Verified meta-analyses show task-specific gains only. If built, label it an optional working-memory game and keep it lowest priority.
- A retrieval-only engine for derivations and multi-step problems.
- An answer-on-demand or hint-on-demand LLM mode in practice.
- One global Socratic, explain-first or struggle-first tutor.
- Learning-styles, hemisphere or brain-age features; "learning pyramid" retention percentages.
- Points, XP, streaks, leaderboards, dopamine-themed rewards, hours-counter gamification.
- Growth-mindset or grit modules; willpower bars; exercise-boost messaging; phone-presence shaming; face or voice emotion detection.
- Sleep-learning, targeted memory reactivation, or reconsolidation "memory editing".
- Custom HLR, DASH or ACT-R schedulers, neural schedulers (revisit above about 100k reviews), forced expanding intervals.
- Real-time cognitive-load dashboards; germane load as a KPI.
- Per-item JOL sliders; confidence-weighted scoring as a headline; "unskilled and unaware" messaging.
- Curiosity teasers before unrelated material; deliberately induced confident errors.
- UI copy presenting productive failure or cognitive load theory as settled.

## 5. Open questions

1. Retrieval, spacing, interleaving and feedback effects at PhD level on high element-interactivity material: no verified evidence.
2. Whether hypercorrection holds for adult problem solving (only a 2026 dissertation, not peer reviewed).
3. A correction notice for Bastani 2025 exists per A3; read it before quoting 17% in product copy.

## 6. Citations and verification status

**V: abstract matched this session (33).** Yang 2021 (doi:10.1037/bul0000309); Pan & Rickard 2018 (10.1037/bul0000151); Roediger & Karpicke 2006 (10.1111/j.1467-9280.2006.01693.x); Rawson & Dunlosky 2011 (10.1037/a0023956); Rawson et al. 2018 (10.1037/xap0000146); Cepeda et al. 2008 (10.1111/j.1467-9280.2008.02209.x); Lindsey et al. 2014 (10.1177/0956797613504302); Kang et al. 2014 (10.3758/s13423-014-0636-z); Brunmair & Richter 2019 (10.1037/bul0000209); Rohrer et al. (10.1037/edu0000367); Van der Kleij et al. 2015 (10.3102/0034654314564881); Bastani et al. 2025 (10.1073/pnas.2422633122); Kestin et al. 2025 (10.1038/s41598-025-97652-6); Barbieri et al. 2023 (10.1007/s10648-023-09745-1); Tetzlaff et al. 2025 (10.1016/j.learninstruc.2025.102142); Sinha & Kapur 2021 (10.3102/00346543211019105); Bisra et al. 2018 (10.1007/s10648-018-9434-x); Kobayashi 2024 (10.1007/s10648-024-09871-4); Alfieri et al. 2013 (10.1080/00461520.2013.775712); Pan & Sana 2021 (10.1037/xap0000345); Latimier et al. 2019 (10.1038/s41539-019-0053-1); Berres & Erdfelder 2021 (10.1037/bul0000350); Mazza et al. 2016 (10.1177/0956797616659930); Hu et al. 2020 (10.1037/bul0000223, sleep TMR g = 0.29, not a software feature); Szpunar et al. 2013 (10.1073/pnas.1221764110); Deslauriers et al. 2019 (10.1073/pnas.1821936116); Melby-Lervåg et al. 2016 (10.1177/1745691616635612); Soveri et al. 2017 (10.3758/s13423-016-1217-0); Macnamara et al. 2014 (10.1177/0956797614535810; deliberate practice explains 26% of variance in games, 21% music, 18% sports, 4% education, <1% professions); Macnamara & Burgoyne 2023 (10.1037/bul0000352; growth mindset d = 0.05, n.s. after bias correction); Ciria et al. 2023 (10.1038/s41562-023-01554-4; exercise-cognition d falls 0.22 to 0.05 after bias correction); srs-benchmark README (github.com/open-spaced-repetition/srs-benchmark).

**C: corrected.** Murray, Horner & Göbel 2025 (10.1007/s10648-025-10035-1): published abstract gives testing g = 0.18 (CI crosses zero) and spacing g = 0.28, not the preprint's 0.22 and 0.26. srs-benchmark: 349.9M evaluated reviews from 9,999 users (raw 727M); FSRS-7 about 0.336 in the table variant I saw (A2 said 0.340).

**T: record or title confirmed, content not read.** Kluger & DeNisi 1996 (10.1037/0033-2909.119.2.254); Mullet et al. 2014 (10.1016/j.jarmac.2014.05.001, title only).

**R: report-level only, not re-checked by me.** All other sources cited in reports A1 to A8 and mentioned above, including Cepeda 2006, Latimier 2021, Vaughn 2016, Hartwig 2022, Foster 2019, Wisniewski 2020, Fyfe 2015, Butler 2008 and 2011, Metcalfe 2017, Rhodes & Tauber 2011, Greving & Richter 2018, Little 2012, van Gog & Sweller 2015 (editorial), Ashman 2020, Loibl 2017, Tan 2025, Noetel 2022, Sibley 2022, Shields 2017, Smith 2016, Albulescu 2022, Biwer 2023, Vohs 2021, Sievertsen 2016, Deci 1999, Vaci 2019, Macnamara & Maitra 2019, Sala 2019, Pashler 2008, Nielsen 2013. Treat their numbers as unconfirmed and do not quote them in product copy.


---

# FILE: docs/tutor/research/nback.md

# SYNTHESIS: Dual N-Back and working-memory training (Tutor)

Verifier/synthesizer: Sonnet 5.5 (high). Inputs: B1-evidence, B2-protocol-engineering, B3-measurement, B4-ux. Date 2026-09-30. I checked 27 citations against Europe PMC and Crossref records (abstract level; every researcher's web-search budget, and mine, was exhausted). Full texts were not opened, so figures found only in full texts are marked unverified (U).

## 1. Bottom line

Dual n-back training reliably improves the trained task and closely similar n-back tasks. Transfer to other working-memory (WM) tests is small. Transfer to reasoning and to real-world or academic outcomes is not established: field-level meta-analyses find it null once treated controls and publication bias are handled, and the one favourable line (Pahor 2022) is a post-hoc mediation result. No researcher found a trial in PhD-level or highly educated adults with academic outcomes (absence of evidence, not evidence of absence).

The best version the evidence allows is an optional, small, honestly labelled WM skill-practice module with a built-in personal experiment that tests any broader benefit on the owner, and is expected to show none. It must not feed FSRS, question difficulty, lesson gating or the tutor's claims about learning ability. The owner's assumption (n-back raises intelligence or eases PhD-level learning) is not supported well enough to build on, and the tutor should say so if asked.

## 2. Verified evidence table

Status: V = record opened, claim and figure match; V(2) = figure confirmed only via secondary sources; C = corrected; U = not verified.

| # | Claim | Source | Figure and notes | Status | Strength |
|---|---|---|---|---|---|
| 1 | Medium gain on untrained n-back; very small on other WM, Gf, cognitive control; no moderation by age, dose, single vs dual | Soveri 2017 PBR, 33 RCTs | g = 0.62 / 0.24 / 0.16. Abstract confirms "medium" and "very small"; numbers quoted in two open-access citing papers. CIs and the 0.11 control-adjusted Gf: U | V, V(2) | Strong (task-specific gain) |
| 2 | Small Gf effect, shrinking with active controls | Au 2015 PBR, 20 studies, ages 18-50 | Overall g = 0.24 (via Dougherty). Active 0.06 vs passive 0.44: U | V, U | Contested |
| 3 | Bayesian reanalysis of Au | Dougherty 2016 PBR | Passive controls favour an effect; active controls modestly favour the null | V | Moderate |
| 4 | No convincing far transfer vs treated controls; no evidential value after bias analysis | Melby-Lervåg, Redick, Hulme 2016 PPS, 87 publications | Qualitative; also confirms reliable short-term WM gains | V | Strong |
| 5 | Well-powered placebo-controlled dual n-back RCT: no transfer to any ability test | Redick 2013 JEP:G | 20 sessions; adaptive visual-search placebo plus no-contact group | V | Strong (young adults) |
| 6 | Trained gains persist at least 6 months; no transfer | Thompson 2013 PLoS ONE | Sample size 58: U | V | Strong (young adults) |
| 7 | Apparent WM gains depend on test similarity; Gf not improved; dose and type irrelevant | Rodas 2024 PBR, 52 comparisons | SMD 0.18 overall vs 1.15 for training-like tasks | V | Moderate to strong |
| 8 | Far-transfer mean null with bias control | Sala 2019 Collabra (Crossref); Gobet & Sala 2023 PPS (advocacy-toned) | Qualitative | V | Strong (direction) |
| 9 | Far transfer null vs active controls | Sala & Gobet 2020 PBR | g = 0.001 (SE 0.055). **Typically developing children, all WM programmes, not n-back or adults** | **C** | Do not cite as adult n-back evidence |
| 10 | Older adults: absent transfer | Guye & von Bastian 2017 (n = 142); Sala 2018 preprint (N = 2,140) | Preprint: trained g 0.877, near 0.274, far 0.121, far with active controls -0.008 | V | Moderate (older adults) |
| 11 | Middle-aged adults: nulls | Lawlor-Savage & Goghari 2016 (RCT, n = 57, 30-60 y, Bayesian null); Ripp 2022 (n = 55, no cognitive or imaging change) | As stated | V | Moderate |
| 12 | Far transfer may depend on near transfer to an unrelated n-back | Pahor 2022 Nat Hum Behav, 3 RCTs, N = 460 | Mediation in all three, but no intervention effect on Matrix Reasoning in RCTs 2 and 3. An Author Correction exists (10.1038/s41562-022-01514-4), unread | V, U | Weak to moderate; post hoc |
| 13 | n-back is unreliable for individual differences and weakly related to complex span | Jaeggi 2010; Redick & Lindsey 2013; Kane 2007 | "Insufficient reliability"; "cannot be used interchangeably". The r of about 0.20: U | V | Strong (direction) |
| 14 | Retest effects on WM tests | Scharfen 2018 PBR, 95 samples | g = 0.28 first to second test; rises to fourth | V | Strong |
| 15 | Expectancy can manufacture apparent IQ gains | Foroughi 2016 PNAS | Suggestive-flyer group gained 5-10 IQ points' equivalent after one hour | V | Contested |
| 16 | Gamification raises motivation, not cognition | Vermeir 2020, 9 RCTs | g = 0.72 motivation; -0.52 "demanding"; no training-domain effect | V | Moderate |
| 17 | Trained gains grow with length; no transfer | Zhong 2025 Acta Psychol, preregistered double-blind RCT | Abstract only | V | Moderate |
| 18 | Spacing matters "only minimally" | Jaeggi 2020 J Gerontol B (older adults) | No spacing benefit | V | Weak |
| 19 | Higher-ability learners progress faster, drop out less | Ørskov 2021, N = 217, 12 weeks | Task progress, not transfer | V | Moderate |
| 20 | Gains reflect strategy, not capacity | Malinovitch 2021 (n = 14 per group: two taught sessions matched 40 untaught); Ripp 2022 | As stated | V | Weak to moderate |
| 21 | Academic harm in children | Roberts 2016 JAMA Pediatr, N = 452 | Maths -3.0 (95% CI -5.4 to -0.7) at 2 years; Cogmed, 6-7-year-olds | V | Strong for that trial only |
| 22 | Updating-to-span transfer small | Fu 2026 PBR, 55 studies | g = 0.176; span-to-updating 0.048 (ns) | V | Moderate |
| 23 | Methods too divergent for strong conclusions | Pergher 2020 | Count of 122 transfer tasks: U | V, U | Moderate |
| 24 | Sleep restriction impairs cognition | Lowe 2017, 61 studies | g = -0.383; intelligence domain: insufficient evidence | V | Strong, about restriction only |
| 25 | Exercise effect small, bias-sensitive | Ciria 2023 Nat Hum Behav | d 0.22, then 0.13, then 0.05 (CI -0.09 to 0.14) | V | Weak |
| 26 | Original dose-dependent Gf claim | Jaeggi 2008 PNAS | Abstract asserts dose dependence; not reproduced in rows 1, 5, 6, 7 | V | Historical |
| 27 | Reliable-change threshold about 18 IQ points at reliability 0.8 | B3's arithmetic | I recomputed 1.96 x sqrt(2) x 15 x sqrt(0.2) = 18.6 | V | Illustrative |

No citation failed outright. One was corrected (row 9).

## 3. Rules for Tutor, ranked by evidence strength times impact

Tags: [PROMPT] voice or chat tutor prompt, [SCHED] scheduling, [QGEN] question generation, [FEATURE], [UI], [MEAS] measurement, [CORPUS].

1. [PROMPT][UI] Never claim that n-back raises IQ, learning speed, grades, "brain power" or WM capacity. Use: "This trains the n-back task; transfer to other skills is small or unproven, and any broader effect is being tested on you." (rows 1, 4, 5, 7, 8)
2. [PROMPT][SCHED] The tutor never recommends n-back as a study strategy or instead of retrieval practice or sleep. It may offer it as an optional warm-up after due reviews. (rows 4, 8, 9)
3. [SCHED][QGEN] N-back level must not drive FSRS intervals, question difficulty, gating or curriculum. The voice tutor gets no n-back tools beyond reading its own stats. (rows 7, 13)
4. [UI][MEAS] Separate panels: "Training curve (trained task only)", "Transfer probes", and learning outcomes (FSRS retention, quiz accuracy). Never blend them into one cognitive score, WM age or IQ estimate. (rows 7, 13)
5. [PROMPT] In progress talk, always distinguish trained-task, untrained-n-back, WM-probe and reasoning changes, and cite the owner's own data with intervals; if inconclusive, say so and do not round up. (rows 1, 12, 14)
6. [SCHED] Dose default: 15-20 min, up to 3 sessions a week, about 60 min a week cap, after due reviews, learner-adjustable, no streaks. A convention chosen for opportunity cost, not an evidence-derived optimum (section 5).
7. [FEATURE][MEAS] Log per trial (seed, n, stimulus, response, hit, miss, false alarm, correct rejection, lure flag, timing, focus loss). Report d' and criterion c per modality with a correction for rates of 0 or 1, plus raw counts. (Meule 2017 supports this; its abstract was empty, so U.)
8. [MEAS][FEATURE] A transfer-check block every 4 weeks: single n-back with different stimuli and timing, labelled "measurement", ungamified, fixed settings. It tests the only transfer the evidence supports (row 12).
9. [MEAS][FEATURE] Opt-in "working-memory experiment" card on the Plan screen: 3-5 baseline probe sessions over two weeks, locked plan, matched active control task (adaptive visual search, as in Redick and Thompson), randomized blocks where possible, reliable-change or Bayesian within-person analysis. Show the minimum detectable change first (about 18 IQ points for one matrix probe under stated assumptions). State that the literature expects no far transfer and that one unblinded person cannot exclude expectancy or practice effects. (rows 14, 15, 27)
10. [MEAS] Probes use formats unlike the trained task: a different-stimulus n-back, short-form operation/symmetry span, backward digit span, and open reasoning items (ICAR, MaRs-IB) drawn without repeats, rotating parallel forms, never adaptive, never reused in training or tutoring. ICAR and MaRs-IB psychometrics are U. (row 14)
11. [MEAS] Log expectation (one rating per block), sleep hours, exercise and fatigue each session. Advice stays modest: protect sleep (row 24); claim no cognitive benefit from exercise (row 25).
12. [UI] No streaks, XP, leaderboards, multipliers or badges in n-back; gamification alters the demand being scored (row 16). A quiet session log suffices.
13. [FEATURE] Timing: one AudioContext created on the Start click; letter clips pre-rendered once (ElevenLabs or recorded), trimmed under 450 ms, loudness-normalised, decoded at startup; lookahead scheduling on the audio clock; visuals on the frame nearest the scheduled time; log intended vs actual onsets and `outputLatency`; void a block on focus loss, tab hiding or device change. No live ElevenLabs calls or `speechSynthesis` in the trial loop. Response time is a within-person trend only. B2 rests on MDN, W3C and Microsoft docs, not WebView2 measurements, so label timing "unmeasured" until a calibration screen runs.
14. [UI][FEATURE] Accessibility: pair colour with shape or text (WCAG 2.2 SC 1.4.1), colour-blind-safe palette, high contrast, reduced motion, remappable keys (default A and L), audio-only and visual-only modes (which are also the single-modality modes).
15. [FEATURE] Show level and adaptation rule on screen and log the rule per session; no silent adaptivity. Block-end summary instead of per-trial correctness flashes by default (design judgment; no source tests it); opt-in in-trial feedback marks a session non-comparable.
16. [FEATURE] Seeded stimulus generator (store the seed), about 25% matches per modality, lures off at first, then optional and tagged.
17. [FEATURE] One-line post-session question, "Which strategy did you use?" (row 20). After the experiment window, suggest stopping and retesting rather than training indefinitely.
18. [QGEN] Do not condition question difficulty on n-back level. Design inference (not from these reports): limit simultaneous elements per item and give worked steps.
19. [CORPUS] Add own-note metadata (no full text) for Soveri 2017, Melby-Lervåg 2016, Redick 2013, Pahor 2022, Rodas 2024, Scharfen 2018, Jaeggi 2010 and Redick & Lindsey 2013 so the tutor can cite. Brain Workshop is GPL-2.0: take design ideas only and write original Rust and React code.

## 4. Default design spec (configurable; conventions, not evidence-derived)

- Modes: dual (8 positions on a 3x3 grid with centre unused, plus 8 non-rhyming consonants), single position, single letter. Default dual, start at n = 1.
- Trial: 500 ms stimulus, 3,000 ms trial, 20 + n trials per block, about 6 targets per modality including dual matches (Thompson-style; B2's "4 auditory, 4 visual, 2 dual" reading of Jaeggi 2008 is U).
- Adaptation per block (Jaeggi/Thompson convention): n up at 2 or fewer errors in each stream, down above 5. A session is 12-20 blocks with 10 s rests.
- Charts: each session as a point with rolling median and uncertainty band; level, hit rate, false-alarm rate and d' per modality; the line "n-back scores are noisy and do not measure intelligence."
- Storage: local SQLite with CSV export.

## 5. Conflicts and resolutions

1. Gf effect. B1 gives g = 0.24 (Au) but 0.06 against active controls; B2 and B4 say "small but significant"; B3 and B4 lean null. Resolution: active-control numbers are the relevant comparison, and Dougherty (verified) supports that direction. Verdict: lean null, unproven.
2. Dose. B1: 15-20 min, at most 3 days, 60 min cap (rests on Lampit 2014, older adults and general training, unverified). B2: 15-25 min, 3-5 days. B3: 15-20 min daily for 4 weeks. B4: 10-15 min, 3-5 days (its own suggestion). Evidence: Soveri and Rodas (verified) find no dose moderation, Jaeggi 2020 finds spacing minimal, Jaeggi 2008's dose claim is not reproduced, Zhong 2025 shows only trained-task gains growing. Resolution: no dose is evidence-based, so I choose the lowest proposal (15-20 min, up to 3 sessions a week) since extra time has no documented benefit and competes with retrieval practice, and expose it as a setting. The experiment card may fix a 4-week protocol. The 3-day cap is weakly supported.
3. Adaptation rule. B2: Jaeggi per block. B4: Brain Workshop 80% up, three sessions under 50% down, two blocks before any change. B1: about 90% up, 70% down. Resolution: per-block Jaeggi by default, since 15-minute sessions make a three-session fallback too slow and the replication literature used it; thresholds configurable; B4's two-block smoothing optional. No source tests any of these.
4. Feedback. B4 and B2 agree on block-end summaries. Untested, kept as a flagged judgment.
5. Who benefits. Ørskov 2021 (magnification, verified) vs Traut 2021 (compensation, abstract only, unverified). Contested and about task progress, not transfer. Nothing in the app depends on it.
6. Pahor 2022. B1 and B2 flag it as post hoc and from the original lab; B3 and B4 use it to justify the probe. Resolution: it justifies a cheap measurement, not any claim of benefit. Redick 2013, Melby-Lervåg 2016 and Rodas 2024 outweigh it.
7. Timing. B1 (`performance.now()`, local clips) and B2 (AudioContext scheduling, `event.timeStamp`) do not truly conflict; B2 is the detailed version and is adopted. Neither measured WebView2.
8. Attribution errors. B1 and B4 cite Sala & Gobet 2020 (g = 0.001) as far-transfer evidence; it covers children (row 9). B4's note that Lawlor-Savage 2016 is an RCT, not a meta-analysis, is right.

## 6. What not to build

- IQ, brain-age, WM-age or "brain power" scores or badges; composites mixing trained and untrained tasks (rows 7, 13, 15).
- A single pre/post IQ test as proof of benefit (rows 14, 27).
- Marketing copy ("boost cognition for PhD study") or imaging-style visuals implying capacity growth (rows 11, 20).
- Streaks, XP, leaderboards, RT leaderboards (row 16; browser RT is coarse).
- Long fixed programmes justified by dose-response (rows 1, 7, 18, 26).
- Triple, quad, arithmetic or variable-n modes, or triple n-back marketed as superior; no evidence.
- tDCS or other stimulation (Pergher 2022, small net effect; not opened by me).
- Gating of lessons, FSRS intervals or question difficulty by n-back (rows 13, 22).
- `speechSynthesis`, per-trial network audio, AudioWorklet paths, WebView2 browser flags.
- n-back-styled subject content; train subject skills on subject material.
- Code or assets copied from Brain Workshop (GPL-2.0).
- Silent adaptive difficulty.

## 7. Citation list with verification status

V = opened and matches; V(2) = figures via secondary source; C = corrected; U = unverified by me.

- V: Soveri 2017 (10.3758/s13423-016-1217-0, V(2) for figures); Au 2015 (10.3758/s13423-014-0699-x, g = 0.24 via Dougherty; subgroups U); Dougherty 2016 (10.3758/s13423-015-0865-9); Melby-Lervåg 2016 (10.1177/1745691616635612); Redick 2013 (10.1037/a0029082); Thompson 2013 (10.1371/journal.pone.0063614); Pahor 2022 (10.1038/s41562-022-01384-w; correction 10.1038/s41562-022-01514-4 unread); Rodas 2024 (10.3758/s13423-024-02466-8); Sala 2019 (10.1525/collabra.203, Crossref); Gobet & Sala 2023 (10.1177/17456916221091830); Sala 2018 preprint (10.31234/osf.io/5frzb); Guye & von Bastian 2017 (10.1037/pag0000206); Lawlor-Savage 2016 (10.1371/journal.pone.0151817); Ripp 2022 (10.1038/s41398-022-02272-7); Jaeggi 2008 (10.1073/pnas.0801268105); Jaeggi 2010 (10.1080/09658211003702171); Jaeggi 2011 (10.1073/pnas.1103228108); Jaeggi 2020 (10.1093/geronb/gbz090); Redick & Lindsey 2013 (10.3758/s13423-013-0453-9, r U); Kane 2007 (10.1037/0278-7393.33.3.615); Scharfen 2018 (10.3758/s13423-018-1461-6); Foroughi 2016 (10.1073/pnas.1601243113); Vermeir 2020 (10.2196/18644); Roberts 2016 (10.1001/jamapediatrics.2015.4568); Ørskov 2021 (10.1007/s41465-021-00216-5); Malinovitch 2021 (10.3758/s13423-020-01824-6); Fu 2026 (10.3758/s13423-026-02994-5); Zhong 2025 (10.1016/j.actpsy.2025.105499); Pergher 2020 (10.1007/s41465-019-00134-7); Lowe 2017 (10.1016/j.neubiorev.2017.07.010); Ciria 2023 (10.1038/s41562-023-01554-4).
- C: Sala & Gobet 2020 (10.3758/s13423-019-01681-y), children only.
- Exists, claim U: Meule 2017 (10.3389/fpsyg.2017.00352), abstract empty.
- Researcher-reported only, unverified by me: Wiemers 2019, Traut 2021, Tsai 2018, Byrne 2024, Syed 2024 (figures seen only in a table row), Cao 2026, Li 2026, Pergher 2022, Lampit 2014, Wilson 2019 (85% rule; theory, weak for n-back), Bogg & Lasecki 2014, Schmiedek 2009/2014, Bridges 2020, Pronk 2020, Anwyl-Irvine 2021, ICAR (Condon & Revelle 2014), MaRs-IB (Chierchia 2019), Foster 2015, Conway 2005, Hedge 2018, Rouder & Haaf 2019, Monk 2011, Scharinger 2023, Brain Workshop docs and source. Treat their figures as unverified.

## 8. Open questions

- No adequately powered trial in highly educated adults with academic outcomes was found; the owner may be the only data point.
- Read Soveri's CIs, the Au 2015 active/passive split and the Pahor correction from full texts before any number enters prompts or UI copy.
- Check ICAR item counts and norms, MaRs-IB psychometrics and licensing before building the probe bank; ceiling effects are likely for a high-ability learner.
- Real audio-to-visual latency and `outputLatency` on this PC in WebView2 are unmeasured.
- Whether thresholds or in-trial feedback matter is untested in the sources read.


---

# FILE: docs/tutor/research/tools.md

# Subject-specific interactive tools and visual identity: synthesis

Verified 2026-09-30 by Sonnet 5.5 (high). Sources: reports G1 (math), G2 (science/chemistry), G3 (English and visual identity). I re-checked 24 papers and 8 library or licence facts against Crossref, OpenAlex, ERIC, Europe PMC, npm, PyPI and GitHub. I could not open some abstracts (publisher pages returned 403, and web search was exhausted). Those items are marked "unconfirmed" and nothing below rests on them alone.

## 1. Bottom line

1. The tool stack is settled and cheap: KaTeX, MathLive, function-plot, JSXGraph, SymPy sidecar, RDKit.js, Ketcher, SmilesDrawer, Harper, Open English WordNet. All render on CSS, SVG or Canvas2D (RDKit.js and SymPy run on the CPU) and work offline.
2. Interactive tools help only when wrapped in guidance: predict, run, explain, then retrieve. Unassisted exploration is worse than explicit instruction (verified, Alfieri 2011).
3. Per-subject looks are for orientation, not memory. The memory effect of context is small and did not replicate in the one replication we verified (Murre 2021, N = 16). Decoration inside learning material costs about -0.16 SD (Cheng 2026, verified). So: accent colour and icons on app chrome, neutral learning panes, and functional colour only where it marks structure (d = 0.38, verified).
4. No study found tests any of this on adult PhD-track learners. Everything is extrapolated from K-12 and undergraduates. The Evals screen should run the owner's own within-person comparisons.
5. The owner's hope that Chinese institutional teaching material is open source was not supported by G2 (no open-licensed Chinese simulation set found). This area did not test it further.

## 2. Verified evidence table

Status key: V = verified against an abstract or record I opened. C = corrected. U = unconfirmed (exists, figure not seen).

| Claim | Source | Verified figure | Status | Limit |
|---|---|---|---|---|
| Digital tools raise secondary maths/science learning; dynamic maths tools and ITS beat hypermedia | Hillmayr 2020 | g = 0.65, k = 92, grades 5-13; larger when added to instruction | V | Secondary school; short, researcher-made tests |
| Active learning beats lecture in STEM | Freeman 2014 | +0.47 SD (158 studies); failure odds ratio 1.95 (67 studies); 225 studies | V | Classroom active learning in general, not visualization |
| Unassisted discovery is worse than explicit instruction | Alfieri 2011 | d = -0.38 (580 comparisons); enhanced discovery d = 0.30 [0.23, 0.36] (360) | V | School-age mostly; the printed CI for the -0.38 figure is garbled in the source |
| Guidance in inquiry works | Lazonder & Harmsen 2016 | Learning outcomes d = 0.50 [0.37, 0.62], 72 studies | V | Age-group conclusions limited |
| Attempt before instruction (productive failure) | Sinha & Kapur 2021 | g = 0.36 [0.20, 0.51], 53 studies; 0.37-0.58 for true productive-failure designs | C | G2 omitted that grade level moderates: grades 2-5 and domain-general skills did better with instruction first |
| Simulation learning in higher education | Chernikova 2020 | g = 0.85 [0.69, 1.02], 145 studies; high-prior-knowledge learners gain more from reflection, low-prior from examples | V | Complex professional skills (medicine, teaching), not conceptual chemistry |
| Spatial training transfers | Uttal 2013 | g = 0.47 (SE 0.04), 217 studies; durable, transfers to untrained spatial tasks | V | Transfer to chemistry grades not shown |
| ITS effects (median) | Kulik & Fletcher 2016 | Median 0.66 SD, 50 evaluations; larger on locally built tests | V | Inflated by test alignment |
| ITS for K-12 maths | Steenbergen-Hu & Cooper 2013 | g = 0.01-0.09, 26 reports | V | Schools |
| ITS for college students | Steenbergen-Hu & Cooper 2014 | g = 0.32-0.37, 35 reports; earlier studies larger than recent | V | Closest to an adult learner; use 0.3-0.4 for planning |
| Step-based tutoring near human tutoring | VanLehn 2011 | The abstract I retrieved was garbled and swapped the two numbers. G1's figures (step-based about 0.76, human about 0.79) match the widely cited result; both about 0.76-0.79 | U (partly) | Direction unconfirmed by me; conclusion "about equal" is consistent either way |
| Dynamic geometry software | Chan & Leung 2014 | SMD 1.02 [0.56, 1.48], 9 quasi-experiments, n = 587 | V | Inflated; short interventions, younger pupils. Do not plan on it |
| Learning styles have no support | Pashler et al. 2008 | "Not adequate evidence base"; matching studies contradict the meshing hypothesis | V | Preferences exist but do not predict outcomes |
| Active learning feels worse than it is | Deslauriers 2019 | "Learn more, but feel like they learn less" (randomized, physics) | V | No numbers seen; single site |
| Seductive details harm learning | Cheng et al. 2026 | g = -0.16 overall; mechanism is extraneous load (via ERIC snippet) | V (partial) | 50 studies / 177 effects not seen |
| Same, earlier meta-analysis | Rey 2012 | 39 effects; small-to-medium retention, medium transfer harm | V | Lab lessons |
| Signalling helps | Alpizar, Adesope & Wong 2020 | d = 0.38; 44 effects from 29 studies, N = 2,726; larger in high-quality studies that control prior knowledge | V | Signals inside multimedia lessons, not app chrome |
| Emotional design helps (contested) | Wong & Adesope | 28 articles; g+ = 0.35 retention, 0.27 transfer, 0.29 comprehension | C | Published 2021 (online 2020) per ERIC; in-lesson graphics only |
| Context-dependent memory does not reliably replicate | Murre 2021 | N = 16 divers, no same-context advantage | V | The F and p values in G3 were not seen; not a clean refutation |
| Animation beats static graphics under conditions | Berney & Bétrancourt 2016 | Paper exists (Comput Educ 101:150-167). g = 0.226, 0.883 (no text) and 0.336 (audio) not seen | U | Directionally supported by Höffler & Leutner 2007 (d = 0.37, 26 studies, via an ERIC snippet) |
| Expertise reversal | Kalyuga et al. 2003 | Exists (Crossref, Educ Psychologist). No effect size extracted | U | Pattern well known; size unknown |
| Environmental context effect d about 0.25 | Smith & Vela 2001 | Title "Environmental context-dependent memory: A review and meta-analysis" confirmed; 0.25 unconfirmed (secondary via Murre) | U | |
| Varied contexts plus retrieval practice | Smith & Handy 2014 | Exists (doi:10.1037/xlm0000019); finding unconfirmed | U | Face-name pairs only |

## 3. Tool facts I re-checked

| Tool | Checked facts | Note |
|---|---|---|
| KaTeX | 0.18.10, MIT, released 2026-09-30 | Matches G1 |
| MathLive | MIT; npm latest is now 0.111.0 | G1 said 0.110.0 (2026-06-09); a newer release landed. Pin a version |
| Mafs | 0.21.0, MIT, published 2024-10-20 | Stale but tiny; fork if needed |
| SymPy | 1.14.0, BSD, 2025-04-27 is still the latest on PyPI | Resolves G1's open question |
| RDKit.js | 2026.3.6, BSD-3-Clause | Matches G2 |
| 3Dmol.js | 2.5.5, BSD-3-Clause | Matches G2 |
| Harper | v2.11.0, 2026-09-16 | Matches G3 |
| Lean 4 | v4.34.1, 2026-09-24 | Matches G1 |
| PhET | Pre-2026-03-29 sims are CC BY 4.0 (PhET historical-licence page). States-of-matter repo is GPL-3.0 (GitHub API) | Current sims being CC BY-NC was read by G2 from a live sim header; I could not reach a page stating current terms. Treat as firm for personal use, unconfirmed for redistribution |

Not re-checked (taken from reports): licences and sizes of Ketcher, SmilesDrawer, JSXGraph, function-plot, OEWN, Wiktionary, CMUdict, LanguageTool, Giac, Symbolica.

## 4. Rules for Tutor, ranked by evidence times impact

1. **Attempt first, reveal second** (voice/chat prompt, UI). Every dictionary, etymology, IPA, hint and library lookup appears only after the learner's attempt. Basis: practice tests beat restudy (Adesope 2017, strong per G3; I did not re-open it), and active practice beats passive (Freeman, verified).
2. **Guided discovery, never "play around"** (prompt, feature). Require a prediction, run the sim or plot, then require an explanation of the gap. Basis: Alfieri and Lazonder, verified. Exception: for a topic the learner has not met, give a worked example first (Chernikova: low-prior learners benefit from examples; Sinha & Kapur: grade and domain moderate the prediction-first benefit).
3. **Machines judge correctness, not the LLM** (question generation, feature). Route algebra and calculus through a cached SymPy sidecar, structures through RDKit.js canonical SMILES or InChI, and units through UnitMath or `uom`. The sim and plot numbers come from code. Basis: engineering rationale plus PAL (unconfirmed by me). Highest impact, lowest evidence risk.
4. **Step-level items with a misconception field** (question generation). Generate each step, check it with SymPy, and verify that each distractor's algebra produces the stated wrong answer. Plan for an effect of about 0.3-0.4 SD, not 0.66 (college ITS evidence).
5. **A sim or viewer session is an encoding event, not an FSRS review** (scheduling). Give each topic 3-5 retrieval cards with randomised parameters, first due after one night. Grade "predict, then manipulate" cards on the prediction. This is a design inference from the testing effect, not a tested result.
6. **Fade scaffolds with mastery** (prompt, UI). After two unhelped correct answers, or once FSRS stability passes a threshold, hide hints and default to symbols first. Sequence: worked plot, blank plane, symbols only. Basis: expertise reversal (pattern confirmed to exist, size unknown).
7. **Theme the chrome only** (UI). Per-subject accent palette and icons on navigation, headers and progress. Question, answer, explanation, sim and plot panes stay neutral and identical across subjects. No mascots, decorative images or animated backgrounds. Basis: Cheng g = -0.16, Rey 2012, Alpizar d = 0.38 for functional colour.
8. **Text-free animation, voice narration** (UI, voice prompt). Animated views carry no paragraph text; the ElevenLabs channel narrates. Basis: Berney (unconfirmed figures, directionally consistent with Höffler & Leutner).
9. **Do not tune from satisfaction** (measurement). Deslauriers (verified) shows felt learning and actual learning diverge. Log accuracy, delayed retention and transfer only.
10. **Within-person experiments in Evals** (measurement). Randomise per item between interactive-first and symbol-first; log prediction correctness, time to first correct step and 7-day transfer. Separately log `theme_id` and `context_variant` per review and compare delayed FSRS recall (expect at most about 0.25, plausibly 0).
11. **Vary retrieval context by default** (scheduling, UI). Rotate voice, text and layout rather than binding items to one look. Evidence is thin (Smith & Handy unconfirmed), so keep it cheap.
12. **English tools** (feature, corpus). Harper on the learner's self-edited draft, logging accepted and ignored suggestions. Close reading with the CSS Custom Highlight API, fixed functional layers, and a required one-line own-words gloss per highlight (the gloss is a design inference). Annotations stored with W3C TextQuote plus TextPosition selectors.
13. **Corpus hygiene** (corpus). Build vocabulary items from OEWN (CC BY 4.0) plus Wiktionary etymology (CC BY-SA 4.0 and GFDL, so store source id and licence per item). Stream kaikki JSONL once into SQLite outside OneDrive. Include untrained-word items to measure transfer.
14. **GPU policy** (UI). CSS, SVG and Canvas2D by default. One WebGL context at a time, 3Dmol only behind a "Show 3D" button, rendering on change, torn down when hidden. Nothing was measured on this machine.
15. **PhET launcher** (feature). Local single-file HTML in an iframe, PhET logo and CC BY-NC attribution visible. Personal use only. Regular builds expose no event data, so measure through Tutor-side prediction logs.

## 5. Conflicts and how I resolved them

| Conflict | Resolution |
|---|---|
| G1 rule 12 styles plots per subject; G2 and G3 say chrome only | Chrome only. Plot and sim panes use one neutral style with functional colour. The evidence supports signalling, not decoration |
| Seductive-details harm (g = -0.16, verified) vs emotional design benefit (g+ 0.27-0.35, verified, contested) | Both are measured on graphics inside lessons, so neither covers app chrome. Take the conservative side for learning panes and allow chrome identity as a design choice with no claimed learning benefit |
| Simulation effect g = 0.85 (Chernikova) vs unassisted discovery d = -0.38 (Alfieri) | Not contradictory. The large figure is complex professional skills with scaffolding. Unassisted exploration is the weak arm. Stay with guided sims |
| ITS effect: 0.66 (Kulik), 0.01-0.09 (K-12 maths), 0.32-0.37 (college), about 0.76 (VanLehn) | The higher figures come from locally built tests and mixed controls. The college meta-analysis is the nearest population. Plan on 0.3-0.4 and let Evals decide |
| Prediction-first (G1, G2) vs grade and prior-knowledge moderation (Sinha & Kapur, Chernikova) | Prediction-first for topics where the learner has prerequisite knowledge; example-first for new ground |
| Per-subject theming as memory aid (owner's assumption) vs context evidence | Not supported. Smith & Vela d about 0.25 is unconfirmed and secondary, the replication failed (N = 16), and varied contexts may beat constant. Identity stays a design choice |
| MathLive version (0.110.0 vs 0.111.0) | Registry shows 0.111.0 today. Pin whichever is tested |
| Interactivity in general vs visualization (Freeman) | Freeman's +0.47 SD is for classroom active learning, not visual tools. Do not cite it for interactive plots |

## 6. What not to build

- Desmos or GeoGebra embeds (key-gated, non-commercial, network-dependent, terms unverified for Desmos).
- A browser-only Lean editor, or Lean as core. Offer an opt-in "Proof Lab" with a 560-810 MB toolchain, flagged experimental (evidence is one qualitative study).
- Default 3D or WebGL plotters, VR or AR labs, a custom physics or molecular dynamics engine.
- A home-grown or Rust CAS (SymPy sidecar instead; symengine crate stale since 2020; Giac is GPL-3).
- Learning-style profiles (Pashler 2008, verified).
- Free-explore sim rooms.
- Decorative per-subject backgrounds, mascots or animated characters in learning panes.
- A fixed theme marketed as a memory aid.
- Satisfaction-driven adaptation.
- Commercial redistribution of PhET, periodic-table data from Wikipedia-derived JSON, bundling eSpeak NG (GPL-3), LanguageTool n-grams by default (8 GB), the Hypothesis client, the 24 GB kaikki file, the stale etymology-db.

## 7. Citation list with verification status

V = verified, C = corrected, U = unconfirmed.

1. Hillmayr et al. 2020, Comput Educ, doi:10.1016/j.compedu.2020.103897. V
2. Freeman et al. 2014, PNAS, doi:10.1073/pnas.1319030111. V
3. Alfieri et al. 2011, J Educ Psychol, doi:10.1037/a0021017. V (OpenAlex returned a wrong abstract for this DOI; figures confirmed via Semantic Scholar)
4. Lazonder & Harmsen 2016, Rev Educ Res, doi:10.3102/0034654315627366. V
5. Sinha & Kapur 2021, Rev Educ Res, doi:10.3102/00346543211019105. C (moderators added)
6. Chernikova et al. 2020, Rev Educ Res, doi:10.3102/0034654320933544. V
7. Uttal et al. 2013, Psychol Bull, doi:10.1037/a0028446. V
8. Kulik & Fletcher 2016, Rev Educ Res, doi:10.3102/0034654315581420. V
9. Steenbergen-Hu & Cooper 2013, doi:10.1037/a0032447. V
10. Steenbergen-Hu & Cooper 2014, doi:10.1037/a0034752. V
11. VanLehn 2011, Educ Psychol, doi:10.1080/00461520.2011.611369. U (partly; abstract garbled)
12. Chan & Leung 2014, J Educ Comput Res, doi:10.2190/ec.51.3.c. V
13. Pashler et al. 2008, Psychol Sci Public Interest, doi:10.1111/j.1539-6053.2009.01038.x. V
14. Deslauriers et al. 2019, PNAS, doi:10.1073/pnas.1821936116. V (no numbers)
15. Cheng et al. 2026, Educ Psychol Rev, doi:10.1007/s10648-025-10099-z. V (g only)
16. Rey 2012, Educ Res Rev, doi:10.1016/j.edurev.2012.05.003. V (via ERIC snippet)
17. Alpizar, Adesope & Wong 2020, ETR&D, doi:10.1007/s11423-020-09748-7. V
18. Wong & Adesope, Educ Psychol Rev, doi:10.1007/s10648-020-09545-x. C (year 2021 in ERIC)
19. Murre 2021, R Soc Open Sci. V
20. Berney & Bétrancourt 2016, Comput Educ, doi:10.1016/j.compedu.2016.06.005. U (figures)
21. Kalyuga et al. 2003, Educ Psychol, doi:10.1207/s15326985ep3801_4. U (size)
22. Smith & Vela 2001, Psychon Bull Rev, doi:10.3758/bf03196157. U (0.25 is secondary)
23. Smith & Handy 2014, J Exp Psychol LMC, doi:10.1037/xlm0000019. U (finding)
24. Ponce, Mayer & Méndez 2022 (learner-generated vs instructor-provided highlighting), Educ Psychol Rev. Exists per ERIC; g 0.36/0.20/0.44 not seen. U
25. Adesope, Trevisan & Sundararajan 2017, Rev Educ Res, doi:10.3102/0034654316689306. Not re-checked (taken from G3)
26. Höffler & Leutner 2007, doi:10.1016/j.learninstruc.2007.09.013. Seen only as an ERIC snippet (d = 0.37, 26 studies, 76 comparisons). Partly V
27. Gao et al. 2023, PAL, arXiv:2211.10435. Not re-checked
28. Not re-checked (cited by reports, no load-bearing use here): Finkelstein 2005, Olympiou & Zacharia 2012, Stieff 2011, Goodwin & Ahn 2013, Colenbrander 2024, Shen 2025, Shin 2021, Fyfe 2014, Rau 2017, Ainsworth 2006, Fiorella & Zhang 2018, Thoma & Iannone 2021, Flynn & Featherstone 2017.

## 8. Open items for the owner or a later run

- Measure real GPU load of WebView2, 3Dmol and PhET on this machine.
- Runtime-check the CSS Custom Highlight API in the installed WebView2.
- Test whether Ketcher supports curved-arrow mechanisms before promising mechanism items.
- Read the full text of Berney & Bétrancourt, Kalyuga 2003 and Smith & Handy before quoting numbers.
- Confirm PhET's current CC BY-NC terms on a rendered page if the app might ever be sold.
- Check China's national education platform terms separately; nothing here supports "all open source".
