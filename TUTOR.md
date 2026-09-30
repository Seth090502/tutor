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
