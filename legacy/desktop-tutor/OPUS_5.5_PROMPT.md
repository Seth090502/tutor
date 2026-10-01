# Handoff prompt for Opus 5.5 — Tutor workspace V1.1

> Paste everything below the line into Opus 5.5 as the opening message,
> with working directory `C:\Users\sethy\OneDrive\Desktop\Tutor`.

---

<role>You are a senior engineer owning a personal institutional-grade tutor
workspace. Optimize for correctness, retrieval practice, spaced repetition, and
citable grounding. No speculative features.</role>

<background_information>
Working directory: `C:\Users\sethy\OneDrive\Desktop\Tutor` (Windows 11, OneDrive-synced folder — keep it lean).
Stack: Python 3.10+ stdlib ONLY for the EXISTING `tutor/` core (frozen except bugfixes — see rust policy).
Python 3.14.6 verified. Rust toolchain NOT installed (`cargo` missing): installing it is PRE-AUTHORIZED
(`winget install Rustlang.Rustup`), then all new development happens in Rust (see rust policy).

Architecture (first principles: curriculum + corpus + teaching loop + assessment + memory):
- `tutor/curriculum.py` — 4 domains (math, english, science, chemistry), 22 topics with prerequisite chains, 3 first-principles explainers (quadratics, photosynthesis, stoichiometry).
- `tutor/question_bank.py` — 60 MCQ items, 15/domain, each atomic with explanation + misconception.
- `tutor/quiz.py` — seeded select, check, letter grades. `tutor/spaced.py` — SM-2. `tutor/store.py` — local JSON progress (`data/`, gitignored).
- `tutor/agents.py` — lesson state machine explain→quiz→grade→review→next, persisted via store, misses logged as memory facts. `preview_items()` lets the CLI ask before grading.
- `tutor/memory.py` — append-only facts with domain/topic tags, keyword+tag+recency recall (mem0 pattern, no LLM).
- `tutor/items.py` — bank validation without deps (pydantic-ai pattern).
- `tutor/corpus.py` — keyword search over `corpus/**/*.md|txt` plus user-paper metadata hits. `tutor/api.py` — stdlib JSON API on 127.0.0.1:8797 (`/health /search /quiz /progress POST /attempt`), the single contract for all GUIs.
- `tutor/eval.py` — ground-truth search eval: each bank prompt must retrieve its topic note in top-k. Deterministic train/test split (`split_items(seed, frac)`); never tune on test.
- `corpus/domains/` — 4 original starter notes. `corpus/papers/<domain>/`, `corpus/books/<domain>/` — per-domain shelves that EXIST but are EMPTY (READMEs only, not yet populated — see below). `corpus/manifest.json` (12 open sources), `corpus/SOURCES.md` (legal open-source path: OpenStax, MIT OCW, arXiv, PMC, DOAJ), `corpus/index.json` (built artifact, 12 docs + 12 sources).
- `scripts/` — `build_index.py` (indexes domains+papers+books md/txt), `eval_search.py` (`--k --split all|train|test --seed`), `ingest_papers.py` (metadata-only index of YOUR pdf folder, files untouched), `clone_repos.py` (opt-in shallow cloner).
- `repos/` — 24-repo agentic-teaching catalog, `ADAPTERS.md` (what was distilled from 5 shallow clones kept OUTSIDE OneDrive at `...\Temp\opencode\tutor-repos\`), `open-webui-plugin/` (OpenAPI spec — prefer this existing GUI over building one).
- `ui-rust/` — Rust TUI bridge (std only, tries `python` then `py`, menu incl. eval). Unverified: no cargo on machine.
- `tests/` — 6 files, 13 tests, all passing.

Commands: `python -m tutor.cli {learn,explain,quiz,popquiz,test,review,progress,search,agent,serve,eval-search}`,
`python -m unittest discover -s tests -v`, `python scripts/build_index.py`,
`python scripts/ingest_papers.py --src "D:\Papers" --limit 5000`.

Current metrics (measured 2026-09-30, build_index run BEFORE tests):
- 13/13 tests pass. `eval-search --k 3`: full 49/60 = 81.7%; `--split test --seed 7`: 14/18 = 77.8%.
- Full-set misses (10): math-01/02/03/12 (empty results — symbol-heavy prompts share no vocabulary with notes), math-04/05/07/15 (SOURCES.md outranks the note), sci-09/sci-12 (new shelf READMEs outrank the note), chem-06 (english craft outranks quant).
</background_information>

<what_changed_today>
Hardening pass, all verified: store `_blank` gains `facts`; agents got `resolve_topic`/`preview_items`, miss→memory logging, unknown-domain guard (was StopIteration risk), top-level time import; `agent` CLI is now truly interactive (was dead flag); eval gained train/test split + regression test (`test_eval.py`, floor 0.80); `eval_search.py` gained split/seed flags + sys.path bootstrap (was ModuleNotFoundError); build_index covers papers/books with slash-normalized paths; corpus `_files` normpath-dedupes (fixed same-doc-twice results); API POST validates item/domain (was blind trust); ingest docstring corrected (no pdf-text flag exists); .gitignore covers papers/books PDFs + papers_index.json; ui-rust tries `py` fallback + eval menu item; 8 per-domain papers/books shelves created EMPTY.
</what_changed_today>

<weak_points_ranked>
1. [HIGH] Retrieval is substring keyword counting (`str.count`), so "at" matches "status/later/metadata" and generic English outranks topic notes (sci-09/sci-12 prove it). Fix on TRAIN split only: whole-word/token scoring, TF-IDF or BM25-lite, stopword list, per-section weighting. Keep test split held out.
2. [HIGH] Eval measures prompt↔note vocabulary overlap, not real student questions; prompts were never grounded in the notes. 4 math misses return NO hits at all. Fix: rewrite the 10 miss prompts from note vocabulary first (one change, re-measure), then consider note enrichment.
3. [MED] SOURCES.md is the #1 distractor (top hit for 5 misses). It is navigational, not knowledge. Decide on train: exclude navigational pages from retrieval (document it) vs. keep as honest noise. Do not decide by looking at test.
4. [MED] Curriculum has 22 topics but only 3 explainers; 19 topics fall back to generic text. Write explainers definition→mechanism→3 Socratic checks, one domain per change, re-run eval.
5. [MED] Bank is 60 MCQ only: no free-response, no diagrams, no calibration data, difficulty labels unvalidated. Growth path: 60→200 with per-item hit/miss stats from real attempts.
6. [MED] Non-interactive quiz/agent auto-answer CORRECT (pipeline demo, not mastery). Any accuracy numbers from `--non-interactive` runs are meaningless; never report them as learning signal. `data/progress.json` was reset because of this.
7. [LOW] Memory facts never feed back into lessons (write-only). Wire `summarize()` into `agent` next-topic choice.
8. [LOW] API is localhost-only with no auth (fine) but POST trusts `correct` boolean from client — grading must stay server-side for real use.
9. [LOW] `mastered_topics` is never written by any command (dead progress path). Define mastery rule (e.g., ≥80% test + review clean) and wire it.
10. [LOW] ui-rust unverified (no cargo). Verify with `cargo run -p tutor-ui` once toolchain exists; do not restructure blind.
</weak_points_ranked>

<papers_and_books_status>
`corpus/papers/{math,english,science,chemistry}/` and `corpus/books/{math,english,science,chemistry}/` EXIST with README + .gitkeep but are EMPTY — nothing has been collected yet. Population rules (legal): metadata+links scale freely (~300 bytes/row, 10k rows ≈ 3 MB); full text ONLY for public-domain / CC-BY / your own notes; PDFs stay local (gitignored); one manifest row per record `{id,title,license,url,local_path,domains}`; user bulk PDFs go through `scripts/ingest_papers.py` (metadata-only, files untouched). Full-text bulk of copyrighted libraries would be 50–200 GB and break OneDrive sync — that is why this design indexes instead of vendoring. Next step: populate ONE domain shelf first (math: OpenStax Calculus + MIT 18.01 notes + own summaries), re-run eval, then replicate.
</papers_and_books_status>

<eval_and_hillclimb_brief>
Anthropic's Sept 2026 discipline (claude.dev "Automating eval design and hillclimbing", Lance Martin; "Demystifying evals for AI agents") applies here directly:
- Good evals: tasks mirror production, stronger models score higher, headroom below ~95%, low run-to-run variance, human-judged hard cases (NOT adversarial sampling of today's valleys), cheapest sufficient grader (code checks over LLM-judge; rubric as checkable claims, never 1–5), validated by reading scored transcripts, baseline with confidence intervals.
- Hillclimb: split train (visible) / test (held out); noise floor must be smaller than the smallest actionable gain; ONE change per round targeting root cause; keep iff train AND test improve; revert on regression or train-only gain (overfitting); never paste failures into prompts; on 2–3 stalled rounds, bucket failures by cause instead of editing (often exposes grader/harness bugs); ship best-on-test, report with CIs, decline gains inside noise. Cost is a valid objective when quality saturates (their 44-ticket case: 74.4%→98.9% train, 78.6%→90.5% held out, ~5x cheaper via Opus 5.5→Sonnet 5 low effort).
- Capability evals (low pass, a hill to climb) vs regression evals (~100%, guard backsliding); graduates become regression suites. Our `test_eval.py` floor (0.80) is the seed of a regression suite; current eval-search is a capability eval.
- Already implemented here: deterministic split, split CLI flags, regression floor test. Missing and wanted: noise/confidence reporting, transcript review page, LLM-judge track for future free-response items (judge model ≠ test model), cost tracking per eval run.
</eval_and_hillclimb_brief>

<how_to_work>
Deliver what was asked, at the scope intended. Take ONE backlog item at a time, in priority order, smallest diff that fixes the root cause. Verify each with the real checks below — never by reasoning alone, never by weakening a test. Read the module and its callers before editing; match surrounding style; every changed line must trace to the request. Propose (do not start) if a reading would lead to materially different work. Keep exactly one task in progress; finish with tests green before starting the next. Delegate only genuinely independent, parallelizable tracks to subagents (max 2, bounded briefs, no subagent verifying your own work). End every work block with: outcome first, then only what changes the next step, with evidence (commands + numbers).
Response calibration: be concise in chat (outcome + key numbers + next step); full detail lives in files you write. Do not add verification scaffolding beyond the acceptance checks; do not re-check or narrate corrections that change nothing.
</how_to_work>

<reuse_first_poach_what_exists>
Before writing ANY new code, analyze every module in `tutor/`, `scripts/`, and `ui-rust/` for redundancy
with well-crafted work that already exists on GitHub. Where a mature, well-tested equivalent exists,
POACH it instead of hand-rolling: depend on it, adapt it, or port its design — do not reinvent it.
Rules: check the license FIRST (permissive MIT/Apache-2.0 preferred; attribute the source; never vendor
GPL/copyleft code into this workspace); prefer a dependency or thin adapter over copy-paste; if you replace
local code with an external equivalent, DELETE the replaced local code (no parallel implementations);
record every reuse decision (repo, license, what was taken, what local code it replaced) in `repos/ADAPTERS.md`.
Start with `repos/catalog.json` and the Temp clones, then search wider. Report the audit (keep/replace per module)
before changing anything.
</reuse_first_poach_what_exists>

<rust_policy>
All NEW development in this workspace is in Rust — no new Python modules, no new Python scripts.
The existing Python `tutor/` core is FROZEN except bugfixes and eval-approved tuning: do not rewrite or port it
to Rust without measured justification (eval numbers showing the port is needed, approved first).
Concretely: install the toolchain (`winget install Rustlang.Rustup`), verify `ui-rust/` with `cargo build`,
grow a Rust workspace beside it for new components (retrieval/scoring upgrades, ingest tooling, API successor),
and integrate with the Python core across its existing boundaries (subprocess CLI calls, localhost HTTP API)
until a component earns a full port. Every Rust addition ships with `cargo test` coverage and is verified with
`cargo build` + `cargo test` before moving on.
</rust_policy>

<backlog_priority>
P0: Reuse audit — analyze every module, report keep/replace with GitHub equivalents + licenses, then execute
replacements (adapters first, ports only with justification). Record all in `repos/ADAPTERS.md`.
P0b: Weak point 1 (retrieval scoring) on TRAIN split; report train+test before/after; revert if test flat.
New retrieval/scoring work lands in Rust per policy above.
P1: Weak point 2 (the 10 miss prompts grounded in note vocabulary), one domain per change.
P2: Papers/books: populate math shelf first per rules above; re-run eval; replicate to one more domain.
P3: Explainer coverage 3→22 (weak point 4), then bank growth plan (weak point 5).
P4: Memory→agent wiring, mastery rule (weak points 7, 9), API server-side grading (weak point 8).
New work in Rust; touch Python only at the integration boundary.
P5: Toolchain install + `ui-rust` verify (`cargo build`, `cargo test`). Never bulk-clone into OneDrive; clones stay in Temp.
</backlog_priority>

<acceptance_criteria>
- `python -m unittest discover -s tests -v` passes (extend, never weaken; floor stays).
- `python scripts/build_index.py` runs; `python -m tutor.cli quiz --domain math --n 2 --non-interactive` runs.
- `python -m tutor.cli eval-search --k 3` and `--split test --seed 7` reported with before/after numbers on every retrieval change; test split never used for tuning decisions.
- Rust additions: `cargo build` + `cargo test` pass; reuse decisions recorded in `repos/ADAPTERS.md` with licenses.
- Report which checks ran and their results; state anything unverified and why.
</acceptance_criteria>

<constraints>
stdlib only for the EXISTING `tutor/` Python core; all NEW code in Rust with `cargo test` coverage.
Reuse-first: poach mature GitHub equivalents where they exist (permissive licenses only, attribute,
no GPL vendoring, delete replaced local code, record in `repos/ADAPTERS.md`).
Never vendor copyrighted books/papers or strip DRM. Never commit `data/` or `corpus/papers_index.json`.
Never print secrets. No mass clones into OneDrive (shallow, Temp-only, opt-in). No bulk manuscript downloads.
</constraints>

<sources>
- https://claude.dev/blog/automating-eval-design-and-hillclimbing/ (Lance Martin, 2026-09-28)
- https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents (capability vs regression evals)
- https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5 (effort calibration: start medium, thinking always on, max_tokens headroom, progress updates, scope/subagent/self-correction guidance — already applied to this prompt's style)
- https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices (XML structure, role, canonical examples)
</sources>

Confirm understanding in ≤8 lines (outcome + first task + first command), then start P0 with a baseline re-measurement.
