# Failure summary (2026-09-30, commit d48dce1f, Windows 11, Python 3.14.6)

## Headline
- The full-suite command as specified (`pytest -q -p no:cacheprovider -rfE tests --junitxml=...`) does NOT complete: a KeyboardInterrupt aborts it after 7852 of ~8464 tests (exit code 2). This reproduced identically in a second run (same 77 failures, same interrupt point).
- Trigger (confirmed by bisect, mechanism NOT proven): running `tests/services/workspace/test_chat_workspaces.py::test_migration_blocks_new_runtime_contexts` followed by `tests/services/workspace/test_content_workspace.py` gives 12 passed and then a KeyboardInterrupt delivered asynchronously (with --full-trace it surfaced inside pytest's capture machinery during setup of test_content_workspace.py::test_pipeline_bound_exec_keeps_source_hidden_and_artifact_public; in the full run it surfaced at C:/Python314/Lib/asyncio/runners.py:133). Each file alone does not interrupt (chat file alone: 13 passed, exit 0; content file alone: 24 passed, 1 failed, exit 1). Suspect: that test runs inside `WorkspaceCatalog.maintenance()`, where `assert_available` (deeptutor/services/workspace/catalog.py:402) calls `os.kill(own_pid, 0)`; signal 0 is CTRL_C_EVENT on Windows (deeptutor/runtime/process.py documents that os.kill(pid, 0) is destructive on Windows). However a standalone probe (`os.kill(os.getpid(), 0)`, then a subprocess and asyncio.run) did NOT raise KeyboardInterrupt (scratch/probe_killzero.py), so the suspect is unconfirmed.
- To get a complete run, the baseline adds `--deselect tests/services/workspace/test_chat_workspaces.py::test_migration_blocks_new_runtime_contexts` (log `pytest-backend-complete.log`: 85 failed, 8311 passed, 68 skipped, 1 deselected, 461.8 s). The required-command run plus a run of the not-yet-executed files (`pytest-backend-remainder.log`: 7 failed, 614 passed, 3 skipped) gives the same coverage; the union of their failures is 84 tests, the complete run has those 84 plus 1 flaky test.
- Tracebacks in the pytest logs show `C:/Users/sethy/Projects/deeptutor-fork/tests/...` paths and `???` for source lines: stale assertion-rewrite .pyc files under tests/**/__pycache__ were compiled when the repo lived at deeptutor-fork. Cosmetic only (results are unaffected), but it hides source lines in tracebacks.
- Rust, frontend, evals, lint-imports, legacy CLI workspace: no failing tests. Lint: 1 ruff-check violation and 1 ruff-format file (listed at the bottom of failures.csv, suite `ruff-check` / `ruff-format`).

Total failing pytest tests recorded in failures.csv: 85 (77 in the required-command run, +7 only in the remainder run, +1 flaky only in the complete run). Tests whose file changed since upstream ef2d9e5: 1 (tests/services/session/test_sqlite_store.py::test_sqlite_store_migrates_legacy_chat_history_db).

## Groups by apparent root cause

| group | count | root cause |
|---|---|---|
| A | 25 | Turn executor raises WorkspaceError 'The selected workspace folder does not exist.' (deeptutor/services/workspace/service.py:210, called from session/turns/executor.py:910) |
| B | 10 | Windows path/command form differs from the POSIX literal the test expects |
| C | 3 | POSIX file permission bits not enforced on NTFS |
| D | 34 | POSIX shell/executables or POSIX venv layout unavailable on Windows |
| E | 4 | Default text encoding is cp1252 (PYTHONUTF8 unset): UnicodeDecodeError 'charmap' in open()/read_text() without encoding |
| F | 5 | Windows file locking / SQLite and multi-process behaviour |
| G | 4 | ':' in file names (session keys like 'telegram:42' / 'bot1:...' used as file names; ':' is invalid on NTFS) |
| total | 85 | |

### Group A (25): Turn executor raises WorkspaceError 'The selected workspace folder does not exist.' (deeptutor/services/workspace/service.py:210, called from session/turns/executor.py:910)

The turn ends in an error event, so assertions see ['session','error','done'] or a missing assistant message. Verified by captured log for 19 of the 25 tests (the 6 consultation[0-5] params and the regenerate test have the same assertion signature but my log-section regex did not match their headers). Why the built-in general workspace folder is missing under the test setup was NOT isolated (catalog.py `_ensure_builtin_workspaces` creates it via `ensure_private_directory`); could be Windows-specific or a custom-branch regression.

- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_replays_events_and_materializes_messages[consultation0]` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_replays_events_and_materializes_messages[consultation1]` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_replays_events_and_materializes_messages[consultation2]` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_replays_events_and_materializes_messages[consultation3]` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_replays_events_and_materializes_messages[consultation4]` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_replays_events_and_materializes_messages[consultation5]` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_persists_private_provider_response_state` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_persists_llm_selection_in_turn_snapshot` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_session_persona_persists_falls_back_and_clears` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_allows_model_switching_within_same_session` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_bootstraps_question_followup_context_once` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_persists_deep_research_session_preference` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_turn_runtime_injects_memory_and_refreshes_after_completion` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_prior_image_attachments_are_reattached_on_the_next_turn` [pytest-backend]
- `tests/api/test_unified_ws_turn_runtime.py::test_reattaching_the_same_image_does_not_duplicate_it` [pytest-backend]
- `tests/app/test_multiworker_turn_application.py::test_remote_worker_subscribes_and_cancels_owner_turn` [pytest-backend]
- `tests/app/test_multiworker_turn_application.py::test_remote_worker_reply_reaches_owner_waiter` [pytest-backend]
- `tests/app/test_waiting_turn_recovery.py::test_live_owner_still_receives_replies` [pytest-backend]
- `tests/services/session/test_capability_routing.py::test_quiz_requests_stay_in_chat_by_default` [pytest-backend]
- `tests/services/session/test_capability_routing.py::test_enabled_explicit_quiz_routes_for_one_turn` [pytest-backend]
- `tests/services/session/test_capability_routing.py::test_auto_route_false_overrides_global_setting` [pytest-backend]
- `tests/services/session/test_capability_routing.py::test_per_turn_flag_opts_in_when_global_default_is_off` [pytest-backend]
- `tests/services/session/test_capability_routing.py::test_a_one_turn_quiz_leaves_the_conversation_in_chat` [pytest-backend]
- `tests/services/session/test_regenerate.py::TestRegenerateLastTurn::test_end_to_end_skips_memory_refresh_and_no_duplicate_user` [pytest-backend]
- `tests/services/session/test_turn_runtime_subscribe.py::test_reconnect_after_turn_completion_still_carries_message_ids` [pytest-backend]

### Group B (10): Windows path/command form differs from the POSIX literal the test expects

Backslash vs slash ('\tmp' vs '/tmp', 'outputs\chat...' vs 'outputs/chat/...'), drive-letter paths, `python` vs `python3` in exec command (test_builtin_tools), macOS/Linux-only PATH entries (/opt/homebrew/bin), bwrap mount paths.

- `tests/api/test_subagents_router.py::test_connect_list_and_disconnect_roundtrip` [pytest-backend]
- `tests/core/test_builtin_tools.py::test_exec_tool_runs_python_source_by_default` [pytest-backend]
- `tests/core/test_builtin_tools.py::test_exec_tool_runs_python_via_sandbox` [pytest-backend]
- `tests/core/test_builtin_tools.py::test_exec_tool_compiles_cpp` [pytest-backend]
- `tests/services/cli_apps/test_provider.py::test_the_apps_own_bin_comes_first_and_the_host_path_is_not_discarded` [pytest-backend]
- `tests/services/parsing/test_cache.py::test_load_ir_absolutizes_relative_img_paths` [pytest-backend]
- `tests/services/sandbox/test_sandbox.py::test_bwrap_mounts_only_the_active_virtualenv_read_only` [pytest-backend]
- `tests/services/sandbox/test_sandbox.py::test_bwrap_mounts_only_the_uv_python_runtime_roots` [pytest-backend]
- `tests/services/workspace/test_content_workspace.py::test_execution_results_expose_only_workspace_relative_paths` [pytest-backend-remainder]
- `tests/tools/builtin/test_read_skill_tool.py::test_missing_file_names_the_existing_skill` [pytest-backend-remainder]

### Group C (3): POSIX file permission bits not enforced on NTFS

Expected 0o700/0o600, got 0o777/0o666 (S_IMODE 438 == 0o666, 511/'777').

- `tests/services/cli_apps/test_state_and_paths.py::test_the_tree_is_created_owner_only` [pytest-backend]
- `tests/services/mcp/test_oauth.py::test_the_store_file_is_owner_only` [pytest-backend]
- `tests/video_learning/test_invidious_account.py::test_authorization_uses_minimal_scopes_and_one_time_state` [pytest-backend-remainder]

### Group D (34): POSIX shell/executables or POSIX venv layout unavailable on Windows

WinError 2 (file not found: printf/sh/bash-style executables), WinError 193 (running a shell script directly), shell-quoting fallback tests get empty stdout, venv `bin` directory is `Scripts` on Windows (cli_apps installer). test_argv_exec alone is 26 of the 34.

- `tests/runtime/test_macos_command_launcher.py::test_macos_command_launcher_forwards_home_and_arguments` [pytest-backend]
- `tests/scripts/test_pre_commit_hook.py::test_hook_stops_when_repo_hygiene_fails` [pytest-backend]
- `tests/services/cli_apps/test_installer.py::test_a_python_app_gets_its_own_venv_and_the_pinned_requirement` [pytest-backend]
- `tests/services/cli_apps/test_installer.py::test_a_failed_update_puts_the_working_install_back` [pytest-backend]
- `tests/services/cli_apps/test_installer.py::test_a_successful_install_records_what_it_can_be_checked_against` [pytest-backend]
- `tests/services/cli_apps/test_installer.py::test_a_floating_third_party_install_records_no_pin` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[; rm -rf /]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[$(whoami)]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[`id`]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[a b c]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[--flag=value with spaces]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched['quoted']` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched["double"]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[back\\slash]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[new\nline]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[$HOME]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[&& touch /tmp/pwned]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_local_backend_passes_a_hostile_argument_through_untouched[|tee /tmp/pwned]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[; rm -rf /]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[$(whoami)]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[`id`]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[a b c]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[--flag=value with spaces]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent['quoted']` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent["double"]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[back\\slash]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[new\nline]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[$HOME]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[&& touch /tmp/pwned]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_shell_fallback_is_quoted_well_enough_to_be_equivalent[|tee /tmp/pwned]` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_runner_prefers_argv_over_the_shell_string` [pytest-backend]
- `tests/services/sandbox/test_argv_exec.py::test_the_runner_does_not_expand_a_hostile_argv_element` [pytest-backend]
- `tests/services/sandbox/test_sandbox.py::test_runner_server_rejects_workdir_outside_allowed_roots` [pytest-backend]
- `tests/services/skill/test_skill_hub.py::test_command_provider_failure_cleans_up` [pytest-backend]

### Group E (4): Default text encoding is cp1252 (PYTHONUTF8 unset): UnicodeDecodeError 'charmap' in open()/read_text() without encoding

VERIFIED: with PYTHONUTF8=1 the same 4 tests (plus their file neighbours, 9 tests in total) pass (scratch/diag-utf8-charmap.log).

- `tests/services/test_task_models.py::test_every_task_model_call_site_names_a_kind` [pytest-backend]
- `tests/tools/test_code_execution_guidance.py::test_chat_prompts_delegate_tool_syntax_to_tool_guidance` [pytest-backend-remainder]
- `tests/tools/test_code_execution_guidance.py::test_solve_prompts_delegate_tool_routing_to_shared_guidance` [pytest-backend-remainder]
- `tests/tools/test_pptx_workspace_flow.py::test_pptx_dependency_is_packaged_for_pip_source_and_docker_runner` [pytest-backend-remainder]

### Group F (5): Windows file locking / SQLite and multi-process behaviour

WinError 32 (file in use) in test_data_migration (test_initialized_empty_feature_can_receive_migration fails in every run; test_task_board_export_and_migration_preserve_archived_cards failed only in the third full run = flaky), legacy chat_history.db still exists after migration (test_sqlite_store; the ONLY failing file that also changed since upstream), 'error:OperationalError' from concurrent processes (test_turn_repository), lost concurrent preference writes (test_setup_capability). Cause inferred from messages, not isolated.

- `tests/capabilities/test_setup_capability.py::test_concurrent_preference_writes_do_not_lose_updates` [pytest-backend]
- `tests/services/session/test_sqlite_store.py::test_sqlite_store_migrates_legacy_chat_history_db` [pytest-backend]
- `tests/services/session/test_turn_repository.py::test_only_one_process_can_begin_an_active_turn_per_session` [pytest-backend]
- `tests/services/workspace/test_data_migration.py::test_initialized_empty_feature_can_receive_migration` [pytest-backend-remainder]
- `tests/services/workspace/test_data_migration.py::test_task_board_export_and_migration_preserve_archived_cards` [pytest-backend-complete]

### Group G (4): ':' in file names (session keys like 'telegram:42' / 'bot1:...' used as file names; ':' is invalid on NTFS)

Inferred from test source (tests/services/memory/test_snapshot_adapters.py writes session 'telegram:42'); not verified.

- `tests/services/memory/test_snapshot_adapters.py::test_partner_sessions_become_tagged_entities` [pytest-backend]
- `tests/services/memory/test_snapshot_adapters.py::test_archived_sessions_included_and_flagged` [pytest-backend]
- `tests/services/memory/test_snapshot_adapters.py::test_missing_config_uses_dir_id_as_name` [pytest-backend]
- `tests/services/memory/test_snapshot_adapters.py::test_fingerprint_changes_when_conversation_grows` [pytest-backend]

## Flaky / run-dependent

- `tests/services/workspace/test_data_migration.py::test_task_board_export_and_migration_preserve_archived_cards`: failed (WinError 32) in the complete run, passed in the remainder run.
- A subset run of the group A/F files (scratch/diag-groupA.log) failed 26 once and 27 the next time, so at least one of tests/services/session/test_turn_repository.py, tests/capabilities/test_setup_capability.py or tests/app/* is timing-dependent.

## Not failures but worth knowing

- The required-command run and run 2 were byte-for-byte the same in counts (77 failed, 7710 passed, 65 skipped, then KeyboardInterrupt).
- legacy CLI eval-search: `hit@3 [all]: 54/60 = 90.0%` with misses math-04, math-05, eng-07, eng-11, sci-01, sci-11 (no earlier number to compare against; the Rust `tutor-search` binary of that workspace was not built here).
