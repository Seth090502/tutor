# QA report

Every check that was actually run, with the exact command, result and artifact. Logs that could contain
local paths or data live under `data/qa/` (gitignored, never committed); this file records what they show.
A check that was not run is listed as not run, with the reason. Test counts are copied from the command
output, never estimated.

## Phase 0 baseline (2026-09-30, `custom` d48dce1f)

Environment, full suites and the named failure list: pending the baseline worker
(`data/qa/baseline-2026-09-30/`).

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

## Not yet run

- Desktop walkthrough with screenshots (task P1-6): waits for the Tutor and Languages integrations.
- Real-provider tests: no keys or budget in this session.
