# Phase 0 study: game and engine codebases (what makes them hard, who handles it well)

Worker report for the lead engineer. Study date 2026-10-01. Read-only: nothing from the studied repositories was built or run. All claims cite a path and line range in a local shallow clone (`C:/Users/sethy/Projects/refs/code/<short-name>`) or a `gh api` raw read of the upstream tree. Anything I could not verify is marked UNVERIFIED.

## 0. Method, limits and ambiguities

- Clones (depth 1, single branch) are under `C:/Users/sethy/Projects/refs/code/`: quake3, bevy, ts-hlsl, ts-glsl, ts-gdscript, ts-lua, ts-csharp, ts-cmake, ts-cpp, godot-mcp-coding-solo, gdtoolkit, godot-docs-mcp, unreal-copilot, unreal-clangd. Godot (1.89 GB on GitHub) was NOT cloned; I studied it through `gh api` (full recursive tree, 14,416 blobs, plus raw reads of about 12 files). The folder names `aider`, `zoekt`, `probe`, etc. in the same refs directory belong to other workers and are not part of this report.
- Extra projects added (limit 3): `Scony/godot-gdscript-toolkit`, `syan2018/UnrealCopilot`, `boocs/unreal-clangd`. Also touched lightly (metadata or README only): `nuskey8/godot-docs-mcp` (cloned, tiny), `ayeletstudioindia/unreal-analyzer-mcp`, `FabianFG/CUE4Parse`, `atenfyr/UAssetAPI`, `AstroTechies/unrealmodding`, `tree-sitter-godot-resource`, gdshader and wgsl grammars (metadata only).
- Tool limits hit: the WebSearch budget (200) was exhausted mid-task, so three planned Unreal searches did not run. The Epic EULA page returned HTTP 403, so Unreal licence terms are UNVERIFIED from the primary source. Three Epic docs pages were fetched through a summarizing fetcher (reflection, modules, UHT); one more (source layout) returned nothing usable.
- Unreal Engine source is gated and I did not read it. A GitHub code search for `GenerateClangDatabase` returned file paths inside several public repositories that appear to be unauthorised mirrors of Epic's source. I read only the path listing, not those files, and recommend the vault never ingest such mirrors.
- No tree-sitter CLI or Python binding was installed, and I was told not to install or run anything. So every statement about "does grammar X parse construct Y" is inferred from `grammar.js`, test corpora and issue trackers, and is UNVERIFIED by execution unless stated.
- Ambiguity recorded, not resolved: "fit" is defined for ingesting tools. Godot, Quake III and Bevy are not indexers, they are the thing to be indexed. For them the "ingests / indexes / ranks / exposes" fields are answered as "what the codebase presents to an ingester" and the fit class is `reference_only` (study material and test corpus), except where noted.

## 1. Summary table

| Repo | Short name | HEAD (date) | Licence | Status 2026-10-01 | Fit |
|---|---|---|---|---|---|
| godotengine/godot | (api only) | 084a2caa05119b625a99b6b51d44b459a26362de (2026-09-30) | MIT (`LICENSE.txt`); `thirdparty/` carries per-library licences | Active. dev 4.8.0 (`version.py`), latest stable 4.7.2 (2026-08-18) | reference_only (test corpus, structure model) |
| id-Software/Quake-III-Arena | quake3 | dbe4ddb10315479fc00086f08e25d968b4b43c49 (commit date 2012-01-31, GitHub push 2024-08-02) | GPL-2.0 (`COPYING.txt`); some files non-GPL (zlib-style `unzip.c`, see `README.txt`) | Frozen historical release (2005 source drop). Not archived | reference_only (test corpus for C with heavy macros). GPL: do not redistribute derived artifacts |
| bevyengine/bevy | bevy | 1fd5d65d02f091dcd869eda79bae2a2016e83017 (2026-10-01) | MIT OR Apache-2.0 | Very active. 0.20.0-dev, latest release v0.19.1 (2026-08-13) | reference_only (test corpus, Rust macro model) |
| tree-sitter/tree-sitter-cpp | ts-cpp | c009222808634c1014f82438d4883753516a2c24 (2026-09-17) | MIT | Active | adopt_dependency (with a macro pre-pass, section 6) |
| tree-sitter-grammars/tree-sitter-hlsl | ts-hlsl | bab9111922d53d43668fabb61869bec51bbcb915 (2025-03-23) | MIT | Low activity (18 months since last real commit), not archived | adopt_dependency with caution; own tags needed |
| tree-sitter-grammars/tree-sitter-glsl | ts-glsl | 24a6c8ef698e4480fecf8340d771fbcb5de8fbb4 (2025-03-16) | MIT | Low activity, not archived | adopt_dependency with caution; own tags needed |
| PrestonKnopp/tree-sitter-gdscript | ts-gdscript | 8ecb27ed77c09edd7cca123bbecebd44930b765c (2026-09-18) | MIT | Active, lags Godot 4 syntax | adopt_dependency (own tags needed) |
| tree-sitter-grammars/tree-sitter-lua | ts-lua | 10fe0054734eec83049514ea2e718b2a56acd0c9 (2026-02-26) | MIT (`LICENSE.md`) | Maintained | adopt_dependency |
| tree-sitter/tree-sitter-c-sharp | ts-csharp | 8c0abe0b84a3681d3e6852ce2e2a1eecb4f731c5 (2026-09-29) | MIT | Active | adopt_dependency (extend tags) |
| uyha/tree-sitter-cmake | ts-cmake | 58993af75218bc99a1f5a04c832a5937e7c422cb (2026-09-13) | MIT | Maintained. `tree-sitter-grammars/tree-sitter-cmake` does not exist (GraphQL: could not resolve), so uyha is the canonical one | adopt_dependency |
| Scony/godot-gdscript-toolkit (extra) | gdtoolkit | f61a1f3a9f2f757813bd5176a28903502bd44959 (2025-10-09) | MIT | Maintained, slow | reference_only (Python) |
| syan2018/UnrealCopilot (extra) | unreal-copilot | aec733200e4f16b831bd94e5a7165ef79f986f52 (2026-03-15) | NONE found (no LICENSE file; GitHub reports none) | Active-ish, 27 stars | reference_only (cannot copy code) |
| boocs/unreal-clangd (extra) | unreal-clangd | 5f130767b359e20198e115aab7e00e0b61da7a9c (2026-06-17) | NONE found | Active, 281 stars; this branch holds only a README | reference_only |
| Coding-Solo/godot-mcp | godot-mcp-coding-solo | 1209744fad78f3998f98c7394fd0f6ef50da5281 (2026-04-16) | MIT | Active, 5,897 stars | reference_only (editor-control tool, not a code reader) |
| nuskey8/godot-docs-mcp | godot-docs-mcp | 3b378332cd5741e91ba9bf700584286c9d200467 (2025-10-02) | MIT | Stale-ish, 25 stars | reference_only |

---

## 2. Godot (godotengine/godot), structure study via `gh api`

**What it is.** MIT-licensed 2D/3D engine. C++ core with SCons build; GDScript (own language), C# (modules/mono), GDExtension C API, GLSL shaders, text XML class reference. Mature (decade+), extremely active (pushed within the last day).

**Scale (from the recursive git tree, `truncated=false`).**
- 14,416 files, 323 MB. `thirdparty/` is 123 MB (38%). `.po` translation files are 107 files and 116 MB (36% of all bytes) and carry zero code meaning.
- By extension: `.h` 4,337 files / 56.2 MB; `.cpp` 2,486 / 68.8 MB; `.c` 942 / 27.6 MB (mostly thirdparty); `.xml` 1,262 (833 of them `doc/classes/*.xml`); `.gd` 718 (326 KB, nearly all test fixtures and templates); `.cs` 314; `.glsl` 154 (104 in `servers/rendering/renderer_rd/shaders/`); `SCsub` 219; `.py` 133; `.out` 516 (expected-output files for GDScript tests).
- 59 modules under `modules/` (full list in the tree), each an optional unit (`modules/gdscript/config.py`, `modules/gdscript/SCsub`).
- Largest meaningful files: `modules/visual_shader/editor/visual_shader_editor_plugin.cpp` 503 KB, `servers/rendering/rendering_device.cpp` 465 KB, `servers/rendering/shader_language.cpp` 410 KB, `editor/editor_node.cpp` 390 KB, `modules/gdscript/gdscript_analyzer.cpp` 279 KB, `doc/classes/ProjectSettings.xml` 388 KB. Whole-file reads are impossible for an agent; outline plus line-range reads are mandatory.

**How a code vault sees it (ingestion hazards).**
1. Vendored code is explicitly marked: `.gitattributes:3` has `thirdparty/* linguist-vendored`, and lines 1-2 map `*.h` and `*.inc` to C++. An ingester that honors `.gitattributes` (`linguist-vendored`, `linguist-generated`, `linguist-language`) gets the vendored boundary for free. Unmarked generated or vendored-ish files also exist (for example `drivers/alsa/asound-so_wrap.c` 607 KB and `core/input/gamecontrollerdb.txt` 590 KB), so a content heuristic (large size, generated header comment, very long lines) is still needed.
2. Generated code is not in the repo. `.gitignore` excludes `bin`, `*.gen.*`, `compile_commands.json`. `core/object/SCsub` generates `gdvirtual.gen.h` via `make_virtuals.py`; `core/extension/SCsub` generates `ext_wrappers.gen.h` and `gdextension_interface.gen.h`. 30+ `*_builders.py` / `make_*.py` scripts do this (for example `core/core_builders.py`, `glsl_builders.py`, `scu_builders.py`). Consequence: a macro such as `GDVIRTUAL1(_process, double)` (`scene/main/node.h:433-436`) expands to code a repository-only index can never see. Treat the macro invocation itself as the symbol.
3. Reflection is by runtime registration, not annotation. `GDCLASS(m_class, m_inherits)` is a multi-line macro at `core/object/object.h:245-285+` (it creates a `GDType`, a static class name, `_bind_methods` hook). Bindings are string-keyed: `ClassDB::bind_method(D_METHOD("add_child", "node", "force_readable_name", "internal"), &Node::add_child, DEFVAL(false), DEFVAL(0));` (`scene/main/node.cpp:3807`), signals via `ADD_SIGNAL(MethodInfo("ready"))` (`node.cpp:4068`), registration via `GDREGISTER_CLASS` (`core/object/class_db.h:562`). The GDScript-visible name lives in a string literal, the C++ identity is the `&Class::method` pointer on the same line. That pair is a cheap, regex-extractable cross-language alias edge ("GDScript `add_child` is `Node::add_child`").
4. Build membership is by glob in SCons: `core/object/SCsub` is `env.add_source_files(env.core_sources, "*.cpp")`; `modules/gdscript/SCsub` adds editor files only `if env.editor_build` and language-server files only if `module_jsonrpc_enabled` and `module_websocket_enabled`. So "is this file compiled" depends on build options, not on the tree. `SConstruct:259-260` defines `compiledb` / `compiledb_gen_only`, and `SConstruct:1212-1213, 1269-1272` write `compile_commands.json`, which is gitignored, so unavailable without a build.
5. Directory-level exclusion marker: `.gdignore` (found once at `platform/android/java/app/src/instrumented/assets/.godot/.gdignore`). Godot projects use it to hide folders from the editor; an ingester should honor it.
6. Data files are text. Example fixture `modules/gdscript/tests/scripts/completion/get_node/get_node.tscn` (all lines): header `[gd_scene load_steps=2 format=3 uid="uid://c8wekfd5ql7bc"]`, `[ext_resource type="Script" path="res://completion/class_a.notest.gd" id="1_ldc4g"]`, `[node name="GetNode" type="Node"]`, `[node name="AnimationPlayer" type="AnimationPlayer" parent="."]`, `script = ExtResource("1_ldc4g")`. This is a line-oriented INI-like format; a 100-line parser yields the node tree, script attachments and `res://` dependency edges without any grammar. `.import` sidecars (e.g. `icon.svg.import`) and `project.godot` follow the same style.
7. Test fixtures are plentiful and are executable specifications: `modules/gdscript/tests/scripts/` has analyzer 442, parser 364, completion 291, runtime 249, lsp 12 files (counts from the tree listing), many with `.out` expected output. Excellent learning material for GDScript semantics, but must be role-tagged `test` so they do not outrank real definitions.

**What carries meaning for a learner (ranked).**
- `doc/classes/*.xml` (833 files). Example `doc/classes/Node.xml` (1,427 lines): line 2 `<class name="Node" inherits="Object" api_type="core" ...>`, then `<brief_description>`, long `<description>` with the lifecycle explanation (`_enter_tree` then `_ready` ordering), `<methods>`, `<members>`, `<signals>`. This is the best machine-readable class reference and inheritance graph in any engine I looked at, shipped in the repo, schema at `doc/class.xsd`.
- `modules/gdscript/README.md` (19.9 KB): a real architecture document (module integration, `ScriptLanguage`/`ScriptServer`/`ClassDB` interplay, tokenizer, parser, analyzer, compiler stages). Pipeline source files: `gdscript_tokenizer.cpp`, `gdscript_parser.cpp` (230 KB), `gdscript_analyzer.cpp` (279 KB), `gdscript_compiler.cpp` (133 KB), `gdscript_vm.cpp` (131 KB), `gdscript_byte_codegen.cpp`.
- `core/` (object model: `core/object/object.h`, `class_db.h`), `scene/` (nodes), `servers/` (rendering/physics/audio servers), `modules/`, `platform/` (OS ports), `drivers/` (GL/Vulkan/audio backends), `editor/` (largest by file count, 1,819 files).
- CLI-generated artifacts that need a built editor and therefore cannot come from a static ingest: `--doctool`, `--dump-extension-api`, `--dump-extension-api-with-docs`, `--dump-gdextension-interface-json` (`main/main.cpp:670-673, 1587-1617, 1704`). An optional "sidecar import" for `extension_api.json` would give a complete JSON class/method/signature dump.

**Exclude or down-rank for a learner.** `thirdparty/` (index metadata only, or separate low-priority scope), `*.po` translations (`doc/translations`, `editor/translations`), `editor/icons` and other `.svg` (1,210 files), `.woff2`/`.ttf`/`.webp`/`.png`, `platform/android/java` build scaffolding, `misc/dist` (installer assets), generated/dumped giant files, `.patch` files in thirdparty (87).

**Evidence on tooling.** There is no official Godot indexer. The editor ships its own GDScript language server (`modules/gdscript/language_server/`, built conditionally per `modules/gdscript/SCsub`) and the third-party GDScript grammar README tracks "latest Godot commit syntactically synced". Not an LLM-oriented source of scale or performance numbers.

**Fit: reference_only.** Use Godot as (a) the primary stress test for "huge C++ with registration macros plus doc XML plus GDScript", (b) a source of the XML schema and sidecar idea.

**Concrete ideas to adopt (all for the vault, licence-safe because they are ideas, not code).**
1. Honor `.gitattributes` `linguist-vendored|generated|language` and `.gdignore` during the walk. Effort S. Crate: `ignore` for .gitignore handling; parse `.gitattributes` with a small custom parser (about 80 lines) or `gix-attributes` (MIT/Apache) if already in the dependency tree.
2. Ingest `doc/classes/*.xml` as first-class documents and as a structured class graph (inherits, methods with signatures, signals, enums). Effort M. Crate: `quick-xml` (MIT). Output: per-class synthetic "reference" document that links to the C++ definition by name; powers `define("Node")` and an inheritance-tree section in the repository document.
3. Extract "binding aliases" from `bind_method(D_METHOD("x"...), &C::y)`, `ADD_SIGNAL`, `ADD_PROPERTY`, `GDREGISTER_CLASS`, `GDCLASS(C, Base)`: store as edges (script-name <-> C++ symbol; class <-> base). Effort M (regex over trigram candidates, verified by the grep crates). Gives the "follow definitions across languages" capability for exactly the pattern the owner will meet.
4. Treat `*.tscn`/`*.tres`/`project.godot`/`*.import` with a line parser: node tree, script attachment and `res://` reference edges; index as outline items. Effort S-M. No grammar crate needed. A grammar exists (`PrestonKnopp/tree-sitter-godot-resource`, MIT, 21 stars, pushed 2025-09-27) if later wanted.
5. Per-module architecture map: derive units from SCons `SCsub`/`config.py` presence plus top-level directories; list per module LoC, key types by reference count, and entry registration function (`register_types.cpp` exists per module, for example `modules/gdscript/register_types.cpp`). Effort M.
6. Large-file protocol: if file > N KB, `read` returns outline plus line map, not content, unless a range is given. Effort S (policy), the tools already exist.

---

## 3. Quake III Arena (id-Software/Quake-III-Arena)

**What it is.** The 2005 GPL source release (commit message in the single mirrored commit: "The Quake III Arena sources as originally released under the GPL license on August 20, 2005."). C (plus C++ for the Radiant editor), 1999-era style. Licence: GPL-2.0 (`COPYING.txt`); `README.txt` lines 12-30 list non-GPL portions (`code/qcommon/unzip.c` and `libs/pak/unzip.cpp` under a zlib-style notice by Gilles Vollant, and an MD4 file). Maturity: frozen. Currency: last upstream push 2024-08-02, no development; the community fork ecosystem (ioquake3) superseded it, not examined here.

**Scale and layout (from `git ls-files`).** 1,409 files. `code/` 614 files 13.5 MB (engine, renderer, botlib, VM modules); `lcc/` 440 files 5.2 MB (a vendored C compiler with tests); `q3radiant/` 229 (editor, C++); `q3map/`, `libs/`, `common/`, `q3asm/`. By extension: `.c` 445 files 355k lines; `.h` 337 files 65k lines; `.cpp` 129 files 77k lines. Noise: `.sbk`/`.1bk`/`.2bk` compiler test baselines (about 263 files, 215k lines), seven `.exe` binaries (e.g. `lcc/bin/q3rcc.exe` 848 KB), `.vcproj` x16, `.pbxproj`, `Makefile`, `Conscript`/`Construct` (the Cons build tool), no `.gitignore`. Game assets (shaders, maps, `.pk3`) are not in the repo; searching the tree finds no `.shader`, `.bsp`, `.pk3`.

**Why it is hard (and a good test).**
1. Same source file, several build units. Each VM module has a `.q3asm` file listing its sources: `code/game/game.q3asm` (g_main, g_syscalls, bg_misc, bg_lib, bg_pmove, bg_slidemove, q_math, q_shared, ai_*, g_*), `code/cgame/cgame.q3asm` (cg_*, plus the same bg_* and q_* files), `code/ui/ui.q3asm`, and `q3_ui`. The shared files (`bg_misc.c`, `bg_pmove.c`, `q_shared.c`) are compiled into game and cgame, and again with `-DMISSIONPACK` into a second set of objects (`code/unix/Makefile:1134-1135`: `$(B)/missionpack/cgame/bg_misc.o : $(GDIR)/bg_misc.c; ... -DMISSIONPACK`). 391 `MISSIONPACK` references in 48 files; 16 `Q3_VM` references. A "definition of X" is therefore per build target and per `#ifdef`.
2. Macro and conditional-compilation density: 5,727 `^#define` lines in `code/`. Example: `VectorCopy` has three definitions selected by `#if` branches (`code/game/q_shared.h:614, 623, 636`).
3. Integer-ID cross-boundary ABI instead of symbol linkage. The game module asks the engine to do work with `syscall(G_PRINT, fmt)` inside wrapper `trap_Printf` (`code/game/g_syscalls.c:26-35, 44-49`), where `G_PRINT` etc. are enum members in `code/game/g_public.h` (`GAME_API_VERSION 8` at line 26, `G_LOCATE_GAME_DATA` at 145). The engine implements them in `SV_GameSystemCalls` with `case G_PRINT:` (`code/server/sv_game.c:317-319`); the client side uses `CG_PRINT` (`code/cgame/cg_public.h:75`). Inbound calls go through `vmMain( int command, ...)` (`code/game/g_main.c:197-203`) with `GAME_INIT` etc. (`g_public.h:398+`). No grep for a function name connects `trap_Printf` to its implementation; only the enum constant does. The same pattern repeats for cgame (`CG_*`) and ui (`UI_*`).
4. Table-driven configuration: `gameCvarTable[]` in `code/game/g_main.c:99-107` declares cvars as data rows; "where is `g_cheats` registered" is a search for a string, not a definition.
5. Build metadata is in five different dialects (Makefile, Cons, vcproj, pbxproj, q3asm), none is CMake.

**What carries meaning for a learner.** `code/game/` (server-side game logic, 35 `.c` files), `code/cgame/` (client-side), `code/qcommon/` (shared engine services: cvars, filesystem, VM), `code/renderer/`, `code/botlib/`, `code/server/`, `code/client/`, the `*.q3asm` files and `g_public.h` / `cg_public.h` / `bg_public.h` (the real module interfaces). **Exclude/down-rank:** `lcc/` (vendored compiler, 5.2 MB with 215k lines of test baselines), `q3radiant/` (editor, separate subject), `jpeg-6/`, binaries, IDE project files, `.sbk` baselines, `code/win32/mod-sdk-setup/` (binary toolchain copy: `q3rcc.exe` is duplicated there).

**Ingest/index/rank/expose.** N/A (not an indexer). Relevant for the vault: it is the best small test of "same file in multiple build contexts" and "ID-linked cross-boundary calls".

**Licence flag.** GPL-2.0. Ingesting for private study is fine. If the vault ever exports or shares generated summaries or maps, those summaries quote GPL code; keep per-repo licence metadata on every document and make it visible in tool output.

**Fit: reference_only (test corpus).**

**Ideas.**
1. Build-target discovery from `.q3asm` lists (and the generic equivalents: `CMakeLists.txt` `add_library/add_executable`, `Cargo.toml` workspace members, `SCsub`, `*.Build.cs`) producing a `module` attribute on every file. Search and `define` results carry `module=`. Effort M.
2. Record, do not evaluate, the preprocessor condition stack at each symbol (`MISSIONPACK`, `Q3_VM`, `_WIN32`). Show "defined under #if MISSIONPACK" on `define` results. Effort M (tree-sitter-c exposes `preproc_if/ifdef` nodes; tag stack while walking).
3. "Enum link" edges: if wrapper function `trap_X` contains `syscall(G_X ...)` and a `case G_X:` exists elsewhere, link them. Generalization (S): for every enum constant, show both its definition and every `case`/use site; the existing `citations`/`define` tools already cover this if the symbol index includes enumerators. Effort S for indexing enumerators, M for explicit "ABI link" labelling.
4. Data-table awareness: index string literals in initializer rows as search-able "registration" entries. Effort S (trigram already finds them; just rank string-in-table hits near the definition of the table).

---

## 4. Bevy (bevyengine/bevy)

**What it is.** Rust data-driven game engine on an ECS. License `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE` at root; 73 copies of each, one per crate). Version `0.20.0-dev`, edition 2024, `rust-version = "1.96.0"` (`Cargo.toml:3-` header, `crates/bevy_ecs/Cargo.toml:11` says 1.95.0 for that crate). Very active (HEAD commit "UI layout benchmarks (#25973)", 2026-10-01). Latest tagged release v0.19.1.

**Scale.** 3,055 files. 1,925 `.rs` files = 664k lines / 23.3 MB; 63 crates under `crates/` (`bevy_a11y` ... `bevy_world_serialization`, `bevy_internal` aggregates). `assets/` is 381 files and 56 MB (ktx2, png, glb, bin, ogg, ttf): pure noise for code study. `examples/` 526 files, 442 `[[example]]` entries in the root `Cargo.toml` (e.g. lines 817-834) each with `[package.metadata.example.<name>]` giving display name, `category`, and flags like `hidden = true` / `wasm = true`. `examples/README.md` is generated (header comment line 2: "Use 'cargo run -p build-templated-pages ...'"). `_release-content/migration-guides` (11 files in this snapshot), `errors/` (`B0001.md`...), `docs/` (5 files), `tools/` (53 files: CI, example showcase, wasm builder).

**Macro/reflection situation.**
- Procedural macros define most of the engine API surface: `#[proc_macro_derive(Bundle...)]` at `crates/bevy_ecs/macros/src/lib.rs:58`, `SystemParam` :275, `QueryData` :559, `Event` :613, `Message` :642, `Resource` :669; reflection in `crates/bevy_reflect/derive/src/lib.rs:416` (`Reflect`), :448 (`FromReflect`), :494 (`TypePath`) plus function-like macros at :687, :751, :777, :842, :886; plus `Asset` (`bevy_asset/macros/src/lib.rs:17`), `States`, `ExtractResource`, `AsBindGroup`-style derives in `bevy_render/macros`. Counting `grep` hits: 132 `macro_rules!` definitions, 19 files using `all_tuples!` (generates trait impls for tuples of arity 1..N), 1,563 `derive(...Reflect...)` lines, 1,769 `derive(...Component...)` lines.
- Consequence: `#[derive(Component)] struct Foo;` creates `impl Component for Foo` invisibly. Questions like "list every Component", "what Resources exist" are answerable only by treating the derive list as data attached to the struct, not by finding `impl` blocks.
- Feature gating: 1,754 `#[cfg(feature = ...)]` occurrences in 334 files; features are documented in `docs/cargo_features.md` and per-crate `Cargo.toml` (`crates/bevy_ecs/Cargo.toml` `[features]`, with `##` doc comments on each feature). Cargo features play the role Q3's `MISSIONPACK` plays.
- Shaders are WESL (`.wesl`, a WGSL extension): 212 files, 28,986 lines, for example `crates/bevy_pbr/src/render/fog.wesl` with `import package::render::{mesh_view_bindings::fog, mesh_view_types::Fog,};` at lines 6-9. `Cargo.toml:5859-5870` has `[package.metadata.wgsl-analyzer]` giving `root = "assets/shaders"` and dependency crates, a ready-made shader module map. No WESL grammar turned up in my GitHub searches (UNVERIFIED that none exists); a WGSL grammar for Bevy's older import dialect exists (`tree-sitter-grammars/tree-sitter-wgsl-bevy`, MIT, pushed 2026-09-13).

**Learner-meaningful files.** `crates/bevy_ecs` (core), `bevy_app`, `bevy_reflect`, `bevy_render`/`bevy_pbr`/`bevy_core_pipeline`, `bevy_asset`, `bevy_scene`; `examples/` (organized by category directories: `2d`, `3d`, `ecs`, `reflection`, `shader`, `ui`, ...); `docs/`, `errors/`, `_release-content/migration-guides/` (API change narratives). **Exclude/down-rank:** `assets/`, `.ktx2/.glb/.bin`, dedupe 146 copies of LICENSE files (hash-based), `.stderr` (44 compile-fail expectation files), `benches/` (108 files), `.github/`, `crates/**/tests` data. `Cargo.lock` is gitignored, so dependency resolution is unavailable.

**Ingest/index/rank/expose.** N/A (not an indexer).

**Fit: reference_only (test corpus).**

**Ideas.**
1. Workspace map from `Cargo.toml` (parse with `toml`/`toml_edit`; no `cargo` invocation): crate list, path dependencies between crates, features. Gives crate dependency graph and a ranked "core crates" list by in-degree. Effort S-M. This pattern generalizes to CMake `target_link_libraries`, Godot module `config.py` dependencies, UE `Build.cs` `PublicDependencyModuleNames`.
2. Attach derive lists and attribute macros to symbols: `struct Foo` -> `derives=[Component, Reflect]`, `#[require(...)]`. Search `derive:Component` returns every Component. Effort S (tree-sitter-rust `attribute_item` nodes).
3. Example catalogue: use `[package.metadata.example.*]` as a curated "start here" index: name, category, description. Effort S.
4. Dedupe repeated files (73 identical LICENSE pairs) by content hash so they do not flood grep. Effort S.

---

## 5. Unreal Engine (public documentation only, source gated)

**What I verified from Epic docs (fetched via summarizing fetcher, so paraphrases; page URLs under `dev.epicgames.com/documentation/en-us/unreal-engine/`).**
- Module structure (`unreal-engine-modules`): each module has `[ModuleName].Build.cs` deriving `ModuleRules`; dependency lists `PublicDependencyModuleNames` (used in public headers) and `PrivateDependencyModuleNames` (used in .cpp); a `Public/` folder (headers visible to dependants) and `Private/` folder (everything else); `IMPLEMENT_MODULE` in a `[ModuleName]Module.cpp`; `.uproject`/`.uplugin` carry a `Modules` list with `Name`, `Type` ("Runtime" or "Editor"), `LoadingPhase`. UBT ignores IDE solutions and builds from `Build.cs`.
- UHT (`unreal-header-tool-for-unreal-engine`): "custom parsing and code generation tool"; UBT invokes UHT first, then the C++ compiler; UHT is written in C#; built-in exporters are `Codegen`, `Json` (package descriptions) and `Stats`; plugins are recognised by a `<PLUGIN_NAME>.ubtplugin.csproj`. The page does not say where generated files go. The JSON exporter is the interesting finding: a machine-readable dump of the reflection database exists, but only when UHT runs on a real engine/project build.
- Reflection overview page names `UCLASS`, `USTRUCT`, `UPROPERTY`, `UFUNCTION` but gave nothing about `GENERATED_BODY` or `.generated.h`; those details are from my background knowledge (UNVERIFIED here): headers include their `*.generated.h` last, UHT emits `*.generated.h` and `*.gen.cpp` into a per-module `Intermediate/Build/<Platform>/.../Inc/<Module>/UHT` tree.
- VS Code page: `GenerateProjectFiles.bat -vscode` produces a `.code-workspace`; it says nothing about `compile_commands.json`.
- Licence: UE source requires accepting Epic's EULA and is not on public GitHub (EULA page 403, UNVERIFIED). Treat any Unreal engine source folder as licensed-private: never export, never mirror, and keep it out of any shared or published corpus. The owner's own project code (`Source/<Game>/`, `*.Build.cs`, `.uproject`) is fine.

**Hard parts for an ingester (combining docs with the evidence in sections 6-7).**
1. API export macros between `class` and the name (`class FOO_API AActor : public UObject`). tree-sitter-cpp's `class_specifier` is `'class' _class_declaration` with only `ms_declspec_modifier`/attributes allowed before the name (`ts-cpp/grammar.js:229-258`), and UnrealCopilot reports the symptom directly: "tree-sitter-cpp misidentifies these macros as the class name, breaking hierarchy extraction" and strips them with `\b(class|struct)\s+[A-Z][A-Z0-9_]*_API\s+` while preserving line count (`unreal-copilot/Content/Python/unreal_copilot/cpp_analyzer/analyzer.py:424-432`). Upstream tree-sitter-cpp has open macro-related issues (#85 "Macro prevents correct parsing of class", #40 "Macro annotations", #296, #299, #300, #307, #309, #317).
2. Annotation macros `UCLASS(...)`, `UPROPERTY(...)`, `UFUNCTION(...)`, `USTRUCT`, `UENUM`, `UINTERFACE`, `GENERATED_BODY()`, and `DECLARE_DYNAMIC_*_DELEGATE*` appear as bare call-like statements before declarations. UnrealCopilot detects them by regex (`cpp_analyzer/patterns.py:28-52`, macro set at 55-72) rather than via the AST. The specifier lists inside the parentheses (`BlueprintCallable`, `EditAnywhere`, `Category=...`) carry the semantics that tell a learner what is exposed to Blueprint/editor (`patterns.py` has a `BLUEPRINT_SPECIFIERS` set).
3. Generated headers do not exist until a build runs. `unreal-clangd`'s README explains that `GENERATED_BODY` uses `__LINE__` and that after editing above it you must build to clear errors (`unreal-clangd/README.md:459-461`), and that the extension refreshes via UBT to produce compile commands (`README.md:182-190`); its roadmap says "Retry using GenerateClangDatabase again" (`README.md:163`), implying an earlier attempt with that UBT mode was dropped. A code search listing showed `GenerateClangDatabase.cs` under `Engine/Source/Programs/UnrealBuildTool/Modes/`, confirming the mode exists (path only; I did not open it).
4. Blueprints are binary `.uasset`/`.umap`. They hold gameplay logic and cannot be indexed as text. Open readers: `atenfyr/UAssetAPI` (C#, MIT, pushed 2026-08-31, 504 stars), `FabianFG/CUE4Parse` (C#, Apache-2.0, pushed 2026-09-30, 648 stars), `AstroTechies/unrealmodding` (Rust, MIT, pushed 2026-07-18, 88 stars; crate name for the asset reader UNVERIFIED, check before use). UnrealCopilot instead analyses Blueprints by running inside the live editor and exposing an HTTP bridge (`ARCHITECTURE.md`: UE built-in Python plus plugin HTTP API on port 8080, MCP on 19840).
5. Shaders are `.usf`/`.ush` (HLSL with heavy macros and virtual `/Engine/...` includes) under `Engine/Shaders` (UNVERIFIED; not in fetched docs).
6. Scale: UE is millions of lines across `Engine/Source/Runtime|Editor|Developer|Programs|ThirdParty` and `Engine/Plugins` (layout from background knowledge, UNVERIFIED; the layout doc fetch failed).

**What carries meaning for a learner.** The owner's own project `Source/`, `Plugins/*/Source`, `*.Build.cs`/`*.Target.cs`, `.uproject`/`.uplugin`, `Config/*.ini` (DefaultEngine.ini, DefaultInput.ini). For engine code (if the owner has licensed access): `Runtime/Core`, `CoreUObject`, `Engine`, plus public headers. **Exclude:** `Intermediate/`, `Binaries/`, `Saved/`, `DerivedDataCache/`, `ThirdParty/`, `Content/**/*.uasset|umap` (binary: index metadata stubs only), `*.generated.h`/`*.gen.cpp` (generated; either exclude or tag `generated` and down-rank).

**Fit: reference_only.** No Unreal-source repository to adopt; ideas below.

**Ideas.**
1. A "macro profile" pre-pass, selected per repository or per language detection (`unreal`, `godot`, `quake3`, `generic`): line-count-preserving rewrite before tree-sitter parsing (blank out `*_API` between `class|struct` and the name; for `UCLASS(...)`/`UPROPERTY(...)`/`UFUNCTION(...)` capture the macro text as an attached annotation object for the next declaration and replace the text with spaces; leave `GENERATED_BODY()` as an ignorable statement). Because the replacement keeps byte offsets and newlines, addresses `doc_id#L40-L72` still point at verbatim stored text. Effort M. Crate: `regex` (MIT/Apache) or hand-rolled scanner; no GPL dependency. This is the single highest-leverage item for the Unreal case.
2. Parse specifier lists into structured metadata (`blueprint_callable`, `category`, `replicated`) stored in SQLite so `find_docs`/`search` can filter "all BlueprintCallable functions in module X". Effort M.
3. Build-file parsers for `*.Build.cs` (C# class with `PublicDependencyModuleNames.AddRange(new string[] { ... })` calls: extract string literals from those calls with tree-sitter-c-sharp or a regex), `.uproject`/`.uplugin` (JSON), `Config/*.ini`. Effort S-M. Produces the module dependency graph for the architecture map.
4. Optional sidecar ingestion of UHT JSON exporter output or `compile_commands.json` placed by the user next to the repo, when present. Effort S (just a documented drop-in file); do not run UBT/UHT from the vault.
5. Binary assets (`.uasset`, `.umap`, `.blend`, `.fbx`, textures): metadata stub document (path, size, type, referenced-by from text files) so `grep`/`workspace` can see that the asset exists; never store the bytes as a document. Effort S. Later, optional extraction via CUE4Parse/UAssetAPI is out of the Rust dependency tree (C#), so run as an external one-shot tool by the owner.

---

## 6. Tree-sitter grammars (the parsing layer for the cluster's languages)

All are MIT and have Rust bindings (`Cargo.toml` `tree-sitter-language = "0.1"` shim, so they are ABI-compatible with the tree-sitter runtime regardless of the minor version each pins). Crate names confirmed from each `Cargo.toml` `name`: `tree-sitter-cpp`, `tree-sitter-hlsl`, `tree-sitter-glsl`, `tree-sitter-gdscript`, `tree-sitter-lua`, `tree-sitter-c-sharp`, `tree-sitter-cmake`. Whether each exact version is published on crates.io was NOT checked (Cargo versions: cpp 0.23.4, hlsl 0.2.0, glsl 0.2.0, gdscript 6.1.0, lua 0.5.0, c-sharp 0.23.5, cmake 0.7.5).

| Grammar | grammar.js | generated parser.c | Test corpus | queries shipped | Notes |
|---|---|---|---|---|---|
| cpp | 1,603 lines | 25.9 MB (772k lines) | many files incl. `microsoft.txt`, `reflection.txt`, `c/` | highlights, injections, tags | `tags.scm` covers struct, union, class, function, method, type, enum only (no namespaces, fields, variables, macros) |
| hlsl | 101 lines | 20.8 MB | 1 file (`basic.txt`) | none (open issue #70 "feature: highlights.scm") | `const CPP = require("tree-sitter-cpp/grammar")` and `grammar(CPP, {...})` (`grammar.js:1-3`); adds `semantics` (`: SV_DispatchThreadID`), `hlsl_attribute` (`[numthreads(...)]`), `cbuffer`, qualifiers (`groupshared`, `row_major`, ...), `discard` |
| glsl | 158 lines | 5.5 MB | 1 file (`basic.txt`) | highlights only | extends tree-sitter-c (`grammar.js:1`); adds `subroutine`, `preproc_extension`; removes K&R function definitions |
| gdscript | 951 lines | 2.6 MB | 15 files incl. `issues/` | none in repo (queries live in nvim-treesitter per its README) | external scanner 628 lines for indentation; README: "Latest Godot Commit Syntactically Synced" with a hash and "some commits may have been missed" |
| lua | 634 lines | 356 KB | 4 files | highlights, injections, locals, tags | `tags.scm` captures function declarations, methods, assigned functions, table-field functions and `reference.call` |
| c-sharp | 2,420 lines | 32.0 MB (largest) | 19 files incl. `preprocessor.txt` | highlights, tags | `tags.scm` has class, interface, method, object creation, invocation, namespace; no struct/enum/property/field definitions |
| cmake | 117 lines | 539 KB | 13 files | folds, highlights, indents, injections | no tags; `function_call` nodes are the commands (`add_library`, `target_link_libraries`) |

Total generated C across these seven is about 88 MB of parser source, which matters for compile time and binary size when statically linked (not measured).

**Per-grammar findings.**
- **HLSL and GLSL are thin extensions of the C++ and C grammars** and track them via `package.json` pins (GLSL pins `tree-sitter` `^0.22.1`; the repos' last real commits are 2025-03-23 and 2025-03-16 though GitHub shows pushes in 2026-09, probably automation). Corollaries: (a) they inherit C/C++ macro weaknesses; (b) they lack tags queries, so symbol extraction needs our own `.scm`; (c) tests are minimal, so unusual syntax is a risk. HLSL gaps I can see from the grammar (UNVERIFIED by running): no explicit `register(b0)`/`packoffset` rule, so `: register(...)` likely goes through the generic `semantics: ":" identifier` or parses as error; Unity ShaderLab wrappers and Unreal `.usf`/`.ush` virtual-path includes are out of scope for the grammar.
- **Godot's own shaders are not plain GLSL.** `servers/rendering/renderer_rd/shaders/effects/blur_raster.glsl` begins with `#[vertex]`, `#version 450`, `#VERSION_DEFINES`, `#include "blur_raster_inc.glsl"`. The `#[vertex]`/`#[fragment]`/`#[compute]` section markers and `#VERSION_DEFINES` are Godot-specific preprocessing and will probably be parse errors or odd preproc nodes (UNVERIFIED). Godot's user shading language `.gdshader` has only tiny community grammars (`GodOfAvacyn/tree-sitter-gdshader` 9 stars, pushed 2025-08-02; `airblast-dev/tree-sitter-gdshader` 3 stars, pushed 2025-11-23): not production grade.
- **GDScript grammar lags the language.** Open issues at the time of study include #96 (NULL pointer check in scanner.c, CWE-476), #95 and #92 (trailing comma in match patterns), #94 and #89 (trailing separators in hex/binary literals), #93 (`$` and `%` together in `get_node`), #90 (empty match statement). All are parse errors on valid Godot 4 code. `grammar.js:379-408` has the nodes needed for tags: `signal_statement`, `class_name_statement`, `extends_statement`, plus functions, variables and constructors; `annotations` are first-class (`@export`, `@onready`).
- **C# for Unity/Godot-mono:** `tags.scm` exists but is shallow; the grammar handles preprocessor directives (`test/corpus/preprocessor.txt`).
- **tree-sitter-cmake:** fine for extracting targets and link edges; CMake is also macro-like (`function`, `macro`, generator expressions `$<...>`); extraction should be pattern-based on command names.

**Fit and ideas.**
- `adopt_dependency` for all seven grammars, with these conditions: write and ship our own `tags`-style queries per language (the repository-supplied ones are too shallow for "map any repository"), pin versions, and add a self-test that parses each repo in a fixture set and reports the ERROR-node ratio per file.
- Parse-failure policy: never drop a file because parsing fails. Keep the verbatim document, trigram index and ripgrep-verified grep, and mark the symbol layer `partial` with the ERROR ratio. That keeps the target use case working even where macros break the symbol layer.
- Effort: S per grammar for wiring (`tree-sitter` crate, MIT), M overall for the query set per language (tags + outline + reference captures). Licence-safe (MIT). Consider lazy loading: C# (32 MB) and C++ (26 MB) parsers dominate size.

---

## 7. Tooling that already tries to serve LLMs or IDEs on engine code

### 7.1 UnrealCopilot (syan2018, extra, reference_only; no licence file so do not copy code)

- What: UE5 editor plugin (C++, `Source/UnrealCopilot`, 13 `.cpp` and 7 `.h` files) plus an MCP server in Python (`Content/Python/unreal_copilot`, about 3,159 lines in the tools and analyzer files, `analyzer.py` 1,396 lines) running inside UE's embedded Python. Stats from `git ls-files`: 82 files. HEAD aec7332 (2026-03-15).
- Ingest: no walking policy beyond scopes `project|engine|plugin|all` from config (`cpp_analyzer/analyzer.py:395-425`, `get_source_paths`). Parses on demand and caches ASTs, max 1,000 entries with FIFO eviction (`analyzer.py:321-349`). No file-watching, no persistent index.
- Index: tree-sitter-cpp (`analyzer.py:26-28, 312-313`) plus regex pattern detection for UE macros (`patterns.py`). Class info (methods, properties, base types), hierarchy, references. No trigram, no embeddings, no call graph.
- Rank: none observed (listing results; `search` is a unified search over C++/Blueprint/Asset in `tools/unified.py`).
- Exposes (`README.md:176-183`): `search`, `get_hierarchy`, `get_references` (outgoing/incoming/both), `get_details`, `get_blueprint_graph` (Mermaid/summary/JSON), `detect_ue_patterns`, `trace_reference_chain`, `find_cpp_class_usage`; skill tools `list_unreal_skill(query?, include_hidden?)`, `read_unreal_skill(skill_name, path?)`, `run_unreal_skill(skill_name?, script?, args?, python?)`. Blueprint and asset tools require the live editor (HTTP bridge), so they are not offline.
- Scale/perf evidence: none given.
- Value for us: the only concrete public evidence of the `_API` parse failure and its fix; the idea of "cross-domain reference chain" (Blueprint to C++ to asset); the "skills" mechanism (SKILL.md plus scripts, discoverable via list/read/run).
- Adopt: idea only. `port_ideas`: line-preserving macro pre-pass (M), cross-domain `trace_reference_chain` (L, depends on asset stubs).

### 7.2 unreal-clangd (boocs, extra, reference_only; no licence file)

- A VS Code extension for UE 5.2-5.7 using clangd. This repository branch contains only `README.md` (816 lines, 39 KB) and `.gitignore`; extension code is on another branch (`v3`) that I did not clone.
- Evidence that matters: it drives UBT to refresh the project and produce compile commands (`README.md:182-190`); the roadmap lists "Retry using GenerateClangDatabase again" (`README.md:163`); clangd preparse includes and macro completions are slow for UE, so it ships a curated header list with a fast/slow toggle (`README.md:216-245`); `GENERATED_BODY` uses `__LINE__`, so edits above it cause red squiggles until rebuild (`README.md:459-461`); full-source-engine projects are supported (`README.md:526`).
- Lesson: accurate semantic indexing of UE requires UBT-generated compile commands and generated headers, which the vault will not have. So the vault's symbol layer must be heuristic and tolerant, with optional compile-DB import for users who have one. `reference_only`.

### 7.3 Godot MCP servers

- `Coding-Solo/godot-mcp` (MIT, 5,897 stars, HEAD 1209744, 2026-04-16): 14 tools registered in `src/index.ts` (lines 671-913): `launch_editor`, `run_project`, `get_debug_output`, `stop_project`, `get_godot_version`, `list_projects`, `get_project_info`, `create_scene`, `add_node`, `load_sprite`, `export_mesh_library`, `save_scene`, `get_uid`, `update_project_uids`. Executes a real Godot binary headless with `src/scripts/godot_operations.gd` (1,186 lines) via `execFile` with an argument array. It edits and runs projects; it does not search or read source. Not what the vault does. `reference_only`.
- `nuskey8/godot-docs-mcp` (MIT, 25 stars, 2025-10-02): three tools `godot_docs_search(query)`, `godot_docs_get_page(url)`, `godot_docs_get_class(className)`. Fetches the live docs site at call time: `src/tools.ts:11-15` loads `https://docs.godotengine.org/en/stable/searchindex.js`; `:55` and `:73` fetch pages and `class_<name>.html`. No offline store, no versioning. The vault should do this offline from `doc/classes/*.xml` (in-repo, versioned with the source) and the `godotengine/godot-docs` RST (licence shows "Other/NOASSERTION" in GitHub's API; check its `LICENSE.txt` before ingesting; I believe it is CC BY 3.0, UNVERIFIED). `reference_only`.
- Others seen by metadata only: `yurineko73/Godot-MCP-Native` (816 stars), `youichi-uda/godot-mcp-pro` (paid, $15, closed), `ee0pdt/Godot-MCP` (616 stars, last push 2025-03).

### 7.4 gdtoolkit (Scony, extra, reference_only)

- Python (MIT, 1,615 stars, HEAD f61a1f3, 2025-10-09): a GDScript parser built on `lark` (LALR) from `gdtoolkit/parser/gdscript.lark` (318 lines) with a custom indenter, plus linter, formatter, and `gdradon` complexity metrics. Dependencies pinned at `lark[regex]==1.2.2` and `radon==6.0.1`. 391 test fixtures. Not usable from Rust, but its 318-line grammar and 353 `.gd` test files are a second oracle for validating a tree-sitter GDScript setup and a reference for GDScript syntax rules.

### 7.5 unreal-analyzer-mcp (ayeletstudioindia; README read via `gh api`, not cloned)

- MIT, 159 stars, last push 2025-08-06, tree-sitter ^0.20.1 / tree-sitter-cpp ^0.20.0 (old). Tools named in the README include `analyze_class`, class hierarchy, reference finding, code search, pattern detection, API doc query, and `analyze_subsystem`. The `analyze_subsystem` example output (core classes, key features, use cases for "Physics") looks like a hand-written knowledge base rather than something derived from the source (inference from the README sample; UNVERIFIED in code). Superseded in activity by UnrealCopilot.

---

## 8. Cluster summary

### 8.1 What makes game/engine codebases hard (observed, with evidence)

1. Macros that change what a declaration means: Unreal `*_API`, `UCLASS/UPROPERTY/UFUNCTION/GENERATED_BODY`; Godot `GDCLASS`, `GDVIRTUAL*`, `D_METHOD`; Quake III `#define` tables with per-platform variants; Bevy derive and `macro_rules!` families (132 definitions, tuple-impl generators). A plain tree-sitter symbol pass gets some of these wrong (tree-sitter-cpp issues #40, #85, #296-#317; UnrealCopilot's workaround).
2. Reflection and registration decouple names from definitions: Godot string-literal bindings (`node.cpp:3807`), Unreal UHT-generated code, Quake III integer syscall IDs, Bevy derive-generated trait impls.
3. Code generation at build time: Godot `*.gen.*` (gitignored, 30+ generator scripts), Unreal `.generated.h`/`.gen.cpp`, Bevy proc macros. The static repository never contains the final code.
4. The same file compiles in several contexts: Quake III shared `bg_*` files in game/cgame/ui and `-DMISSIONPACK`; Godot modules gated by build options; Bevy Cargo features (1,754 `cfg(feature)` uses).
5. Bytes dominated by non-code: Godot translations 116 MB of 323 MB and thirdparty 123 MB; Bevy assets 56 MB; Quake III compiler test baselines and binaries. An unfiltered walk spends most of its budget on junk.
6. Meaning lives outside source files: Godot `doc/classes/*.xml` (833 classes), `.tscn/.tres` scene text with `res://` and `ExtResource` references, Unreal Blueprints in binary `.uasset`, Bevy `Cargo.toml` example metadata and migration guides.
7. Huge single files (400-500 KB) that cannot be read whole.
8. Several shader dialects with non-standard preprocessing (Godot `#[vertex]`/`#VERSION_DEFINES`, Bevy WESL `import package::...`, Unreal `.usf` virtual includes) and thin or stale grammars for them (HLSL/GLSL: one test file each, last real commits March 2025).
9. Build systems vary (SCons `SCsub`, Cargo, Cons/Makefile/`.q3asm`, UE `Build.cs`, CMake) and none is a universal source of truth for "which files form which module".

### 8.2 Best practices the cluster converges on

- Stay on tree-sitter for structure plus a text/regex layer for macros the grammar cannot handle (UnrealCopilot, unreal-analyzer-mcp both do exactly this), with line-count-preserving preprocessing so positions stay valid.
- Treat the engine's own structured docs as the architecture map: Godot's XML class reference, module READMEs, Bevy crate metadata, Epic's module/`Build.cs` layout.
- Honor the repository's own ignore and classification signals: `.gitignore`, `.gitattributes` `linguist-*`, `.gdignore`, `thirdparty/` conventions.
- For binary or editor-resident data (Blueprints), the viable pattern is either live-editor introspection (UnrealCopilot's HTTP bridge) or an offline asset parser (CUE4Parse/UAssetAPI), never text indexing.
- Accept that perfect semantic resolution needs a build (clangd with UBT-generated compile commands, `scons compiledb`); tools that avoid a build use heuristics and say so.

### 8.3 What the vault must do to beat them for the target use case

(Target: an agent studies 5-7 folders of engine and game code and must map, search, follow definitions across files and repositories, and read exactly what it needs in few calls.)

1. Ship a per-repository ingest profile, auto-detected and overridable: file roles (`src`, `header`, `test`, `shader`, `build`, `data`, `doc`, `asset-binary`, `vendored`, `generated`, `translation`), with defaults for the engines above. Walk with `ignore`, add `.gitattributes` linguist rules, `.gdignore`, a generated-file heuristic (name patterns `*.gen.*`, `*.generated.h`, header banner text, extreme line length), and size caps. Excluded content becomes a metadata stub (path, size, role) rather than vanishing, so `workspace` can say "translations: 107 files, excluded". Rank role-aware: `src`/`header`/`doc` above `test` above `vendored`.
2. Macro profiles with the pre-parse rewrite (section 5 idea 1), annotation capture, and an explicit `partial` flag per file when the ERROR ratio is high. No other tool surveyed does this generically; UnrealCopilot hardcodes one regex.
3. Cross-language and cross-boundary edges, extracted cheaply and verified by exact text: Godot binding aliases, enum-ID call links (Quake), derive lists (Bevy), `ExtResource`/`res://` edges (scenes), include/import edges for shaders, `Build.cs`/`Cargo.toml`/`CMakeLists.txt`/`.q3asm` module membership. Each edge is a row in SQLite with provenance (file, line range) so `citations(direction)` can return them and every answer can quote real lines.
4. Build-target scoping: tag symbols with module and the enclosing `#if` condition stack; `define` returns all variants with their conditions, not one arbitrary hit. This beats clangd-based tools for the no-build case.
5. An architecture-level repository document generated deterministically (no LLM): module list with LoC and role mix, module dependency edges from build files, top types by inbound references, entry points (`main`, `vmMain`, `App::new`, `_bind_methods` registration roots), inheritance trees (from class declarations plus Godot XML `inherits`), shader module map, "start here" examples (Bevy example metadata), and links to any architecture READMEs found (for example `modules/gdscript/README.md`). Budget it to a fixed token size with drill-down by module via `outline`.
6. Output budgeting for large files: `outline(doc_ids)` must handle 500 KB files (symbols plus line ranges, nested), and `read(addresses, expand)` should expand around a symbol to its enclosing definition boundaries from the symbol table, not by fixed line counts.
7. Make trigram plus symbol exact match the first-class path for the long tail where the symbol layer is partial (macros, string-keyed bindings): results from `grep` should still be grouped by module and role and ranked with definition-like lines (those matching a tag query or a registration pattern) first.
8. Optional sidecars, never mandatory: Godot `extension_api.json`, UHT JSON, `compile_commands.json`. Documented drop-in locations; no build invocation from the vault.
9. Ingest-time safety for gated or restrictive licences: per-repo `licence` and `redistributable` fields (GPL: Quake III; Unreal source: licensed-private, never mirrored), surfaced in tool output and respected by any export feature.

### 8.4 Open questions for the lead (recorded, not decided)

- Is the vault allowed to hold Unreal Engine source at all, even locally? Needs the Epic EULA read directly (my fetch returned 403).
- How far should the symbol layer go for HLSL/GLSL given the thin grammars: tree-sitter only, or regex outline for shaders (function and cbuffer declarations) as the default with tree-sitter as an upgrade?
- Whether to build the macro pre-pass as configurable regex rules (data) or code. I recommend data (TOML rules per profile) so a new engine needs no recompile, but this is a design decision for the lead.
- Parser-size budget: statically linking C# (32 MB generated C) and C++ (26 MB) parsers is heavy; consider feature flags per language or dynamic loading.
- tree-sitter-hlsl and tree-sitter-glsl are effectively pinned to older cpp/c grammar versions; decide whether to vendor and regenerate or accept the lag.
