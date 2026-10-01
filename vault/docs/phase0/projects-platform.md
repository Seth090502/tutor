# Phase 0: Rust platform crates, status and pin recommendations

Prepared 2026-09-30 by a Sonnet 5.5 worker for the lead engineer. Scope: the Rust crates named in the libraryd design (XML, chunking, tokenizers, MCP, gRPC, HTTP, cache, SQLite, hashing, compression, near-duplicate detection, language detection, benchmarking, load generation). Nothing was compiled, installed or run: every claim comes from registry and GitHub API calls made today, from source files read at the stated tag or branch, or is labelled as an estimate. Recommendations are recommendations; the lead decides.

## 0. How to read this report

Evidence labels used on numbers and claims:

- **[M]** measured today: crates.io API, GitHub API, a source file or manifest I read, or a local file check.
- **[D]** documented by the project (README, CHANGELOG, doc comment, release notes, issue text). The project's own claim, not independently verified.
- **[E]** my estimate or inference. Not tested.

Conventions: "release date" is the crates.io `created_at` of that version [M]. "Last commit" is the newest commit on the default branch from the GitHub API [M]. Where that commit is dependabot-only I say so, because it overstates activity. The GitHub rate limit was hit once mid-session; the one affected query (Windows issue search for BLAKE3, tonic) was retried after the reset and succeeded.

### Machine facts that matter for Windows builds [M]

- A Rust toolchain exists at `C:\Users\sethy\.rustup\toolchains\stable-x86_64-pc-windows-msvc` (manifest dated 2026-09-03, rustc 1.98.1 (48a229cea 2026-09-01); file `lib\rustlib\multirust-channel-manifest.toml`). It is the MSVC host, which is the toolchain every Windows issue below says works. All crate MSRVs in this report (highest: text-splitter 1.90, rmcp 1.88, tonic 1.88, oha 1.88) are below 1.98.
- In the PowerShell tool session, `Get-Command` found none of `rustc`, `cargo`, `cl`, `link`, `cmake`, `nasm`, `clang`, `protoc` on PATH (python is at `C:\Python314\python.exe`). The only thing seen in `C:\Users\sethy\.cargo\bin` was `rustup.exe`. I did not look for a Visual Studio install (glob timed out). Consequences: `protoc` is absent (see tonic/prost), and anything that needs CMake/NASM/clang (aws-lc-sys, see reqwest) would fail unless those are installed or the feature is avoided.

## 1. Summary table (all from registries on 2026-09-30)

| Crate | Latest (release date) | Last commit | Licence (file in repo) | Status | Pin recommendation |
|---|---|---|---|---|---|
| quick-xml | 0.42.0 (2026-08-22) | 2026-09-02 | MIT (`LICENSE-MIT.md`) | Active; breaking 0.x churn (0.39.4 on 05-08 to 0.42.0 on 08-22) | `=0.42.0` only if used; see section 2 |
| roxmltree | 0.21.1 (2025-10-12; 0.21.0 yanked) | 2026-09-22 | MIT OR Apache-2.0 (`LICENSE-APACHE` + MIT) | Active; unreleased fix for stack overflow on deep nesting | `0.21.1` (primary XML parser, see section 2) |
| text-splitter | 0.33.0 (2026-09-24) | 2026-09-29 (dependabot) | MIT (`LICENSE.txt`) | Active, one maintainer, ~monthly breaking minors | `=0.33.0`, features `markdown`, `tokenizers` |
| tokenizers | 0.23.2 (2026-09-03, "last v0"); 1.0.0-rc.2 (2026-09-21) | 2026-09-23 | Apache-2.0 (`LICENSE`) | v0 frozen; v1 rewrite is a release candidate with open regressions | `=0.23.2`, `default-features = false, features = ["onig"]` |
| rmcp | 3.5.0 (2026-09-28) | 2026-09-28 | Apache-2.0 on crates.io; repo `LICENSE` is Apache-2.0 plus a preamble saying some contributions remain MIT | Very active (about weekly minors) | `=3.5.0` (or `~3.5`) |
| tonic | 0.14.6 (2026-05-07) | 2026-09-30 (repo is now grpc/grpc-rust) | MIT (`LICENSE`) | Released line stable; master preparing breaking changes | `=0.14.6` (and the other tonic-* crates at 0.14.6) |
| prost | 0.14.4 (2026-06-07; 0.14.0 and 0.14.2 yanked) | 2026-07-05 | Apache-2.0 (`LICENSE`) | Maintenance only (bug and security fixes) | `0.14.4` |
| quinn | 0.11.12 (2026-09-14) | 2026-09-30 | MIT OR Apache-2.0 | Active (status only) | not needed |
| axum | 0.8.9 (2026-04-14) | 2026-09-30 | MIT (`LICENSE`) | Active; main branch has unreleased breaking changes | `0.8.9` |
| reqwest | 0.13.5 (2026-09-08) | 2026-09-30 | MIT OR Apache-2.0 | Active | `0.13.5`, `default-features = false` |
| tokio | 1.53.1 (2026-07-20) | 2026-09-30 | MIT | Active | `>=1.53.1` (1.53.0 broke MSRV on Windows) |
| rayon | 1.12.0 (2026-04-14) | 2026-08-28 | MIT OR Apache-2.0 | Active, slow cadence | `1.12` |
| moka | 0.12.16 (2026-08-09) | 2026-08-09 | (MIT OR Apache-2.0) AND Apache-2.0 | Active; open crash reports | `>=0.12.16`, see risk note |
| rusqlite | 0.40.2 (2026-08-08) | 2026-09-25 | MIT (`LICENSE`) | Active; breaking 0.x every 2-3 months | `0.40.2`, feature `bundled` |
| blake3 | 1.8.7 (2026-08-20) | 2026-09-10 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception (`LICENSE_A2`) | Active | `1.8.7`, features `rayon`, `mmap` |
| zstd | 0.14.0 (2026-09-04) | 2026-09-10 | BSD-3-Clause (`LICENSE`) | Active; 0.14 is a new breaking line | `0.14.0` |
| gaoya | 0.2.2 (2026-06-15) | 2026-05-29 | MIT (`LICENSE`) | Revived after a 3-year release gap; thin | `=0.2.2` (recommended MinHash crate, with caveats) |
| probminhash | 0.1.12 (2025-06-11) | 2026-01-22 | MIT OR Apache-2.0 on crates.io; no licence file found in repo (licence endpoint 404) | Single maintainer, niche | not recommended for this use |
| lsh-rs | 0.4.0 (2020-05-09) | 2023-06-20 | MIT | Abandoned | no |
| whichlang | 0.1.1 (2025-01-21) | 2026-03-30 (dependabot) | MIT (`LICENSE`) | Stable, feature-frozen | `0.1.1` (recommended language detector) |
| lingua | 1.8.0 (2026-03-09) | 2026-09-03 (dependabot) | Apache-2.0 (`LICENSE`) | Active | optional second opinion |
| whatlang | 0.18.0 (2025-10-16) | 2025-12-24 | MIT (`LICENSE`) | Low activity | no |
| criterion | 0.8.2 (2026-02-04) | 2026-09-08 (dependabot) | Apache-2.0 OR MIT | Active (community-maintained org) | `0.8.2` (dev-dependency) |
| oha | 1.16.0 (2026-08-23) | 2026-09-10 | MIT (`LICENSE`) | Active | recommended HTTP load tool (prebuilt Windows exe) |
| rlt | 0.5.0 (2026-01-28) | not queried | MIT OR Apache-2.0 | Maintained (not deeply checked) | recommended base for a custom gRPC/MCP load tool |
| goose | 0.18.1 (2025-08-14) | 2026-03-10 | Apache-2.0 | Slow | no |

Supporting versions seen in manifests [M]: hyper 1.11.1 (2026-08-28), tower 0.5.3, tower-http 0.7.1, hyper-util 0.1.21, tokio-util 0.7.19, schemars 1.2.2, tantivy 0.26.2 (2026-09-08).

## 2. Top flags for the lead

1. **tokenizers is mid-rewrite.** 0.23.2 is described by its own release notes as "the last v0 release" and v1.0.0-rc.2 is a release candidate with open regressions on real tokenizer files (section 4). text-splitter 0.33.0 depends on `tokenizers ^0.23`, so it cannot use 1.0 anyway. Pin 0.23.2. Qwen3's `tokenizer.json` has `"truncation": null, "padding": null` in its first bytes [M, 400-byte range read of the Hugging Face files for Qwen3-Embedding-0.6B and Qwen3-Reranker-0.6B], so the truncation caveat in text-splitter 0.31 does not apply to it.
2. **text-splitter cannot guarantee that code fences, tables and display equations stay whole.** It keeps a block together only if the block fits the chunk capacity. An oversized block is split at lower levels, and there is no "atomic block" option (section 3). Two open upstream issues confirm this (#798 code fence, #422 tables). If the design needs atomic blocks, they have to be carved out before the splitter runs.
3. **rmcp has no server-side bearer-token verifier.** Auth for the HTTP server is an axum/tower middleware in front of `StreamableHttpService`, as in the official example. The crate's `auth` feature is the OAuth 2.1 client. By default the server accepts only loopback `Host` headers, so Stage 2 (AI PC on the LAN) needs `with_allowed_hosts(...)` or every request is rejected [M, source `crates/rmcp/src/transport/streamable_http_server/tower.rs`].
4. **rmcp 3.x speaks MCP 2026-07-28 by default** (stateless, no `initialize` handshake) and still serves 2025-11-25 and earlier. I did not verify which protocol version Claude Code negotiates; that needs a real test.
5. **tonic/prost are in a transition.** tonic's repo is now `grpc/grpc-rust` and its master branch says it is "preparing breaking changes" while hosting a new official `grpc` crate (0.9.0, 2026-05-28). prost's README says the maintainer is not taking new features and expects Google's `protobuf` crate to take over. Staying on tonic 0.14.6 + prost 0.14.4 is safe today; plan for a migration later (section 5).
6. **moka has two open crash reports** (#605, a heap-corruption crash with a Windows `ntdll` stack trace on 0.12.15; #600, intermittent SIGSEGV that the maintainer says the original TSan report for was a false positive). See section 7. If the cache is on a correctness-critical path, consider `quick_cache` 0.7.0 as an alternative.
7. **roxmltree rejects any document with a DOCTYPE by default** (`ParsingOptions::allow_dtd = false`, error `DtdDetected`) [M, `src/parse.rs`]. JATS and TEI files frequently carry a DOCTYPE, so you must set `allow_dtd: true`. Also set `nodes_limit` (default is `u32::MAX`). A stack overflow on deeply nested elements is fixed on master but unreleased [D, CHANGELOG "Unreleased"].
8. **reqwest 0.13 default TLS pulls aws-lc-rs**, which is painful on Windows when CMake/NASM/clang are absent (issue #2937, commenters say it is still unresolved). The sidecars (llama.cpp, GROBID, MinerU) are plain HTTP on localhost or LAN, so use `default-features = false` and enable no TLS.
9. **zstd 0.14 vs tonic's `zstd` feature (0.13).** Both resolve to one `zstd-sys` only if zstd-safe 7.3.0 picks zstd-sys 2.1.0 (section 5). Check with `cargo tree -d` before relying on tonic zstd compression.
10. **None of this was built.** Phase 1 should run a first `cargo build` and `cargo tree -d` on the MSVC toolchain to confirm the feature sets below, especially `onig` (tokenizers), `bundled` SQLite, and `blake3`.

## 3. XML: quick-xml and roxmltree (for JATS and TEI)

### quick-xml
- Version 0.42.0, 2026-08-22, MSRV 1.86, MIT [M]. Repo tafia/quick-xml, last commit 2026-09-02 ("Fix changelog"), 91 open issues, 1,569 stars [M].
- Churn: 0.39.4 (05-08), 0.40.0 (05-11), 0.40.1 (05-15), 0.41.0 (06-29), 0.42.0 (08-22) [M]. The 0.42 changelog calls itself "a large release": names, events and `BytesStart` etc. now use `&str`/`String` instead of bytes, the reader now rejects non-UTF-8 input unless wrapped in `DecodingReader` [D, `Changelog.md`]. Expect further breaking changes; pin exactly.
- Features [M]: default none; `encoding` (encoding_rs), `serialize`/`serde-types`, `async-tokio`, `escape-html`, `overlapped-lists`. Pull-style reader/writer with namespace resolution; the roxmltree README notes that streaming parsers "usually do not provide a proper string unescaping, DTD resolving and namespaces support" [D].
- Windows: no Windows-titled issues found [M].
- Use: right choice only if streaming is needed (very large book XML) or if you want serde-derived structs for fixed schemas. JATS is deeply mixed-content, so serde derive is a poor fit [E].

### roxmltree
- Version 0.21.1, 2025-10-12, MIT OR Apache-2.0, MSRV 1.60, only dependency `memchr` [M]. 0.21.0 was yanked [M]. Repo RazrFalcon/roxmltree, last commit 2026-09-22 ("Parse element content iteratively"), 8 open issues [M]. The 0.21.1 release is 11 months old but the repo is clearly alive.
- Read-only DOM tree, whole document in memory, no unsafe, namespace resolution, entity references, `positions` feature on by default (gives byte offsets and line/column, useful for the `{doc_id}#L{start}-L{end}` addressing) [D, README feature table; M, feature list].
- 0.21.0 added `ParsingOptions::entity_resolver` for external entities via public ID and URI [D, CHANGELOG]. Internal entity reference depth is capped at 10 [M, `parse.rs`].
- Defaults to be aware of [M, `parse.rs`]: `allow_dtd: false` (so a DOCTYPE is an error), `nodes_limit: u32::MAX`.
- Stack-overflow risk: master's CHANGELOG "Unreleased" says "Stack overflow when parsing deeply nested elements. Element content is now parsed iteratively" [D]. Not in 0.21.1. For untrusted XML, either cap input size and nesting before parsing, parse on a thread with a large stack [E], or depend on a git commit (not recommended).
- Windows: no Windows-titled issues [M].
- Recommendation (lead decides): roxmltree as the primary parser for JATS and TEI (documents are individually small; mixed content and namespaces are easier on a DOM), with `allow_dtd: true` and a `nodes_limit`. Keep quick-xml in reserve for streaming very large inputs. Memory for a DOM is typically several times file size [E].

## 4. text-splitter and tokenizers

### text-splitter 0.33.0
- Released 2026-09-24, MIT, MSRV 1.90.0 (raised in 0.33.0), edition 2021 [M]. Cadence: 0.30.0 and 0.30.1 on 04-17, 0.31.0 on 06-04, 0.32.0 on 06-16, 0.33.0 on 09-24 [M]. Last commit 2026-09-29 is dependabot (`bump taiki-e/install-action`); real activity is the 09-24 release. 9 open issues, 633 stars, single maintainer [M].
- Features [M, `Cargo.toml`]: `markdown = ["dep:pulldown-cmark"]` (pulldown-cmark 0.13), `tokenizers = ["dep:tokenizers", "tokenizers/onig"]` (tokenizers `^0.23`, `default-features = false`), `tiktoken-rs` (0.12), `code` (tree-sitter 0.27). Always-on dependencies include `icu_segmenter ^2.3`, `icu_provider ^2.3`, `itertools ^0.15`, `ahash`, `memchr`.
- Chunk sizing by a Hugging Face tokenizer [D, README; M, `src/chunk_size/huggingface.rs`]: `TextSplitter::new(ChunkConfig::new(max_tokens).with_sizer(tokenizer))`, also `MarkdownSplitter`, and the capacity can be a range (`500..2000`). The sizer calls `tokenizer.encode_fast(chunk, false)` (no special tokens) and counts ids, skipping pad tokens, plus overflow encodings. Since 0.31 the README tells you to call `tokenizer.with_truncation(None)` first, because tokenizers 0.23 may return the truncated size; Qwen3's file has truncation null already [M].
- Offsets for `{doc_id}#L{start}-L{end}`: `chunk_indices` and `chunk_char_indices` return byte and char offsets per chunk (`ChunkCharIndex { chunk, byte_offset, char_offset }`) [M, doc comment in `src/splitter/markdown.rs`]. Converting byte offset to line numbers is a separate step in our code.
- How the markdown splitter works [M, `src/splitter/markdown.rs`]: it parses with `pulldown_cmark::Options::all()` (so tables, GFM, footnotes, math, definition lists, metadata blocks are all recognised). Semantic levels from low to high: characters, graphemes, words, sentences, soft line breaks, inline elements (text, emphasis, links, table cells, inline code, inline math), block elements, thematic breaks, headings by level. Code blocks, HTML blocks, table rows, tables, lists, block quotes and **display math** (`Event::DisplayMath`) are `Element::Block`. The README method: pick the highest level whose first item fits, merge neighbours "of this level or above", so higher-level boundaries are never crossed [D].
- **Code fences, tables, display equations.** A block that fits within the capacity is not cut internally, because it is a single unit at the Block level. A block larger than the capacity is split at the next lower level. There is no option to make a block atomic: the crate has no such feature or builder method in the README or manifest [M, absence of evidence, not a proof]. Concretely:
  - Open issue #798 "Insufficient markdown code block fence chunking" (2025-09, still open): when a code block is bigger than the capacity, the opening fence lands in its own chunk. The reporter used a Qwen tokenizer with the Python binding [M].
  - Open issue #422 "Smarter Table Splitting with Header Preservation": an oversized table is split by rows without repeating the header [M].
  - Display math has no children in the event stream, so an oversized `$$...$$` would be split at sentence/word/grapheme boundaries inside the equation text [E, from the parse code; not tested].
  - `Options::all()` turns on `$...$` math parsing, which can mis-read prices such as "$5 and $10" as inline math. That only shifts chunk boundaries, not the text [E].
  - Workaround if atomic blocks are required (design choice for the lead): walk the document with `pulldown-cmark` offsets yourself, emit code/table/display-math blocks as their own chunks (allowing them to exceed the token budget, or splitting tables by row with header repetition), and give only the prose runs between them to `MarkdownSplitter`.
- Other open items [M]: #1137 "Min chunk size" (small crumb chunks are possible even with a range capacity), #726 chunk metadata, #116 return preceding headers. #1178 (fixed 2026-06-16) and the 0.32.0 notes cover performance for oversized sections.
- Chinese text: sentence and word boundaries come from `icu_segmenter` (UAX #29) [D, README]. Whether the word segmenter uses a dictionary for Chinese depends on which ICU4X constructor text-splitter calls, which I did not determine. Test with a Chinese paragraph before trusting boundaries [E]. A search of its issues for "chinese OR cjk OR japanese" returned only dependabot noise [M].
- Windows: no Windows-titled issues [M]. The `tokenizers` feature compiles Oniguruma (C) via the `onig` crate, which is fine on MSVC (next section).
- Pin: `text-splitter = { version = "=0.33.0", features = ["markdown", "tokenizers"] }`.

### tokenizers 0.23.2 vs 1.0.0-rc.2
- Versions [M]: 0.23.2 (2026-09-03), 1.0.0-rc.2 (2026-09-21). Release notes of 0.23.2: "This is the last v0 release, we are moving to v1" [D]. Last commit 2026-09-23, 209 open issues, 11,144 stars, Apache-2.0 [M].
- Loading Qwen3: `Tokenizer::from_file("tokenizer.json")` [D, README of text-splitter tests use it; E for Qwen3 specifically, not run]. The Qwen3 file starts `{"version":"1.0","truncation":null,"padding":null,"added_tokens":[{"id":151643,"content":"<|endoftext|>",...},{"id":151644,"content":"<|im_start|>",...` [M]. text-splitter's own open issue #798 shows a Qwen tokenizer being used with the splitter, and the Qwen2-style byte-level BPE is a mainstream case for tokenizers 0.x [E].
- Features of 0.23.2 [M]: default = `progressbar`, `onig`, `esaxx_fast` (`esaxx-rs/cpp`); optional `http` (hf-hub), `rustls-tls`, `unstable_wasm` (fancy-regex), `parity-aware-bpe`. The `esaxx_fast` feature compiles C++ and is only needed for Unigram training, not inference [E]. text-splitter's `tokenizers` feature turns defaults off and enables only `onig` [M], which is the configuration I recommend: `tokenizers = { version = "=0.23.2", default-features = false, features = ["onig"] }`. Add `http` only if you ever call `from_pretrained`.
- Windows [M, issue text]:
  - #1581 (open, last comment 2026-07-30): `onig_sys` fails with "two or more data types in declaration specifiers" on the **GNU/MinGW** target; a commenter says it does not happen on MSVC. This machine's toolchain is MSVC [M].
  - #1914 (closed 2025-12-16): `esaxx-rs` forces the static CRT (/MT) and breaks linking with `ort`/`tch`. `esaxx-rs` last released 2023-10-05 and its fix PRs are unmerged per that issue [M, crates.io + issue text]. Avoiding `esaxx_fast` sidesteps it [E].
- v1 risk [M, GitHub issues]: #2447 (open, 2026-09-28) `Tokenizer::from_file` fails in 1.0.0-rc.2 for ModernBERT and bloomz files ("Byte atom 0xC0 not found in the vocabulary") that load in 0.23.x; #2460 (open) rc.2 rejects ByteLevel `add_prefix_space=true`, which 0.23.2 supports; #2458 (open) panic in Python bindings under threads; #2094 (open) a crafted `tokenizer.json` aborts the process during load. `REQUIRED_FOR_V1.md` lists authoring features deliberately dropped from the rc and still to return [D]. Conclusion: stay on 0.23.2; loading only our own known Qwen files avoids the #2094 class of problem.

## 5. rmcp (official Rust MCP SDK)

- Version 3.5.0 (2026-09-28), with rmcp-macros 3.5.0; MSRV 1.88, edition 2024; repo modelcontextprotocol/rust-sdk, last commit 2026-09-28, 52 open issues, 3,964 stars [M]. 3.0.0 shipped 2026-07-28; later releases 3.1.4 (08-20), 3.2.0 (08-31), 3.3.0 (09-10), 3.4.0 (09-15), 3.4.1 (09-23), 3.5.0 (09-28) [M, GitHub releases]. About one release per week: pin exactly and read the migration guide (discussion #969, "Upgrading to RMCP 3.x") before bumping [D].
- Licence: crates.io says Apache-2.0. The repo `LICENSE` begins with a notice that the MCP project is moving from MIT to Apache-2.0, that new work is Apache-2.0, and that contributions whose authors have not consented stay MIT [M]. GitHub's detector reports NOASSERTION because of that preamble. Both licences are permissive.
- Protocol version [M, `crates/rmcp/src/model.rs`]: `ProtocolVersion::LATEST = V_2026_07_28`; `LATEST_WITH_INITIALIZE = V_2025_11_25`; known versions 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25, 2026-07-28. From 2026-07-28 the `initialize` handshake is gone (SEP-2567, lifecycle moves into per-request `_meta`). The README says the SDK implements 2026-07-28 and stays "fully compatible with the 2025-11-25 release and earlier" [D]. Server results carry `resultType` for 2026-07-28 peers only [D, migration guide].
- Streamable HTTP server transport [M, README and `tower.rs`]:
  - Cargo feature `transport-streamable-http-server` (pulls `server-side-http`, `tower-service`, `http`, `http-body-util`, `sse-stream`, `rand`, `uuid`). Minimal server feature set: `server`, `macros`, `transport-streamable-http-server` [M, feature table].
  - `StreamableHttpService::new(|| Ok(handler), LocalSessionManager::default().into(), StreamableHttpServerConfig::default())` is a Tower service, mounted with `axum::Router::new().nest_service("/mcp", service)`.
  - Config fields [M]: `sse_keep_alive` (default 15 s), `sse_retry` (3 s), `legacy_session_mode` (default true; `with_legacy_session_mode(false)` makes legacy versions stateless too), `json_response`, `cancellation_token`, `allowed_hosts` (default `["localhost","127.0.0.1","::1"]`, "to prevent DNS rebinding"), `allowed_origins` (default empty, so Origin validation is off; `enforce_origin_validation()` turns it on), `session_store` (optional, for multi-instance recovery), `max_request_body_bytes` (413 on excess), `stateless_protocol_metadata_required`.
  - For the 2026-07-28 protocol the server is stateless automatically: no `Mcp-Session-Id`, no GET/DELETE stream, no resumption [D]. The service factory runs per request, so shared state (DB pools, caches) must live in a cloneable handle captured by the closure [D].
- Auth/bearer hooks [M, `examples/servers/src/simple_auth_streamhttp.rs`, `docs/OAUTH_SUPPORT.md`]: no verifier inside the crate. The example installs an axum `middleware::from_fn_with_state` that reads `Authorization: Bearer ...` and returns 401, applied to the route that nests the MCP service. The inbound `http::request::Parts` are inserted into the request extensions, so tool handlers can read them (for example the verified identity) via `Extension<Parts>` [M, `tower.rs` doc comment]. The `auth` Cargo feature is the OAuth 2.1 client (PKCE, RFC 9728/8414/7591, token refresh) and pulls `oauth2` and `reqwest` [M]. Do not enable it on the server.
- Tool schemas [D, README]: `#[tool]`, `#[tool_router]`, `#[tool_handler]`, `Parameters<T>` where `T: schemars::JsonSchema`; input and output schemas are generated as JSON Schema 2020-12 from field names, types and doc comments (schemars `^1.0`; the type name and docs on `T` itself are ignored). As of 2026-07-28 (SEP-2106) `outputSchema` may be any JSON Schema type.
- Examples [M, directory listing]: `examples/servers/src/` has `counter_streamhttp.rs`, `counter_hyper_streamable_http.rs`, `simple_auth_streamhttp.rs`, `complex_auth_streamhttp.rs`, `cimd_auth_streamhttp.rs`, `structured_output.rs`, `subscriptions_streamhttp.rs`, `progress_demo.rs`, `task_stdio.rs`, `mrtr.rs`, plus stdio ones; also `examples/clients`, `transport`, `wasi`. The server examples use axum 0.8, reqwest 0.13.2, hyper 1.
- Dependency compatibility [M, crates.io]: rmcp 3.5.0 needs tokio 1, tokio-util 0.7, optional hyper ^1, http ^1, reqwest ^0.13.2, schemars ^1.0, rand ^0.10, base64 ^0.23. It depends on no axum version itself, only on tower-service ^0.3, so it composes with axum 0.8.9.
- Windows: no Windows-specific open bug found in the top search hits [M, search is not exhaustive]. Open items relevant to us: #1301/#1304 (stdio transport runs reads and writes on tokio's blocking pool and caps throughput; a non-blocking `stdio_pipes` PR is open) matter only if Claude Code launches libraryd over stdio; #1256 (session and GET stream at 2026-07-28) and #1308 (cache hints default for 2026-07-28 peers) show the new protocol path is still being polished [M].
- Pin: `rmcp = { version = "=3.5.0", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }`.

## 6. tonic, prost, quinn

### tonic 0.14.6
- Released 2026-05-07 (145 days before today); MIT; MSRV 1.88 [M]. The GitHub repo is now `grpc/grpc-rust` (API `full_name`; `hyperium/tonic` redirects) with 12,486 stars and 356 open issues, last commit 2026-09-30 ("grpc/service_config: ...") [M].
- The README says: "tonic's master branch is currently preparing breaking changes. For the most recently released code, look to the 0.14.x branch" [D]. The workspace now also contains the new `grpc` 0.9.0 and `grpc-protobuf` 0.9.0 crates (released 2026-05-28, MIT, "The official Rust implementation of gRPC") plus xDS crates [M]. I found no 0.15 plan in issues (search returned nothing) [M].
- Features [M, manifest]: default `router`, `transport`, `codegen`; `gzip`, `deflate` (flate2), `zstd` (zstd `^0.13.0`); `server`, `channel`; TLS via `tls-ring`, `tls-aws-lc`, `tls-native-roots`, `tls-webpki-roots`. **Compression** is enabled per service with `CompressionEncoding` (see `examples/src/compression`) [M, example file]. **Streaming**: server, client and bidirectional streaming are listed features [D], with `examples/src/streaming`.
- Version compatibility in one binary [M, crates.io dependency lists]:
  - tonic 0.14.6: prost via `tonic-prost` (`prost ^0.14`), `axum ^0.8` (optional, enabled by `router`), `hyper ^1`, `hyper-util ^0.1.11`, `h2 ^0.4`, `tower ^0.5`, `http ^1.1`, `tokio-rustls ^0.26.1`.
  - axum 0.8.9: `hyper ^1.1` (optional), `tower ^0.5.2`. Note axum's default features include `http1` but not `http2`; gRPC needs HTTP/2, so enable `axum`'s `http2` feature if axum serves the gRPC listener [M, axum feature list; E for the consequence].
  - reqwest 0.13.5: `hyper ^1.1`, `tower ^0.5.2`. rmcp: `hyper ^1` optional.
  - So tonic 0.14.6 + prost 0.14.4 + axum 0.8.9 + reqwest 0.13.5 + rmcp 3.5.0 share one hyper 1 / http 1 / tower 0.5 line [E, inferred from the requirement ranges].
  - tonic 0.14 split code generation: `tonic-build` has no prost dependency; use `tonic-prost-build` (build-dependency) and `tonic-prost` (runtime) [M, manifests].
  - One gRPC and HTTP port: `tonic::service::Routes` (and `into_axum_router`, from the 0.12.2 notes) lets tonic services be merged into an axum `Router`; the repo has `examples/src/multiplex` and `examples/src/h2c` [D/M]. Separate listeners are simpler [E]; the lead decides.
- `zstd` conflict check [M, crates.io]: tonic's `zstd` feature uses zstd 0.13 (zstd-safe 7.x); our zstd 0.14.0 uses zstd-safe 8.0.0 (`zstd-sys ^2.1.0`). zstd-safe 7.3.0, published the same day as zstd 0.14.0 (2026-09-04), requires `zstd-sys ^2.0.15`, which allows 2.1.0. So a lockfile can hold a single `zstd-sys` (a native-library link name can only appear once) [E]. Verify with `cargo tree -d -p zstd-sys`.
- `protoc`: prost-build requires `protoc` (README: "protoc will be required ... Prost will no longer provide bundled protoc") [D], and none is on PATH here [M]. Options for the lead: install `protoc`, or use `protox` 0.9.1 (pure-Rust protobuf compiler, MIT OR Apache-2.0, released 2025-12-02) with tonic-prost-build's file-descriptor entry point [E, API not checked].
- Windows [M, issue search on grpc/grpc-rust]: all 8 Windows-titled hits are closed, most recently #254 "Windows UDS Support" (2026-09-01) and #2621 plugin resolution for `protoc-gen-rust-grpc` on Windows (2026-05-28). No open Windows build issue seen.
- Pin: `tonic = "=0.14.6"`, `tonic-prost = "=0.14.6"`, `tonic-prost-build = "=0.14.6"`, optionally `tonic-health`, `tonic-reflection` at 0.14.6.

### prost 0.14.4
- Released 2026-06-07; Apache-2.0; MSRV 1.85; last commit 2026-07-05; 242 open issues [M]. **0.14.0 and 0.14.2 are yanked** [M]; 0.14.1, 0.14.3 and 0.14.4 are not.
- README "Contributing": "The current maintainer is not contributing new features and doesn't have the time to review new features. Bug fixes and small improvements are welcome ... security fixes will be released as soon as possible. The maintainer expects the official `protobuf` project to release their rust library soon" [D]. Status: maintenance mode, not abandoned.
- Windows: two closed old issues only (#626, #630 about cmake/protoc) [M].
- Replacement path to watch (not a recommendation to move now): Google's `protobuf` crate (4.36.2-release, BSD-3-Clause, 2026-09-17) plus the official `grpc` + `grpc-protobuf` crates at 0.9.0 (pre-1.0) [M].
- Pin: `prost = "0.14.4"`, `prost-types = "0.14.4"`.

### quinn (status only)
- 0.11.12 (2026-09-14); quinn-proto 0.11.19 was released today, 2026-09-30; MIT OR Apache-2.0; MSRV 1.85; 5,277 stars, 184 open issues; last commit 2026-09-30 [M]. Defaults: `runtime-tokio`, `rustls-ring`, `platform-verifier`, `log`, `bloom`.
- Windows issues [M]: #2682 open (panic binding a non-wildcard IPv6 address on Windows 11, `os error 10022`), #2041 open (possible GRO bug on Windows); #2818 closed 2026-09-03 (GSO probe leaves `UDP_SEND_MSG_SIZE` set on the caller's socket).
- Not part of the gRPC/axum/rmcp stack; it appears only as an optional HTTP/3 dependency of reqwest and oha. No action.

## 7. axum, reqwest, tokio, rayon, moka (latest versions and notes)

- **axum 0.8.9**, 2026-04-14, MIT, MSRV 1.80, last commit 2026-09-30 [M]. The CHANGELOG "Unreleased" section lists several `breaking:` items (router fallback merging, `axum::serve` changes, `from_request` rejection types) [M], so a 0.9 is coming; tonic 0.14 and the rmcp examples are on 0.8. Windows: one closed issue from 2024 [M]. Pin `0.8.9`; features needed beyond default: `http2` (gRPC), `macros` optional.
- **reqwest 0.13.5**, 2026-09-08, MIT OR Apache-2.0, MSRV 1.85, 472 open issues [M]. Default features: `default-tls` (= `rustls`, which enables `__rustls-aws-lc-rs` and `rustls-platform-verifier`), `charset`, `http2`, `system-proxy` [M]. Windows [M, issue text]: #2937 `aws-lc-sys` requires CMake/NASM/clang, closed but commenters in Feb-Apr 2026 say it is unresolved, workaround `rustls-no-provider` or `native-tls`; #2873 open (system proxy override for local addresses ignored on Windows). Recommendation: `reqwest = { version = "0.13.5", default-features = false, features = ["json"] }`, no TLS, for plain-HTTP sidecar calls; add `http2` only if the sidecars speak it. If TLS is later needed, choose the provider deliberately.
- **tokio 1.53.1**, 2026-07-20, MIT, MSRV 1.71 [M]. 1.53.0 (07-17) used `OnceLock::wait` in the Windows signal handler and broke the declared MSRV (issue #8299); 1.53.1 restores it ("signal: restore MSRV by removing OnceLock::wait from the Windows handler") [M, CHANGELOG]. Issue #8342 (runtime freeze after Windows sleep/resume) was closed 2026-08-09 by a maintainer as "an application bug" [M]. Patch releases 1.51.4 and 1.52.4 also appeared on 2026-07-16 [M]. Pin `>=1.53.1`.
- **rayon 1.12.0**, 2026-04-14, MIT OR Apache-2.0, MSRV 1.80, last commit 2026-08-28; 1.12.0 fixed unsound `Range<char>` iteration at 0xE000 [M, RELEASES.md]. Windows: one open feature request (process groups), the rest closed and old [M]. Pin `1.12`.
- **moka 0.12.16**, 2026-08-09, MSRV 1.71.1 [M]. Licence field on crates.io is `(MIT OR Apache-2.0) AND Apache-2.0`; why the extra `AND Apache-2.0` term is there was not investigated, but all terms are permissive [M]. Features: default empty; `future` (async cache), `logging`, `quanta`, `atomic64` [M]. 0.12.16 fixes a bug where, with the non-default LRU policy, eviction could stall permanently, and with the default TinyLFU a milder phantom-entry leak (#592) [D, CHANGELOG]: use at least 0.12.16. **Risk:** two open reports. #605 (opened 2026-08-10, no replies) reports a rare crash in `unlink_node_ao_from_deque` on 0.12.15 with a Windows `ntdll!RtlFreeHeap` heap-corruption stack. #600 (2026-07-23) reports an intermittent SIGSEGV under concurrent eviction; the maintainer's analysis on 2026-08-02 says the original ThreadSanitizer report is a known TSan false positive, but the issue stays open [M]. moka uses `unsafe` internally and `crossbeam-epoch`. Alternatives to consider if a cache bug would corrupt answers: `quick_cache` 0.7.0 (MIT, 2026-06-27, about 7.2M recent downloads) [M]. `mini-moka` is stale (last release 2024-01-08) [M].

## 8. rusqlite, blake3, zstd

### rusqlite 0.40.2 (+ libsqlite3-sys 0.38.2)
- Released 2026-08-08, MIT; last commit 2026-09-25; 4,420 stars; 171 open issues [M]. Breaking minors: 0.38.0 (2025-12-20), 0.39.0 (2026-03-15), 0.40.0 (2026-05-26) [M]; pin the minor.
- **Bundled SQLite version: 3.53.2** (`SQLITE_VERSION_NUMBER 3053002`, source id dated 2026-06-03), read from `libsqlite3-sys/sqlite3/sqlite3.h` at tag `v0.40.2` [M].
- Feature `bundled` compiles the amalgamation with `cc`; `bundled-windows` bundles only when targeting Windows [M, features]. The build script (`libsqlite3-sys/build.rs`, master) passes `-DSQLITE_ENABLE_FTS3`, `FTS3_PARENTHESIS`, **`FTS5`**, `JSON1`, `RTREE`, `STAT4`, `LOAD_EXTENSION=1`, `COLUMN_METADATA`, `DBSTAT_VTAB`, `API_ARMOR`, `-DSQLITE_THREADSAFE=1` [M; read at master, not at the tag]. **WAL** is a runtime setting (`PRAGMA journal_mode=WAL`), available in any SQLite build; the crate needs no feature for it [E]. Optional crate features include `functions`, `backup`, `blob`, `serialize`, `limits`, `hooks`, `vtab`, `window`, `modern_sqlite` [M]. `LIBSQLITE3_FLAGS` can add or remove defines [D, README].
- Windows [M, issue text]: #1861 (closed 2026-06-26) "Linker errors on Windows 11". Root cause found in the thread: an environment variable `CC=gcc` (from a MinGW install) made `cc` build the bundled SQLite with GCC while linking with MSVC. Check `CC` and `CXX` env vars on this machine before the first build [E]. #1331 closed (always recompiled on Windows). #966 open concerns `bundled-sqlcipher` on Windows only, which we do not use.
- Pin: `rusqlite = { version = "0.40.2", features = ["bundled"] }`.

### blake3 1.8.7
- Released 2026-08-20; licence `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception` (repo file `LICENSE_A2`); last commit 2026-09-10; 6,454 stars [M].
- Features [M]: default `std`; `rayon` (multithreaded hashing), `mmap` (memory-mapped file hashing via memmap2), `zeroize`, `digest`/`traits-preview`, `pure` (no C/asm), `no_avx512` and similar kill-switches. README: "The `rayon` feature provides multithreading" [D]. The build script uses `cc` (build dependency) for the SIMD implementations [M]. Using `update_mmap_rayon` for large files requires both `mmap` and `rayon` [E, from memory of the API; not read here].
- Windows [M, issue search]: open #578 (Cygwin build gets SysV assembly for a Windows-ABI target and segfaults), #456 and #435 (Windows 7), #509 (compiler warning); #517 closed (VirusTotal false positive on the 1.8.2 Windows b3sum binary). None affects an MSVC x86_64 build as far as the titles show.
- Pin: `blake3 = { version = "1.8.7", features = ["rayon", "mmap"] }`.

### zstd 0.14.0 (+ zstd-safe 8.0.0, zstd-sys 2.1.0+zstd.1.5.7)
- Released 2026-09-04 after 0.13.3 (2025-02-20): a new breaking line; BSD-3-Clause (GitHub licence detection of `LICENSE`); last commit 2026-09-10; 657 stars, 73 open issues [M]. Bundled C library is zstd 1.5.7 (from the `zstd-sys` version string) [M].
- Features [M, README table and manifest]: default `legacy`, `arrays`, `zdict_builder` (dictionary training); optional `zstdmt` (multithreaded compression inside the C library), `pkg-config`, `vendored`, `bindgen`, `no_asm`, `thin`, `fat-lto`, `experimental`. The README's performance section points at `zstdmt` for compression throughput [D].
- Windows [M, issue titles]: open #244 (cross compile to MSVC via lld, 2026-09-04), #306 and #304 (gnu target / 0.13.2 linking error, 2024), #151 (wasm on Windows). No open MSVC-native build failure seen.
- Ecosystem lag: tonic's `zstd` feature is still on zstd 0.13 (see section 6).
- Pin: `zstd = "0.14.0"`.

## 9. MinHash / LSH near-duplicate detection

Candidates examined (crates.io and GitHub, all [M] unless marked):

| Crate | Version / date | Licence | Provides | Maintenance | Verdict |
|---|---|---|---|---|---|
| gaoya | 0.2.2 / 2026-06-15 | MIT | 64/32/16/8-bit MinHash, 64/128-bit SimHash, `MinHashIndex` banded LSH (bands, band width, Jaccard threshold), rayon, Python bindings | 83 stars; 171,671 downloads, 63,575 recent; versions 0.1.0 (2022-02) to 0.2.0 (2023-06-23), then 0.2.2 on 2026-06-15; last commit 2026-05-29; 6 open issues | best fit, with caveats |
| probminhash | 0.1.12 / 2025-06-11 | MIT OR Apache-2.0 (no licence file in the repo) | ProbMinHash2/3/3a, SuperMinHash, SetSketch, OPH densification: sketching only, no LSH index | 31 stars, last commit 2026-01-22; heavy deps (argmin, anyhow, env_logger, etc.) | primitives only |
| lsh-rs | 0.4.0 / 2020-05-09 | MIT | vector LSH for ANN | last commit 2023-06-20 | abandoned |
| lsh-rs2 | 0.4.1 / 2025-06-05 | MIT | one-commit fork of lsh-rs | 0 stars | no |
| datasketch-minhash-lsh | 0.0.1 / 2021-09-21 | n/a | port of Python datasketch's MinHash LSH | abandoned | no |
| minhash-rs | 0.6.0 / 2026-07-06 | MIT | const-generic MinHash sketches, sparse variants, union | 5,225 downloads; sketches only, banded LSH is left to the user | primitives only |
| txtfp | 0.3.2 / 2026-09-06 | MIT | NFKC canonicalisation, MinHash + banded LSH index, SimHash, TLSH, optional jieba CJK tokenizer (`cjk` feature), optional ONNX embeddings | 2,673 downloads (2,534 recent), created recently; README still shows `txtfp = "0.2"` | young; interesting CJK handling |
| incrededup | 0.3.1 / 2026-07-05 | Apache-2.0 | disk-backed MinHash LSH (redb sidecars), CLI not a library | 47 downloads, 0 stars; README says it was written by agents over about six months and "may" contain bugs; claims production use on ~50M records and ~0.9 TB of sidecar indexes [D] | design reference only |
| simhash | 0.3.0 / 2026-03-09 | MIT | 64-bit SimHash | 1.6M downloads | SimHash only |
| datasketches | 0.5.0 / 2026-09-04 | Apache-2.0 | Apache DataSketches (HLL etc.; MinHash not seen in README) | active | not applicable [E] |

gaoya details [M]: dependencies `ahash`, `crossbeam-utils`, `fnv`, `itertools 0.14`, `num-traits`, `rand ^0.10.1`, `rand_pcg`, `random_choice`, `rayon ^1.12.0`, `seahash`, `sha-1`, `shingles ^0.1.1`, `siphasher`, `smallvec ^2.0.0-alpha.12` (a **pre-release** dependency), `triomphe`; edition 2024; no `serde`, so the index is in-memory only and would be rebuilt from stored signatures [E]. Its CHANGELOG.md was last updated 2023-06-29 [M], inconsistent with the 2026 crates.io release, so release notes are not a reliable guide. Open issues are feature requests only (faster hash, clustering example) [M]. The README Rust example builds a `MinHashIndex::new(num_bands, band_width, jaccard_threshold)` over `MinHasher32` signatures and queries it with `query_owned` [D].

**Recommendation: gaoya 0.2.2, pinned exactly, kept behind a small internal trait** so it can be swapped or replaced by a ~200-line in-house implementation (MinHash signature plus banding over blake3 or xxh3 shingle hashes) if the pre-release `smallvec` dependency or the thin maintenance becomes a problem. Reasons: it is the only candidate that gives signatures, a banded LSH index, multithreading and SimHash in one MIT crate with a 2026 release and a meaningful download count; probminhash, minhash-rs and simhash give primitives only; lsh-rs and datasketch-minhash-lsh are abandoned; txtfp and incrededup are too new to depend on. Caveats the lead should weigh: nothing was built or benchmarked in Phase 0; the index is in memory, so at 6M chunks a 128-hash 32-bit signature set is about 3 GB and 64-bit about 6 GB [E, 6M x 128 x 4 or 8 bytes], which is heavy on the 32 GB AI PC. Document-level dedup (about 160k documents, about 80 to 165 MB of signatures) is comfortable [E]. Chunk-level dedup can use exact hashes first [E]. txtfp's jieba-based CJK tokenizer is worth a look when Chinese shingling quality is tested [E].

## 10. Language detection

Candidates [M unless marked; accuracy and speed figures are the projects' own, labelled D]:

| | whichlang 0.1.1 | lingua 1.8.0 | whatlang 0.18.0 |
|---|---|---|---|
| Release / last commit | 2025-01-21 / 2026-03-30 (dependabot) | 2026-03-09 / 2026-09-03 (dependabot) | 2025-10-16 / 2025-12-24 (README) |
| Licence | MIT | Apache-2.0 | MIT |
| Languages | 16: Arabic, Dutch, English, French, German, Hindi, Italian, Japanese, Korean, Mandarin, Portuguese, Russian, Spanish, Swedish, Turkish, Vietnamese [D] | 75, Chinese and Japanese separate; Cargo feature per language [M] | 70, with script detection [D] |
| Dependencies | none [M] | per-language model crates, about 300 MB total when all are fetched [D]; "only a few dozen megabytes of memory during runtime" because models are FSTs [D] | `hashbrown` only [M] |
| Method | multiclass logistic regression over hashed 2-4-grams and codepoint features [D] | rule engine plus Naive Bayes over 1-5-grams, no neural nets, no dictionaries [D] | trigrams [D] |
| Speed (author benchmarks, 2000 sentences, 16 common languages, iMac i9) | 4.47 ms single thread; 105 MiB/s short strings [D] | 640 ms (high accuracy, 16 languages); 3.12 s (high accuracy, all 75 languages) [D] | 36 ms (16 languages); 89 ms (all languages) [D] |
| Accuracy | average 97.03% over its 16 languages; Mandarin 98.65% overall and 98.69% on texts of at most 20 characters; English 91.84% at <=20 characters [D, whichlang README] | the lingua README argues it is the most accurate, including on single words and short phrases, with confidence values and language-subset filtering [D]; I did not find a per-language short-text table in its README that I could quote | average 91.69% on the same 16 languages; Mandarin 96.09%; English only 57.82% at <=20 characters [D, whichlang README's benchmark] |
| Output | `detect_language(&str) -> Lang`: always returns one of 16, no confidence, no unknown [M, `src/lib.rs`; open issues #24 and #16 ask for exactly that] | confidence values per language; can restrict to a set; low-accuracy mode; lazy or preloaded models [D] | language, script, confidence, reliability flag [D] |

Notes: none of the three distinguishes Simplified from Traditional Chinese [E]. Lingua's open issues (24) are mostly requests for more languages [M]. Whichlang's open issues are requests for confidence and `Option` return [M].

**Recommendation: whichlang 0.1.1** as the bulk detector. Reasons: zero dependencies and a few hundred KB of weights [E]; the speed leaves per-section and per-chunk detection essentially free (about 5 microseconds on long strings per its README, so about 30 s for 6M chunks [E]); it covers the languages this corpus will mostly contain (English, Chinese, plus German, French, Spanish, Portuguese, Russian, Japanese, Korean); no Windows build risk (no C code, no models to fetch). Limitations to design around: no confidence and no "unknown" (a Polish abstract will come back as some other language), only 16 languages, and the accuracy figures are the authors' own. Suggested guard rails (the lead's design call): detect per section rather than per document (Chinese papers often carry English abstracts), run on at least some minimum length, and keep lingua restricted to a short language list (via its subset builder) as an optional second opinion for short or low-agreement text. Validate on a small labelled sample of our own before trusting any of the published numbers.

## 11. criterion

- criterion 0.8.2, 2026-02-04, Apache-2.0 OR MIT, MSRV 1.86 [M]. The crates.io repository field points at `criterion-rs/criterion.rs` (created 2025-10-22; last commit 2026-09-08, dependabot; 31 open issues). The original `bheisler/criterion.rs` is not archived and was last pushed 2026-04-23 [M]. So it is community-maintained and active, with a slow cadence.
- Features [M]: default `rayon`, `plotters`, `cargo_bench_support`; `html_reports` is an empty feature; `async_tokio`, `async_futures`, `async_smol`, `csv_output`, `real_blackbox`. Windows: no Windows-titled issues [M].
- Alternative: `divan` 0.1.21 (2025-04-10, MIT OR Apache-2.0; last commit 2026-07-19; 51 open issues); text-splitter's own benches use divan [M]. `gungraun` (Valgrind-based) is not suitable on Windows [E].
- Pin: `criterion = { version = "0.8.2", features = ["async_tokio"] }` as a dev-dependency.

## 12. HTTP/gRPC load generator

| Tool | Version / date | Licence | What it does | Fit |
|---|---|---|---|---|
| oha | 1.16.0 / 2026-08-23 | MIT | HTTP/1.1 and HTTP/2 (`--http2`, `-p` parallel streams), fixed rate `-q`, `--latency-correction` (avoids coordinated omission), request bodies `-d`/`-D`, JSON/CSV output, per-request SQLite output, TUI; optional `http3` feature; no gRPC [D, README]. Windows: prebuilt `oha-windows-amd64.exe` and a PGO build are release assets; README lists Windows 10 as tested [M]. Windows issues all closed (latest 2024-04) [M] | Good for MCP Streamable HTTP POSTs and REST endpoints |
| ghz | v0.121.0 / 2025-12-07 | Apache-2.0 | Go gRPC load tool: `.proto` or protoset input, client/server/bidi streaming options, rate and concurrency; Windows zip release asset [M]. 3,356 stars, last commit 2026-09-01 [M] | The standard gRPC tool, but not Rust and not installed here |
| goose | 0.18.1 / 2025-08-14 | Apache-2.0 | Locust-style scenario HTTP load testing on reqwest 0.12; no gRPC [M]. Last commit 2026-03-10 | No: older reqwest, no gRPC |
| rlt | 0.5.0 / 2026-01-28 | MIT OR Apache-2.0 | Library for building your own load tool (HTTP, gRPC, DB, anything): you implement a bench suite trait, it provides concurrency, statistics, a real-time TUI and baseline comparison [D, README]. 562,728 downloads, 41,426 recent [M] | Best Rust base for a custom client |
| stoke | 0.1.0-alpha / 2026-04-25 | n/a | "HTTP and gRPC load generator CLI", 21 downloads [M] | Too immature |
| custom tokio client | n/a | n/a | tonic client plus `hdrhistogram` 7.6.0 (2026-07-18) | Roughly 150 lines [E] |

**Recommendation:** use **oha** for the HTTP and MCP endpoints (ready-made Windows binary, latency correction), and build a small **rlt**-based Rust binary with a tonic client for the gRPC streaming API and for any multi-step MCP flows that a single POST body cannot express. Treat ghz as an optional cross-check for gRPC, not a dependency. In Stage 1 the load generator and server share the same CPU, which perturbs results; run the generator from the other machine in Stage 2 [E].

## 13. Suggested Cargo pins (a starting point for the lead; not tested)

```toml
[dependencies]
rmcp            = { version = "=3.5.0", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }
tonic           = "=0.14.6"
tonic-prost     = "=0.14.6"
prost           = "0.14.4"
prost-types     = "0.14.4"
axum            = { version = "0.8.9", features = ["http2"] }
tokio           = { version = ">=1.53.1", features = ["full"] }
reqwest         = { version = "0.13.5", default-features = false, features = ["json"] }
rayon           = "1.12"
moka            = ">=0.12.16"          # see risk note in section 7
rusqlite        = { version = "0.40.2", features = ["bundled"] }
blake3          = { version = "1.8.7", features = ["rayon", "mmap"] }
zstd            = "0.14.0"
roxmltree       = "0.21.1"
text-splitter   = { version = "=0.33.0", features = ["markdown", "tokenizers"] }
tokenizers      = { version = "=0.23.2", default-features = false, features = ["onig"] }
whichlang       = "0.1.1"
gaoya           = "=0.2.2"

[build-dependencies]
tonic-prost-build = "=0.14.6"           # needs protoc, or switch to protox

[dev-dependencies]
criterion = { version = "0.8.2", features = ["async_tokio"] }
```

Commit `Cargo.lock`. Phase 1 should run `cargo build` on the MSVC toolchain, then `cargo tree -d` (duplicates: expect several `rand`, `itertools`, `base64`, `smallvec` majors, which are harmless; `zstd-sys` must be single) and `cargo tree -i aws-lc-sys` (should be absent with the features above).

## 14. Open questions and items outside the brief

1. Which MCP protocol version does Claude Code negotiate, and does it work against rmcp 3.5's default stateless path or only the legacy session mode? (Needs a live test.)
2. Atomic code/table/equation chunks: accept text-splitter's oversize behaviour, or pre-segment blocks ourselves (section 3)?
3. Chinese segmentation behaviour of text-splitter's `icu_segmenter` path is untested.
4. `protoc` is not installed (or not on PATH): install it, or use `protox`.
5. One port or two for MCP (HTTP) and gRPC; `axum` `http2` feature required if axum serves gRPC.
6. moka versus `quick_cache` given open crash reports (#605 on Windows).
7. Check `CC`/`CXX` environment variables before the first `rusqlite` build (issue #1861 root cause); I did not read environment values.
8. Outside the requested list but relevant: tantivy 0.26.2 (2026-09-08) has an open Windows issue #2847 (`ManagedDirectory` commit fails with `PermissionDenied`, 2026-07-28) and older open #2272; another worker may be covering tantivy [M, issue titles only].
9. Everything above reflects data seen on 2026-09-30; rmcp in particular releases weekly.

## 15. Sources

Registries and APIs queried today: `https://crates.io/api/v1/crates/<name>` (and `/<version>`, `/<version>/dependencies`) for every crate named; `https://api.github.com` through `gh api` for repos, commits, releases, licences, issue search, issue bodies and comments.

Files read (raw.githubusercontent.com unless noted):
- text-splitter: `benbrandt/text-splitter` `README.md`, `CHANGELOG.md`, `Cargo.toml`, `src/splitter/markdown.rs`, `src/chunk_size/huggingface.rs`; issues #798, #422, #1137.
- tokenizers: `huggingface/tokenizers` `README.md`, `tokenizers/Cargo.toml`, `REQUIRED_FOR_V1.md` (via API); issues #1581, #1914, #2447, #2460, #2458, #2094; releases v0.23.2 and v1.0.0-rc.2. Qwen3 files: `https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/tokenizer.json` and `.../Qwen3-Reranker-0.6B/...` (first 400 bytes only).
- rmcp: `modelcontextprotocol/rust-sdk` `README.md`, `LICENSE`, `docs/OAUTH_SUPPORT.md`, `crates/rmcp/src/model.rs`, `crates/rmcp/src/transport/streamable_http_server/tower.rs`, `examples/servers/Cargo.toml`, `examples/servers/src/simple_auth_streamhttp.rs`, `examples/servers/src/counter_streamhttp.rs`; GitHub releases; discussion #969 (`https://github.com/modelcontextprotocol/rust-sdk/discussions/969`).
- tonic/prost: `hyperium/tonic` (now `grpc/grpc-rust`) `README.md`, `Cargo.toml`, `examples/src/compression/server.rs`, `examples/src/h2c/server.rs`, GitHub release `tonic-v0.14.6`; `tokio-rs/prost` `README.md`, `CHANGELOG.md`.
- quick-xml `README.md`, `Changelog.md`; roxmltree `README.md`, `CHANGELOG.md`, `src/parse.rs`.
- rusqlite `README.md`, `libsqlite3-sys/build.rs`, `libsqlite3-sys/sqlite3/sqlite3.h` at tag `v0.40.2` (first 30 KB); issue #1861.
- moka `README.md`, `CHANGELOG.md`, issues #600 and #605; tokio `tokio/CHANGELOG.md`, issues #8299 and #8342; axum `axum/CHANGELOG.md`; reqwest `CHANGELOG.md`, issue #2937; rayon `RELEASES.md`; blake3 `README.md`; zstd-rs `Readme.md`; quinn `README.md`; criterion `README.md`.
- gaoya `README.md`, `CHANGELOG.md`, `Cargo.toml`; probminhash `README.md`; incrededup `README.md`; txtfp `README.md`; minhash-rs `README.md`; datasketches `README.md`.
- whichlang `README.md`, `src/lib.rs`; lingua-rs `README.md`; whatlang-rs `README.md`.
- oha `README.md` and release assets; ghz `README.md` and release assets; goose `README.md`; rlt `README.md`.
- Local: `C:\Users\sethy\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\multirust-channel-manifest.toml`; PowerShell `Get-Command` for build tools.

Intermediate API dumps were kept in the session scratchpad (`...\scratchpad\crates.json`, `details.json`, `gh.json`, `raw\`) and are not part of the deliverable.
