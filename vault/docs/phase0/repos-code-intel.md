# Code intelligence cluster: definitions, references, call graphs

Phase 0 read-only code study for the libraryd vault. Worker report, written 2026-10-01.
Cluster: precise and fuzzy code intelligence (SCIP and its indexers, stack-graphs, tree-sitter tags, Kythe, Glean, Serena, multilspy, Joern) plus three extras (universal-ctags, clangd, mcpls).

Method and limits. Every repository was shallow-cloned to `C:/Users/sethy/Projects/refs/code/<name>` and read with Read/Grep/Glob. Nothing was built or run. Claims about runtime behaviour (for example how tree-sitter-cpp parses Unreal macros) are therefore marked "unverified" where I only know them from reading. Where a path has no line range it is because I read the section by heading; the file path is exact. Paths below are relative to the clone folder named in each section. clangd was studied through `gh api` (llvm-project is far above the 1.5 GB budget).

## 1. Snapshot

| Repo (canonical URL) | HEAD (short) | Last commit | Status on 2026-10-01 | Licence (from LICENSE file) | Flag |
|---|---|---|---|---|---|
| oraios/serena | d0f7f92 | 2026-09-30 | very active, 29.9k stars; `main` is 2.0.0.dev0 | per-component: `src/solidlsp` MIT, everything else GPL-3.0-or-later (`LICENSE:4-5, 22-46`); last MIT release v1.7.0 (`LICENSE:64-66`) | **GPL-3.0-or-later app** |
| microsoft/multilspy | b9a0ef1 | 2026-08-20 | alive but quiet; Serena's SolidLSP descends from it (`serena/docs/01-about/050_acknowledgements.md:26`) | MIT | none |
| scip-code/scip (moved from sourcegraph/scip) | 9c4a0a3 | 2026-09-29 | active, CLI v0.10.0, now under neutral `scip-code` org | Apache-2.0 | none |
| sourcegraph/scip-clang | 90dbe3f | 2026-03-24 | beta (v0.4.0), 6 months since last commit | Apache-2.0 | Linux/macOS only |
| sourcegraph/scip-typescript | e22988d | 2026-09-11 | active | Apache-2.0 | none |
| sourcegraph/scip-python | 8b60bbc | 2025-09-05 (default branch `scip`) | stale (>1 year on default branch), Pyright fork | MIT (Pyright's text, `LICENSE.txt:1-3`); GitHub reports NOASSERTION | none |
| scip-code/scip-java (moved) | 1ba0342 | 2026-09-26 | active, status badge "development" | Apache-2.0 | needs Gradle/Maven/Bazel build |
| sourcegraph/scip-dotnet, scip-code/scip-rust | not cloned (read via `gh api`) | scip-dotnet pushed 2026-09-30 | active | Apache-2.0 | scip-rust is a wrapper over `rust-analyzer scip` |
| github/stack-graphs | fcb7705 | 2025-09-09 | **archived**; README says "no longer supported or updated by GitHub" (`README.md:1`) | Apache-2.0 OR MIT (`LICENSE-APACHE`, `LICENSE-MIT`) | archived |
| tree-sitter/tree-sitter (tags crate) | 3643db1 | 2026-10-01 | very active, v0.28.0 (`Cargo.toml:25`) | MIT | none |
| kythe/kythe | 69141f0 | 2026-09-18 | alive, slow cadence; v0.0.76 on 2026-07-16 (`RELEASES.md:3`) | Apache-2.0 | Bazel-centric |
| facebookincubator/Glean | 20db446 | 2026-09-30 | active | BSD-3-Clause style ("Neither the name Facebook...", `LICENSE:17`); GitHub reports NOASSERTION | Linux-only, Haskell |
| joernio/joern | 98a815c | 2026-09-30 | active | Apache-2.0 (`LICENSE`, `build.sbt:154`); c2cpg bundles a re-published Eclipse CDT under **EPL-2.0** (`joern-cli/frontends/c2cpg/eclipse-cdt/CCorePlugin.java:5-9`) | JVM, memory hungry |
| Extra: universal-ctags/ctags | 99b7257 | 2026-09-22 | active | **GPL-2.0-or-later** (`COPYING`, `main/main.c:7-8`) | GPL |
| Extra: clangd (llvm/llvm-project, via gh api) | n/a | repo pushed 2026-10-01; clangd release 23.1.0 on 2026-09-02 | very active | Apache-2.0 WITH LLVM-exception (header of `clang-tools-extra/clangd/index/dex/Dex.h`) | none |
| Extra: bug-ops/mcpls | 0ddbca9 | 2026-09-30 | active, 74 stars, Rust | Apache-2.0 OR MIT | none |

Scouted but not studied (to avoid scope creep): SimplyLiz/ckb (SCIP-backed MCP, licence NOASSERTION), blackwell-systems/agent-lsp (MIT, 65 tools), jonrad/lsp-mcp (MIT, stale since 2025-03).

## 2. Verdict table

| Project | Fit | One-line reason |
|---|---|---|
| Serena | port_ideas | Best agent-facing design in the cluster (name-path addressing, output-shortening ladder, REPL batching, Unreal guide). App code is GPL-3.0-or-later: copy ideas, not code. |
| multilspy | reference_only | Thin Python LSP client; superseded in practice by SolidLSP. |
| SCIP protocol + `scip` Rust crate | **adopt_dependency** | The one precise-index interchange format with Rust bindings, a relationship model, roles bitset and C++ macro/forward-declaration vocabulary. Use it as the import format for the "precise tier". |
| scip-clang | reference_only | Best C/C++ precision, but Linux x86_64 and macOS arm64 binaries only and needs a built tree plus compile_commands.json. Cannot index Unreal on this Windows machine. |
| scip-typescript / python / java / dotnet / rust | reference_only | Optional producers the owner can run themselves; scip-dotnet (C#) and rust-analyzer scip (Rust) matter for the target. |
| stack-graphs | port_ideas | Right idea (build-free scope-based resolution), but archived, pinned to tree-sitter 0.24, no C/C++/C#/Rust/Lua rules. Do not depend on it. |
| tree-sitter + tags | **adopt_dependency** (tree-sitter crate, optionally tree-sitter-tags) | Foundation of the fuzzy tier. Shipped C/C++ tags have no reference captures, so the vault must author its own queries. |
| Kythe | reference_only | Rich schema (ref/call, ref/expands, claiming) but needs Bazel-extracted compilation units; no Windows story in docs. |
| Glean | reference_only | Fact-ownership incrementality and the Glass symbol API are worth copying. Linux-only Haskell system; C++ indexer tested on clang 11/12. |
| Joern | reference_only | Build-free tolerant C/C++ parse (Eclipse CDT) with macro handling; far too heavy (JVM, tens of GB for big trees) and aimed at vulnerability research. |
| universal-ctags | port_ideas | Best-in-class macro neutralisation for fuzzy C/C++ (`-D` macro definitions). GPL-2.0+, so do not link; borrow the idea. |
| clangd | reference_only now, optional sidecar later | The engine under Serena for C/C++; Apache-2.0 so spawning it is licence-safe. |
| mcpls | reference_only | Rust LSP-to-MCP bridge with position-based tools; a model for a later Rust sidecar. |

## 3. Per-repository findings

### 3.1 oraios/serena (the main agent-facing example)

What it is. An MCP server ("the IDE for your coding agent", `README.md:9`) that gives agents symbol-level retrieval, editing and refactoring by driving language servers over LSP (or, as a paid alternative, a JetBrains plugin). Python 3.11+, `pyproject.toml` version 2.0.0.dev0. Licence split as in the snapshot table. The GPL switch is recent and not retroactive (`LICENSE:51-66`, `CHANGELOG.md:1-12`).

Ingestion. There is no crawler of its own for semantics. Files are discovered by walking the tree with `os.listdir` and skipping ignored paths (`src/solidlsp/ls.py:2092-2164`, `request_full_symbol_tree`). Ignore rules: language-specific directory defaults, `.gitignore` honoured by default (`src/serena/config/serena_config.py:335`, `ignore_all_files_in_gitignore: bool = True`; `src/serena/project.py:94`), plus user `ignored_paths` compiled with `pathspec` GitWildMatch (`ls.py:581-582`). Language detection is by file extension through each language server's source-file matcher (`ls.py:538`). Unreal special case: when a `*.uproject` exists, the clangd and ccls adapters skip `Binaries`, `DerivedDataCache`, `Intermediate`, `Saved` (`src/solidlsp/language_servers/common.py:188-195`, `clangd_language_server.py:106-112`). No size limit or binary/generated heuristics found beyond that.

Indexing. Serena indexes nothing itself. It asks the language server for `textDocument/documentSymbol` per file and caches two pickles per language server: raw document symbols and a processed symbol tree, each keyed by file content hash (`ls.py:349-368, 548-561, 1857-1896`). The C++ adapter folds `compile_commands.json`'s MD5 plus clangd version into the cache fingerprint (`clangd_language_server.py:83-104`). Global `find_symbol` walks every file's cached document symbols rather than using `workspace/symbol` (`src/serena/symbol.py:737-760`; `request_workspace_symbol` exists at `ls.py:3133` but is not on that path). References come from `textDocument/references`, then each hit is mapped to its enclosing symbol and a code snippet (`lsp_api.py:530-564`, `symbol.py:812-864`). clangd is launched with `--background-index` (`clangd_language_server.py:313-316`), pinned to 19.1.2 (line 231), while upstream clangd is at 23.1.0 (stale pin; downloadable override via `clangd_version`). Incremental: file-system changes are synced before reference queries (`lsp_api.py:550-552`); known limitation that files created after server start are not indexed until opened (`docs/03-special-guides/cpp_setup.md:88-96`). No call graph, no embeddings, no trigram index.

Ranking. None in the retrieval sense. Results are exact LSP matches; fuzzy search is only `substring_matching` on the last name segment (`symbol_tools.py:120-122`).

What it exposes to agents (exact tool names from `src/serena/tools/*.py`). Symbolic read: `get_symbols_overview(relative_path, depth, max_answer_chars)`; `find_symbol(name_path_pattern, depth, relative_path, include_body, include_info, include_kinds, exclude_kinds, substring_matching, max_matches, max_answer_chars)` (`symbol_tools.py:73-151`); `find_referencing_symbols(name_path, relative_path, include_kinds, exclude_kinds, max_answer_chars)` (`:158-194`); `find_implementations` (`:197`); `find_declaration(relative_path, regex, containing_symbol_name_path, include_body, include_info)` (`:238`); `get_diagnostics_for_file` / `_for_symbol` (`:279, :313`). Symbolic edit: `replace_symbol_body`, `insert_after_symbol`, `insert_before_symbol`, `rename_symbol`, `safe_delete_symbol` (`:352-468`). Files and search: `read_file(relative_path, start_line, end_line, max_answer_chars)`, `list_dir`, `find_file`, `search_for_pattern` (regex with context lines, include/exclude globs, `restrict_search_to_code_files`) (`file_tools.py:45-119, 315-362`). Edits: `create_text_file`, `replace_content`, `replace_in_files`, line tools. Workflow and memory: `onboarding`, `initial_instructions`, `write_memory`, `read_memory`, `list_memories`, etc. Multi-project: `activate_project`, `list_queryable_projects`, `query_project(project_name, tool_name, tool_params_json)` which runs only read-only tools in another registered project (`query_project_tools.py:10-75`). Beta: `SerenaReplTool` lets the agent run Python against facades (`s.lsp.*`, `s.edit.*`), "like a notebook cell", with `s.info("<facade>")` for docs (`repl_tools.py:11-45`; `CHANGELOG.md` "Add the Serena REPL ... reducing the tool set to a minimum").

Addressing. A "name path" inside a file, for example `MyClass/my_method`, with `[i]` overload index, relative suffix patterns and absolute `/class/method` (`symbol_tools.py:97-112`). Line numbers are 0-based in `read_file`/`search_for_pattern` (`file_tools.py:55-56, 345`).

Output budgeting. Every tool takes `max_answer_chars`; default `default_max_tool_answer_chars = 150_000` (`serena_config.py:921`), tool timeout 240 s (`:53`). `_limit_length` tries progressively shorter renderings before giving up (`tools_base.py:264-281`): results are grouped by file and kind (`lsp_api.py:316-318`), then reduced to name-path lists or kind counts (`lsp_api.py:169, 201-212`). `max_matches` aborts with a name-path listing so the agent can refine (`lsp_api.py:521-526`). Symbol bodies are opt-in (`include_body=False` default); `depth` default is 0 so a class overview does not dump methods.

C++, Unreal, shaders, other languages. C/C++ via clangd (or ccls) and requires `compile_commands.json` at the repo root; relative `directory` entries are rewritten to absolute into `.serena/compile_commands.json` (`clangd_language_server.py:135-200`, `cpp_setup.md:21-40`). For Unreal the adapter refuses to start without a non-empty database and prints the exact UnrealBuildTool command (`clangd_language_server.py:114-133`). The guide (`docs/03-special-guides/unreal_engine_setup_guide_for_serena.md:32-98`) says: run `UnrealBuildTool.exe -mode=GenerateClangDatabase -project=... <Proj>Editor Win64 Development -OutputDir=...`; build the editor target once first so `*.generated.h` exist; MSVC-flavoured databases omit system includes so add `.clangd` flags (`-ferror-limit=200`); `GENERATED_BODY()` goes stale with `__LINE__` after edits until the next build; first index takes "a few minutes"; unscoped `find_symbol` "visits every translation unit" so pass `relative_path` (`:112-130`). C# via Roslyn LS (needs .NET 10, plus `pwsh` on Windows), Rust via rust-analyzer, Lua/Luau, GDScript via the running Godot editor's built-in LSP on TCP 6008 (not launched by Serena), HLSL/GLSL/WGSL via `shader-language-server` with **reference search not supported** (`docs/01-about/020_programming-languages.md:57-61, 79-82, 94-97`). CMake not listed.

Windows. First-class: clangd Windows zip is auto-downloaded (`clangd_language_server.py:246-255`), Unreal guide is Windows-flavoured. LSP servers launched on demand, one set per active project.

Scale and performance evidence. Vendor-authored only. The "evaluation" is agents grading themselves on Python (Tianshou), Java and a monorepo and counting calls: e.g. cross-file rename 1 call versus roughly 8-12 with built-ins (`docs/04-evaluation/030_results/010_cc_on_tianshou.md:175-198`); `find_symbol(include_body)` saves about one call (`:96`). No C++ or Unreal measurements, no independent benchmark. Not independent.

Concrete ideas to adopt in a Rust vault.
1. Name-path addressing as a second address form next to `doc_id#L40-L72`: `read` and `outline` accept `repo:Class/method[1]`, resolved through the symbol table. Effort S-M. Own implementation (the design is described in a docstring; no code copy).
2. The shortening ladder for any list-returning tool: full, grouped by file and kind, name-paths only, kind counts, with a hard `max_matches` that errors with a refinable summary. Effort S. Own implementation.
3. Content-hash-keyed caches with a fingerprint that includes the build inputs (compile_commands hash, tool version). Effort S. For SCIP imports the fingerprint is the SCIP file hash plus each document's hash.
4. A batch/code-execution interface to cut tool calls (Serena REPL). In the vault this is cheaper as arrays on every tool (already in the plan) plus one composite `trace(symbol)` response. Effort M.
5. Unreal ergonomics: detect `.uproject`, skip the four UE build directories, tell the agent when the precise tier is unavailable and why. Effort S.
6. Onboarding plus memories: a generated per-repo orientation note stored as a vault document. Effort S-M; matches the vault's "generated repository document".
Licence note: SolidLSP (MIT) could be ported to Rust freely; the Serena application (GPL-3.0-or-later) must not be copied.

Fit: port_ideas.

### 3.2 microsoft/multilspy

What it is. A Python LSP client library from the NeurIPS 2023 "Monitor-Guided Decoding" research (`README.md:4-9`). MIT. Supports java (JDTLS), python (jedi), rust-analyzer, C# (OmniSharp), TypeScript/JS, go, dart, ruby, kotlin, php, cpp (clangd) (`README.md:48-61`).

Ingestion and indexing. None of its own; it launches a server, sends `initialize`, and exposes `request_definition`, `request_references`, `request_completions`, `request_hover`, `request_document_symbols` by file, line and column. clangd is launched as plain `clangd` with no `--background-index` and no compile-database handling beyond a comment telling the user to supply one (`src/multilspy/language_servers/clangd_language_server/clangd_language_server.py:27-40`). Pinned clangd 19.1.2 (line 76). 892-line `language_server.py`.

Ranking: none. Agent exposure: none (library only). Scale evidence: none. Maturity: research artefact, one merge since August 2026; Serena's SolidLSP is the maintained descendant (Serena acknowledgements, line 26), with C#, Lua, GDScript, shaders, caches and diagnostics that multilspy lacks.

Ideas to adopt: only the observation that a position-based LSP facade is easy but wrong for agents (agents do not know columns); Serena and the vault should address by symbol or by line range. Nothing to port.

Fit: reference_only.

### 3.3 SCIP protocol (scip-code/scip)

What it is. Sourcegraph's Protobuf schema for precise code-intelligence data, now hosted at `scip-code/scip`. Apache-2.0. Go CLI plus bindings for Go, Rust, TypeScript, Java, Kotlin, Haskell, .NET (`bindings/`). The Rust crate `scip` 0.10.0 (`bindings/rust/Cargo.toml:2-3`) is Apache-2.0 and depends on `protobuf = "=3.7.2"` (rust-protobuf, pinned to match the generated code).

Data model. `Index { metadata, repeated Document, repeated external_symbols }` designed for streaming, metadata first (`scip.proto:20-39`). `Document { language (string), relative_path ('/'-separated, no symlinks), occurrences, symbols, optional text, position_encoding }` (`:76-119`). Position encoding is explicit: UTF-8 bytes, UTF-16 or UTF-32 code units from line start (`:122-145`), which matters when converting to vault line/byte addresses. `Symbol` is a URI-like string `<scheme> <manager> <package> <version> <descriptors>` with descriptor suffixes namespace `/`, type `#`, term `.`, method `().`, **macro `!`** (`:148-194, 205-224`). `SymbolInformation` carries documentation, `kind` (96 fine-grained kinds), signature documentation, `enclosing_symbol`, and `relationships` (`:253-462`). `Relationship { symbol, is_reference, is_implementation, is_type_definition, is_definition }` is how SCIP gets "find implementations" and override-aware references (`:465-517`). `Occurrence { range, symbol, symbol_roles bitset, override_documentation, syntax_kind, diagnostics, enclosing_range }` (`:692-806`). Roles: Definition 0x1, Import 0x2, WriteAccess 0x4, ReadAccess 0x8, **Generated 0x10, Test 0x20, ForwardDefinition 0x40** (explicitly for C/C++ forward declarations) (`:524-546`). `enclosing_range` is stated to support call hierarchies ("what symbols are referenced from the body of a function") and outline breadcrumbs (`:747-756`), which is the cheap way to derive a call graph from occurrences: a reference occurrence inside the enclosing range of a function definition is a call edge candidate. The `Language` enum includes C, CPP, CUDA, CSharp, Lua, Luau, Rust, Odin, Makefile but **no GDScript, HLSL, GLSL or CMake** (`:843-940`); the field is a string so any value is legal.

Design stance (`docs/DESIGN.md:10-15, 63-75`): a transmission format, not a storage or query format; no integer IDs, no graph encoding (blast-radius argument); compresses to 10-20%. The CLI (`docs/CLI.md:28-35`) has `lint`, `print --json`, `snapshot`, `stats`, `test`, and an experimental `expt-convert` to SQLite that stores occurrences as opaque blobs (`:173`).

Ranking and agent exposure: none, by design. Scale: size claims only (10-20% compressed). Maturity: active, v0.10.0.

Concrete ideas to adopt.
1. Import tier: parse `.scip` files with the `scip` crate (or generate with prost from `scip.proto`, Apache-2.0), stream documents, and load into SQLite as `symbols(symbol, doc_id, kind, display_name, enclosing)`, `occurrences(doc_id, symbol_id, start_line, start_col, end_line, end_col, roles)` with ranges converted to the vault's address space (honour `position_encoding`), `relationships(from, to, flags)`. Effort M. Licence-safe.
2. Reuse SCIP's role bitset (Definition, Import, Generated, Test, ForwardDefinition) as the vault's internal reference vocabulary even for fuzzy results, so precise and fuzzy rows share one schema and a `precision` column (`scip` | `heuristic` | `syntactic`). Effort S.
3. Use `enclosing_range` to derive call edges and caller/callee lists. Effort S once occurrences are loaded.
4. Staleness: SCIP carries paths and ranges, not hashes. Store the document hash at import time and refuse or flag occurrences if the file on disk differs. Effort S.

Fit: **adopt_dependency** (the `scip` crate and schema).

### 3.4 sourcegraph/scip-clang (C and C++ producer)

What it is. A Clang-21-based indexer emitting SCIP for C, C++ and CUDA (`README.md:1-5`). Beta (`README.md:1`; `CHANGELOG.md` v0.4.0). Apache-2.0.

Requirements. Binary releases for x86_64 Linux (glibc 2.16+) and arm64 macOS only (`README.md:42-46`; `CHANGELOG.md` v0.4.0 dropped macOS x86). **No Windows binary.** It needs a JSON compilation database from CMake, Bazel, Meson, Bear or compiledb (`README.md:55-62, 127-170`) and must run after the build, because it re-runs type checking and needs generated code (`README.md:172-177`). Precompiled headers unsupported (`:77-79`). About 2 MB temp space per translation unit and 2 GB RAM per core (`:113-125`); on Linux about 2 MB of `/dev/shm` per core. Must be invoked from the project root.

Architecture (`docs/Design.md:20-67`). One driver process plus worker processes (robust to crashes and hangs; hung workers are killed and replaced), per-worker queues, shards written per job then merged. Header work is de-duplicated across translation units by hashing post-expansion bytes (`:92-228`, same idea as Kythe and Apple Clang). Templates: "Optimistic" approach, dependent names resolved by approximate name lookup, matching clangd's imprecise dependent lookup (`:229-311`).

Macros. A macro symbol's name includes the defining file and (line, column), so conditional definitions and def/undef textual-include patterns stay unambiguous (`:317-353`), and references to macros are indexed (README examples: "Find references for macros" in llvm). Forward declarations are modelled with the `ForwardDefinition` role (`:558`).

Ranking, agent exposure: none (produces an index for Sourcegraph). Scale evidence: only sizing guidance and links to public Sourcegraph pages for Chromium, LLVM, Boost (`README.md:14-24`); no timings. First-party.

Unreal fit. Cannot run on this Windows machine. Unreal's MSVC-flavoured database would also be a mismatch. WSL plus a Linux-built tree is theoretically possible but is a per-repo project, so not a default path.

Ideas to adopt: macro symbol naming with definition site, forward-declaration role, and header-content hashing for de-duplicated parsing (the vault can hash shadow-parsed header ranges to avoid re-extracting symbols from a header included in thousands of translation units; for a syntactic indexer each file is parsed once anyway, so mostly irrelevant). Effort S for the first two.

Fit: reference_only (optional external producer for Linux-buildable C++ repos, imported through SCIP).

### 3.5 SCIP producers for other target languages

scip-typescript (`scip-typescript/README.md`, `src/FileIndexer.ts`): TypeScript/JavaScript on the compiler API; needs `npm install` first; `--infer-tsconfig` for JS, `--yarn-workspaces`, `--pnpm-workspaces`; skips files above `--max-file-byte-size` (`FileIndexer.ts:42-60`); OOM mitigations (`--no-global-caches`). Not in the owner's target list; ignore except as an example of workspace handling.

scip-python (`scip-python/README.md`): Sourcegraph fork of Pyright (`pyright-last-sync` file), needs Python 3.10+, Node 16+ and an activated virtualenv for dependency discovery. Default-branch last commit 2025-09-05, so effectively stale. Not in the target list.

scip-java (`scip-java/docs/design.md:1-50`): Java compiler plugin producing one shard per source file, merged afterwards; also Kotlin via `scip-kotlinc`; needs Gradle, Maven or Bazel. Per-file shard plus merge is a good incrementality pattern.

scip-dotnet (read via `gh api repos/sourcegraph/scip-dotnet/readme`): C# and Visual Basic, needs .NET 8, run as `scip-dotnet index` at the project root, uses MSBuild/Roslyn so the project must restore and compile. Relevant for Unity/Godot-C# code only if the repo builds standalone, which Unity projects usually do not.

scip-rust (`gh api repos/scip-code/scip-rust/readme`): a wrapper over `rust-analyzer scip .`; needs a working cargo toolchain. Works on Windows. The most practical precise producer for Rust game repos.

All are Apache-2.0 (scip-python MIT). Fit: reference_only; the vault should simply accept their `.scip` output (idea 3.3.1).

### 3.6 github/stack-graphs (archived)

What it is. Rust implementation of stack graphs: name-resolution rules per language written as tree-sitter-graph DSL (`.tsg`), producing per-file graphs that are stitched at query time, so resolution is incremental and needs no build tools (`README.md:7-11`). Archived 2025-09-09. Dual Apache-2.0/MIT.

Ingestion/indexing. `tree-sitter-stack-graphs index SOURCE_DIR` skips already-indexed files unless `-f` and stores per-file graphs and partial paths in SQLite (schema version 6, tables `metadata`, `graphs`, `file_paths`, `root_paths`, `stack-graphs/src/storage.rs:37-55`; CLI in `tree-sitter-stack-graphs/README.md:20-70`). Query: `query definition SOURCE_PATH:LINE:COLUMN`. Language rule sets exist only for Java, JavaScript, Python and TypeScript, about 14,100 lines of `.tsg` in total (TypeScript alone 6,297; `languages/*/src/stack-graphs.tsg`). **No C, C++, C#, Rust, Lua, GDScript or shaders.** Pinned to `tree-sitter = "0.24"` and `tree-sitter-graph = "0.12"` (`tree-sitter-stack-graphs/Cargo.toml:77-81`) while tree-sitter is at 0.28.0, so using it means a fork.

Ranking: path-precedence rules inside the graph only. Agent exposure: CLI and an optional LSP feature; none for agents. Scale evidence: a benchmark corpus of 15 TypeScript compiler files (21,981 lines of code) in `data/typescript_benchmark.md` but no timing numbers in the repo. GitHub used it for precise navigation (first-party claim only).

Ideas to adopt. (1) The principle that each file produces a self-contained graph and cross-file linking is deferred to query time, which makes per-file invalidation trivial; the vault's fuzzy tier can mimic this by keeping per-file `defs`, `refs`, `imports/includes` rows and resolving refs to defs at query time with scoring. Effort M, no code needed. (2) Do not write `.tsg` rule sets for six languages: effort L per language and no maintainer.

Fit: port_ideas (reject as dependency).

### 3.7 tree-sitter and tags (tree-sitter/tree-sitter)

What it is. Incremental parser library (`README.md:7`), MIT. The `tree-sitter-tags` crate (`crates/tags`) generates definition and reference "tags" from a language's `queries/tags.scm`.

How tags work (`docs/src/4-code-navigation.md:9-26, 78-90, 119`). Captures named `@definition.<kind>` / `@reference.<kind>` with an inner `@name`; optional `@doc` with `#strip!` and `#select-adjacent!` predicates; standard vocabulary: class, function, interface, method, module definitions; call, class, implementation references. Optional locals query (`@local.scope`, `@local.definition`) lets the runner drop names that are locally defined (`crates/tags/src/tags.rs:376-400, 442-462`). A `Tag` carries byte range, name range, line range, point span, UTF-16 column range, docs, `is_definition`, `syntax_type_id` (`tags.rs:58-68`). Tags whose name node has a parse error are silently skipped (`tags.rs:438`), so a macro-confused parse yields missing symbols rather than wrong ones. Cancellation flag supported (`tags.rs:281-300`). GitHub's "search-based code navigation" is built on this approach (documented link in the doc, line 5).

What the grammars ship (fetched with `gh api` from each grammar's `queries/tags.scm`):
- tree-sitter-c: definitions only (struct, union, function declarator, typedef, enum). **No `@reference.*` at all.**
- tree-sitter-cpp: adds class, qualified method definitions; **no references**.
- tree-sitter-rust: definitions plus `@reference.call` (functions, methods, macro invocations) and `@reference.implementation`.
- tree-sitter-c-sharp: classes, interfaces, methods, namespaces, with `@reference.class`, `@reference.send` (invocations), base lists.
- tree-sitter-lua: functions, methods, and `@reference.call`.
- tree-sitter-hlsl and tree-sitter-glsl (MIT, maintained under tree-sitter-grammars), tree-sitter-gdscript (MIT, 117 stars), tree-sitter-cmake (uyha, MIT): **no `tags.scm` shipped** (HLSL, GDScript: 404; GLSL and CMake only highlights/folds/indents/injections).

Macro risk for Unreal (unverified, from reading only): constructs like `UCLASS(...) class MYGAME_API AFoo : public AActor` and `GENERATED_BODY()` are not C++ the tree-sitter-cpp grammar can interpret without help; expect ERROR nodes and, per `tags.rs:438`, dropped tags. Must be tested on real UE headers before relying on it.

Windows: tree-sitter is a Rust and C library built with MSVC or clang; no issue. Performance evidence: the README claims parsing fast enough for every keystroke (first-party); no numbers collected.

Concrete ideas to adopt.
1. Use the `tree-sitter` crate (MIT) as the parse layer for symbol extraction, outline and chunk boundaries (symbol-aligned chunks for embeddings). Effort M for the six or seven target languages; each new language is S once a tags query exists.
2. Author the vault's own `tags.scm` for C/C++ (adding `@reference.call`, type references, base-class references, `#include` imports), HLSL, GLSL, GDScript, CMake. Effort M for C/C++, S each for the others. Licence-safe (queries are written fresh, MIT grammars).
3. Reuse the `@role.kind` vocabulary as the vault's tag schema.
4. Add a "shadow buffer" pass for macro-heavy C++ (see ctags section): blank annotation macros with spaces of identical length before parsing, keep the original bytes for addressing. Effort M.

Fit: **adopt_dependency** (tree-sitter and grammar crates; tree-sitter-tags is optional because the vault needs scope, enclosing-symbol and ref-resolution data that Tag does not carry).

### 3.8 kythe/kythe

What it is. Google's language-agnostic graph schema and toolchain for cross-references (`kythe/docs/kythe-overview.txt:53-80`). Go services plus C++ (Clang) indexer, Java, Go, TypeScript, OCaml and a Rust extractor (`kythe/cxx`, `kythe/go`, `kythe/java`). Apache-2.0. Alive but slow: last release v0.0.76 on 2026-07-16 (`RELEASES.md:3`).

Ingestion. Indexing is a two-stage pipeline: extractors capture each compile action into a `.kzip` compilation unit (all inputs content-addressed by SHA-256), then indexers replay those units offline; "to index a source file, Kythe usually must first build that file, which we do using Bazel" (`kythe/docs/kythe-compilation-database.txt:22-30`). C++ extractor has Bazel and CMake front ends (`kythe/cxx/extractor/`). No Windows statement anywhere in the docs (grep for windows/msvc in `docs/*.txt` returned nothing).

Indexing and schema. Entries are tabular facts and edges; storage format is explicitly not optimised for query (`kythe-storage.txt` summary); serving tables are built separately (`kythe/go/serving`, `kythe/go/services/xrefs`, `graph`, `filetree`). Edges relevant here: `ref/call` (callgraph, `docs/schema/callgraph.txt:15-216`), `ref/expands` and `ref/expands/transitive` for macros (`docs/schema/schema.txt:1277-1302`), includes, overrides, definitions versus declarations. The C++ indexer assigns the same node ID to a header's declarations across translation units ("claiming", `kythe/cxx/indexer/cxx/DESIGN.md:40-80`, `claiming.md`) and records macro definitions and expansions in `IndexerPPCallbacks.cc:134-143`.

Ranking, agent exposure: none; services expose xrefs, graph, filetree, identifiers over gRPC/HTTP for UIs. Scale evidence: Google-internal origin (first-party narrative only); no numbers in the repo.

Ideas to adopt. (1) Edge vocabulary for the vault's graph table: `ref/call`, `ref/expands` (macro expansion sites, valuable for Unreal where `UFUNCTION`/`GENERATED_BODY` behaviour is invisible), `ref/includes`, overrides. Effort S (schema only). (2) "Prefer incomplete over incorrect" principle (`kythe-overview.txt:75-90`) matches a confidence-tagged result model. (3) Content-addressed storage of inputs for reproducible re-indexing; the vault already stores files verbatim.

Fit: reference_only (reject as dependency).

### 3.9 facebookincubator/Glean

What it is. Meta's system for storing and querying facts about code, with the Angle query language, RocksDB or LMDB storage, schema-driven typed facts (`README.md:10-24`, `glean/website/docs/implementation/db.md:3-25`). Haskell plus C++ runtime. BSD-3-Clause style licence.

Ingestion. Language indexers emit facts: C/C++ via a clang wrapper ("drop-in replacement for the C++ compiler", linked against libclang and libllvm; tested against clang 11 and 12, `glean/website/docs/indexer/cxx.md`), Hack, Haskell, Flow, Python, and SCIP/LSIF ingestion for Rust, Go, TypeScript, Java, Python, .NET (`README.md:19-35`). Indexing C++ "scales to very large repos" but the parallel multi-machine driver is not in the open source release (`cxx.md`). `make glean-clang` build from source.

Indexing/incrementality. Facts are owned by "units" (strings, such as a file); stacked databases let a new DB layer hide or replace the facts of changed units without re-indexing everything (`glean/website/docs/implementation/incrementality.md`). A derived-fact pass (`clang-derive`) computes find-references tables.

Ranking: none. Agent exposure: Glass, a Thrift symbol server with `documentSymbolListX`, `findReferenceRanges`, `describeSymbol`, `searchSymbol`, `searchRelated`, `fileIncludeLocations`, `clangUSRToDefinition` (`glean/glass/ARCHITECTURE.md` section 2, `glass/README.md:1-20`); `glean-lsp` gives a generic read-only LSP (go to definition, references, hover, outline, symbol search; `glean/lsp/README.md:1-20`). No MCP.

Platform: "Linux. The build is only tested on Linux so far" (`website/docs/building.md:19-20`), GHC 9.6.7 and cabal. Not runnable on this Windows machine except in WSL or Docker.

Scale evidence: Meta-internal claims ("accurately and efficiently on a large-scale codebase", `introduction.md`); "indexer performance on C++ code is similar to running the clang frontend" (`cxx.md`). First-party, not independent.

Ideas to adopt. (1) Ownership-based incrementality: every row in the vault's symbol, occurrence and edge tables carries the owning `doc_id` so a changed file deletes and re-adds exactly its rows. Effort S. (2) A language-agnostic "codemarkup" layer: one unified query surface over differently-sourced facts (our `precision` column). Effort S. (3) Glass's `fileIncludeLocations`-style tool for include/import graphs. Effort S-M.

Fit: reference_only (reject as dependency).

### 3.10 joernio/joern

What it is. Code-property-graph platform for vulnerability research and program analysis (`README.md:8-16`). Scala 3, JDK 21 required (`README.md:21-24`). Apache-2.0 (Eclipse CDT piece EPL-2.0). Frontends under `joern-cli/frontends`: c2cpg (Eclipse CDT), csharpsrc2cpg, rust2cpg, javasrc2cpg, jimple2cpg, kotlin2cpg, jssrc2cpg, pysrc2cpg, gosrc2cpg, php2cpg, rubysrc2cpg, swiftsrc2cpg, ghidra2cpg, abap2cpg. **No Lua, GDScript, HLSL/GLSL or CMake.** Tools: `joern`, `joern-parse`, `joern-scan`, `joern-slice`, `joern-flow`, `joern-export`, `joern-vectors`. No MCP server in the repo (grep for MCP only hit unrelated source).

Ingestion. c2cpg parses source directly with Eclipse CDT with no build needed (`frontends/c2cpg/README.md`); optional `--define`/`--include`, optional `compile_commands.json` parsing that extracts only `-D` and `-I` (`parser/JSONCompilationDatabaseParser.scala:14-40`), `--exclude` and `--exclude-regex`. It auto-discovers system include paths by invoking `gcc -E -v` (with a Windows branch, `utils/IncludeAutoDiscovery.scala:16-25`) and optionally falls back to defaults when gcc is absent. It logs parse problems rather than failing (`parser/ParseProblemsLogger.scala`), and CDT's recursive parser can stack-overflow on heavy macro/template input (comment at `parser/CdtParser.scala:200-207`).

Macros. CDT expands macros; `MacroHandler.scala:21-45` re-creates a synthetic CALL node for a macro invocation and wraps the expanded AST under it, so macro-generated code stays attributed to the invocation. C++ feature coverage matrix is in `frontends/c2cpg/CPP_Features.md` (for example concepts and `using enum` not supported, templates partially).

Indexing and graph. The CPG has AST, CFG, call graph (CALL nodes linked to METHOD by full name), type and data-flow overlays; the default linking passes live in the external codepropertygraph and semanticcpg libraries, which I did not read. Storage is flatgraph, an in-memory columnar store (v4.0 changelog).

Ranking: none. Agent exposure: a Scala REPL DSL and CLI; slices as JSON (`joern-cli/JOERN_SLICE.md:1-11`: data-flow and usages).

Scale evidence (first-party, from the Joern author on his workstation, `changelog/4.0.0-flatgraph.md`): Linux 4.1.16 CPG has 48M nodes and 431M edges; import takes 11 minutes with 20 GB heap after GC and 30 GB minimum `-Xmx` (joern 2 needed 80 GB; 18 minutes). Linux 6 needs `-Xmx90g`. File size 400 MB on disk. Not independent.

Ideas to adopt. (1) The tolerant C/C++ posture: never fail a file, log parse problems, keep going. (2) Macro-invocation-as-call modelling: a macro expansion site becomes a call-like node, which helps agents see `UCLASS`, `check()` and `DECLARE_DELEGATE` as semantic events. Effort M inside a tree-sitter C++ extractor (invocation recognised syntactically, no expansion). (3) Optional `-D`/`-I` extraction from compile_commands.json when present, used to pick `#if` branches and include search paths. Effort S-M. (4) `joern-slice` usages mode (calls made by a local) is a nice composite but out of scope.

Fit: reference_only (reject as dependency: JVM and 20-90 GB heaps are wrong for a local-first vault).

### 3.11 Extra: universal-ctags (fuzzy, macro-aware symbol tables)

What it is. The maintained successor of Exuberant Ctags; C, GPL-2.0-or-later. Parsers for C/C++/CUDA (`parsers/cxx/`, `parsers/cpreprocessor.c`), C# (`parsers/c-based.c`), Rust, Lua, GDScript, CMake (optlib), Python, Go, and more (`parsers/` listing); **no HLSL or GLSL parser** found. Windows builds supported (`win32/`, `mk_mingw.mak`, `mk_mvc.mak`).

Key capability. Real preprocessor-macro definitions on the command line: `-D "foreach(arg)=for(arg;;)"`, `-D IGNORE_THIS`, token pasting, stringification and variadics; `-I` is the empty-replacement shorthand (`docs/parser-cxx.rst:70-170`). Experimental auto-expansion of macros defined in the same file with `--param-CPreProcessor._expand=1` (`:173-229`). That is exactly what Unreal-style annotations (`UCLASS(...)`, `UFUNCTION(...)`, `MYGAME_API`) need so the parser sees plain C++.

Ranking, agent exposure: none in the tool itself (tags file or JSON output, interactive mode). Scale evidence: none gathered.

Ideas to adopt. A per-repo "macro neutralisation table" (blank-or-replace patterns) applied to a shadow copy before tree-sitter parsing, shipped with defaults for Unreal (`*_API`, `UCLASS`, `USTRUCT`, `UENUM`, `UFUNCTION`, `UPROPERTY`, `GENERATED_BODY`, `GENERATED_UCLASS_BODY`), Qt (`Q_OBJECT`, `Q_PROPERTY`), and user-editable per repo. Effort M. Port the idea, not the code (GPL).

Fit: port_ideas.

### 3.12 Extra: clangd (via gh api)

What it is. LLVM's C++ language server and the engine under Serena, multilspy and mcpls for C/C++. Apache-2.0 with LLVM exception. Release 23.1.0 on 2026-09-02.

Indexing (`clang-tools-extra/clangd/index/`): per-file shards written by `BackgroundIndex` into `<compile-db-dir>/.cache/clangd/index/` (`Background.h` comment, lines ~46-52), a priority queue with low-priority worker threads that never re-indexes a file with the same key; in-memory **Dex** index with trigram posting lists, scope and type tokens, built for low-latency fuzzy symbol search (`index/dex/Dex.h` header comment, `Trigram.cpp`, `PostingList.cpp`); refs carry kinds Declaration, Definition, Reference (`index/Ref.h:28-50`). Needs `compile_commands.json` (or per-directory flags) for correctness. clangd operates on code with errors, so Serena's UE guide reports it indexes through missing headers with "index may be incomplete" and symbol trees truncating after ~20 errors unless `-ferror-limit` is raised (`unreal_engine_setup_guide_for_serena.md:136-147`).

Takeaways. Trigram posting-list fuzzy symbol search is the same trick the vault plans for code text; shard-per-source-file storage keyed by file is the same incrementality pattern as scip-java and Glean's units; the ranking signals clangd feeds into its Dex scorer (symbol quality, proximity to the current file, name match) are a ready-made list of boosts. I did not read the scorer, so exact weights are unverified.

Fit: reference_only now; later optional sidecar, launched on demand by libraryd for a "precise upgrade" of one query.

### 3.13 Extra: bug-ops/mcpls

What it is. Rust MCP server that bridges any LSP server to MCP, built on `rmcp` 3.4 and `gen-lsp-types` (`Cargo.toml:29-32`); Windows binaries shipped; Apache-2.0 OR MIT. Tools are position-based (file, line, column): `get_hover`, `get_definition`, `get_references`, `get_completions`, `get_document_symbols`, `workspace_symbol_search`, `go_to_implementation`, `go_to_type_definition`, `get_diagnostics`, `get_code_actions`, `rename_symbol`, `format_document`, and **call hierarchy** `prepare_call_hierarchy`, `get_incoming_calls`, `get_outgoing_calls` (`README.md:146-203`). Graceful degradation when one server fails. No caching, ranking or budgeting layer found in the README.

Takeaway: confirms that call hierarchy via LSP is exposed by some bridges (Serena has none), and that a Rust LSP client plus MCP server is a few thousand lines. Position-based addressing is exactly what agents are bad at; the vault should wrap any future LSP sidecar behind symbol-based tools.

Fit: reference_only.

## 4. Cross-cutting matrix for the target use case

Target: 5-7 repositories of game source (C/C++ with heavy macros such as Unreal, C#, Rust, Lua/GDScript, HLSL/GLSL, CMake) on Windows 11, mapped and navigated by an agent in few calls.

| Approach | Precision | Needs | Windows | C++ macros and Unreal | Generated code |
|---|---|---|---|---|---|
| tree-sitter tags (+ own queries) | syntactic definitions; references by name only | nothing | yes | parse breaks without macro neutralisation (unverified); fixable with shadow buffer | not applicable; mark `Generated` by path |
| universal-ctags | syntactic definitions | nothing | yes (win32 build) | `-D` macro definitions solve annotation macros | not applicable |
| stack-graphs | scope-correct for supported languages | nothing | yes | no C/C++ rules; archived | not applicable |
| Joern c2cpg | fuzzy-semantic call graph, data flow | nothing (compile db optional) | yes (JVM, `.bat` scripts) | CDT expands macros, synthetic macro-call nodes; some C++20 gaps | not applicable |
| clangd via LSP (Serena, mcpls) | precise, project-wide | compile_commands.json plus built tree for generated headers | yes | works on UE only after `UnrealBuildTool -mode=GenerateClangDatabase` and an editor build (`unreal_engine_setup_guide_for_serena.md:32-98`) | needs a build first; ignore `Intermediate` |
| scip-clang | precise, Clang AST | compile db, built tree, Clang 21 toolchain | **no** (Linux, macOS arm64) | macros indexed with definition sites; PCH unsupported | needs a build first |
| Kythe | precise, includes macro expansion and call edges | Bazel or CMake extraction into kzips | not documented | macro `ref/expands` edges | needs build |
| Glean clang indexer | precise | libclang, GHC toolchain, CMake | **no** (Linux only) | tested only on clang 11/12 | needs build |
| rust-analyzer scip | precise | cargo workspace | yes | proc-macros handled by the analyzer | build scripts run |
| scip-dotnet | precise | .NET SDK, restoring projects | yes | not relevant | source generators run as part of the build |

## 5. Cluster summary

### 5.1 What the cluster converges on

1. Two tiers everywhere. Cheap syntactic or fuzzy symbol data for breadth (ctags, tree-sitter tags, GitHub search-based navigation, Joern c2cpg) and a precise, build-dependent tier for depth (clangd, SCIP indexers, Kythe, Glean). Nobody tries to make one engine do both. Every serious system also tolerates incomplete data ("emit incomplete rather than incorrect", Kythe overview).
2. Precise C/C++ is gated by the build: a compilation database, generated headers, and a matching toolchain (Serena `cpp_setup.md:21-40`, scip-clang `README.md:172-177`, Kythe KCD doc, Glean). For Unreal that means `UnrealBuildTool -mode=GenerateClangDatabase` and a full editor build per project, and even then clangd needs `-ferror-limit` tuning. The two most capable C++ indexers (scip-clang, Glean) do not run on Windows.
3. Stable, human-readable symbol identity: SCIP descriptor strings (with a macro suffix), Kythe VNames, Serena name paths. All avoid integer IDs at the interface.
4. Per-file units with content hashes make invalidation cheap (Serena caches, clangd shards, scip-java shards, Glean units, stack-graphs per-file graphs).
5. Definition and reference as roles on occurrences, with flags for forward declaration, generated, test (SCIP roles bitset).
6. Agent-facing layers all converge on symbol-level addressing, opt-in bodies, depth limits, and bounded output with refinable truncation (Serena). Position-based tools (mcpls, multilspy) are the weaker pattern for LLMs.
7. Nobody ranks. Serena, SCIP, Kythe, Glean, mcpls return exact or unranked sets; only clangd's completion index and Sourcegraph/GitHub search ranking internally. PageRank-style centrality, symbol-definition boosts and recency are absent from this cluster (they belong to other clusters such as Aider's repo map).
8. Cross-repository navigation is the weak spot: LSP is per workspace (Serena handles several repos by switching the active project or `query_project`, one result set per call), SCIP and Kythe support cross-repo via package identity but need every repo indexed with consistent package maps.

### 5.2 What the vault must do to beat them for 5-7 game repos

Principle: a vault answer must always exist (no build required), say how precise it is, and get more precise when the owner supplies better data.

1. Always-on fuzzy tier, uniform across languages. tree-sitter parse of every source file into definitions, references (by name), imports and includes, enclosing symbol, doc comments, stored per document in SQLite with the SCIP role vocabulary and a `precision` column. This is something none of the studied precise tools provide on Windows without a build. Effort L overall (about one language per few days after the first).
2. Make C++ macros a first-class configuration, not a parser failure. Shadow-buffer neutralisation tables (ctags `-D` idea), defaults for Unreal and Qt, per-repo overrides, automatic detection of `.uproject`/`.uplugin`/`*.Build.cs`, and the Serena exclusion set (`Binaries`, `DerivedDataCache`, `Intermediate`, `Saved`). Record `UFUNCTION/UPROPERTY` specifiers as attributes on the following declaration. Effort M. This directly targets the case where Serena and clangd fail loudly without a compile database.
3. Heuristic reference resolution with explainable confidence. Resolve a reference to candidate definitions by name, then rank by same file, same directory, include/import reachability, same CMake target or Cargo/Build.cs module, same repo, other repo. Return the top candidates with `precision: heuristic` and the reason. This is the "search-based navigation" tier GitHub uses, extended with cross-repository fall-through, which stack-graphs, Serena and scip-clang cannot do. Effort M.
4. SCIP import as the precise tier. Accept `.scip` produced by scip-clang (Linux/WSL), `rust-analyzer scip`, scip-dotnet, scip-typescript, scip-java; override heuristic rows for the files they cover; check document hashes for staleness; keep the producer name and version in metadata. Effort M. Document in the vault's orientation note which producers are worth running for which repo.
5. Derive call graphs from occurrences with enclosing ranges (SCIP) and, for the fuzzy tier, from call-reference captures inside function bodies. Return `callers` and `callees` with depth and budget caps. Effort S-M. None of Serena, scip-clang or Glean (open source) exposes a call-graph tool to agents; mcpls does only via LSP.
6. Repository-level map, cheaper than any studied tool: generated repository document with module/target graph (Build.cs, CMake targets, Cargo workspace, .csproj/asmdef, Godot project files), top symbols by centrality over the include/import and reference graph, language mix, entry points. This is the "map 5-7 folders" requirement. Effort M-L. (This builds on ideas from other clusters; none of this cluster's tools produces it.)
7. Fewer calls. Keep array-valued tools and add a composite `trace(symbol_or_address)` returning definition, signature, top references grouped by file, callers, callees and the verbatim lines needed, with Serena's output-shortening ladder and `max_matches` refinable error. Accept symbol addresses (`repo:Class/method`) in `read` and `outline`. Effort M. This is the Serena REPL's benefit without a code sandbox.
8. Honest degradation. When a language lacks a tags query (HLSL, GLSL, GDScript, CMake today) fall back to lexical definition heuristics plus trigram grep, and say so. Serena's shader server returns no references at all; the vault can still give name-based references.
9. Optional later: spawn clangd or rust-analyzer on demand for a single "upgrade this answer" query behind the same tool surface, using a Rust LSP client (mcpls shows it is small). Effort L; defer until the fuzzy plus SCIP tiers are measured.

### 5.3 Risks, unknowns and things to verify before building

- tree-sitter-cpp on Unreal headers is unverified. Build a 50-file UE corpus (Actor, Component, GameMode, `*.generated.h` excluded) and measure how many class and function definitions survive with and without the shadow-buffer pass. This is the first check to run in Phase 1.
- HLSL, GLSL, GDScript and CMake grammars ship without tags queries; confirm grammar quality on real game shader and script files before committing to them. Serena itself reports shaders without reference search.
- Whether GPL ctags may be bundled as a separate process is a licence question I did not resolve; the recommendation is not to depend on it.
- Joern's call-linking passes live in the external codepropertygraph repo; I did not read how C++ calls are resolved (name plus arity versus types).
- No independent benchmarks were found for any of these tools. All performance numbers above are first-party (Joern changelog, scip-clang resource guidance, Glean docs) or vendor self-evaluation (Serena). clangd latency numbers were not collected.
- Godot's SCons `compiledb=yes` option for generating compile_commands.json is from memory and unverified in this session.
- Serena's clangd pin (19.1.2) trails upstream clangd (23.1.0); if the owner ever runs Serena beside the vault, set `clangd_version` explicitly.
- Serena `main` is a 2.0.0.dev0 branch with a licence change and a new REPL interface; names and behaviour may shift before release.

### 5.4 Ranked adoption list

| # | Idea | Source | Effort | Rust crate or port | Licence-safe |
|---|---|---|---|---|---|
| 1 | tree-sitter symbol extraction with own tags queries incl. references | tree-sitter tags | M | `tree-sitter` plus per-language grammar crates | yes (MIT) |
| 2 | SCIP import into SQLite with role bitset and position-encoding conversion | scip | M | `scip` 0.10.0 (Apache-2.0, protobuf =3.7.2) or prost from `scip.proto` | yes |
| 3 | Macro neutralisation shadow buffer, Unreal and Qt defaults | universal-ctags idea, Serena UE guide | M | own code, regex or token pass | yes (idea only) |
| 4 | Heuristic cross-repo ref resolution with confidence and reason | stack-graphs and GitHub search-based nav ideas | M | own code | yes |
| 5 | Output-shortening ladder, `max_matches` refine-error, symbol name-path addressing | Serena | S-M | own code | yes (do not copy GPL code) |
| 6 | Per-document row ownership and content-hash invalidation, fingerprint includes build inputs | Glean units, Serena caches | S | own code | yes |
| 7 | Call edges from enclosing ranges and call captures, `callers`/`callees` tool | SCIP, Kythe `ref/call` | S-M | own code | yes |
| 8 | Kythe-style edge kinds `ref/expands`, `ref/includes` | Kythe | S | own schema | yes |
| 9 | Composite `trace()` tool to cut round trips | Serena REPL motivation | M | own code | yes |
| 10 | Optional LSP sidecar for one-off precise queries | mcpls, Serena | L | `rmcp`/own LSP client; spawn clangd | yes (Apache-2.0 LLVM-exception, MIT) |
| 11 | Per-language Unreal/engine detection and ignore sets | Serena `common.py:188-195` | S | own code | yes |

Output file: `C:/Users/sethy/Projects/tutor/vault/docs/phase0/repos-code-intel.md`.
