# Phase 0 - Rust building blocks for ingesting source-code repositories (Windows 11, MSVC, rustc 1.98)

Date: 2026-10-01. Author: worker (Sonnet 5.5) for the lead (Opus 5.5). Scope: verification and recommendations only. No design decisions are made here; where the choice is the lead's, the question is listed in section 13.

Evidence tags on numbers and claims:
- **[M]** measured by me today. Sources: crates.io API and downloaded `.crate` tarballs, GitHub API, and two throwaway Cargo projects I compiled and ran on this machine (rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1, MSVC 14.51.36231 from Visual Studio 18 BuildTools, x86_64-pc-windows-msvc).
- **[D]** documented by the cited source and not run by me. "D-self" means the project's own README/CI/comment; "D-ind" means an independent registry or platform (crates.io, GitHub API, iso.org, spdx.org).
- **[E]** my estimate or inference.

What I did and did not do (limits):
- I did not clone any repository (everything came from `gh api` raw reads, crates.io tarballs and WebFetch). I ran no code from any repository. I ran read-only `git` queries (`config`, `rev-parse`, `ls-files`, `log`) in the existing clone `C:\Users\sethy\Projects\refs\rag\jieba-rs`.
- I compiled crates.io dependencies in two scratch Cargo projects. That executes their `build.rs` scripts (C compilers via cc-rs) and goes slightly past "do not install packages", so I isolated it: `CARGO_HOME=C:\Users\sethy\tsc\h`, target dirs under `C:\Users\sethy\tsc\`, nothing installed globally, nothing written into the vault or OneDrive. The scratch directories were deleted at the end (section 14).
- WebSearch was exhausted (200 of 200 calls) after the first query, so the ISO side of ISO/IEC 18670 is not independently verified (section 5). iso.org returned HTTP 403.
- Process launches cost about 300 ms here; shell work was batched into a few scripts.

---------------------------------------------------------------------------

## 0. Summary and recommendations (the lead decides)

1. **Tree-sitter.** Pin `tree-sitter =0.27.0` and `tree-sitter-tags =0.27.0` (both 2026-08-30, MIT, MSRV 1.90). Link per-language grammar crates statically behind cargo features. All 36 grammar crates I built (the requested languages plus Kotlin-sg, OCaml, Elixir, Dart, Zig, Nix) compiled with MSVC in **24 s cold** and every one loaded into the 0.27 runtime [M]. Core runtime ABI is 15 with minimum compatible 13 [M]; grammar crates in the wild are ABI 13, 14 or 15 and all work [M]. Do not use `tree-sitter-language-pack` as the core path (runtime downloads, ABI 14 builds, Windows parser build failures in 2 releases, section 1.6). Do not mix in `tree-sitter-graph` / `tree-sitter-stack-graphs` (pinned to 0.24, stack-graphs archived).
2. **tags.scm coverage is the real limit, not the parsers.** Only 15 of the grammars I tested yield definition tags straight from a shipped `TAGS_QUERY` constant (Rust, Python, JavaScript, Go, Java, C, C++, Swift, Ruby, PHP, Lua, R, OCaml, Elixir, Dart); TypeScript, C#, Scala and Kotlin-sg work after patching or vendoring the query file; Haskell, Julia, Bash, PowerShell, SQL, Kotlin (ng), HTML/CSS/JSON/YAML/TOML/Markdown/LaTeX/Dockerfile ship none. C# ships a `tags.scm` that `tree-sitter-tags` 0.27 rejects. TypeScript needs the JavaScript query prepended. I wrote 3 small own queries (Bash, Haskell, Julia) and a SQL one and they worked first time [M] (section 1.5).
3. **Language detection.** `linguist` 0.1.16 (drshade, MIT, Linguist 9.x data, 2025-10 to 2026-09) for filename, extension and content heuristics, plus about 30 lines of our own shebang/interpreter matching (its data model already carries `interpreters`). `hyperpolyglot` is the most accurate in my micro-test (31/32) but its crate is from 2020, depends on `pcre2` (C build) and its data is old. Do not use `palate` (GPL-3.0-or-later).
4. **Licence detection.** `spdx` 0.13.5 with `detection` + `detection-inline-cache` (an in-crate fork of askalono; askalono itself is archived). Detected MIT, Apache-2.0, BSD-3-Clause, AGPL-3.0 texts correctly in 0.5-6 ms each [M]. `scallion` 0.1.0 (2026-09, new askalono fork) is a watch item. ScanCode (Python) is a reference, not a dependency; I found no Rust port.
5. **Secret scanning.** Use the **gitleaks rule file as data** (`gitleaks.toml`, MIT, 222 rules) and write a small engine (Aho-Corasick keywords, then `regex`, then entropy, then allowlists). All 221 regex rules compile with the Rust `regex` crate once `size_limit` is raised above the 10 MiB default [M]. Entropy plus allowlists cut 1,149 raw matches to 54 on a 64 MiB corpus, and 51 of the 54 were one rule (`generic-api-key`) [M]. Betterleaks (the gitleaks author's successor, MIT, 465 rules) has much faster regexes (about 25 vs 1.6-2.2 MiB/s single thread) but relies on an Expr filter language and is at v2.0.0-rc.1 (section 4). Avoid TruffleHog (AGPL-3.0) and Nosey Parker (archived 2026-02).
6. **SWHID.** About 40 lines of our own SHA-1 code reproduce git's blob, tree and commit ids exactly (checked against `git`, `gix` and the `swhid` crate) [M]. **Hash blobs from the git object database, not the working tree**: this machine has `core.autocrlf=true`, which makes every working-tree directory id differ from the committed one [M]. ISO/IEC 18670:2025 is reported adopted on 2025-04-23 by swhid.org (D-self; ISO page unreachable); spec v1.2 released 2025-11-07 [D-ind via GitHub release].
7. **Git metadata.** `gix =0.88.0` with `default-features = false, features = ["sha1"]` opened a repo in 36 ms and returned HEAD id, branch, default remote URL, ref count and commit author on Windows [M]. Packed refs are read transparently.
8. **CITATION.cff / codemeta.json.** `serde-saphyr` 1.3.0 plus `serde_json::Value`. Parsed the real CFF 1.2.0 file and the real CodeMeta 4.0 file [M]. CodeMeta 4.0 was released on 2026-09-29 and changed the namespace; CFF is still 1.2.0 (2021).
9. **Notebooks.** The `nbformat` 3.0.0 crate works but returns its `Legacy` variant for nbformat 4.0 to 4.4 notebooks and needs an explicit upgrade step [M]. A 40-line own serde reader is the alternative (section 8).
10. **SCIP.** The `scip` 0.10.0 crate (protobuf `=3.7.2`) builds on Windows and round-trips an index [M]. Indexers: scip-clang and scip-ruby publish Linux/macOS binaries only, scip CLI and scip-go publish no Windows asset (D-ind via release asset lists). All need a build/dependency environment. Treat as opt-in.
11. **Identifier tokenizer.** No existing tantivy crate does camelCase/snake_case splitting. My own 60-line `Tokenizer` for tantivy 0.26.2 compiled and produced the expected splits [M] (section 10).

---------------------------------------------------------------------------

## 1. Tree-sitter: core, grammars, ABI, tags

### 1.1 Core crates (crates.io, [M] unless noted)

| Crate | Version | Released | Licence | MSRV | Note |
|---|---|---|---|---|---|
| tree-sitter | 0.27.0 | 2026-08-30 | MIT | 1.90 | ABI 15, min compatible 13 (header `lib/include/tree_sitter/api.h`, [M]); `links = "tree-sitter"` so only one core per build. 0.26 line still maintained (0.26.13 on 2026-08-23, D-ind GitHub release). |
| tree-sitter-language | 0.1.8 | 2026-08-30 | MIT | 1.90 | `LanguageFn` shim; grammars depend on `^0.1`, so they are decoupled from the core version. 0.27 requires `^0.1.8`. |
| tree-sitter-tags | 0.27.0 | 2026-08-30 | MIT | 1.90 | Definitions and references from `tags.scm`. |
| tree-sitter-highlight | 0.27.0 | 2026-08-30 | MIT | - | Not needed for ingestion. |
| tree-sitter-loader | 0.27.0 | 2026-08-30 | MIT | - | Dynamic grammar loading; needs a C compiler at runtime. Not recommended. |
| tree-sitter-graph | 0.12.0 | 2024-12-11 | MIT/Apache-2.0 | - | Depends on `tree-sitter ^0.24`: cannot sit beside 0.27 once `links` applies. Stale. |
| tree-sitter-stack-graphs | 0.10.0 | 2024-12-13 | MIT/Apache-2.0 | - | Repo `github/stack-graphs` archived (last push 2025-09-09, [D-ind GitHub API]). Do not use. |

GitHub: `tree-sitter/tree-sitter` pushed 2026-10-01, 27,097 stars, MIT [D-ind]. Its CI builds `windows-x64`, `windows-x86` and `windows-arm64` (MSVC targets) [D-self workflow].

### 1.2 Grammar crates for the requested languages

"ABI" is `TREE_SITTER_LANGUAGE_VERSION` read from each crate's `src/parser.c`, and it agreed with `Language::abi_version()` at runtime [M]. "tags.scm" = a `queries/*tags.scm` file inside the published crate; the pair in brackets is the count of `@definition.*` / `@reference.*` captures in that file [M]. "const" = the crate exports a `TAGS_QUERY` constant (compile-checked where I used it).

| Language | Crate | Version | Released | Licence | ABI | tags.scm in crate | TAGS_QUERY const |
|---|---|---|---|---|---|---|---|
| Rust | tree-sitter-rust | 0.24.2 | 2026-03-27 | MIT | 15 | yes (9/5) | yes |
| Python | tree-sitter-python | 0.25.0 | 2025-09-11 | MIT | 15 | yes (3/1) | yes |
| JavaScript | tree-sitter-javascript | 0.25.0 | 2025-09-01 | MIT | 15 | yes (13/3) | yes |
| TypeScript and TSX | tree-sitter-typescript | 0.23.2 | 2024-11-11 | MIT | 14 | yes (6/2), needs JS query prepended | yes |
| Go | tree-sitter-go | 0.25.0 | 2025-08-29 | MIT | 15 | yes (5/2) | yes |
| Java | tree-sitter-java | 0.23.5 | 2024-12-21 | MIT | 14 | yes (3/4) | yes |
| C | tree-sitter-c | 0.24.2 | 2026-04-22 | MIT | 15 | yes (5/0) | yes |
| C++ | tree-sitter-cpp | 0.23.4 | 2024-11-11 | MIT | 14 | yes (8/0) | yes |
| C# | tree-sitter-c-sharp | 0.23.5 | 2026-04-14 | MIT | 15 | yes (4/7), **rejected by tags 0.27 as shipped** | yes |
| Kotlin | tree-sitter-kotlin-ng | 1.1.0 | 2025-01-09 | MIT | 14 | no | no |
| Kotlin (ast-grep fork) | tree-sitter-kotlin-sg | 0.4.1 | 2026-05-09 | MIT | 14 | yes (7/3) | no (use `include_str!` of a vendored copy) |
| Swift | tree-sitter-swift | 0.7.3 | 2026-06-01 | MIT | 15 | yes (7/0) | yes |
| Ruby | tree-sitter-ruby | 0.23.1 | 2024-11-11 | MIT | 14 | yes (8/2) | yes |
| PHP | tree-sitter-php | 0.24.2 | 2025-08-18 | MIT | 15 | yes (7/5) | yes (`LANGUAGE_PHP`, `LANGUAGE_PHP_ONLY`) |
| Scala | tree-sitter-scala | 0.26.2 | 2026-08-08 | MIT | 15 | yes (15/5) | no (vendored copy works) |
| Haskell | tree-sitter-haskell | 0.24.1 | 2026-09-30 | MIT | 15 | no (highlights, injections, locals only) | no |
| Lua | tree-sitter-lua | 0.5.0 | 2026-02-26 | MIT | 15 | yes (4/1) | yes |
| R | tree-sitter-r | 1.3.0 | 2026-06-19 | MIT | 14 | yes (4/2) | yes |
| Julia | tree-sitter-julia | 0.23.1 | 2024-11-11 | MIT | 14 | no | no |
| MATLAB / Octave | tree-sitter-matlab | 1.3.1 | 2026-08-26 | MIT | 15 | only under `queries/neovim/` (2/2); not tested | declared, not tested |
| Bash | tree-sitter-bash | 0.25.1 | 2025-12-02 | MIT | 15 | no | no |
| PowerShell | tree-sitter-powershell | 0.26.4 | 2026-05-04 | MIT | 15 | no (highlights only) | declared, not tested |
| SQL | tree-sitter-sequel (DerekStride) | 0.3.11 | 2025-10-01 | MIT | 14 | no | declared, not tested |
| HTML | tree-sitter-html | 0.23.2 | 2024-11-11 | MIT | 14 | no | no |
| CSS | tree-sitter-css | 0.25.0 | 2025-09-28 | MIT | 15 | no | no |
| JSON | tree-sitter-json | 0.24.8 | 2024-11-11 | MIT | 14 | no | no |
| YAML | tree-sitter-yaml | 0.7.2 | 2025-10-07 | MIT | 14 | no | no |
| TOML | tree-sitter-toml-ng | 0.7.0 | 2024-12-03 | MIT | 14 | no | no |
| Markdown (block + inline) | tree-sitter-md | 0.5.3 | 2026-02-26 | MIT | 15 | no | no (`LANGUAGE`, `INLINE_LANGUAGE`) |
| LaTeX | codebook-tree-sitter-latex | 0.6.1 | 2025-12-23 | MIT | 15 | no | no |
| Dockerfile | tree-sitter-containerfile (wharflab) | 0.9.2 | 2026-07-04 | MIT | 15 | no (highlights, injections) | no |
| OCaml (extra) | tree-sitter-ocaml | 0.26.0 | 2026-09-13 | MIT | 15 | yes (11/24) | yes |
| Elixir (extra) | tree-sitter-elixir | 0.3.5 | 2026-03-02 | Apache-2.0 | 14 | yes (2/3) | yes |
| Dart (extra) | tree-sitter-dart | 0.2.0 | 2026-04-26 | MIT | 15 | yes (24/2) | yes |
| Zig (extra) | tree-sitter-zig | 1.1.2 | 2024-12-22 | MIT | 14 | no | no |
| Nix (extra) | tree-sitter-nix | 0.3.0 | 2025-07-18 | MIT | **13** | yes (5/3) in tarball | **no** (referencing `TAGS_QUERY` failed to compile) |

Notes:
- **Octave**: there is no separate Octave grammar on crates.io (`tree-sitter-octave` not found [M]). `tree-sitter-matlab` is the one grammar; the Octave `.m` dialect is not separately covered [E].
- **LaTeX provenance**: `tree-sitter-latex` 0.1.0 (2025-02-08, 113 recent downloads) points to a repository `tree-sitter/tree-sitter-latex` that returns 404 [M]; it is an unofficial publish. `codebook-tree-sitter-latex` 0.6.1 carries the `latex-lsp/tree-sitter-latex` grammar (repo pushed 2026-08-01, MIT) but is published by a third party (the codebook project, authors field names Patrick Foerster) [M]. Both ship no queries in the crate. I used the codebook one [E: provenance not independently verified].
- **Dockerfile**: `tree-sitter-dockerfile` 0.2.0 (2024-05) depends on `tree-sitter ^0.20` and returns the old-core `Language` type, so it cannot be used with the 0.27 runtime [E]. `tree-sitter-containerfile` 0.9.2 (first release 2026-04-25, 161,724 recent downloads, 10 versions) is the maintained one [M].
- **Old crates that collide with the 0.27 core** [M for the first, E for the rest]: `tree-sitter-kotlin` 0.3.8 requires `tree-sitter >=0.21, <0.23`; Cargo refused to resolve it beside 0.27 with "package `tree-sitter` links to the native library `tree-sitter`, but it conflicts with a previous package". `tree-sitter-dockerfile` 0.2.0 and `tree-sitter-toml` 0.20.0 (require `^0.20`) did resolve (0.20.x has no `links` key) but they would link a second copy of the C runtime and hand out a different `Language` type; I did not attempt the link. Same family: `tree-sitter-sql` 0.0.2 (2021), `tree-sitter-markdown` 0.7.1 (2021), `tree-sitter-markdown-fork` 0.7.3, `tree-sitter-sql-bigquery` 0.8.0.
- Repository activity (pushed, [D-ind GitHub API]): most `tree-sitter/*` grammar repos were pushed 2026-09-13 to 2026-09-29. Lagging crate releases against active repos: javascript/python/go (repo 2026-09, crate 2025), c-sharp repo 2026-09-29 vs crate 2026-04-14, Java and C++ and Ruby crates are from 2024-11/12 with repos active. `tree-sitter-julia` repo pushed 2025-11-08, crate 2024-11. A git dependency on a pinned commit is the fallback if a fix is needed [E].

### 1.3 ABI and version compatibility [M]

- The 0.27 runtime reports `LANGUAGE_VERSION=15`, `MIN_COMPATIBLE_LANGUAGE_VERSION=13`. I called `set_language` for 36 grammars with ABIs 13 (Nix), 14 (TypeScript, TSX, Java, C++, Kotlin, Ruby, R, Julia, SQL, HTML, JSON, YAML, TOML, Elixir, Zig) and 15 (the rest); none failed [M].
- All grammar crates depend only on `tree-sitter-language ^0.1` at normal-dependency level (the core appears as a dev-dependency), so they do not pin the core. Only the old crates listed above pin `tree-sitter` itself [M, from crates.io dependency lists].
- Upgrade rule of thumb [E]: move the core and `tree-sitter-tags` together; grammar crates can lag. `tree-sitter-language-pack` 1.19.0 notes the same coupling ("0.27 requires `tree-sitter-language ^0.1.8`").

### 1.4 Windows evidence

- **[M]** All 36 grammar crates plus `tree-sitter 0.27.0` and `tree-sitter-tags 0.27.0` built in one project with MSVC 14.51 in **24 s** (cargo home warm, cold target dir), producing a 58.8 MB release `.exe` (about 1.6 MB per grammar on average; the large ones are Haskell, Swift, C#, PHP, Scala, TypeScript [E]). The project's target dir was 826 MB.
- **[M]** The full first build (including downloads) failed at the Rust stage only because of my test code (`tree_sitter_nix::TAGS_QUERY` does not exist); all C parsers had compiled.
- **[D-self]** CI with Windows runners: tree-sitter core (windows-x64/x86/arm64), tree-sitter-rust, -python, -c-sharp, -haskell (`windows-latest`). No Windows runner in tree-sitter-swift's workflows (0 matches).
- **Throughput [M]**: single thread, 2 MiB synthetic file made by repeating a 4-line sample, best of 3, release build, machine under load from other sessions (a first run was 1.5-2x faster for some languages, so treat these as +/-50%): Rust 3.8, Python 3.1, JavaScript 3.0, TypeScript 4.4, Go 3.2, Java 4.1, C 3.8, C++ 3.9, C# 2.4, Ruby 2.5, Scala 2.7, Swift 2.9, Markdown 2.4, JSON 4.6, YAML 7.5, Haskell 1.3 MiB/s. (PHP, Haskell and YAML reported `has_error` on the repeated file because repeating `<?php`, a `module` header or duplicate keys is not valid; ignore their error flags.) At about 3 MiB/s per thread, 10 GB of source is about 1 hour on 1 thread or about 5 minutes on 12 threads [E]; parsing is usually not the bottleneck compared with tantivy and embedding.
- **Observed parse anomalies [M] (sample vs grammar not isolated)**: Kotlin-ng reported a `MISSING _class_member_semi` for the one-line `class A { fun m(...) = n(x) }`; PowerShell reported an `ERROR` node for `function Get-Foo { param($x) Write-Output $x }` (one-liner with `param` inside the braces). I did not test multi-line forms. Both are small samples, so do not read them as grammar defects without a larger test.

### 1.5 tags.scm findings [M]

Measured with `TagsConfiguration::new(language, query, "")` and `TagsContext::generate_tags` on 4-line samples per language (definitions D / references R):
- Working shipped queries: Rust 3D+2R, Python 3D+1R, JavaScript 4D+1R, Go 3D+5R, Java 5D+1R, C 3D, C++ 3D, Swift 4D, Ruby 4D+5R, PHP 4D, Lua 2D+1R, R 2D+2R, OCaml 6D+2R, Elixir 3D+1R, Dart 3D+1R.
- C and C++ and Swift and PHP queries return **definitions only** (no `@reference.*` in C, C++, Swift; PHP has reference patterns but none matched my sample). A name-matched reference graph for those languages needs our own `@reference` patterns [E].
- **TypeScript** query alone found 1 tag on the sample (only interface); concatenating JavaScript's `TAGS_QUERY` and TypeScript's gave 5 tags (4 definitions). TSX behaves the same way. The TS file is written to be used on top of the JS file [M].
- **C#**: the shipped `tags.scm` ends with `(namespace_declaration name: (identifier) @name) @module`. `tree-sitter-tags` 0.27 rejects it: "Invalid capture @module. Expected one of: @definition.*, @reference.*, @doc, @name, @local.(scope|definition|reference)". Deleting that last pattern made it work (5 tags). Vendor a patched copy [M].
- **Scala** (15/5) and **Kotlin-sg** (7/3) ship good `tags.scm` files but no `TAGS_QUERY` constant. Both worked when I `include_str!`-ed a vendored copy: Scala 6 tags (5D+1R), Kotlin-sg 6 tags (5D+1R) [M]. Licence MIT, so vendoring is allowed.
- **No upstream tags.scm** (checked the repos' `queries/` directories via the GitHub API [D-ind]): Haskell, Julia, Bash, PowerShell, SQL, HTML, CSS, JSON, YAML, TOML, Markdown, LaTeX, Dockerfile, Kotlin-ng. Own queries written against the printed S-expressions worked first time [M]:
  - Bash: `(function_definition name: (word) @name) @definition.function` and `(command name: (command_name (word) @name)) @reference.call` gave `D:f R:echo R:f`.
  - Haskell: `(function name: (variable) @name)`, `(data_type name: (name) @name)`, `(apply function: (variable) @name)` gave `D:f R:g D:g D:T`.
  - Julia: `module_definition`, `function_definition (signature (call_expression (identifier)))`, `struct_definition (type_head (identifier))`, `call_expression` gave 5 tags including module and struct.
  - SQL (sequel): `(create_table (object_reference name: (identifier) @name))` gave `D:t R:t`.
- For data/config formats the outline is structural, not tag-based: Markdown needs the block grammar plus the inline grammar (`tree-sitter-md` exports both); JSON/YAML/TOML key paths come from walking the tree [E].
- Fork-style alternative (D-ind): `ast-grep-language` 0.45.3 (MIT, 2026-08-31) pins this same crate family (e.g. `tree-sitter-kotlin-sg 0.4.1`, `tree-sitter-md 0.5.3`, `tree-sitter-scala 0.26`, `tree-sitter-bash 0.25`), which is independent evidence that the choices above co-exist in production.

### 1.6 Bundles

| Bundle | Version / date | Licence | What it is | Findings |
|---|---|---|---|---|
| tree-sitter-language-pack (xberg-io; repo moved from Goldziher) | 1.20.0, 2026-09-14 | MIT (grammars permissive only) | 371 languages, compiled parsers at **ABI 14** mostly, "downloaded on first use and cached"; also queries (highlights, injections, locals, tags, indents, folds) and a "code intelligence" extractor | Default features `dynamic-loading` + `download` (D-self Cargo.toml): a vault server would fetch DLLs from the network at runtime and its `build.rs` has network build-dependencies (`ureq`). Windows: its own CI comment says Windows was "exercised nowhere in CI" and the parser build "failed for v1.14.1 and v1.14.2 (issue #174)"; a `build-parsers-windows` job exists now (D-self). Per its README table, tags coverage for our languages: C, C++, C#, Go, Java, Kotlin, Lua, MATLAB, Nix, PHP, Python, R, Ruby, Rust, Scala, Swift, TS/TSX, Elixir, Dart yes; Bash, Haskell, Julia, PowerShell, SQL, HTML, CSS, JSON, YAML, TOML, Markdown, LaTeX, Dockerfile no (D-self, matches my crate-level findings). Verdict [E]: useful as a long-tail fallback (opt in, `default-features = false`, vendored DLLs), not the core. I did not build it. |
| arborium (bearcove) | 2.18.2, 2026-08-28 | MIT OR Apache-2.0 (grammar sources keep their own licences, `LICENSES.md`) | About 70 permissive grammars as feature-gated crates (`arborium-<lang>`, e.g. `arborium-haskell` ABI 15), highlighting and WASM oriented | Not built by me. Includes queries mostly for highlighting (`arborium-julia` ships `highlights.scm` only) [M]. Watch item. |
| ast-grep-language | 0.45.3, 2026-08-31 | MIT | 25 grammars wired for ast-grep | Application-oriented; pulls its own pins. Reference only. |
| inkjet | 0.11.1, 2024-09-15 | MIT/Apache-2.0 | Highlighting bundle | Repo archived (2025-09-04). Do not use. |

---------------------------------------------------------------------------

## 2. Language detection

### 2.1 Candidates [M crates.io and GitHub, unless noted]

| Crate | Version | Released | Licence | Data source | Detects by | Verdict |
|---|---|---|---|---|---|---|
| linguist (drshade) | 0.1.16 | 2026-09-08 (crate created 2025-10-31, 14 versions) | MIT | GitHub Linguist definitions (`languages.yml`, `heuristics.yml`, `vendor.yml`) synced from upstream (`definitions/UPSTREAM_COMMIT` e0c78d62...) ; Linguist upstream is at v9.7.0 (2026-08-26) | exact filename, extension, content heuristics (`fancy-regex`), vendored-file check | Best fresh data. **No shebang/interpreter detection** in the API (`detect_language_by_extension`, `detect_language_by_filename`, `disambiguate`, `is_vendored`), although its `Language` type carries an `interpreters` field. Small project (18 stars, one author) [D-ind]. |
| hyperpolyglot | 0.1.7 | 2020-07-26 (repo last push 2024-03-22) | MIT OR Apache-2.0 | Linguist data circa 2020, plus Naive Bayes classifier | filename, extension, shebang, heuristics, classifier | Most accurate in my test but its data is frozen at 2020 (languages added to Linguist since are missing) and it depends on `pcre2` (C library built by `pcre2-sys`). `hyperpolyglot_fork` 0.1.7 (2025-05-05, 849 recent downloads) exists, content not examined. |
| linguist-rs | 1.1.0 | 2023-06-04 | MIT | Linguist data | - | Stale. Skip. |
| tokei | 15.0.0 | 2026-09-06 | MIT OR Apache-2.0 | own `languages.json` | extension, filename, shebang | A line counter, not a detector: `.h` always "C Header", Perl for Prolog `.pl`, Objective-C for MATLAB `.m`, Coq for Verilog `.v` in my test. Fine for statistics only. |
| palate | 0.3.9 | 2026-03-18 | **GPL-3.0-or-later** | tft + hyperpolyglot | - | **Avoid (GPL).** |
| infer | 0.22.0 | 2026-07-15 | MIT | magic numbers | binary type | Useful only for binary-vs-text and archive sniffing. |

### 2.2 Micro-test [M] (32 tiny files, 1-5 lines each; indicative only)

Correct answers: hyperpolyglot **31/32** (wrong: `w.cs` with only `class A {}` returned Smalltalk). `linguist` crate **25/32** as I called it (ambiguous candidate lists for `m.m` MATLAB-vs-ObjC, `w.cs` C#-vs-Smalltalk, `v.v` Verilog-vs-Rocq; empty for the three shebang files `run`, `run2`, `rb`; empty for `r.R` because the extension map did not match upper-case `.R`). Adding our own shebang step would fix 3 of those 7 and case-folding the extension 1 more [E]. tokei: header files all "C Header", Prolog as Perl, MATLAB as Objective-C, Verilog as Coq.

Per-file results (hyperpolyglot / linguist crate / tokei) are in the scratch log; the interesting rows: `a.h` C++/C++ (via heuristics)/C Header; `x.pl` Perl/Perl/Perl; `y.pl` Prolog/Prolog/**Perl**; `CMakeLists.txt` CMake/CMake (filename)/CMake; `nb.ipynb` Jupyter Notebook in all three; `t.tsx` TSX in all three.

### 2.3 Recommendation [E]
Detection order: exact filename, then shebang/interpreter (own code over Linguist's `interpreters` lists), then extension, then content heuristics only when the extension is ambiguous, then (optional) a classifier. Use `linguist` 0.1.16 for the data and heuristics; if its single-author maintenance lapses, the definitions are plain YAML from github-linguist (MIT, v9.7.0) and a 150-line own loader is cheap. Also use `linguist::is_vendored` plus our own ignore rules for the exclusion step.

---------------------------------------------------------------------------

## 3. Licence detection

### 3.1 Candidates

| Crate | Version | Released | Licence | Status | Notes |
|---|---|---|---|---|---|
| spdx (EmbarkStudios) | 0.13.5 | 2026-08-06 | Apache-2.0 | Active (repo pushed 2026-09-22, 6.08M recent downloads, MSRV 1.85) | Parses/evaluates SPDX expressions. Feature `detection` is an inlined askalono (its source says so, "Askalono is not really maintained", used by cargo-deny and cargo-about). Embeds SPDX license list **3.28.0** (`spdx::identifiers::VERSION`); SPDX released **3.29.0** on 2026-09-16 [D-ind], so it is one version behind. |
| askalono / askalono-cli | 0.5.0 | 2024-10-22 | Apache-2.0 | **Archived** ("no longer maintained, forks welcome") [D-ind GitHub] | Do not depend on it. |
| scallion | 0.1.0 | 2026-09-29 (first crate 2026-07-28) | Apache-2.0 (CLI Apache-2.0 AND CC0-1.0) | New askalono fork on Codeberg (`commons-rs/scallion`), 76 downloads | Same Sorensen-Dice bigram algorithm. Watch item, too new to pin. |
| license | 3.9.0+3.29.0 | 2026-09-17 | MIT OR Apache-2.0 | Active | Embeds SPDX 3.29.0 data (ids and texts), no detection. |
| spdx-rs, spdx-expression | 0.5.5 / 0.5.2 | 2023-09 / 2022-04 | MIT | Stale | Skip. |
| license-fetcher | 0.10.1 | 2026-06-10 | **MPL-2.0** | Active | Dependency-licence embedding, not detection. File-level copyleft; flag. |

ScanCode Toolkit (reference only): Python, `aboutcode-org/scancode-toolkit`, v32.5.0 on 2026-01-15, v33.0.0rc1 on 2026-05-19 [D-ind]; code Apache-2.0, **reference datasets CC-BY-4.0** (D-self README); runs on Windows. I found no Rust port in two GitHub repository searches (not exhaustive) [E]. SPDX license-list-data v3.29.0 (2026-09-16); GitHub shows no licence for the repo (the list is published as CC0-1.0 per SPDX; I did not verify that here) [E].

### 3.2 Measured with `spdx` 0.13.5 (`detection`, `detection-inline-cache`, `detection-parallel`) [M]
`Store::load_inline()` took 91 ms and holds 680 licences. Detection per file (score, time): gitleaks LICENSE: MIT 1.000, 2.9 ms; regex LICENSE-MIT: MIT 1.000, 0.7 ms; scip LICENSE: Apache-2.0 1.000, 2.9 ms; BSD-3-Clause text: BSD-3-Clause 1.000, 0.7 ms; tokei LICENCE-MIT (different copyright line): MIT 0.991, 0.5 ms; TruffleHog LICENSE (AGPL v3 text): **AGPL-3.0-or-later** 0.985, 5.6 ms. The last one shows the known limit: GPL-family texts are identical for "-only" and "-or-later"; the distinction lives in headers, not in the licence text [E]. Whole-text matching does not find licences inside source headers; for those use `SPDX-License-Identifier:` tag extraction (regex) and `spdx::Expression::parse` to validate [E]. Package manifests (`Cargo.toml` `license`, `package.json` `license`, `pyproject.toml`) are another cheap source [E].

---------------------------------------------------------------------------

## 4. Secret scanning

### 4.1 Candidates

| Tool | Version / date | Licence | Language | Windows | Verdict |
|---|---|---|---|---|---|
| gitleaks rules (`config/gitleaks.toml`) | v8.30.1 release 2026-03-21; master pushed 2026-09-30; 222 rules; 100,940 bytes | MIT | Go (rules are TOML, regex plus keywords, entropy, allowlists) | Release assets for Windows x64/x32/arm64 (D-ind) | **Use as data.** |
| betterleaks | v1.9.0 on 2026-09-29; **v2.0.0-rc.1 on 2026-09-30** | MIT | Go; "maintained by the folks who made Gitleaks, including the original author" (D-self) | Windows x64/arm64 assets | 465 rules, `betterleaks.toml` is 619,639 bytes. Rules add `confidence`, `filter` (Expr), `validate` (HTTP, 227 rules) and `analyze` (42 rules). Strong candidate for a later switch; v2 is a prerelease. |
| Kingfisher (MongoDB) | bin 2.8.0 on 2026-09-29; library crates 1.0.0 published 2026-09-29 | Apache-2.0 (rules crate Apache-2.0 AND MIT) | Rust, Vectorscan engine (`kingfisher-vectorscan` 0.1.2) | Windows x64/arm64 zips (D-ind) | Uses the Betterleaks catalogue plus Veles detectors (D-self). Library crates are 3 days old (13-17 downloads), MSRV 1.96. Heavy: validation and revocation HTTP features we do not need. Watch item. |
| ripsecrets | 0.1.11, 2025-05-27 (repo push 2025-12-15) | MIT | Rust | no Windows release asset | Hard-coded private pattern list (about a dozen), randomness test, CLI-oriented (`find_secrets` writes to a console buffer writer). Not a fit as a library [M from source]. |
| Nosey Parker (Praetorian) | repo archived 2026-02-21 | Apache-2.0 | Rust | - | **Archived.** Skip. |
| TruffleHog | repo pushed 2026-09-30 | **AGPL-3.0** | Go | - | **Avoid (AGPL).** |
| secrets_scanner 0.2.3, secretscan 0.2.3, keyhog 0.5.86, aiguard-scanner-secrets 0.1.0 | 2026-08/09 | MIT | Rust | - | Small, young, unaudited; skip. |

### 4.2 Can the gitleaks/betterleaks rules compile with the Rust `regex` crate? [M]
Yes. With `regex` 1.13.1 and `RegexBuilder::size_limit(256 << 20)`:
- gitleaks.toml: 222 rules, 221 with a `regex`, **221 compiled, 0 failed** (1 rule is path-only). With the default 10 MiB limit, **3 failed** with "Compiled regex exceeds size limit": `generic-api-key`, `pypi-upload-token`, `vault-batch-token`. Total compile time 2.0-2.7 s.
- betterleaks.toml: 465 rules, 464 compiled, 0 failed with the raised limit (2 failed at the default: `pypi-upload-token`, `vault-batch-token`). Compile time 1.7 s.
- The files contain no look-around or backreference constructs by construction (Go RE2 syntax).

### 4.3 Engine and measurements [M]
My test engine: Aho-Corasick over all rule keywords (case-insensitive) to choose rules per file, `regex` per chosen rule, Shannon entropy gate on the secret group, global and per-rule allowlist regexes and stopwords. Corpus: first 64.4 MiB of text files (3,421 files, `.rs .py .md .toml .json .ts .js .go .java .c .h .cpp .txt .yml .yaml`) under `C:\Users\sethy\Projects\refs\rag`, which is other people's open-source code (so true secrets should be about zero), single thread.

| Rule set | Time | MiB/s | Raw matches | After entropy gate | After allowlists |
|---|---|---|---|---|---|
| gitleaks, all 221 | 40.3 s | 1.6 | 1,149 | 327 | **54** (51 from `generic-api-key`) |
| gitleaks without `generic-api-key` | 29.8 s | 2.2 | 3 | 3 | 3 (`curl-auth-user`, `hashicorp-tf-password`, `private-key`) |
| betterleaks, 464 regexes only (no Expr filters) | 2.6 s | 24.7 | 1,822 | not implemented | not implemented (`generic-api-key` 1,143, `generic-username` 558, ...) |

Slowest gitleaks rules over the corpus: `generic-api-key` 7.2 s, `cohere-api-token` 2.3 s, `lob-api-key` 1.6 s, `discord-client-secret` 1.3 s, `discord-client-id` 1.3 s. Betterleaks rewrote these (its slowest rule `generic-username` took 0.92 s). Extrapolation [E]: at 2 MiB/s per thread, 10 GB of source is about 80 minutes on 1 thread, under 10 minutes on 12 threads; with betterleaks regexes about 10x faster.

Betterleaks `filter`/`prefilter` are Expr programs. Functions used in `betterleaks.toml`: `entropy` (413 uses), `matchesAny` (155), `tokenRatio` (124, BPE based), `containsAny` (46), `len` (61) plus HTTP helpers in `validate`. A Rust re-implementation of `entropy`, `matchesAny`, `containsAny`, `len` is easy; `tokenRatio` needs a BPE vocabulary [E]. Treat `validate` and `analyze` (network calls) as out of scope for an offline vault.

### 4.4 Recommendation [E]
Take the gitleaks v8.30.1 `gitleaks.toml` as a pinned data file (record the commit), implement the small engine above, drop or restrict `generic-api-key` (it dominated false positives and runtime), and re-evaluate Betterleaks when v2.0.0 is stable. Run it once at ingest per file, not at query time. Policy on a hit (skip, redact, flag) is the lead's call (section 13).

---------------------------------------------------------------------------

## 5. Software Heritage identifiers (SWHID)

### 5.1 Standard status
- SWHID spec **v1.2** released 2025-11-07 (v1.1 2023-12-14, v1.0 2023-06-20) [D-ind GitHub releases of `swhid/specification`].
- ISO/IEC 18670: swhid.org's home page says "SWHID has been officially adopted as ISO/IEC 18670:2025 on April 23, 2025" and links `https://www.iso.org/standard/89985.html`; the spec foreword says the PAS "was used as a starting point for the creation of the ISO/IEC standard 18670" [D-self]. I could not open the iso.org page (HTTP 403) and WebSearch was exhausted, so the ISO status is **not independently verified**. The `swhid` crate README repeats "ISO/IEC 18670:2025" (D-self).
- Spec chapter 5 (D-self, I read it): content = `sha1("blob " + len + NUL + bytes)`; directory = `sha1("tree " + len + NUL + entries)`, entries sorted by name with directories sorted as if suffixed `/`, mode strings `100644`, `100755`, `120000`, `40000` (and `160000` for revision refs); revision = the git commit serialization; release = tag; snapshot = branch list. Section 5.8 says compatibility with git is "practical, but incidental and is not guaranteed". Empty directories **are** part of a SWHID directory (git cannot store them) [E from the algorithm].

### 5.2 Rust options [M crates.io]
| Crate | Version | Licence | Notes |
|---|---|---|---|
| swhid | 0.2.2, 2026-01-21 | MIT | "SWHID v1.2 reference implementation", content and directory hashing, optional git (rev/rel/snp) via `git2`; has a `permissions` module for the Windows executable-bit problem; 851 total downloads. Deps: `sha1collisiondetection`, `clap`, `hex`, `thiserror`. |
| swhid-mint | 0.1.1, 2026-07-05 | MIT | VCS-agnostic minting, 125 downloads. |
| swh-graph | 12.1.2 | **GPL-3.0-or-later** | Archive graph tooling. Avoid. |
| swh-model (Python reference) | repo pushed 2026-09-17 | **GPL-3.0** | Reference only. |
| sha1 | 0.11.0, 2026-03-27 | MIT OR Apache-2.0 | Plain SHA-1 (my test used it). |
| sha1-checked | 0.10.0, 2024-03-27 | MIT OR Apache-2.0 | SHA-1 with collision detection (git/gix use this approach). Use if untrusted input matters [E]. |

### 5.3 Measured [M]
- Test repo: `jieba-rs` shallow clone, 33 tracked files, HEAD `bac49e65b86f83882f325ef6569358225ed25bcd`, tree `ef46614a5586e7f76a9b58e09cde56d8544384e9`.
- Commit id: sha1 of `"commit <len>\0" + raw commit bytes` from gix equals HEAD. Own empty-tree id `4b825dc642cb6eb9a060e54bf8d69288fbee4904` and `"hello\n"` blob `ce013625030ba8dba906f756967f9e9ca394464a` match git and `swhid::Content`.
- **Directory id from the object database** (re-hash every blob with sha1, build the tree manifest, hash): **equals the HEAD tree**, 0 mismatches over blobs and trees, 31.5 ms.
- **Directory id from the working tree**: `0d23e7795992...` which does not equal the HEAD tree. The `swhid` crate's `DiskDirectoryBuilder` (with `.git` excluded) gave the identical `0d23e779...`, so my code and the crate agree with each other and both disagree with git. Cause: this machine has `core.autocrlf=true` (global git config, M); `git ls-files --eol` shows 32 files `i/lf w/crlf` and 1 `i/mixed w/mixed`. Naively converting CRLF to LF still did not match (`9345c147...`), because one blob really contains mixed endings. So a Windows checkout cannot reproduce committed SWHIDs from the files on disk. Hash from `.git` objects for git repos [E]; for non-git folders, hash what is on disk and say so.
- Windows has no executable bit: every file is `100644` from the file system. A git repo with `100755` scripts will differ again (this test repo had 0 executable files). `swhid` exposes a permissions source (auto, manifest, policy) for this [D-self].
- The `swhid` crate `DiskDirectoryBuilder::swhid()` over the 33-file worktree took 43-120 ms.

---------------------------------------------------------------------------

## 6. Reading .git metadata without running git (gix)

### 6.1 Versions [M]
`gix` 0.88.0 (2026-09-25, MIT OR Apache-2.0, MSRV 1.88, 11.5M recent downloads); the gitoxide family releases together (gix-hash 0.27.0, gix-object 0.65.0, gix-ref 0.68.0, gix-config 0.61.0, gix-discover 0.56.0, gix-odb 0.85.0, ...). Repo `GitoxideLabs/gitoxide` pushed 2026-10-01, 11,993 stars. Windows: its CI runs on `windows-2025` (D-self workflow). `git2` 0.21.0 (2026-05-18) is the libgit2 alternative (C dependency).

### 6.2 What I ran [M]
Feature set `default-features = false, features = ["sha1"]` (the default set is `max-performance-safe, comfort, basic, extras, auto-chain-error, sha1`; `basic` adds blob-diff, revision and index, and `extras` adds status, dirwalk, credentials, mailmap and more; `sha1` is now an explicit feature, `sha256` the other). It compiled and ran. The whole second scratch project (gix + tantivy + spdx + tokei + linguist + hyperpolyglot + scip + nbformat + others) needed 117-136 s per cold build attempt.

```rust
let repo = gix::open(path)?;                                   // 36 ms cold on this machine
let head_id = repo.head_id()?;                                 // bac49e65...
let branch  = repo.head_name()?.map(|n| n.shorten().to_string());   // Some("main")
if let Some(Ok(remote)) = repo.find_default_remote(gix::remote::Direction::Fetch) {
    let url = remote.url(gix::remote::Direction::Fetch).map(|u| u.to_bstring().to_string());
    // Some("https://github.com/messense/jieba-rs"), remote name "origin"
}
let n_refs = repo.references()?.all()?.count();                // 3 (packed-refs present)
let commit = repo.head_commit()?;                              // author, time, tree_id(), message
```
Observed: HEAD id, branch `main`, default remote `origin` and its URL, 3 references with `.git/packed-refs` present, commit author and time all returned correctly. `gix` 0.88 uses a new error type (`gix_error::Exn<Message>`) that does not implement `std::error::Error`; with `Box<dyn Error>` call sites I had to map errors through `format!("{e:?}")`. Tree walking: `repo.find_tree(id)?.iter()` yields entries with `.mode()`, `.filename()`, `.oid()`; `repo.find_object(oid)?.data` gives blob bytes.

### 6.3 Gaps to decide [E]
Deliveries may be plain folders without `.git`, bare tarballs, shallow clones (as here), worktrees or submodules; `gix::open` only works on a repository, `gix::discover` walks upward. Folders that are not git repos need a fallback (no commit, branch or remote). I did not test worktrees, submodules, a non-git folder or a repository with a very large pack.

---------------------------------------------------------------------------

## 7. CITATION.cff and codemeta.json

### 7.1 Schemas
- **CITATION.cff**: latest released version is **1.2.0** (2021-08-09 on GitHub; changelog entry 2021-05-31), `schema.json` `$id` `https://citation-file-format.github.io/1.2.0/schema.json`, `cff-version` pattern `^1\.2\.0$`; the repo's main branch has an `[Unreleased]` section with no new format version (pushed 2026-09-29) [D-ind]. Fields used by us: `cff-version`, `message`, `title`, `authors`, `version`, `date-released`, `doi`, `identifiers` (including `type: swh`), `license`, `repository-code`, `abstract`, `keywords`, `preferred-citation`, `references`.
- **codemeta.json**: CodeMeta **4.0 released 2026-09-29** (two days ago), 3.1 on 2025-10-13, 3.0 on 2023-07-13, 2.0 on 2020-09-28 [D-ind]. 4.0 is breaking: namespace moves from `https://codemeta.github.io/terms/` to `https://w3id.org/codemeta/terms/`, schema.org is referenced over `https://`, `relatedSoftware` removed, new terms (funding, `alternateName`, `creditText`, ...). The real 4.0 file I parsed has `"@context": "https://w3id.org/codemeta/4.0"`. Files in the wild will mostly be 2.0 (`https://doi.org/10.5063/schema/codemeta-2.0`), 3.0 or 3.1 [E]. Treat the file as plain JSON and read properties by local name; no JSON-LD processor needed [E].

### 7.2 Rust crates [M]
| Crate | Version | Date | Verdict |
|---|---|---|---|
| serde-saphyr | 1.3.0 | 2026-09-16 | MIT OR Apache-2.0, MSRV 1.89, 8.46M downloads (6.3M recent), created 2025-09-27: the de-facto serde YAML successor. **Use.** |
| serde_yaml | 0.9.34+deprecated | 2024-03-25 | Archived by its author. Avoid. |
| serde_yml | 0.0.13 | 2026-05-27 | Archived, `[DEPRECATED]`, "RUSTSEC-2025-0068 structurally fixed in 0.0.13. Migrate to a maintained alternative". Avoid. |
| serde_yaml_ng | 0.10.0 | 2024-05-26 | MIT, repo pushed 2025-09-14. Acceptable but older. |
| serde_norway | 0.9.42 | 2024-12-21 | Acceptable. |
| serde_yaml_bw | 2.5.8 | 2026-09-07 | Same author as serde-saphyr; alternative. |
| saphyr / yaml-rust2 | 0.1.0 / 0.13.0 | 2026-09-19 / 2026-09-11 | Lower-level YAML parsers. |
| cff | 0.5.0 | 2019-01-02 | Abandoned. |
| citeworks-cff | 0.1.1 | 2022-08-31 | Typed CFF serde structs, stale. |
| codemeta | - | - | No crate exists [M]. |
| jsonschema | 0.58.3 | 2026-09-30 | If you want to validate against `schema.json`; I recommend lenient parsing instead, because real-world CFF files are often invalid [E]. |

Measured: `serde_saphyr::from_str` into a struct with `#[serde(rename = "cff-version")]` parsed the real file of the Citation File Format repo (4,077 bytes): version 1.2.0, title "Citation File Format", doi `10.5281/zenodo.5171937`, license CC-BY-4.0, 9 authors, date-released 2021-08-09. `serde_json::Value` parsed the real CodeMeta 4.0 `codemeta.json` (7,927 bytes) with `@type` SoftwareSourceCode, version 4.0, license `https://spdx.org/licenses/Apache-2.0`, 2 authors [M].

---------------------------------------------------------------------------

## 8. Jupyter notebooks (nbformat)

- Reference format: `jupyter/nbformat` v5.11.1 (2026-08-17); its schema `nbformat.v4.schema.json` has `nbformat: 4` and `nbformat_minor >= 5` as current [D-ind GitHub].
- **`nbformat` crate 3.0.0** (runtimed, BSD-3-Clause, 2026-04-26, created 2024-10-27, 26 versions, 86,951 recent downloads; Windows CI workflow exists in `runtimed/runtimed` [D-self]). Dependencies: `jupyter-protocol` 2.0.x, `chrono`, `uuid`, `anyhow`, `serde`, `serde_json`, `thiserror`. API: `parse_notebook(&str) -> Result<Notebook>` where `Notebook` is a non-exhaustive enum (`V4`, `Legacy`, `V3`), `upgrade_legacy_notebook`, `upgrade_v3_notebook`, `serialize_notebook`. Ships schemas 4.0 to 4.5.
- **Measured [M]**: `parse_notebook` on the official `jupyter/nbformat` test file `tests/test4.ipynb` (17,454 bytes, nbformat 4.0 style) returned **`Legacy`**, not `V4`; after `upgrade_legacy_notebook` it yielded 4 code cells, 5 markdown cells, 0 raw. So code must handle all three variants, or always call the upgrade path.
- Others: `ipynb` 0.0.1 (2021, abandoned), `notebookx` 0.1.8 (2026-01, 16 downloads), `nb-cli` 0.0.10 (2026-07), `callisto-core` 0.1.0 (2026-07, nbformat 4.5 IO, brand new). None is a safer choice than the `nbformat` crate.
- Recommendation [E]: because the vault needs only text (cell sources, markdown, optionally text outputs, kernel language) and a stable line map, a 40-line own serde reader (`cells[].cell_type`, `source` as string or array of strings, `outputs[].text` / `data["text/plain"]`, `metadata.language_info.name`) avoids `chrono`, `uuid`, `jupyter-protocol` and the Legacy/V4 split. Large base64 image outputs should be dropped from the canonical text; whether to keep the original `.ipynb` verbatim beside a derived text view is a design question (section 13).

---------------------------------------------------------------------------

## 9. SCIP

- Protocol repo `scip-code/scip` (moved from sourcegraph; Apache-2.0; pushed 2026-09-29; release v0.10.0 on 2026-09-03). The `scip` Rust crate 0.10.0 (2026-09-03, Apache-2.0, MSRV 1.81, 659,267 recent downloads) depends on `protobuf =3.7.2` (exact pin, generated code). protobuf 3.7.2 is 2025-03-10 (MIT) [M, D-self Cargo.toml].
- **Measured [M]**: with `scip = "0.10"` and `protobuf = "=3.7.2"` I built an `Index` with one `Document` and one `Occurrence`, serialized it (65 bytes) and parsed it back. This proves the crate builds on Windows MSVC and the API round-trips; I had no real `.scip` file to read.
- **Indexers** (D-ind release asset lists and READMEs; I ran none):

| Indexer | Languages | Repo (owner) | Last release | Windows evidence |
|---|---|---|---|---|
| scip-typescript | TS, JS | sourcegraph | v0.4.0, 2025-10-02 | npm package; no Windows statement; no asset |
| scip-java | Java, Scala, Kotlin | scip-code | v0.13.1, 2026-07-02 | launcher script; no Windows statement |
| rust-analyzer `scip` command | Rust | rust-lang | rolling 2026-09-28 | official Windows msvc zips exist |
| scip-clang | C, C++ | sourcegraph | v0.4.0, 2026-02-23 | **Linux and macOS binaries only**; needs `compile_commands.json` |
| scip-ruby | Ruby | sourcegraph | v0.5.0, 2026-09-25 | **Linux and macOS only** (gems and binaries) |
| scip-python | Python | sourcegraph | no GitHub release found (distributed via npm) | CI matrix includes `windows-latest` |
| scip-dotnet | C#, VB | sourcegraph | v0.2.14, 2026-05-05 | CI matrix includes `windows-latest` |
| scip-go | Go | scip-code | v0.2.7, 2026-05-25 | assets for darwin and linux only |
| scip-dart | Dart | scip-code (was Workiva) | 1.7.0, 2026-09-24 | none stated |
| scip-php | PHP | davidrjenni | no releases | none stated |

  The `scip` CLI v0.10.0 also ships darwin and linux archives only. The stack-graphs project is archived (section 1.1). SCIP indexers require dependency installation and often a successful build, which a delivered source folder usually will not allow; the companion report `research-code-retrieval.md` reaches the same conclusion [I did not re-check its numbers]. Languages without Windows-capable indexers: C/C++ and Ruby (documented), others unverified.

---------------------------------------------------------------------------

## 10. Identifier splitting for tantivy

No crate found. `heck` 0.5.0 (2024-03-12, MIT OR Apache-2.0) and `convert_case` 0.12.0 (2026-08-27, MIT) split and re-case words but are not tantivy tokenizers; `tantivy-jieba` and `tantivy-stemmers` are for other purposes; crates.io search for "camelcase tokenizer", "tantivy tokenizer code", "split identifier" returned nothing relevant [M]. Quickwit's index-config documentation lists only `raw`, `raw_lowercase`, `default`, `en_stem`, `whitespace`, `chinese_compatible`, `lowercase` (no code tokenizer) [D-self, fetched today].

My own implementation compiled against tantivy 0.26.2 (`default-features = false`) and `tantivy-tokenizer-api` 0.7.0 and ran [M]. Core idea: find identifier runs (alphanumeric or `_`), emit the whole identifier when it has more than one part (same position as the first part, `position_length` = number of parts), then each sub-word at consecutive positions; chain `LowerCaser`.

```rust
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};
#[derive(Clone, Copy, PartialEq)] enum C { Lower, Upper, Digit, Sep }
fn class(c: char) -> C {
    if c.is_lowercase() { C::Lower } else if c.is_uppercase() { C::Upper }
    else if c.is_numeric() { C::Digit } else if c.is_alphabetic() { C::Lower } else { C::Sep }
}
pub fn split_parts(s: &str) -> Vec<(usize, usize)> {           // byte ranges of sub-words
    let cs: Vec<(usize, char)> = s.char_indices().collect();
    let (mut out, mut start) = (vec![], None::<usize>);
    for i in 0..cs.len() {
        let (b, c) = cs[i]; let k = class(c);
        if k == C::Sep { if let Some(st) = start.take() { out.push((st, b)); } continue; }
        match start {
            None => start = Some(b),
            Some(st) => {
                let pk = class(cs[i - 1].1);
                let nk = cs.get(i + 1).map(|x| class(x.1));
                let boundary = (pk == C::Lower && k == C::Upper)                    // fooBar
                    || (pk == C::Upper && k == C::Upper && nk == Some(C::Lower))     // HTTPResponse
                    || ((pk == C::Digit) != (k == C::Digit));                        // HTML5Parser
                if boundary { out.push((st, b)); start = Some(b); }
            }
        }
    }
    if let Some(st) = start { out.push((st, s.len())); }
    out
}
// IdentTokenizer implements Tokenizer { type TokenStream<'a> = IdentStream; ... } building a Vec<Token>
// (offset_from, offset_to, position, text, position_length) and a TokenStream over it (about 30 more lines).
```
Output with `TextAnalyzer::builder(IdentTokenizer).filter(LowerCaser)` [M] (token@position):
- `getHTTPResponseCode` -> `gethttpresponsecode@0 get@0 http@1 response@2 code@3`
- `parse_json_value2` -> `parse_json_value2@0 parse@0 json@1 value@2 2@3`
- `XMLHttpRequest` -> `xmlhttprequest@0 xml@0 http@1 request@2`
- `HTML5Parser` -> `html5parser@0 html@0 5@1 parser@2`
- `MAX_BUFFER_SIZE` -> `max_buffer_size@0 max@0 buffer@1 size@2`
- `__init__` -> `init@0` (single part: only the stripped word; if the dunder form should be searchable add a whole-identifier token for any run containing `_`, a one-line change [E])
- Greek `θαλασσαΚύμα` -> `θαλασσακύμα@0 θαλασσα@0 κύμα@1` (Unicode-aware).
The trigram index covers substring and regex queries; this tokenizer adds BM25 recall for sub-word queries such as "http response" [E].

---------------------------------------------------------------------------

## 11. Recommended Cargo pin block

Everything marked "tested" was compiled and run by me on this machine; other lines are version-checked against crates.io only. Keep `tree-sitter`, `tree-sitter-tags` and the grammar crates behind a feature per language to bound binary size (58.8 MB for 36 grammars) [E].

```toml
# ---- tree-sitter core (tested) -------------------------------------------------
tree-sitter       = "=0.27.0"          # ABI 15, min 13. links = "tree-sitter": exactly one core per build.
tree-sitter-tags  = "=0.27.0"
# tree-sitter-language 0.1.8 arrives transitively.

# ---- grammars (all tested: compiled with MSVC and set_language() succeeded) ------
tree-sitter-rust       = "=0.24.2"
tree-sitter-python     = "=0.25.0"
tree-sitter-javascript = "=0.25.0"
tree-sitter-typescript = "=0.23.2"     # TS + TSX; prepend JS TAGS_QUERY
tree-sitter-go         = "=0.25.0"
tree-sitter-java       = "=0.23.5"
tree-sitter-c          = "=0.24.2"
tree-sitter-cpp        = "=0.23.4"
tree-sitter-c-sharp    = "=0.23.5"     # vendor tags.scm minus the final @module pattern
tree-sitter-kotlin-ng  = "=1.1.0"      # no queries; or tree-sitter-kotlin-sg = "=0.4.1" (has tags.scm, vendor it)
tree-sitter-swift      = "=0.7.3"
tree-sitter-ruby       = "=0.23.1"
tree-sitter-php        = "=0.24.2"
tree-sitter-scala      = "=0.26.2"     # vendor queries/tags.scm
tree-sitter-haskell    = "=0.24.1"     # own tags query
tree-sitter-lua        = "=0.5.0"
tree-sitter-r          = "=1.3.0"
tree-sitter-julia      = "=0.23.1"     # own tags query
tree-sitter-matlab     = "=1.3.1"      # also the only Octave coverage
tree-sitter-bash       = "=0.25.1"     # own tags query
tree-sitter-powershell = "=0.26.4"     # one-liner function sample reported ERROR; test on real scripts
tree-sitter-sequel     = "=0.3.11"     # SQL; own tags query
tree-sitter-html       = "=0.23.2"
tree-sitter-css        = "=0.25.0"
tree-sitter-json       = "=0.24.8"
tree-sitter-yaml       = "=0.7.2"
tree-sitter-toml-ng    = "=0.7.0"
tree-sitter-md         = "=0.5.3"      # block + inline grammars
codebook-tree-sitter-latex = "=0.6.1"  # latex-lsp grammar, third-party publish
tree-sitter-containerfile  = "=0.9.2"  # Dockerfile / Containerfile
# optional extras (tested): tree-sitter-ocaml 0.26.0, -elixir 0.3.5, -dart 0.2.0, -zig 1.1.2, -nix 0.3.0

# ---- ingestion helpers -----------------------------------------------------------
linguist   = "=0.1.16"                                   # tested; add own shebang step
spdx       = { version = "=0.13.5", features = ["detection", "detection-inline-cache", "detection-parallel"] }  # tested
regex      = "1.13"                                      # tested 1.13.1; use RegexBuilder::size_limit for gitleaks rules
aho-corasick = "1.1"                                     # tested 1.1.5
toml       = "1"                                         # tested 1.1.6; reads gitleaks.toml
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
serde-saphyr = "=1.3.0"                                  # tested; CITATION.cff
gix        = { version = "=0.88.0", default-features = false, features = ["sha1"] }   # tested
sha1       = "0.11"                                      # tested for blob/tree/commit hashing; sha1-checked 0.10 if hostile input matters
swhid      = "=0.2.2"                                    # dev-dependency: conformance oracle for our own ~40 lines
scip       = "=0.10.0"                                   # tested round trip; optional feature
protobuf   = "=3.7.2"                                    # must equal scip's pin
nbformat   = "3"                                         # tested; or own reader (section 8)
tantivy    = { version = "=0.26.2", default-features = false }   # tested tokenizer API; share the pin with the search crate
ignore     = "0.4.33"                                    # exclusion rules (not tested here)
```

Not recommended as dependencies: `tree-sitter-language-pack` (runtime downloads, ABI 14, Windows history), `hyperpolyglot` (stale, pcre2), `askalono` (archived), `ripsecrets` (not a library), `tree-sitter-graph` / `tree-sitter-stack-graphs`, `serde_yaml`, `serde_yml`.

---------------------------------------------------------------------------

## 12. Flags: abandoned, copyleft, broken on Windows

**Abandoned, archived or stale** (do not adopt):
- askalono 0.5.0 (repo archived); tree-sitter-stack-graphs (archived 2025-09-09); tree-sitter-graph (last release 2024-12-11, pinned to core 0.24); inkjet (archived); Inflector (archived); serde_yaml (archived 2024-03); serde_yml (archived, RUSTSEC-2025-0068); Nosey Parker (archived 2026-02-21); hyperpolyglot 0.1.7 (crate 2020, repo 2024-03); linguist-rs (2023); cff 0.5.0 (2019); ipynb 0.0.1 (2021); tree-sitter-sql 0.0.2 (2021); tree-sitter-markdown 0.7.1 (2021); tree-sitter-toml 0.20.0 (2022); tree-sitter-dockerfile 0.2.0 (2024-05); tree-sitter-kotlin 0.3.8 (2024-08); tree-sitter-vue 0.0.3 and tree-sitter-svelte 0.10.2 (2022).
- Watch for staleness: tree-sitter-typescript, -java, -cpp, -ruby, -html, -json, -julia crates last released 2024-11/12 while their repos are active (crate lags repo).

**GPL / AGPL / MPL** (licence flags, all verified on crates.io or GitHub [M]):
- GPL-3.0-or-later: `palate` 0.3.9, `swh-graph` 12.1.2. GPL-3.0: `SoftwareHeritage/swh-model` (Python repo). AGPL-3.0: TruffleHog. MPL-2.0: `license-fetcher` 0.10.1, `rust-code-analysis` 0.0.25 (2023). CC-BY-4.0 data (attribution): ScanCode reference datasets, citation-file-format repository content. All other crates in section 11 are MIT, Apache-2.0, BSD-3 or dual licences. `tree-sitter-elixir` is Apache-2.0 (not MIT).
- `scip-python` and `scip-dart` show GitHub licence "NOASSERTION" (custom or mixed; not read) [M]; `tokei` shows NOASSERTION on GitHub but its crate metadata says MIT OR Apache-2.0 [M].

**Broken or fragile on Windows (this machine)**:
- **Long build paths**: with `CARGO_HOME` under the 250-plus-character session scratch path, `zstd-sys 2.1.0+zstd.1.5.7` (pulled by `spdx`'s `detection-cache` feature) and `pcre2-sys 0.2.10` (pulled by `hyperpolyglot`) failed with `fatal error C1083: Cannot open include file '../compress/zstd_compress_internal.h'` and an sljit compile error; with `CARGO_HOME=C:\Users\sethy\tsc\h` both built [M]. Cause assumed to be the 260-character path limit (matches the machine rule "keep generated paths short") [E]. Keep Cargo home, target dir and checkouts on short paths.
- `tree-sitter-language-pack`: Windows parser build failed for 1.14.1 and 1.14.2 (D-self).
- Line endings: `core.autocrlf=true` breaks SWHID-from-working-tree (section 5.3).
- No Windows release binaries: `scip` CLI, `scip-go`, `scip-clang`, `scip-ruby`, `ripsecrets` (D-ind asset lists).

---------------------------------------------------------------------------

## 13. Ambiguities and questions for the lead (recorded, not resolved)

1. **Canonical text for source files**: the design says one Markdown document per file stored verbatim with real line numbers. Whether notebooks, minified JS, generated files, lockfiles and binaries get a derived text view, a stub, or exclusion is open. Notebook line numbers refer to the derived text, not the `.ipynb`.
2. **Secret policy on a hit**: skip the file, redact the span, index with a flag, or quarantine? Scanning runs at ingest; the verbatim-storage rule conflicts with redaction. Which rule set to pin (gitleaks v8.30.1 now, Betterleaks v2 later) and whether `generic-api-key` is allowed at all.
3. **SWHID source of truth**: git object database for repos with `.git` (needed on this autocrlf machine), working tree for plain folders. Whether to store both `swh:1:cnt` per file and `swh:1:dir`/`swh:1:rev` per repository, and how to treat empty directories and executable bits on Windows.
4. **Licence output**: SPDX expression per file (header tags), per repository (LICENSE files plus manifests), or both; handling of "-only" vs "-or-later" ambiguity.
5. **Grammar set and binary size**: which languages are compiled in by default (about 1.6 MB each) and whether the long tail uses an opt-in language-pack or arborium path. Whether to vendor patched `tags.scm` files (C#, Scala, Kotlin-sg, TS+JS) and own queries (Bash, Haskell, Julia, SQL) in the repo; they are MIT and small.
6. **SCIP**: ingest `.scip` files when the owner supplies them, or ignore SCIP in phase 0.
7. **Kotlin grammar choice** (ng with no queries vs the ast-grep fork with tags.scm) and **LaTeX crate provenance** (codebook republish vs git dependency on latex-lsp).
8. Not tested by me and worth a check before pinning: `tree-sitter-language-pack` and `arborium` builds; `gix` on worktrees, submodules and non-git folders; `kingfisher-scanner` library once it ages; `ignore` crate exclusion rules on Windows; real `.scip` files; Octave `.m` files with `tree-sitter-matlab`; PowerShell multi-line functions.

---------------------------------------------------------------------------

## 14. Method, scratch artefacts and clean-up

- Registry data: crates.io API (`/api/v1/crates/<name>`, versions, dependencies) and `.crate` tarballs from `static.crates.io` for about 150 crates; ABI from each tarball's `src/parser.c`; tags.scm presence and capture counts from each tarball; constant names from `bindings/rust/lib.rs`.
- Repo data: `gh api` for repository metadata, releases, workflow files, READMEs, rule files (gitleaks.toml, betterleaks.toml), the SWHID specification chapters; WebFetch for swhid.org and Quickwit docs.
- Scratch projects: `ts-check` (tree-sitter 0.27, tags 0.27 and 36 grammar crates; parse, tags and throughput test) and `misc-check` (gix, sha1, swhid, regex, aho-corasick, toml, serde-saphyr, scip, nbformat, spdx, hyperpolyglot, linguist, tokei, tantivy; sections tokenizer, git, rules, scip, nb, meta, spdx, lang). Cold build times: ts-check 24 s; misc-check 117-136 s per attempt. Binary sizes: 58.8 MB and 15.1 MB.
- Test inputs fetched with `gh api` raw: `jupyter/nbformat` `tests/test4.ipynb`, `CITATION.cff` from the citation-file-format repo, `codemeta.json` from the codemeta repo, licence texts from gitleaks, scip, regex, tokei, trufflehog and SPDX `BSD-3-Clause.txt`.
- Secrets: I did not print any matched secret value; the secret scan reports only rule ids, file names and lengths.
- Clean-up: scratch Cargo homes and target directories (`C:\Users\sethy\tsc`, and `target-p1`, `target-p2`, `cargo-home` under the session scratchpad) were removed at the end; the two scratch projects' source files (`p1`, `p2`) and the fetched sample inputs remain in the session scratchpad only; no background process was left running; nothing was written to the vault except this file.
