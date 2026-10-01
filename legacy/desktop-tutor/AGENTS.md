# AGENTS.md — Tutor workspace rules

## Scope
Personal institutional-grade tutor. Optimize for: correctness, retrieval practice,
spaced repetition, citable grounding. No speculative features.

## Engineering
- Read code + callers before editing. Check `tutor/` modules first.
- Stdlib only for `tutor/` core (so CLI always runs). Optional deps go in scripts/.
- New components are Rust (cargo workspace at root; build output lives outside
  OneDrive per `.cargo/config.toml`). Corpus search is the `tutor-search` binary
  (`search-rs/`), so search/explain/eval/API need `cargo build --release` first.
- Minimum diff: every changed line traces to request. Match surrounding style.
- Never vendor copyrighted books/papers. Corpus stores original notes + metadata +
  links. Full text only for public-domain / CC-BY / your own notes.
- Never commit `data/` (progress is local). Never print secrets.

## Definition of done
- `cargo build --release` and `cargo test` pass (run first; Python search uses the binary).
- `python -m unittest discover -s tests -v` passes.
- `python -m tutor.cli quiz --domain math --n 2 --non-interactive` runs.
- `python scripts/build_index.py` runs and updates `corpus/index.json`.
- `python -m tutor.cli eval-search --k 3` stays at or above its last hit rate.
- Report what check ran and its result.

## Commands
- Quiz/test: `python -m tutor.cli quiz --domain <math|english|science|chemistry> --n 5`
- Review due: `python -m tutor.cli review`
- Build/test Rust (search binary + TUI): `cargo build --release`, `cargo test`
- Search corpus: `python -m tutor.cli search --q "<terms>"`
- Rebuild index: `python scripts/build_index.py`
- Clone teaching repos (opt-in, shallow): `python scripts/clone_repos.py --help`

## Corpus rules
- Add sources via `corpus/manifest.json` (id, title, license, url, local_path).
- Drop inbox files in `corpus/ingest/` then run indexer. Indexer never deletes source.
- Per-domain shelves `corpus/papers/<domain>/` and `corpus/books/<domain>/` hold
  own-notes metadata; PDFs stay local (gitignored). Shelves are EMPTY until populated.
- Question bank: `tutor/question_bank.py` (ITEMS list). Keep items atomic,
  one skill each, with explanation + misconception field.
- Eval discipline: `eval-search` supports `--split train|test --seed N`.
  Never tune against the test split; test split is held out.
