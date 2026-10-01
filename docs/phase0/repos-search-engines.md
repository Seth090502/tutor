# Phase 0 study: code search engines (exact, regex, ranked)

Worker report for the lead engineer of the vault (libraryd). Read-only study, written 2026-10-01.
All clones are shallow (`--depth 1 --single-branch`) under `C:/Users/sethy/Projects/refs/code/<name>`. Nothing from these repositories was built or run. Paths below are relative to the clone folder named in each section. Line numbers refer to the pinned HEAD.

Convention: "(measured)" means I read it in code or in a file in the repo. "(claimed)" means a project statement I did not reproduce. "(unverified)" means I could not confirm it from the clone.

## 0. Index of repositories, pins and currency

| Short name | Canonical URL | Pinned HEAD | Last commit | State on 2026-10-01 | Licence (file) | Fit |
|---|---|---|---|---|---|---|
| zoekt | github.com/sourcegraph/zoekt | 153817f643cde8b229ee388c1dddbcf07f4798af | 2026-09-11 | Active. Maintained source since 2017; `google/zoekt` is archived (last push 2024-01-16) | Apache-2.0 (`LICENSE`) | port_ideas (primary reference) |
| sourcebot | github.com/sourcebot-dev/sourcebot | 390e8b2d912d5c88158f39622e679523b95689e4 | 2026-09-29 | Very active | FSL-1.1-ALv2 (source-available, anti-competing-use) outside `ee/`; `ee/` is a proprietary Enterprise License (`LICENSE.md:1-12`, `ee/LICENSE`) | reference_only |
| livegrep | github.com/livegrep/livegrep | 923d5ad71dfe60900e6c2017b2fa4a5ff902ad71 | 2026-02-10 | Maintained slowly (8 months idle) | BSD-2-clause style (`COPYING`, Nelson Elhage) | reference_only |
| hound | github.com/hound-search/hound (successor of etsy/hound) | cf795f1889a7711cd46bfbef5f865544beeec391 | 2026-09-10 | Active, small | MIT (`LICENSE`) | reference_only |
| codesearch | github.com/google/codesearch | 74a12a911a79b901d1158c48d011b2da1b090fc9 | 2026-08-30 | Alive (recent perf commit), stable, Russ Cox | BSD-3-clause (`LICENSE`) | port_ideas (algorithms) |
| bloop | github.com/BloopAI/bloop | 431e9e82c5a293c40f22aadb3615c3aa387af82e | 2024-12-04 | ARCHIVED (GitHub `archived: true`), 10 months stale | Apache-2.0 (`LICENSE`) | port_ideas (closest Rust analogue to the vault plan) |
| probe | github.com/probelabs/probe (the brief's `buger/probe` redirects here) | 3207ab029ffaab378f5d480137466752bcc5ad83 | 2026-10-01 | Very active | LICENSE file is Apache-2.0, but `Cargo.toml:13` says MIT and `npm/package.json:79` says ISC. Inconsistent; flag before copying anything | reference_only (ideas); do not depend |
| ast-grep | github.com/ast-grep/ast-grep | 25334496c105c9c728f0a6024f3d164aed40cacc | 2026-09-28 | Very active, v0.45.3 | MIT (`LICENSE`) | adopt_dependency (core/outline crates) + port_ideas |
| opengrok | github.com/oracle/opengrok | 68ff6f2baf6c7fd98c3e0fbf6154221be03ce20c | 2026-09-30 | Active maintenance mode (Java, Lucene 9.12.3) | CDDL-1.0 (`LICENSE.txt:1-5`), file-level copyleft; extra third-party licences `LICENSE-*.txt` | reference_only |
| ast-grep-mcp (extra) | github.com/ast-grep/ast-grep-mcp | 149e20d47bb7125fb0c1451feea2f48a98742034 | 2026-08-24 | Active, "experimental" per README | MIT | reference_only |
| cs / codespelunker (extra) | github.com/boyter/cs | a09eff7d39c81c573ea703d5a9d2b3b9f783448e | 2026-09-30 | Very active, 1079 stars | MIT | port_ideas |
| fast_code_search (extra) | github.com/jburrow/fast_code_search | d0acbca4041dac8d353fcd8469e9916914213e02 | 2026-09-08 | Active but tiny (3 stars, one author) | MIT | port_ideas (architecture twin of the vault plan; low maturity) |

Licence flags for the lead: no GPL or AGPL in this cluster. Sourcebot is source-available (FSL: no "Competing Use") plus proprietary `ee/`; do not copy code or prompt text. OpenGrok is CDDL (file-level copyleft; fine to study, a Rust port of ideas is not a derivative, but do not translate files). Probe has an inconsistent licence declaration.

Why no extra "mgrep"/"instantgrep"/"grix"-style tiny projects: a GitHub search (2026-10-01) found several 2026 Rust trigram toys (instantgrep, grix, xgrep, houndr; 3 to 27 stars). I judged fast_code_search the only one with a design (symbols + import graph + gRPC + watcher + persisted mmap index) close enough to the vault to be worth reading. I did not read the others.

Design source outside the repos (vendor blog, not code; summary produced by a fetch tool, treat as claimed): Cursor, "Fast regex search" (https://cursor.com/blog/fast-regex-search): sparse n-grams instead of plain trigrams (weights on character pairs; at query time a covering algorithm picks the minimal n-gram set), a postings file plus an mmap'd lookup table of hashes and offsets, and freshness handled by basing the index on a git commit with a layer for working-tree changes. No latency numbers were given.

---

## 1. zoekt (sourcegraph/zoekt) — the reference implementation

**What it is.** Go, trigram-based code search over git repositories, used by Sourcegraph and Sourcebot. Mature (since 2016). Layout: `index/` (shard format, builder, scoring), `query/` (query tree), `search/` (multi-shard aggregation), `gitindex/` (git ingest, delta builds), `cmd/` (indexers, webserver, mirrors), `internal/ctags`.

**Ingestion.**
- Primary input is git blobs read by `zoekt-git-index` (`gitindex/index.go:503-611`), not a working tree. Consequence: `.gitignore` is implicit (ignored files are never in the tree); a separate `.sourcegraph/ignore` glob file exists (`ignore/ignore.go:24-27`, implicit `**` for patterns without glob characters, lines 52-56) and is explicitly unsupported in delta builds (`gitindex/index.go:1028,1057`).
- Plain directory indexer `cmd/zoekt-index/main.go:87` ignores `.git,.hg,.svn` by default, follows regular files and symlinks (lines 75-79).
- Language detection, test/vendor/generated detection: go-enry (`index/file_category.go:41-85`). Categories persisted per document: Default, Test, Vendored, Generated, Config, DotFile, Binary, Documentation. Low priority = Test, Vendored, Generated, Binary (`file_category.go:89-91`). Size-skipped or binary docs are classified by filename only (`file_category.go:51-53`).
- Skip reasons (`index/document.go:25-34`): too large, too small (under 3 bytes), binary (any NUL byte, `shard_builder.go:686-688`), too many distinct trigrams (probable minified or data file, `shard_builder.go:692-717`), missing object.
- Defaults (`index/builder.go:327-334`): max file size 2 MiB, shard content cap 100 MiB, max 20,000 distinct trigrams per document. Large files can be forced in by glob (`IgnoreSizeMax`, `builder.go:531`, Sourcebot passes `-large_file`).

**Indexing.**
- Positional trigrams over runes (not bytes). Posting lists hold offsets, varint delta coded (`index/bits.go:245-347`); the trigram dictionary is a B-tree (`index/btree.go`) laid out for mmap. Shard layout and tradeoffs documented at `doc/design.md:26-90,129-162`: shard is about 3.5x corpus (content copy + offsets), one goroutine per shard, uint32 offsets cap a shard near 1 GB of content (the 100 MiB default keeps parallelism).
- Query side: a regex is reduced to substring atoms; only two trigrams per substring (start and end) are chosen and checked "at the right distance", picking the rarest pair (`design.md:41-64`). Regex verification uses Go regexp with an optional go-re2 path for large content (`index/matchtree.go:189-194`).
- Case-insensitive search expands all case variants of the candidate trigrams (`design.md:92-98`).
- Branches: per-file bitmask over up to 64 branches, identical blobs stored once (`design.md:112-126`, `doc/faq.md:117-121`).
- Symbols: universal-ctags, run on each document at index time, converted to byte-range sections plus symbol metadata (`index/ctags.go:45-157`); the parser type is pluggable (universal-ctags or scip-ctags, `internal/ctags/parser_bins.go:28-30`). Failure policy is configurable (`CTagsMustSucceed`, `builder.go:97`). ctags is run under seccomp (`doc/ctags.md:10-16`).
- No reference/call graph. "Find references" is a regex over text.
- Incremental: delta builds (`gitindex/index.go:574-601,909-963`). A delta shard stores only changed or removed paths; older shard entries are tombstoned (`index/tombstones.go`). Falls back to a full build if options, branch set, or shard count (`DeltaShardNumberFallbackThreshold`, line 441-447) changed, and not supported with submodules or ignore files. Compound shards merge many repos (`index/merge.go`). No file-watcher; the polling service is `zoekt-indexserver`.

**Ranking** (all measured).
- Line-level score components (`index/contentprovider.go:589-610`): word-boundary match 500, partial word 50, filename full-base match 7000 (edge 5500, inner 4000), symbol-section match 7000 (edge 5500, overlap 4000), symbol-kind boost 100 x per-language factor (1 to 10; for C++ class 10, enum 9, function 8, struct 7, union 6, typedef 5, field 4, variable 3; `contentprovider.go:823-841`; Go exported symbols +0.5, `_test.go` x0.8; also Python, Ruby, PHP, Scala, GraphQL, Markdown headers), atom-count boost up to 400 (`index/score.go:98-198,312-316`).
- File score: best line score + in-file order factor 1, truncated, then `10,000,000 x score + 100 x repo rank + 10 x doc order` so repository rank and doc order only break ties (`score.go:352`, `contentprovider.go:604-609`).
- Optional BM25 mode (`UseBM25Scoring`): constant idf (their evaluation found idf hurt keyword queries, `score.go:355-366`), k=1.2, b=0.75, filename and symbol matches count x5, low-priority files tf divided by 5 (`score.go:249-279`).
- No graph centrality. Repo rank is an external signal (e.g. stars).

**Exposed to agents.** No MCP server in zoekt (grep for MCP across the clone: none). It exposes a JSON API (`doc/json-api.md`) and gRPC (`grpc/`, `api_proto.go`). Search options (`api.go:971-1033`): `ChunkMatches` (multi-line chunks with ranges, plus `NumContextLines`), `MaxDocDisplayCount`, `MaxMatchDisplayCount`, `ShardMaxMatchCount` default 100,000 and `TotalMaxMatchCount` default 10x that (`api.go:1040-1046`), `MaxWallTime`, `FlushWallTime`, `UseBM25Scoring`, `DebugScore`. A `DisplayTruncator` trims files and matches to the display budget (`index/limit.go:20-66`). Query language: `doc/query_syntax.md` fields `file:`, `repo:`, `lang:`, `sym:`, `branch:`, `case:`, `type:` (filematch/filename/repo), `or`, `-neg`, parentheses. `type:filename` and `sym:` are the two bits an agent needs most.

**Scale and performance evidence.**
- Design goal sub-50 ms on Android-sized corpora (about 2 GB text) (`design.md:10-11`). FAQ historical snapshot on cs.bazel.build: rare-string queries in ms; 7-10 ms for "r:torvalds crazy"; Linux kernel (55K files, 545 MB) indexed in about 160 s single-threaded on an X250 laptop (`doc/faq.md:20-31,123-142`). These are project-reported (claimed), not independent. RAM guideline: 20% over corpus size, index about 3.5x corpus (`faq.md:109-115`).
- The FAQ's statement that hound is "about 10x slower" and livegrep needs lots of RAM is a competitor claim (unverified).

**Concrete ideas to adopt (Rust vault).**
1. File-category flags (test, vendored, generated, config, docs, binary) computed at ingest and used as ranking multipliers and as filters. Effort S. In Rust: `hyperpolyglot` (linguist port, used by bloop) or `ignore`/own path rules; "generated" also via `.gitattributes linguist-generated` (hound does this). Licence-safe (idea only; enry is MIT, hyperpolyglot is MIT-ish, verify).
2. Document skip rules: NUL byte, max file size 2 MiB with allow-list override, distinct-trigram cap 20,000 (cheap minified/data detector), min 3 bytes; record the skip reason per file and expose it (`vault_info`, `read` errors) so the agent knows why a file is absent. Effort S.
3. Symbol-section ranking: store symbol byte ranges + kind per document, boost matches whose span equals/edges a symbol, and apply a per-language kind table (class/struct/function above variable). Effort M. Replace ctags with tree-sitter tags queries; tantivy has no native notion so keep ranges in SQLite or a side table and rescore the top K.
4. Final score layering: relevance score first, repo rank and document order only as tie-breakers, with `DebugScore` explaining each component. Effort S. This gives the vault an auditable ranking.
5. Display truncation as a first-class concept (doc cap + match cap + wall time) with a `hasMore`/exhaustive flag (`index/limit.go`). Effort S.
6. Dictionary layout: ngram B-tree mmap for the trigram index if the vault builds its own index rather than leaning on tantivy. Effort L; probably not needed (see section 12).
7. Branch bitmask dedupe for multiple revisions of the same repo. Effort M; only if the vault stores more than one ref per repo.

---

## 2. sourcebot (sourcebot-dev/sourcebot)

**What it is.** TypeScript (Next.js web, Express+BullMQ worker, Prisma/Postgres, Redis) wrapper that clones repos from code hosts, indexes them with a vendored zoekt fork (`.gitmodules`: `vendor/zoekt` -> `sourcebot-dev/zoekt`), and serves a UI, an "Ask" agent and an MCP server. Maturity: production product (about 3,957 stars).

**Licence.** `LICENSE.md:1-12`: FSL-1.1-ALv2 (any purpose except Competing Use; converts to Apache-2.0 after two years) outside `ee/`; anything under `ee/` is a proprietary Enterprise License (`ee/LICENSE`: forbidden to copy, merge, publish). The MCP server itself is at `packages/web/src/ee/features/mcp/server.ts`, i.e. under the Enterprise License; the tool definitions it registers are in `packages/web/src/features/tools/` (FSL). `skills/codebase-guide.md` states Ask and MCP "are EE since v5". Treat everything as reference_only: read for ideas, do not copy code or prompt text.

**Ingestion and indexing.** All delegated to zoekt: backend runs `zoekt-git-index` with `-max_trigram_count`, `-file_limit`, `-branches`, `-large_file` patterns (`packages/backend/src/zoekt.ts:11-28`). Defaults `maxFileSize` 2 MB, `maxTrigramCount` 20000, reindex every hour, connection resync every 24 h (`packages/shared/src/constants.ts:21-24`). Search path is gRPC to the zoekt webserver (`packages/web/src/features/search/zoektSearcher.ts`). Local symbol search needs universal-ctags exactly 6.1.0, or it silently returns nothing (`skills/codebase-guide.md`, "Traps"). Indexing is git-only (clone then index), so a working-tree folder without git does not fit without a wrapper.

**Ranking.** Zoekt's. Nothing added.

**What it exposes to agents** (`docs/docs/features/mcp-server.mdx:306-497`; `ee/features/mcp/server.ts:35-78`). Streamable HTTP at `/api/mcp`, OAuth 2.0 (dynamic client registration) or bearer API key. Tools and parameters:
- `grep(pattern, path?, include?, repo?, ref?, limit=100)` plus `groupByRepo` in code (`features/tools/grep.ts:17-45`). Output is text: `Found N matches in M files`, then `[repo] path:` and `  <lineno>: <line>` lines; lines over 2000 chars truncated; footer "(Results truncated. Consider using a more specific path or pattern...)" when `isSearchExhaustive` is false (`grep.ts:11-15,150-214`). `groupByRepo=true` returns per-repo counts only (10,000 match cap) to triage across many repos.
- `glob(pattern, path?, repo?, ref?, limit)`, `list_repos(query,page,perPage,sort,direction)`, `list_tree(repo, path, ref, depth<=10, includeFiles, includeDirectories, maxEntries<=10000 default 1000)`, `read_file(repo, path, ref, offset, limit<=500)`, `list_commits`, `list_branches`, `get_diff(repo, base, head)`.
- `read_file` budget: max 500 lines, 5 KB output cap, 2000-char line cap, response states the line range read, total line count, and the next offset when truncated (`features/tools/readFile.ts:11-16,60-80`, `readFile.txt`).
- `find_symbol_definitions(symbol, repo)` = zoekt `sym:` regex `\bname\b` (ctags-backed) and `find_symbol_references(symbol, repo)` = plain regex `\bname\b` over content, 1000-match cap, no scope resolution (`features/codeNav/api.ts:13-14,16-125`). Tool text itself says "prefer this over grep" because of "symbol boundaries" (`findSymbolDefinitions.txt`). So "references" is only a word search.
- `ask_codebase(query, repos?, languageModel?, visibility?)`: a server-side agent loop that does the search and returns a markdown answer; description tells the caller not to use it unless explicitly asked (`server.ts:104-106`). Skill tools `create_skill/update_skill/list_skills`.
- Every tool ships a long, behaviour-shaping `*.txt` description (grep.txt, readFile.txt, listTree.txt).

**Scale evidence.** None in repo; product claims only (unverified).

**Ideas to adopt** (re-implement, do not copy).
1. `groupByRepo`-style triage mode: counts per repo/folder before drilling in. For 5-7 repos this is the cheapest "map" step. Effort S.
2. Uniform output shape with explicit truncation and "how to continue" hints (next offset, narrower scope) in the text. Effort S.
3. Honest limits per read call plus a total-lines field so the agent can page deterministically. Effort S.
4. Tool descriptions as separate versioned text files (`.txt` next to each tool) tuned for agent behaviour. Effort S.
5. Do not copy the definition of "references": the vault should do better (section 12).

---

## 3. livegrep (livegrep/livegrep)

**What it is.** C++ backend `codesearch` (src/, about 4.3k lines) plus Go web frontend; Nelhage's suffix-array regex search. gRPC between them (`src/proto/livegrep.proto`). Bazel build. Reference for the suffix-array alternative, not for ranking.

**Ingestion.** Git trees via libgit2 (`src/git_indexer.cc:43-130`, recursive tree walk, optional submodules) or a plain filesystem walk (`src/fs_indexer.cc`). Any file containing a NUL byte is skipped entirely (`src/codesearch.cc:556-557`). Lines at or above `--line_limit` 1024 are not indexed, but an empty line keeps numbering stable (`codesearch.cc:56,576-581`). No `.gitignore` logic (it indexes git trees, so ignore is implicit), no generated or vendored detection. Config is JSON (`src/proto/config.proto`).

**Indexing.**
- Content is split into lines; byte-identical lines are deduplicated within a chunk (`--compress`, default on; `codesearch.cc:51,583-598`) and stored in large chunks (`--chunk_power` default 27 = 128 MiB per chunk, `src/chunk_allocator.cc:20`). Each chunk gets a suffix array built over its data (`src/chunk.h:83-91`; Nelhage's blog says libdivsufsort, `blog.nelhage.com/2015/02/regular-expression-search-with-suffix-arrays/`). A per-chunk interval structure maps data ranges back to the files containing those lines (`chunk.h:28-54,78-81`).
- Query planner: builds a tree of byte ranges from the RE2 parse (`src/query_planner.cc`, `query_planner.h:38-139`), estimates selectivity and weight per candidate "index key", and walks the suffix array with it. If the key hits too much of the chunk (count x 50 > chunk size) it falls back to a full scan (`codesearch.cc:47,780-783`).
- Result matching uses RE2 over candidate line ranges; file-path regexes and repo filters are applied by walking the chunk-to-file intervals (`codesearch.cc:826-868`).
- Optional ctags tag search from a tags file (`src/tagsearch.cc:49-70`, ctags `--format=2 -n --fields=+K` lines), only as filtering, no ranking.
- Incremental updates: none in the engine. Whole index is rebuilt and written with `-dump_index`, loaded with `-load_index` (mmap), and swapped with the `Reload` RPC or `--hot_index_reload` (`src/tools/codesearch.cc:48-56`, `README.md` "Using Index Files"). Index size is "usually 3-5x the indexed text" (`README.md`, "Resource Usage").

**Ranking.** None. Results come in chunk order, default `max_matches` 50, `context_lines` 3 (`src/tools/grpc_server.cc:33-34`); search timeout 1000 ms (`codesearch.cc:54`). Filename hits come back as a separate `file_results` list (`livegrep.proto:64-70`).

**Exposed to agents.** No MCP. gRPC `CodeSearch.Search(Query) -> CodeSearchResult` with `line, file[], repo, tags, fold_case, not_file, not_repo, not_tags, max_matches, filename_only, context_lines`; result carries per-match `context_before/after`, `bounds`, and stats including `exit_reason` (NONE/TIMEOUT/MATCH_LIMIT) (`livegrep.proto:1-80`). The explicit exit reason is the useful bit.

**Scale evidence.** README: "~gigabyte-scale source repositories" (claimed). Public instance livegrep.com (not checked).

**Ideas to adopt.**
1. Return an explicit stop reason (complete / timeout / match limit) with every search. Effort S.
2. Line-level dedup of identical lines inside an index chunk (boilerplate headers, license text, generated tables). Effort M; for the vault this is better done as exact-duplicate-file and near-duplicate suppression at ranking time (cs `--dedup`, section 10).
3. Selectivity-weighted choice among candidate index keys, with a scan fallback once the candidate set is more than about 2% of the data. Effort M. Applies to any trigram plan.
4. Suffix arrays are not recommended for the vault: no incremental updates, no tantivy integration, Windows build via Bazel/libgit2/re2 is heavy. Keep as reference_only.

---

## 4. hound (hound-search/hound)

**What it is.** Go service: polls repos, builds a per-repo Russ Cox trigram index, React UI, small JSON API. Core is a vendored fork of google/codesearch (`codesearch/`). Mature but minimal.

**Ingestion** (`index/index.go`). Walks the checked-out working tree with `filepath.Walk` (`index.go:387-451`), skipping "special" names (`.git`, `.svn`) and dot-files (default `exclude-dot-files: true`, `docs/config-options.md:57`). Text test is the first 2048 bytes being valid UTF-8 (`index.go:21,258-280`). No `.gitignore` support at all. Every indexed file is also stored gzip'd under `raw/` so results never touch the working tree (`index.go:308-331`). `auto-generated-files` are flagged in the UI and, for git, are read from the `linguist-generated` attribute (`docs/config-options.md:58`). Everything excluded is written to `excluded_files.json` with a reason (`index.go:24-28,347-356,453-457`), which is a good transparency pattern.

**Indexing.** File-level trigram posting lists from codesearch (`codesearch/index/write.go`): the on-disk format (`codesearch/index/read.go:8-70`) is prefix-compressed path lists, trigram posting lists of delta-coded file ids, a posting list index with 256-byte alignment for binary search, mmapped. Limits for "is text": no invalid UTF-8, at most 1 GiB per file, no line over 2000 bytes, at most 20,000 distinct trigrams (`codesearch/index/write.go:128-134`). Zoekt copied the 20,000 trigram figure. There are no symbols, no references, no embeddings.
- Update model: every poll with a changed revision builds a complete new `idx-<random>` directory, then atomically swaps and destroys the old one (`searcher/searcher.go`, `swapIndexes`, `buildAndOpenIndex`; defaults poll 30 s, `docs/config-options.md:29`). No incremental indexing.

**Ranking.** None. Files come back in index (file id) order; `MaxResults`, `Offset/Limit`, `FileRegexp`, `ExcludeFileRegexp`, `LinesOfContext` are the only controls (`index.go:42-51,146-247`). API caps max results at 100000 (`api/api.go:21`).

**Exposed to agents.** HTTP JSON only (`api/api.go`), no MCP.

**Scale evidence.** None in repo. Zoekt's FAQ says hound is about 10x slower than zoekt (competitor claim, unverified).

**Ideas to adopt.** (1) The `excluded_files.json` pattern: persist every skip with a reason, expose it. Effort S. (2) Keep a verbatim compressed copy of every indexed file so reads are index-consistent (the vault already stores documents verbatim). (3) `linguist-generated` / `.gitattributes` honouring. Effort S with the `ignore` crate plus a small attributes parser. (4) Reject the full-rebuild-on-change update model.

---

## 5. google/codesearch (Russ Cox)

**What it is.** The 2011 reference implementation of the "regexp via trigram index" article; Go CLI (`cindex`, `csearch`, `cgrep`, `csweb`). Hound's core is a fork. Last commit 2026-08-30 (slight perf change); essentially frozen design. 5.8k lines.

**Ingestion.** `cmd/cindex/cindex.go:123-166`: `filepath.Walk` over roots, skipping any name that starts with `.`, `#`, `~` or ends with `~`, only regular files, optional `-zip` content. No `.gitignore`. Text filter in `index/write.go:128-134` (see section 4).

**Indexing and format.** `index/read.go:8-70` (documented format: roots, sorted prefix-compressed names with a sparse name index every 16th, posting lists with delta file ids, posting-list index in 256-byte padded sections, trailer). Query construction: `index/regexp.go:20-50` turns a regexp into an AND/OR tree of trigrams (`QAll`, `QNone`, `QAnd`, `QOr`). File-level (not positional) trigrams, so a candidate file still has to be scanned by the real regexp.

**Incremental.** `cindex` re-indexes the given roots into a side file and merges it into the master index (`cmd/cindex/cindex.go:149-185`, `index/merge.go`), new entries replacing old ones for the same paths. Coarse: re-walks and re-reads the roots.

**Ranking, agent exposure, scale.** None / none / "large bodies of source code" (claimed, `README`).

**Ideas.** The regexp-to-trigram-query algebra is the part to port: parse with `regex-syntax` (Rust), compute `exact/prefix/suffix/match` sets as in the article (Cox `regexp4`), simplify, and emit an AND/OR over trigram postings. In Rust this is about 400-700 lines (bloop's `query/planner.rs`, 417 lines, is a ready model; see section 6). Effort M. Port the algorithm; the BSD-3 notice is satisfied by keeping the copyright line if any code is translated.

---

## 6. bloop (BloopAI/bloop) — ARCHIVED, but the closest Rust precedent

**What it is.** Rust (`server/bleep`, about 37k lines), tantivy 0.21 + tree-sitter + Qdrant + ONNX embedder + LLM agent, Tauri desktop app. Repository archived (last commit 2024-12-04). The Windows checkout of my clone failed on some path (`git restore` hint at clone time), so I restored only `server/` and a few root files; the model binaries are LFS pointers (`.gitattributes`) and were not fetched.

**Ingestion** (`server/bleep/src/indexes/file.rs`, `repo/iterator.rs`, `repo/iterator/fs.rs`).
- Directory walk with the `ignore` crate: `standard_filters(true)` so `.gitignore`, `.ignore`, global ignores are honoured, `hidden(false)`, `.git` explicitly excluded (`fs.rs:14-30`). Remote repos use a git walker over branch trees (`file.rs:173-180`).
- Extension blacklist: images, fonts, docs, media, archives, binaries, `log`, `lock`, `dat`, etc. (`iterator.rs:25-46`). Vendor patterns by extension: Go (`vendor|third_party`, `.pb.go`), JS-family (`node_modules|vendor|dist`) (`iterator.rs:142-178`; unit-tested at `iterator.rs:184-221`).
- Size gates: `MAX_FILE_LEN = AVG_LINE_LEN(30) x MAX_LINE_COUNT(20000) = 600 KB`, plus a line-count cap (`iterator.rs:19-23,105-107`, `file.rs:723-729`). Per-repo include/exclude overrides (`FileFilterConfig`, `repo/iterator/filters.rs:87-130`). Skipped files still get a stub document with `indexed=false` so they stay listable (`file.rs:644-673`).
- Language: `hyperpolyglot` (Rust linguist port) on path + buffer (`repo/iterator/language.rs:1-47`). Files are read with `read_to_string`, so non-UTF-8 files are dropped (`fs.rs:48-56`).
- Incremental: per-file cache key = blake3 of schema version + path + repo + contents + filter override; unchanged files are skipped, stale hashes are deleted from tantivy by term (`file.rs:56-87,211-216,498-503`). That is a clean, simple incremental scheme.

**Indexing.**
- Tantivy fields (`indexes/schema.rs:68-105`): `relative_path`, `content`, `symbols` (all ngram-tokenised with positions), `symbol_locations` (bincode scope graph), `line_end_indices` (u32 LE), `lang`, `line_length`, `last_commit_unix_seconds`, `branches`, raw fast fields. The ngram tokenizer is `NgramTokenizer::new(1, 3, false)` (`indexes.rs:249`): tantivy used as the trigram index (and 1- and 2-grams too, which is index-size heavy; not measured).
- Query side: literals become the AND of per-trigram term queries, case-insensitive expands each trigram into case permutations only up to length 5 (`query/compiler.rs:20,104-145`). Regexes go through `query/planner.rs` into a `Fragment` tree (`Literal` / `Dense(Or|And, ..)`) that is compiled into a nested `BooleanQuery` (`compiler.rs:160-190`). Candidates are then verified by regex (snippet stage). This is exactly the vault plan's "tantivy + trigram" idea in Rust, written in 2023.
- Symbols: tree-sitter queries per language (`.scm`) build a per-file lexical scope graph (petgraph) with Scope, Def, Import, Ref nodes and edges ScopeToScope, DefToScope, ImportToScope, RefToDef, RefToImport (`intelligence/scope_resolution.rs:25-117`). Languages: C, C++, C#, Go, Java, JS, TS, Python, PHP, R, Ruby, Rust, COBOL (`intelligence/language.rs:23-45`). No Lua, GDScript, HLSL, GLSL, CMake. Cross-file navigation is a heuristic, not a resolver: `code_navigation.rs:1-5` says "scope-graph based handler that operates only in the owning file" plus "search based handler that operates on any file belonging to the repo"; `files_imported/files_importing` match top-level def/import names across docs (`code_navigation.rs:64-183`), and `token_info` chains local defs/refs then repo-wide search (`code_navigation.rs:198-230`).
- Semantic chunks: token-count chunking (not AST) at 256 tokens (`config.rs:308-310`), overlap default 50% of max tokens or N lines (`semantic/chunk.rs:74-156`), file text prefixed with repo+file for tokenisation, newline preferred at 3/4 and a "boundary" at 7/8 of the budget (`chunk.rs:213-295`), noisy chunk filter (>50% digits or punctuation, `chunk.rs:158-174`). Vectors in Qdrant, BERT-family ONNX model shipped via LFS (model name not stated in source I read; unverified).
- Hybrid retrieval: lexical and semantic lists merged by Reciprocal Rank Fusion, k=60 (`semantic.rs:477-520`).

**Ranking** (`query/ranking.rs:1-40`). A tantivy score tweaker over BM25: x1000 if the language is known, divided by clamped average line length (20 to 1000, penalises minified), divided by commit age in seconds capped at 5,000,000 (recency). Fuzzy file-path search uses skim fuzzy matcher with position and filename bonuses (`file.rs:310-350`). No symbol boost, no PageRank.

**Exposed to agents.** An internal LLM agent with tools `code`, `path`, `proc` (read and answer over chosen files via semantic search), `answer` (`agent/tools/*.rs`); HTTP API for the app; no MCP.

**Scale evidence.** None in the repository.

**Ideas to adopt.**
1. Regex -> literal/Dense fragment planner -> tantivy BooleanQuery (`query/planner.rs`, `compiler.rs:160-190`). Effort M to port against `regex-syntax` 0.8 (bloop uses 0.6), and tantivy 0.2x. Apache-2.0: ideas free, translated code needs a notice. The tantivy `NgramTokenizer(3,3)` with positions disabled for the code field is the lighter variant; test index size on a 5-repo corpus before choosing.
2. blake3 per-file cache keys plus term deletion for incremental updates. Effort S.
3. Per-file stub documents for skipped files (listable, flagged `indexed=false`). Effort S.
4. RRF k=60 as the default fusion of BM25/trigram hits with dense hits. Effort S.
5. Scope-graph-per-file with tree-sitter queries gives definitions/references within a file without a language server; but the cross-file link was a name match. This is the gap the vault should close (section 12). Effort L for good coverage.
6. `hyperpolyglot` for language detection in Rust. Effort S.

---

## 7. probe (probelabs/probe; the brief's buger/probe moved)

**What it is.** Rust core (`src/`) plus a Node SDK, MCP server and chat agent (`npm/`). Stateless: no persistent search index (`ARCHITECTURE.md`; `docs/reference/how-it-works.md:1-60`). Pipeline: ripgrep-style scan, tree-sitter parse of matching files, rank, extract whole AST blocks, format within a token budget. Also an optional LSP daemon with a three-layer persistent cache for call hierarchy and references (`docs/indexing-overview.md`; `lsp-daemon/`). Very active, README-driven marketing; rough edges (Cargo.toml has "EXPERIMENT" comments in ranking constants, `src/ranking.rs:363-364`).

**Licence flag.** `LICENSE` is Apache-2.0, `Cargo.toml:13` says MIT, `npm/package.json:79` says ISC. Do not depend; if code is ever reused, ask the author.

**Ingestion.** `ignore` crate walk, `.gitignore` honoured by default (`src/search/file_list_cache.rs:161-183`), `follow_links(true)`, test files excluded unless `--allow-tests` (`how-it-works.md` step 2; `limits.md`), file list cached. Language by extension, parsers from tree-sitter crates: Rust, JS/TS, Python, Go, C, C++, Java, Ruby, PHP, Swift, C#, HTML, Markdown, YAML, Bash, QML, Solidity, Crystal, Haskell (`Cargo.toml:30-52`, `docs/reference/supported-languages.md`). No Lua, GDScript, HLSL, GLSL, CMake. It depends on `ast-grep-core 0.36.1` for its structural `query` command (`Cargo.toml`). A parser pool and AST cache avoid re-parsing (`docs/probe-cli/performance.md`).

**Indexing.** None for text search. Per-query: tokenise (camelCase/snake splitting, stemming via `rust-stemmers`, `decompound`), compile an Elasticsearch-style query (`AND/OR/+/-/"phrase"/ext:/lang:`), scan with ripgrep-like SIMD matching (memchr, aho-corasick), rank.

**Ranking.** BM25 with k1=1.5, b=0.5 as "experiment" (`src/ranking.rs:360-386`), early file-level BM25-like pre-rank with filename boost and a coverage multiplier up to 3x (`src/search/early_ranker.rs:26-76`), hybrid and TF-IDF modes, SIMD ranking, optional BERT reranker (candle, `src/bert_reranker.rs`). Position weights for terms inside identifiers (`how-it-works.md`). No graph centrality, no recency.

**Exposed to agents.** MCP over stdio, `npm/src/mcp/index.ts:213-330`: `search_code(path, query, exact, strictElasticSyntax, session, nextPage, lsp)`, `extract_code(path, files[], allowTests, contextLines, format, timeout, lsp)` where each file is `path:LINE` or `path#symbol`, and `grep(pattern, paths, ignoreCase, count, context)` for non-code files. Notes: result sessions deduplicate across calls (blocks already shown are excluded, `how-it-works.md` "Session-based Caching"); search output is bounded by `--max-results`, `--max-bytes`, `--max-tokens` using tiktoken counting with `truncated` and totals returned (`src/search/limits.rs:1-60`), default 30 s timeout, merge threshold 5 lines for adjacent blocks. Results are whole AST blocks (function/class), not line windows. `probe symbols FILE` returns a table-of-contents with nesting (`docs/probe-cli/symbols.md`). The `nextPage` field's description in the MCP schema is copy-pasted from another parameter ("Skip .gitignore files", `index.ts:236-240`), a quality smell. Its README argues the LLM itself is the semantic layer (boolean queries plus session dedup) and skips embeddings (`docs/reference/comparison.md:92`).

**Scale evidence.** "Milliseconds" and "SIMD-accelerated" (claimed, no numbers I could find in `README.md`, `docs/reference/comparison.md`, `docs/features.md`).

**Ideas to adopt.**
1. Return whole enclosing syntactic blocks for a hit, merged when within N lines, rather than raw line windows. Effort M (tree-sitter enclosing-node lookup; the vault already has line addressing).
2. Server-side session dedup: remember what was already returned to this client and exclude it from later searches ("show me the next 10 new things"). Effort M; fits MCP session ids.
3. Token-budget parameter on every output (`max_tokens`) with a `truncated` flag and totals. Effort S.
4. `path#symbol` and `path:line` extraction addresses; the vault's `doc_id#L40-L72` already covers lines, add `doc_id#symbol`. Effort S.
5. Reject: stateless per-query scanning (no index), LSP daemon dependency, Node orchestration layer.

---

## 8. ast-grep (ast-grep/ast-grep) — structural search and a new outline engine

**What it is.** Rust (workspace of `core`, `config`, `language`, `cli`, `lsp`, `napi`, `pyo3`, `wasm`, `dynamic`, and a new `outline` crate), v0.45.3, tree-sitter 0.27 (`Cargo.toml:37`), MIT, edition 2024, rust 1.88. Very active.

**Ingestion.** CLI walks with `ignore::WalkBuilder` (`crates/cli/src/utils/args.rs:9-90`, ripgrep-style flags `--no-ignore hidden|dot|exclude|global|parent|vcs`). Language by extension (`crates/language/src/lib.rs`: Bash, C, C++, C#, CSS, Dart, Elixir, Go, Haskell, HCL, HTML, JSON, Kotlin, Lua, Markdown, Nix, PHP, Python, Ruby, Rust, Scala, Solidity, Swift, YAML, Zig, plus JS/TS family; custom languages loadable through `crates/dynamic`, "custom language loading policy" in the last commit). No GDScript/HLSL/GLSL/CMake in the built-in list I read.

**Indexing.** None. Per-file parse then pattern match: patterns are code snippets with metavariables (`$NAME`, `$$$ARGS`) matched against the tree-sitter tree, rules are YAML (`kind`, `has`, `inside`, `pattern`, relational and composite). It is a scan tool; no inverted index, no incremental state.

**Outline crate (the part that matters here).** `crates/outline/src`: `model.rs` defines `OutlineItem` (top-level: name, `symbolType` using LSP DocumentSymbol kinds, byte range + line/col, signature, `astKind`, `isImport`, `isExported`, members) and `OutlineMember` (direct children with `isPublic`) and states it "intentionally stops at this item/member boundary ... does not try to build a semantic graph of references" (`model.rs:1-8`). Extraction is driven by per-language YAML rule files, e.g. `default_rules/cpp.yml` (include, function, class, struct, enum, concept), plus c, csharp, go, java, javascript, kotlin, markdown, php, python, ruby, rust, swift, typescript (6164 lines total, 270 for C#, 1065 for TypeScript). Options: filter by symbol type, regex over names/signatures, imports yes/no, exported yes/no, detail `Name` or `Signature`, member filters (`options.rs:1-67`). The CLI `ast-grep outline <paths>` has views `names` (one grouped line per symbol type), `signatures`, `digest` (signatures plus compact member name list), `expanded`, and items modes `structure|exports|imports|all`, with directory input defaulting to `names`/`exports` and a single file to `digest`/`structure` (`crates/cli/src/outline.rs:24-55`), JSON in pretty/compact/stream styles. The changelog shows this appeared recently (0.45.x: "extract embedded language symbols", "Markdown heading support", "bound outline file queue", "load custom language outline rules from config"; `CHANGELOG.md:11-12,36,107,117`). It is exactly the "repository map at several zoom levels" primitive the vault needs.

**Ranking.** None.

**Exposed to agents.** Not in this repo. The separate `ast-grep/ast-grep-mcp` (MIT, Python, shells out to the `ast-grep` binary, "experimental") exposes `dump_syntax_tree(code, language, format=pattern|ast|cst)`, `test_match_code_rule(code, yaml)`, `find_code(project_folder, pattern, language, max_results, output_format=text|json)`, `find_code_by_rule(project_folder, yaml, max_results, output_format)` (`ast-grep-mcp/main.py:109-215`). The agent must write patterns/rules itself, and gets text or JSON with matches only; no outline tool is wired in as of the pinned commit.

**Scale evidence.** None in repo.

**Concrete ideas.**
1. adopt_dependency: use `ast-grep-core` + `ast-grep-language` (MIT, Rust, already a probe dependency) for structural queries over stored verbatim sources, as a vault tool (`pattern_search`) that runs only on a prefiltered candidate set (from trigram/path filters). Effort M. Note the pre-1.0 API churn and tree-sitter grammar-version coupling (they track tree-sitter 0.27).
2. adopt (as dependency or port) the outline rule model for the repository document: `names` view for the 5-7 folder map, `signatures` for a file, `digest` for deeper dives. The YAML rules are data; vendoring rule files under MIT is allowed with notice, and the vault can add rules for Lua, GDScript, HLSL, GLSL, CMake that ast-grep lacks. Effort M.
3. LSP-kind taxonomy for `SymbolType` so the vault's symbol schema matches what clients already understand. Effort S.
4. ast-grep's metavariable pattern language gives the agent a macro-tolerant way to find, for example, `UPROPERTY($$$) $TYPE $NAME;` only if tree-sitter-cpp parses the macro as part of the tree. I did not test how well tree-sitter-cpp copes with Unreal macros (unverified); none of the repos here has special UE handling (grep over all clones for `UCLASS|GENERATED_BODY|UPROPERTY` found hits only in other workers' clones).

---

## 9. opengrok (oracle/opengrok)

**What it is.** Java, Lucene 9.12.3 (`pom.xml:63`), servlet web app + indexer, 20 years old, version 1.14.19. The oldest "symbol-aware cross-reference" design in the cluster.

**Licence.** CDDL-1.0 for most files (`LICENSE.txt:1-5`), per-file CDDL headers, third-party licence files for grammars (Ada, Eclipse, ECMA, mandoc, Ruby, Scala). File-level copyleft; study only.

**Ingestion.** Source root and projects; history from many VCS; file type chosen by `AnalyzerGuru` (name, extension, magic/leading text; `analysis/AnalyzerGuru.java`), about 45 per-language analyzers under `analysis/` (c, csharp, golang, java, javascript, lua, python, rust, sh, sql, typescript, yaml, terraform, ...; no HLSL/GLSL/GDScript/CMake dirs). `.git` etc. via ignored-names patterns (config; not read in detail).

**Indexing.** Each file becomes one Lucene document with separate fields `full`, `defs`, `refs`, `path`, `hist`, `type`, `scopes`, `numl`, `loc`, `lastrev`, `tags`, `project`, `date` (`search/QueryBuilder.java:54-77`). JFlex lexers per language produce symbol tokens (`refs`) and the xref HTML; universal-ctags run as a persistent subprocess pool (`analysis/Ctags.java:61-76`, `index/IndexDatabase.java:1849-1889`) gives definitions with kinds and line numbers, stored serialised in the document as `Definitions` (`analysis/Definitions.java`), which lets the UI link a symbol to its definition line. Incremental: a per-file uid (path + timestamp) term; on each run the indexer walks the source tree and the uid iterator in sorted order together, and skips files whose `lastModified <= indexed date` (`index/IndexDatabase.java:787-822,880-885`). So updates are file-granular and cheap but require a tree walk.

**Ranking.** Lucene default similarity over the separate fields (no custom boosts found by grep for `boost|BM25|Similarity` in `search`, `index`, `analysis`; unverified beyond that). A user asks explicitly for `def:`, `symbol:`, `path:`, `type:`.

**Exposed to agents.** REST API (`opengrok-web/.../SearchController.java:72-88`): `full`, `def`, `symbol`, `path`, `hist`, `type`, `projects[]`, `maxresults`, `start`, `sort`, `maxHitsPerFile`; no MCP. Output includes line numbers and highlighted line snippets.

**Scale evidence.** None in repo; project is widely used on very large codebases (claimed elsewhere, unverified here).

**Ideas to adopt.**
1. Separate query fields for "all text", "definitions", and "symbol occurrences" as explicit user parameters (def: vs symbol: vs full:). The vault's `search(queries[])` could take a `kind` per query: text, symbol-def, symbol-ref, path. Effort S.
2. Store per-file definition tables (name, kind, line) next to the document so `outline` and `define` are O(1) lookups, not re-parses. Effort S (SQLite table).
3. `maxHitsPerFile` as a first-class parameter to avoid one file dominating a page. Effort S.
4. Timestamp-plus-sorted-merge incremental scan; the vault should use content hash + mtime/size and file watching instead (fast_code_search, bloop).

---

## 10. Extras

### 10.1 cs / codespelunker (boyter/cs)

Go, MIT, active. No index at all: scans files per query, runs scc's lexer to classify bytes as code, comment or string and to compute cyclomatic complexity, ranks with weighted BM25, then extracts snippets. Claims: Linux kernel search+rank in about 3 s on an M1 and about 400 ms on a 9950X3D (claimed, `README.md` FAQ).
- Ranking (`README.md` "How does the ranking work"): code 1.0, string 0.5, comment 0.2 field weights; "complexity gravity" multiplies by cyclomatic complexity (`brain` 2.5, `logic` 1.5, default 1.0, `low` 0.2, `off` 0); noise penalty for data blobs; test-file dampening; profiles `balanced|precise|broad` (precise: short focused source files, penalise long and test files; broad: include tests).
- Structural filters: `--only-code|comments|strings|declarations|usages`. Declarations are detected by line-start heuristics for about 30 languages including C, C++, C#, Lua (`README.md` Structural Filtering); a cheap alternative to tree-sitter for long-tail languages.
- `--dedup` collapses byte-identical matches (monorepo duplicates, vendored copies).
- MCP (`mcp.go:250-362`): two tools. `search` with `query, path (directory to search), path_filter, file, language, gravity, profile, dedup, code_filter, snippet_mode (auto|snippet|lines|grep), line_limit, context_before/after/context`; and `get_file(path, start_line, end_line)` returning JSON with line-numbered content plus language/complexity stats for source files. The tool text is unusually directive ("DEFAULT TO GREP for any query containing a specific known term"; "ALWAYS set context when using grep mode"; "always also set language when using code_filter"). Snippet extraction: pick the densest window of query terms, reward rare terms, exact case, whole words, closeness (`README.md` "How do you get the snippets?", `pkg/snippet/snippet.go`).
- Ideas: (1) code/comment/string field weighting (needs a tokenizer-level classification; tree-sitter can provide it) Effort M. (2) Complexity as a ranking prior to favour implementations over interfaces and headers; compute a cheap proxy (branch-node count from tree-sitter) at ingest. Effort S-M. (3) `dedup` of identical hits across repos (Unreal ships many identical third-party copies). Effort S. (4) Search mode that returns "all matching lines with context" versus "ranked snippets" as an explicit parameter, so agents choose exhaustive grep or ranked discovery. Effort S. (5) Do not adopt: no-index design.

### 10.2 fast_code_search (jburrow/fast_code_search)

Rust, MIT, v-tagged releases, 21k lines under `src/`. Single author, 3 stars, but it implements nearly the vault's code plan on a smaller scale.
- Ingestion: `ignore` crate with `.gitignore`/`.ignore`/`.git/info/exclude` honoured by default (`src/search/file_discovery.rs:35-47`), exclude globs, include-extension whitelist, binary extension list, 10 MB max file size (`file_discovery.rs:46`), file watcher `notify-debouncer-full` (`Cargo.toml:85`, `src/search/watcher.rs`).
- Index: trigrams as Roaring bitmap posting lists (`roaring 0.11`, `src/index/trigram.rs`), content via mmap (`memmap2`), persistent atomic checksummed index with reconciling load, in-memory otherwise (`src/index/persistence.rs`; `README.md` "What you get"). Symbols via tree-sitter tags queries for 12 languages (Rust, Python, JS, TS, Go, C, C++, Java, C#, Ruby, PHP, Bash; `Cargo.toml:42-53`, `src/symbols/`). Resolved import graph (`src/dependencies/`) gives a dependents count per file.
- Ranking (`src/search/ranking.rs`, all weights in one file): symbol definition x3, exact case x2, whole phrase / all terms tier, line start x1.5, `src/`/`lib/` path x1.5, inverse line-length factor, public definition x1.6, test/example/mock/fixture path x0.6, synthetic filename hit 3.0, dependency boost `1 + 0.5 x log10(dependents)`, symbol-search kind tweaks (exact name 2.5, prefix 1.4, impl blocks 0.8, variables 0.9). File-level fast ranking adds `log2(symbol_count)` and `log2(dependents)` caps.
- Agent surface: gRPC streaming `Search` and `Index`, REST, CLI `fcs` (`refs NAME`, `symbols NAME`), a separate experimental semantic server. No MCP.
- Benchmarks (self-published by the project's own CI on a GitHub runner, commit f46e7bf, tokio 1.45 + Django 5.2, 6,237 files, 38.9 MB; claimed, not independent): build 4.3 s, 118 MB resident, text search p50 0.95 ms common word and 1.5 ms identifier, regex 0.7-1.7 ms, symbol search 3.7 ms, reference search 0.7 ms, incremental update of one file 10 ms p50, ripgrep 30-80 ms on the same trees, ranking suite precision@1 = precision@5 = 1.00 on 20 labelled queries (`README.md` Benchmarks). The corpus is tiny relative to game-engine source.
- Ideas: (1) Ranking weights in one module with a labelled ranking-quality suite (queries -> expected first hit) that gates CI; the vault should copy this evaluation discipline, which matches the Tutor project's eval-search rule. Effort M. (2) Import-graph dependents count as a prior. (3) Interleave results by file so one file cannot fill a page. (4) Reconciling load: on restart, compare stored fingerprints against the tree and apply only the differences. Effort M. (5) Regex "literal pre-filter through the trigram index" with size limits for pathological regexes. All port_ideas; do not depend on the crate (immature, single maintainer).

---

## 11. Comparative table

| | zoekt | sourcebot | livegrep | hound | codesearch | bloop | probe | ast-grep | opengrok | cs | fast_code_search |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Language | Go | TS + zoekt | C++/Go | Go | Go | Rust | Rust+Node | Rust | Java | Go | Rust |
| Index | positional trigram shards | zoekt | per-chunk suffix array | file trigram | file trigram | tantivy ngram(1-3) + Qdrant | none | none | Lucene | none | roaring trigram, in-memory/mmap |
| Regex acceleration | yes, trigram pairs | yes | yes, RE2 plan | yes | yes | yes, planner | no (scan) | no (scan) | no (Lucene terms; regex via Lucene) | no | yes |
| Symbols | ctags sections | ctags | optional tags file | none | none | tree-sitter scope graph | tree-sitter at query time | tree-sitter outline | ctags + JFlex | line heuristics | tree-sitter tags (12 langs) |
| Refs/call graph | none | regex word search | none | none | none | in-file graph + name match | optional LSP | none | token-level refs | none | refs mode + import graph |
| Ranking | symbol/filename/word/BM25 | zoekt | none | none | none | BM25 x lang / line len / age | BM25 + coverage | none | Lucene | BM25 x code/string/comment x complexity | symbol, imports, path, tier |
| Incremental | delta shards, tombstones | zoekt (hourly) | rebuild + reload | rebuild | merge delta | per-file blake3 cache | n/a | n/a | uid + mtime merge | n/a | file watcher + reconcile |
| MCP | no | yes (EE, 12+ tools) | no | no | no | no | yes (3 tools) | separate repo, 4 tools | no | yes (2 tools) | no |
| Maturity | high | high (product) | medium | medium | stable | archived | medium | high | high | medium | low |

---

## 12. Cluster summary

### What this cluster converges on (best practices)

1. Trigram candidate generation, regex verification. Every serious engine (zoekt, hound, codesearch, bloop, fast_code_search) reduces a regex to an AND/OR over trigrams, intersects postings, then runs the real regex on candidates. Livegrep uses a suffix array instead. Cursor describes sparse n-grams as the next step (claimed). For the vault, the plan of tantivy BM25 plus a trigram index verified by ripgrep's grep crates is on the proven path; bloop already did the tantivy variant.
2. Index text verbatim, address by real lines. Zoekt, hound (raw/ gzip copy), livegrep (line store) all keep the content in the index so results never depend on the working tree. This matches the vault's `doc_id#L40-L72`.
3. Cheap file triage at ingest, with reasons recorded: NUL byte / invalid UTF-8, size cap (about 0.5 to 2 MiB; 10 MB in fast_code_search), distinct-trigram cap 20,000, line-length cap (1024 to 2000), vendor/test/generated flags, ignore rules. Hound and bloop persist a list of skipped files with reasons.
4. Ranking that makes definitions beat usages: zoekt (symbol span match, kind table by language, filename base match), fast_code_search (3x definition, path and test penalties, import-graph dependents), cs (code vs comment vs string weights, complexity prior). Tests/vendored/generated are demoted, not removed. None of the engines uses PageRank; fast_code_search's dependents count is the closest.
5. Output bounded for the consumer: caps on files, matches, bytes or tokens; explicit exhaustive/truncated flag with a hint; chunked matches with context lines; per-call read limits (sourcebot: 500 lines, 5 KB).
6. Agent tool surfaces are small and text-shaped: grep-like search, read file with offset/limit, list tree, find definition/references, repos list. Sourcebot has 10+ tools, probe 3, cs 2. Tool descriptions are long and directive and are where agent behaviour is tuned.
7. Symbols come from ctags or tree-sitter at ingest. Real cross-file reference resolution is absent everywhere in this cluster: sourcebot's "references" is `\bname\b`, bloop's is a name match across files, fast_code_search's is token refs with imports. Nobody here solves it.
8. Incremental updating by content hash or by git delta, atomic swap of index files; file watching only in the small Rust project.

### Where the cluster fails the target use case (and what the vault must do to beat it)

Target: an agent maps 5-7 folders of game source (Unreal-style C/C++ with heavy macros, C#, Rust, Lua/GDScript, HLSL/GLSL, CMake), finds anything, follows definitions and references across files and repos, and reads exactly what it needs in few tool calls.

| Gap in this cluster | What the vault should do |
|---|---|
| Macro-heavy C++ breaks ctags and tree-sitter-cpp assumptions; no project handled Unreal macros (`UCLASS`, `GENERATED_BODY`, `*_API` export decorations), and none added a macro-aware pass (unverified how tree-sitter-cpp copes) | Add a preprocessing-tolerant symbol layer: tree-sitter queries that treat `IDENT_API` before class names and `UPROPERTY(...)`/`UFUNCTION(...)` wrappers as decorators, and a text-level fallback (cs-style declaration regexes) when the parse has errors. Record `parse_quality` per file and surface it. Test on a real Unreal module and on a Godot/GDScript tree early. |
| Long-tail languages (HLSL, GLSL, GDScript, Lua, CMake) are mostly unsupported or only language-detected (ast-grep has Lua; opengrok has Lua; zoekt/sourcebot only name them via linguist) | Generic, grammar-pluggable `tags` extraction: one tree-sitter tags query per language, data files shipped with the vault, plus a regex fallback table for languages without grammars. Treat language packs as data so new ones cost S effort each. |
| No cross-file/cross-repo reference graph: all references are word matches | Build a symbol table (definitions with kind, scope, file, range) and a reference-candidate table (identifier occurrences from tree-sitter with enclosing definition). Link references to definitions by scoped name resolution first (same file, includes/imports, namespace), then by unique global name, then rank ambiguous candidates and mark them as such (`confidence`). Expose `define`, `citations(direction=in|out)` on this graph and compute dependents counts and an optional centrality prior from it (fast_code_search shows a cheap form). Be honest about confidence; never present a name match as a resolved reference. |
| Maps are weak: sourcebot `list_tree`, probe `symbols`, ast-grep outline are separate tools | Generate the repository document once at ingest: tree (with file categories and sizes), per-directory symbol digests at several zoom levels (ast-grep `names/signatures/digest/expanded` model), module/include dependency edges, entry points, build files (CMake targets). Expose levels and budgets so a map of 6 repos fits in a stated number of tokens. |
| Output budgeting is ad hoc (caps per tool, no token awareness except probe) | Make every tool take `max_tokens` (and give a default), return `truncated`, `next` cursor/addresses, and `exhaustive` plus the stop reason (livegrep) so agents never have to guess. Merge hits into syntactic blocks (probe) and interleave by file (fast_code_search). Support `read(addresses[], expand)` where `expand` is `block`, `symbol`, or `lines±n`, so 1 read call replaces several. |
| Few-tool-call workflows: grep then read then outline then define costs 4+ round trips | Batch parameters (`queries[]`, `patterns[]`, `addresses[]`, already in the plan); let `search` return, per hit, the enclosing symbol, address and a snippet sized to the budget; let `grep` return counts per repo/dir in one call (sourcebot `groupByRepo`) to triage 5-7 folders. |
| Ranking is untested in most projects | Copy fast_code_search's ranking-quality suite approach: a labelled query set over the target kind of corpus (Unreal-like C++, Godot, Rust), precision@1 and @5 gating changes, weights in one module, `explain` output for each score (zoekt `DebugScore`). Keep eval discipline (held-out split) as in the Tutor project. |
| Dense vectors: only bloop (and optional semantic servers) pair lexical with embeddings; fusion is simple RRF k=60 | Keep dense search as a second retriever fused with RRF over symbol-level and chunk-level units, never as the primary path for code identifiers. Chunk by AST units (functions/classes), not by tokens (bloop's 256-token window) so chunk addresses stay real lines. |
| Freshness: zoekt/sourcebot hourly polling and git-only; livegrep/hound full rebuilds | Content-hash plus mtime/size per file with a watcher (`notify`), blake3 keys (bloop), reconcile-on-start (fast_code_search), tombstone-style deletes in tantivy by term, and a "generation" number in `vault_info` so agents can detect staleness. Do not require git (folders on disk are the main input). |
| Windows | Zoekt/livegrep/ctags are Linux-oriented (seccomp ctags, Bazel). Stay pure Rust: tantivy, `regex-syntax` + `grep-searcher/grep-regex`, tree-sitter, `ignore`, `notify`, `memmap2`, `roaring`, `blake3`. Avoid shelling out to ctags. |

### Prioritised adoption list for the vault (all licence-safe as ideas; code reuse noted)

1. adopt_dependency: `ignore`, `regex-syntax`, `grep-*` crates (ripgrep family, MIT/Unlicense), `tree-sitter` + grammars, `hyperpolyglot` (check licence), `notify`, `blake3`, `roaring`, `tantivy`. (S each)
2. adopt_dependency (evaluate): `ast-grep-core`/`ast-grep-language` (MIT) as the structural-query engine and as a source of outline rules and YAML schema; fall back to own tags queries for languages it lacks. (M)
3. port_ideas: regex to trigram query planner (codesearch algebra, bloop `planner.rs` structure), with tantivy 3-gram field (positions off) and a scan-fallback threshold (livegrep) and a cheap two-trigram positional verification (zoekt) as an optimisation if needed. (M)
4. port_ideas: ingest triage (zoekt skip reasons and DocChecker, hound excluded-files list, bloop stub documents), file-category flags, per-document persisted skip reason. (S)
5. port_ideas: ranking layers (zoekt symbol/filename/kind tables as defaults; fast_code_search dependents and path priors; cs code/comment/string weights and dedup), final score = relevance then tie-breakers, with `explain`. (M)
6. port_ideas: output contract (stop reason, truncated, next, token budget, group-by-repo counts, per-file caps, session dedup). (S-M)
7. port_ideas: outline views from ast-grep's model for the repository document. (M)
8. build new (L): scoped definition/reference resolution across files and repos with confidence labels, plus macro-tolerant C/C++ symbol extraction, and HLSL/GLSL/GDScript/Lua/CMake language packs. This is the differentiator; nothing in this cluster provides it.
9. reference_only: sourcebot (source-available/EE; read tool descriptions for style only), opengrok (CDDL), livegrep (suffix arrays), probe (inconsistent licence), hound.
10. reject: any dependency on universal-ctags, Qdrant, Postgres/Redis, Node orchestration, or a server-side LLM agent inside the vault (sourcebot's `ask_codebase`, probe's agent): the vault should serve evidence, and the calling agent (Opus, Codex, OpenCode) does the reasoning.

### Open questions and ambiguities recorded for the lead

- Trigram vs sparse n-grams vs tantivy ngram(3): I did not measure index sizes. Zoekt reports about 3.5x corpus (positional, content included); livegrep 3-5x; bloop ngram(1,3) with positions is likely larger (not measured). A spike on a real 5-7 folder corpus is needed before choosing between tantivy-ngram and a custom mmap postings file.
- tree-sitter-cpp behaviour on Unreal macros and on `.usf/.ush` HLSL variants was not tested (no code was run per the brief). Other workers' clones (`ts-cpp`, `ts-hlsl`, `ts-glsl`, `ts-gdscript`, `ts-lua`, `ts-cmake`, `unreal-clangd`, `serena`) exist under `refs/code/` and are the right place to answer this.
- Probe's licence is declared three ways (Apache-2.0, MIT, ISC); sourcebot's MCP lives under `ee/`.
- bloop's embedding model name is not visible without the LFS blobs.
- All performance numbers above are project-reported or from project CI; I found no independent benchmark in these repositories. Zoekt's FAQ numbers and fast_code_search's benchmarks are single-source.
- The clone of bloop is partial on Windows (checkout error at clone time); only `server/` and a few files were restored. Not needed for the conclusions above.
