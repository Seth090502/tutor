# Tutor V1.0 — Institutional-grade personal tutor workspace

Derivation (first principles): learning = curriculum (what + order) + corpus (ground truth)
+ teaching loop (explain -> retrieve -> feedback) + assessment (quiz/test) + memory
(spaced repetition + progress). This repo implements that loop with stdlib-only Python
so it runs immediately, and scales to 10k+ corpus docs via manifest + index (not via
vendored copyrighted books).

## Quick start

```powershell
cd C:\Users\sethy\OneDrive\Desktop\Tutor
python -m tutor.cli learn --domain math --topic quadratics
python -m tutor.cli explain --topic "photosynthesis"
python -m tutor.cli quiz --domain math --n 5
python -m tutor.cli popquiz --domain chemistry
python -m tutor.cli test --domain science --n 10
python -m tutor.cli review
python -m tutor.cli progress
python -m tutor.cli search --q "stoichiometry"
```

Python 3.10+ stdlib only, plus a Rust toolchain for corpus search: run
`cargo build --release` once (and after Rust changes) before `search`, `explain`,
`eval-search`, or the API.

## Layout

- `tutor/` — tutor stack (CLI, curriculum, question bank, quiz/test engine, SM-2, store)
- `corpus/` — knowledge-corpus framework: `manifest.json` + `domains/*.md` starter notes
  + `SOURCES.md` (legal open sources: OpenStax, MIT OCW, Khan, arXiv) + `ingest/` inbox
- `repos/` — agentic-teaching repo catalog (`catalog.json`, `CATALOG.md`) + safe shallow-clone script
- `scripts/` — `build_index.py`, `clone_repos.py`
- `tests/` — stdlib `unittest` checks
- `data/` — local-only progress + schedule (gitignored, never committed)

## What V1.0 is / is not

- IS: working tutor CLI, 60+ seeded questions across math/english/science/chemistry,
  Socratic explanations, quizzes/tests/pop-quizzes, SM-2 reviews, progress tracking,
  keyword search over local corpus, ingest pipeline for PDFs/markdown you legally own.
- IS NOT: a copy of Harvard/MIT libraries or 10,000 PhD papers. That request is not
  legally or physically doable in this folder (copyright + ~50–200 GB + OneDrive sync).
  See `corpus/SOURCES.md` and `corpus/manifest.json` for the legal scale path:
  index metadata/abstracts + link out, ingest open-access full text only.

## Daily loop (expedite learning)

1. `learn` or `explain` one topic (first-principles + 3 Socratic checks).
2. `quiz --n 5` immediately (retrieval).
3. `review` daily (spaced due items only).
4. `test --n 10` weekly per domain; `progress` to find weakest domain.
5. Drop new PDFs/notes in `corpus/ingest/`, run `python scripts/build_index.py`.

See `AGENTS.md` for contributor rules.

## V1.1 (unison: repos distilled, one API, Rust TUI + existing GUI)

- Agent lesson: `python -m tutor.cli agent --domain math`
- Local API (single contract for GUIs): `python -m tutor.cli serve --port 8797`
- Search eval: `python -m tutor.cli eval-search --k 3`
- Your papers (metadata-only, files untouched):
  `python scripts/ingest_papers.py --src "D:\\Papers" --limit 5000`
- Existing GUI first: `repos/open-webui-plugin/` (OpenAPI spec for Open-WebUI).
  Rust TUI: `ui-rust/` (`cargo run -p tutor-ui`). Distillations: `repos/ADAPTERS.md`.
