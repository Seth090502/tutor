# Baseline environment (recorded 2026-09-30, before any check ran)

## Repository
- Path: C:\Users\sethy\Projects\tutor
- Branch: custom
- Commit: d48dce1ff73cd5625b11fc90cd8594173d01ecdd (d48dce1f) "Integrate Languages: screen, provider, real speech commands, per-language config, word gloss route"
- `git status --short`: EMPTY (clean working tree) at start of baseline
- Upstream base ref ef2d9e5: exists (`git cat-file -t ef2d9e5` = commit)
- Existing `target/` directory before baseline: 31 GB

## Machine
- OS: Microsoft Windows 11 Home, version 10.0.26200 (build 26200)
- CPU: AMD Ryzen 7 9800X3D 8-Core Processor, 8 cores / 16 logical processors
- RAM: total 66,491,740 KB (63.4 GiB); free at start 30,547,584 KB (29.1 GiB)
- Disk C: used 1,958,801,031,168 B; free at start 40,473,735,168 B (37.7 GiB)

## Toolchain
- Python (repo venv, `.venv/Scripts/python`): 3.14.6
- Python (PATH `python`, used for legacy workspace): 3.14.6 (C:\Python314\python)
- Node: v25.9.0
- npm: 11.12.1
- rustc: 1.98.1 (48a229cea 2026-09-01)
- cargo: 1.98.1 (797e8a9bc 2026-08-05), invoked as C:\Users\sethy\.cargo\bin\cargo.exe (not on PATH)
- ruff, pytest, lint-imports: present in `.venv/Scripts`
- Env vars PYTHONUTF8, PYTHONIOENCODING, PROTOC: not set in the session
- `.cargo/config.toml` sets PROTOC to `crates/rag-engine/vendor/protoc/bin/protoc.exe` (file exists)

## Manifest versions
| Manifest | name | version |
|---|---|---|
| pyproject.toml (DeepTutor) | deeptutor | dynamic, from `deeptutor/__version__.py` = 1.6.12 (requires-python >=3.11,<3.15) |
| apps/desktop/package.json | desktop | 0.1.0 |
| apps/desktop/src-tauri/Cargo.toml | desktop (lib `desktop_lib`) | 0.1.0, edition 2021 |
| crates/rag-engine/Cargo.toml | rag-engine | 0.1.0, edition 2024, license "MIT OR Apache-2.0", publish = false |

Root Cargo workspace (Cargo.toml): resolver "3", members = ["apps/desktop/src-tauri", "crates/rag-engine"]. Shared Cargo.lock and target/.

## Frontend scripts (apps/desktop/package.json)
dev = vite; build = `tsc && vite build`; preview = vite preview; test = `vitest run`; tauri = tauri. There is NO lint script.

## Run method
Checks run one at a time by `scratch/driver.ps1` (cmd.exe `/c`, stdout+stderr merged to `<name>.log`, per-check timeout, process tree killed by PID on timeout). Machine-readable results: `scratch/results.jsonl`.

## State after all checks (recorded at the end of the baseline)
- `git status --short`: still EMPTY (npm build output `apps/desktop/dist`, `target/`, `data/` are ignored).
- `target/` grew from 31 GB to about 42 GB (du) after the debug test/clippy and release builds; free disk on C: dipped to 9.6 GB during `cargo build --release` (it was 37.7 GB before and about 69 GB at the end, other processes also write to this drive).
- Legacy workspace C:/Users/sethy/OneDrive/Desktop/Tutor: `data/progress.json` (560 bytes) was created by the requested `quiz` command. That workspace is not a git repository.
- Stale assertion-rewrite caches under `tests/**/__pycache__` embed the old repo path `C:\Users\sethy\Projects\deeptutor-fork\tests\...`, so pytest tracebacks show that path and `???` instead of source lines. Harmless for results.
- Extra supplementary runs kept in `scratch/`: diag-*.log (bisect and hypothesis checks), collect.txt, probe_killzero.py.
