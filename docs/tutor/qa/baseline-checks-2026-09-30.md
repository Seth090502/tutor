# Baseline checks (2026-09-30, commit d48dce1f, branch custom)

Every command ran from the directory in the `cwd` column, stdout and stderr merged into the log file, one at a time. Exit codes and wall-clock seconds come from `scratch/results.jsonl` (wall clock includes process start; pytest's own time is in the counts column).
`$CARGO` = `C:\Users\sethy\.cargo\bin\cargo.exe`. No environment variables were set for any run (PYTHONUTF8 unset).

| check | cwd | exact command | exit | counts as printed | wall s | log |
|---|---|---|---|---|---|---|
| pytest backend (as specified) | repo root | `.venv/Scripts/python -m pytest -q -p no:cacheprovider -rfE tests --junitxml=data/qa/baseline-2026-09-30/pytest-backend.xml` | 2 (interrupted) | 77 failed, 7710 passed, 65 skipped, 44 warnings in 399.43s; KeyboardInterrupt at asyncio/runners.py:133 after 7852 of 8464 tests | 404.7 | pytest-backend.log (+ .xml) |
| pytest backend, rerun of the same command | repo root | same, junit to pytest-backend-run2.xml | 2 (interrupted) | 77 failed, 7710 passed, 65 skipped, 44 warnings in 439.71s (identical failures) | 446.9 | pytest-backend-run2.log (+ .xml) |
| pytest backend, files not reached | repo root | `.venv\Scripts\python -m pytest -q -p no:cacheprovider -rfE <48 test files from tests/services/workspace/test_content_workspace.py onward, list in scratch/remainder-files.txt> --junitxml=data/qa/baseline-2026-09-30/pytest-backend-remainder.xml` | 1 | 7 failed, 614 passed, 3 skipped in 29.88s | 32.1 | pytest-backend-remainder.log (+ .xml) |
| pytest backend, complete (trigger deselected) | repo root | `.venv\Scripts\python -m pytest -q -p no:cacheprovider -rfE tests --deselect tests/services/workspace/test_chat_workspaces.py::test_migration_blocks_new_runtime_contexts --junitxml=data/qa/baseline-2026-09-30/pytest-backend-complete.xml` | 1 | 85 failed, 8311 passed, 68 skipped, 1 deselected, 40 warnings in 461.77s | 468.4 | pytest-backend-complete.log (+ .xml) |
| pytest evals | repo root | `.venv/Scripts/python -m pytest -q -p no:cacheprovider evals/tests` | 0 | 66 passed in 11.49s | 11.9 | pytest-evals.log |
| ruff check | repo root | `.venv\Scripts\ruff check .` | 1 | Found 1 error (I001 un-sorted imports, crates/rag-engine/tests/fixtures/make_fixtures.py:17:1, fixable) | 0.5 | ruff-check.log |
| ruff format | repo root | `.venv\Scripts\ruff format --check .` | 1 | 1 file would be reformatted (content/convert_tutor_content.py), 2036 already formatted | 0.2 | ruff-format.log |
| import-linter | repo root | `.venv\Scripts\lint-imports` | 0 | Contracts: 3 kept, 0 broken (1 ignored import in one contract) | 0.3 | lint-imports.log |
| cargo test | repo root | `$CARGO test -j 3 --workspace` | 0 | 182 passed, 0 failed (desktop_lib 55, desktop bin 0, job_object 1, rag_engine bin 121, service 4, survey 1, doc-tests 0); 0 compiler warnings | 193.7 | cargo-test.log |
| cargo clippy | repo root | `$CARGO clippy -j 3 --workspace --all-targets` | 0 | 0 warnings, 0 errors | 168.7 | cargo-clippy.log |
| cargo build --release | repo root | `$CARGO build --release -j 3` | 0 | Finished `release` in 25m 13s, 0 warnings | 1513.4 | cargo-build-release.log |
| npm run build | apps/desktop | `npm run build` (= `tsc && vite build`) | 0 | 3431 modules transformed, built in 661 ms; one vite warning: chunks larger than 500 kB | 5.3 | npm-build.log |
| vitest | apps/desktop | `npx vitest run` | 0 | 41 test files passed, 515 tests passed | 1.6 | vitest.log |
| npm lint | apps/desktop | not run: package.json has no lint script (scripts: dev, build, preview, test, tauri) | n/a | n/a | n/a | n/a |
| legacy unittest | C:/Users/sethy/OneDrive/Desktop/Tutor | `python -m unittest discover -s tests -v` | 0 | Ran 15 tests, OK | 1.3 | legacy-unittest.log |
| legacy quiz | same | `python -m tutor.cli quiz --domain math --n 2 --non-interactive` | 0 | quiz done: 2/2 grade A (writes data/progress.json in that workspace) | 0.1 | legacy-quiz.log |
| legacy eval-search | same | `python -m tutor.cli eval-search --k 3` | 0 | hit@3 [all]: 54/60 = 90.0%; misses math-04, math-05, eng-07, eng-11, sci-01, sci-11 | 0.1 | legacy-eval-search.log |

Not run: `python scripts/build_index.py` (excluded by the brief), `cargo build --release` in the legacy workspace (not in the brief, so the legacy `tutor-search` binary was not built by me).

## Failing-test bookkeeping
- Tests collected by `pytest --collect-only -q tests`: 8463 (scratch/collect.txt). The complete run executed 8464 + 1 deselected (2 more than the collect-only count; parametrised/dynamic tests, not investigated).
- Required-command run: 7852 executed. The 612 not reached are covered by the remainder run and the complete run.
- Union of failures of required run + remainder = 84 tests; complete run = those 84 + 1 flaky (`test_data_migration.py::test_task_board_export_and_migration_preserve_archived_cards`). `failures.csv` has all 85 plus the 2 lint rows.
