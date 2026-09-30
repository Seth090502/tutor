# QA report

Every check that was actually run, with the exact command, result and artifact. Logs that could contain
local paths or data live under `data/qa/` (gitignored, never committed); this file records what they show.
A check that was not run is listed as not run, with the reason. Test counts are copied from the command
output, never estimated.

## Phase 0 baseline (2026-09-30, `custom` d48dce1f)

Logs: `data/qa/baseline-2026-09-30/` in the main checkout (not committed). Committed copies:
`qa/baseline-checks-2026-09-30.md` (every command, exit code, counts, duration), `qa/baseline-env-2026-09-30.md`,
`qa/baseline-failures-2026-09-30.csv` (every failing test id), `qa/baseline-failures-summary-2026-09-30.md`,
`qa/licenses/` (Rust 856 crates, npm 564 packages, Python 212 distributions, with a summary).

Environment: Windows 11 Home 10.0.26200, AMD Ryzen 7 9800X3D (8 cores, 16 threads), 63.4 GiB RAM, Python
3.14.6, Node 25.9.0, npm 11.12.1, rustc and cargo 1.98.1. DeepTutor 1.6.12, desktop 0.1.0, rag-engine 0.1.0.

| Check | Command | Result |
|---|---|---|
| Backend, as specified | `.venv/Scripts/python -m pytest -q -p no:cacheprovider -rfE tests` | Aborts with KeyboardInterrupt after 7,852 of 8,464 tests, twice (77 failed, 7,710 passed, 65 skipped) |
| **Backend, complete (the comparison command from now on)** | the same with `--deselect tests/services/workspace/test_chat_workspaces.py::test_migration_blocks_new_runtime_contexts` | 85 failed, 8,311 passed, 68 skipped, 1 deselected (461.8 s) |
| Evals | `pytest -q evals/tests` | 66 passed |
| ruff | `ruff check .`; `ruff format --check .` | 1 import-order error in `crates/rag-engine/tests/fixtures/make_fixtures.py`; 1 file to reformat (`content/convert_tutor_content.py`) |
| lint-imports | `lint-imports` | 3 contracts kept |
| Rust | `cargo test -j 3 --workspace`; `cargo clippy -j 3 --workspace --all-targets`; `cargo build --release -j 3` | 182 passed; 0 warnings; built (25 min 13 s) |
| Frontend | `npm run build`; `npx vitest run` | ok; 41 files, 515 tests passed |
| Legacy CLI (OneDrive workspace) | `python -m unittest discover -s tests -v`; `tutor.cli quiz --domain math --n 2 --non-interactive`; `tutor.cli eval-search --k 3` | 15 OK; ran; hit@3 90.0% (54/60), the recorded baseline |

**Accepted baseline failures** (85; ids in `qa/baseline-failures-2026-09-30.csv`). 84 are in test files unchanged
since upstream v1.6.12; the exception is `tests/services/session/test_sqlite_store.py::test_sqlite_store_migrates_legacy_chat_history_db`.
None is in a path this work changes. Groups:

| Group | Count | Cause | Shipping relevance |
|---|---|---|---|
| A. Chat turn runtime: "The selected workspace folder does not exist" | 25 | Not isolated (Windows-specific or a fork regression) | **Open.** The DeepTutor chat surface; investigate before any release that ships chat |
| B. Windows path and command forms vs POSIX literals | 10 | Test expectations | Low |
| C. POSIX permission bits on NTFS | 3 | Platform | Low (owner-only files rely on the user profile ACL on Windows) |
| D. POSIX shells, executables, venv layout | 34 | Platform | Low for the desktop app; the sandbox/CLI-app installers are not used |
| E. cp1252 default encoding without PYTHONUTF8 | 4 | Environment; pass with PYTHONUTF8=1 (the shell sets it) | None |
| F. Windows file locking, SQLite, multi-process | 5 | Platform; one flaky | Medium: watch `test_turn_repository` and data migration |
| G. `:` in file names | 4 | Windows file names | Low (partner channels are not used) |

The full suite's self-interruption (a KeyboardInterrupt surfacing after `test_migration_blocks_new_runtime_contexts`
followed by `test_content_workspace.py`) is unexplained; the suspect is `os.kill(pid, 0)` in `catalog.py:402`,
which is a CTRL_C_EVENT on Windows. Open follow-up.

**Licences flagged** (`qa/licenses/licenses-summary.md`): PyMuPDF 1.28.2 is AGPL-3.0 or Artifex commercial (a
direct core dependency: a blocker for any closed commercial distribution unless replaced or licensed); pyte
(LGPL-3.0), python-telegram-bot (LGPL-3.0, partners extra), qq-botpy (custom), tiktoken (non-SPDX field, MIT
text); MPL-2.0 transitives (certifi, bidict, tqdm, lightningcss, cssparser family, option-ext); webgl-constants
without a licence field. The workspace crate `desktop` has no licence field.

## Phase 1 increments

Runs from the `wt/loop` worktree root with the main checkout's venv
(`C:/Users/sethy/Projects/tutor/.venv/Scripts/python`).

| Date | Commit | Command | Result |
|---|---|---|---|
| 2026-09-30 | d48dce1f (`custom`) | `npm run build`, `npx vitest run` (apps/desktop) | build ok; 41 files, 515 tests passed |
| 2026-09-30 | d48dce1f | `cargo test -j 3 -p desktop` | 55 + 1 passed |
| 2026-09-30 | d48dce1f | `pytest` on languages, practice, library, reading and guided-reading suites (11 paths) | 311 passed |
| 2026-09-30 | d48dce1f | `lint-imports` | 3 contracts kept |
| 2026-09-30 | aaa33173 (`wt/loop`) | `pytest tests/api tests/services/languages tests/services/reading` | 936 passed, 3 failed before fixes: route surface (new prefix, fixed by moving to `/api/study`), guided reading (encoded the provisional-in-queue bug; test updated to the C11 amendment), `test_subagents_router.py::test_connect_list_and_disconnect_roundtrip` (`\tmp` vs `/tmp`, a Windows path failure unrelated to this change; pending baseline confirmation) |
| 2026-09-30 | aaa33173 | `pytest tests/api/test_learning_loop.py tests/api/test_practice_attempts.py tests/api/test_practice.py tests/api/test_practice_languages.py tests/api/test_guided_reading.py tests/api/test_canonical_route_surface.py tests/api/test_languages.py` | 166 passed |
| 2026-09-30 | 664d5e3f | `npx vitest run src/lib/study.test.ts` | 8 passed |
| 2026-09-30 | eb63d27e | `pytest tests/services/library tests/api/test_library_router.py` | 129 passed |
| 2026-09-30 | 6ab54e75 | `pytest tests/api/test_practice_attempts.py tests/api/test_learning_loop.py` | 59 passed |
| 2026-09-30 | cad1a661 | `pytest tests/api/test_learning_loop.py`; `npx vitest run src/lib/study.test.ts` | 26 passed; 8 passed |

Ruff (`ruff check`, `ruff format --check`) is clean on every file changed in these commits.

## Contract changes that changed tests

- `tests/api/test_practice_attempts.py`: rows of the rating-policy table and three tests moved from policy 1
  (hint rung 1 = Good, rungs 2-3 = Hard) to review policy 2 (any hint = Again). The mandate and EV-002
  require it; the old expectations encoded the superseded rule.
- `tests/api/test_guided_reading.py`: provisional atoms are now excluded from the practice queue, as the
  C11 amendment specified; the old assertion encoded the bug.

## Phase 0 boundary fixes (`wt/harden` f587c0f0, merged into `wt/loop`)

Worker-reported and re-run by the lead on `wt/loop` b5a7fbae:
- `pytest tests/services/config tests/api/test_request_guard.py tests/api/test_settings_router.py`: 406 passed.
- `cargo test -j 3 -p rag-engine -p desktop`: desktop 56 + 1, rag-engine 125 + 5 + 1 (service) passed;
  `cargo clippy -j 3 -p rag-engine -p desktop --all-targets`: no warnings. (Built with `CARGO_TARGET_DIR`
  pointing at the `wt/harden` cache, `PROTOC` and `PDFIUM_DYNAMIC_LIB_PATH` at the main checkout's vendor folders.)
- The worker showed the new tests fail against d48dce1f (22 Python failures; the rag-engine `foreign_pages_are_refused`
  test got 200 for a foreign Host) and pass after.

## Phase 1 walkthrough (task P1-6, 2026-09-30, `wt/loop` 6acfdf62)

A real backend from `wt/loop` on a scratch data folder (`wt/loop/data`, never the owner's), three items seeded
through the real import API, Vite, and headless Edge driven over the DevTools protocol. Screenshots and page-text
dumps: `wt/loop/data/qa/p1-walkthrough/` (not committed).

- All 9 steps passed: the maths item wrong then repaired with a hint (closed "Correct on a retry", rated Again);
  the Newton free-response self-assessed (Good, `self_report`); the Spanish production card correct (Good).
- The page text before release never contained the explanation, the key or unrequested hints.
- Reviews recorded: `again/verified`, `good/self_report`, `good/verified`, one per attempt.
- Limits: in a plain browser the Tutor screen needs Tauri, so the walkthrough stubbed `window.__TAURI_INTERNALS__`
  (no call, no key); fonts fell back because the worktree's `node_modules` is a junction outside Vite's allow list.
- Findings fixed in b6fd7118: the learner's own answer is now shown at self-assessment; the retry field has a
  visible label. Open (cosmetic): the verdict pill appears twice on a closed card.

## Not yet run

- The same walkthrough inside the real Tauri shell (`npm run tauri dev`) with a live ElevenLabs call: needs the
  owner's key and approval for paid voice minutes.
- Real-provider tests: no keys or budget in this session.
