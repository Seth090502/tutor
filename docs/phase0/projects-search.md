# Phase 0 - Rust search and storage projects: status and pin recommendations

Date of data pull: 2026-09-30 (all "today" figures below are relative to this date). Author: worker for the lead (Opus 5.5). Scope: status audit only; no design decisions, nothing installed or compiled.

Evidence tags used on numbers and claims:
- **[M]** measured today by querying crates.io / GitHub APIs or by reading the published crate source (downloaded `.crate` tarballs from `https://static.crates.io/crates/<name>/<name>-<ver>.crate`, extracted read-only into a scratch folder).
- **[D]** documented by the project itself (README, docs, release notes, code comments, issue text). A [D] claim is only as true as the project's text; I did not run it.
- **[E]** my estimate or inference.

Not done (limits of this audit): no crate was compiled, no Windows build was executed here, no benchmark was run. "Builds on Windows" below means "the project's CI config or source shows a Windows build path", not "I built it".

---------------------------------------------------------------------------

## 1. Summary table (for pinning)

| Project | Latest on crates.io [M] | Date [M] | Default-branch last commit [M] | Licence [M] | Status | Pin recommendation |
|---|---|---|---|---|---|---|
| tantivy | 0.26.2 | 2026-09-08 | 2026-09-30 (main) | MIT | Active; main is already 0.27.0-dev with a breaking change | `tantivy = "=0.26.2"` (MSRV 1.86). Do not track main. |
| tantivy-tokenizer-api | 0.7.0 | 2026-03-31 | (in tantivy repo) | MIT | Active, tracks tantivy | transitive via tantivy 0.26.x |
| tantivy-jieba | 0.20.0 | 2026-05-23 | 2026-09-29 (master) | MIT | Active; HEAD already bumps jieba-rs to 0.11, no release yet | `=0.20.0` now (jieba-rs ^0.10); or git rev `7eb90ba6` if jieba-rs 0.11 is wanted before 0.21 is released |
| cang-jie | 0.21.0 | 2026-09-23 | 2026-09-29 (master) | MIT | Revived: 3 releases since 2026-05 after a 3-year gap (0.18.0 was 2023-11-04) | `=0.21.0` is the alternative to tantivy-jieba (tantivy ^0.26.1, jieba-rs ^0.11) |
| jieba-rs | 0.11.0 | 2026-09-16 | 2026-09-25 (main) | MIT | Active (7 releases in 5 months) | `=0.11.0` if we write our own thin adapter; otherwise whatever the adapter pins |
| lindera | 6.2.0 | 2026-09-26 | 2026-09-30 | MIT | Very active (30+ releases in 12 months), frequent majors | do not pin directly; follow lindera-tantivy |
| lindera-tantivy | 5.0.1 | 2026-08-07 | 2026-08-07 (main) | MIT | Maintained but lags lindera by one major (needs lindera ^5.0.1 while lindera is 6.2.0) | `=5.0.1` with lindera 5.x, only if Japanese/Korean or the CC-CEDICT Chinese dictionary is needed |
| lancedb | 0.39.0 (stable); 0.40.0-beta.12 on GitHub | 2026-09-17 | 2026-09-30 (main) | Apache-2.0 | Very active; breaking changes in most releases; MSRV 1.91 | `=0.39.0`. It hard-pins `lance =12.0.0`, so never pin lance separately. |
| lance | 12.0.0 (stable); 13.0.0-rc.1 on GitHub | 2026-09-17 | 2026-09-30 | Apache-2.0 | Very active, new major roughly every 2-3 weeks | `=12.0.0` only via lancedb 0.39.0. Re-evaluate after lance 13.0.0 stable. |
| usearch (Rust) | 2.26.2 | 2026-08-31 | 2026-08-31 (main) | Apache-2.0 | Active (13 releases in 12 months) | `=2.26.2` |
| hnsw_rs | 0.3.4 | 2026-02-28 | 2026-09-26 (master) | MIT/Apache-2.0 (GitHub shows NOASSERTION) | Maintained by one author; releases every few months | candidate only: `=0.3.4` |
| hannoy | 0.2.0 | 2026-08-14 | 2026-08-14 | MIT | Active, new, LMDB-backed HNSW derived from arroy | candidate only: `=0.2.0` |
| arroy | 0.8.0 | 2026-08-12 | 2025-10-16 on main (0.8.0 was cut from branch `release-v0.8`/main merge dated 2026-08-12) | MIT | Maintained by Meilisearch; random-projection trees (Annoy-like), not HNSW | candidate only: `=0.8.0` |
| diskann (Microsoft, Rust) | 0.59.0 | 2026-09-11 | 2026-10-01 UTC | MIT | Very active, pre-1.0, ~1 release/2 weeks | candidate only; too churny to pin for production |
| diskann-rs (third party) | 0.5.0 | 2026-02-09 | 2026-02-09 | MIT | Small single-author project, 44 stars, quiet for 7 months | not recommended |
| instant-distance | 0.6.1 | 2023-06-26 | 2026-07-21 (dependabot only) | MIT OR Apache-2.0 | **GitHub repo is archived** | do not use |
| hora | 0.1.1 | 2021-08-07 | 2021-10-23 | Apache-2.0 | **Abandoned** | do not use |
| faiss (Enet4 bindings) | 0.13.0 | 2025-11-15 | 2025-11-15 | MIT/Apache-2.0 | Low-activity bindings; need a separately built FAISS C API (CMake, BLAS) | not recommended on Windows |
| grep-searcher | 0.1.17 | 2026-07-15 | 2026-08-04 (ripgrep master) | Unlicense OR MIT | Maintained (BurntSushi); slow cadence | `=0.1.17` |
| grep-regex | 0.1.14 | 2025-10-16 | 2026-08-04 (ripgrep master) | Unlicense OR MIT | Maintained; slow cadence | `=0.1.14` |

Flags the lead asked for:
- **Abandoned:** hora (last code commit 2021-10-23 [M]); instant-distance (archived [M], last release 2023-06-26); tantivy-ik (last release 2024-01-17, tantivy ^0.21 [M], LGPL-2.1-only [M]).
- **Stale against tantivy 0.26:** vaporetto_tantivy 0.24.0 (tantivy ^0.24, 2025-06-14 [M]); tantivy-ik.
- **Windows-broken (confirmed):** none found. **Windows risks:** tantivy has no Windows CI job and two open `PermissionDenied` issues (section 2.1); lance's optional `fp16kernels` are skipped on Windows (section 4.5); lance and lancedb need `protoc` on Windows (section 4.6); faiss bindings have no Windows evidence (section 5.6).
- **Superseded / replacement candidates (recommendation only):** arroy and Annoy-style indexes have a graph-based sibling in hannoy; the Python-era FAISS route is superseded for our purposes by usearch/lance on Windows; instant-distance and hora have hnsw_rs, usearch, hannoy as maintained alternatives.

---------------------------------------------------------------------------

## 2. tantivy

Sources: https://crates.io/api/v1/crates/tantivy ; https://github.com/quickwit-oss/tantivy ; crate source `tantivy-0.26.2` (static.crates.io).

### 2.1 Status, cadence, issues
- Latest 0.26.2, published 2026-09-08 [M]. Prior: 0.26.1 (2026-04-21), 0.26.0 (2026-03-31), 0.25.0 (2025-08-20) [M]. 0.24.0, 0.24.1 and 0.22.0 are yanked [M]. Cadence [E]: a minor every ~4-7 months, patches every 1-5 months. 3 non-yanked releases in the last 12 months [M].
- GitHub "Releases" page lags crates.io: latest GitHub release object is 0.26.1 dated 2026-05-10 [M]; crates.io is the source of truth for versions.
- Default branch `main`, last commit 2026-09-30T17:12Z ("Abstracting columnar from aggregation (#3112)") [M]. Repo not archived, 16,162 stars, 462 open issues+PRs [M].
- `CHANGELOG.md` on main already opens a "Tantivy 0.27.0" section with a breaking change: `.set_fast(..)` now takes a `&str` [M] (raw.githubusercontent.com/quickwit-oss/tantivy/main/CHANGELOG.md). So 0.27 will not be a drop-in bump.
- 0.26.2 bug fixes are aggregation-related only (term-aggregation overflow, nested aggregation flush, buffered-union seek) [M, CHANGELOG].
- Licence: MIT; `LICENSE` file begins "Copyright (c) 2018 by the project authors, as listed in the AUTHORS file." [M]. MSRV 1.86 [M].
- Dependencies relevant to Windows: `winapi ^0.3.9` under `[target."cfg(windows)".dependencies]`; optional `fs4 ^0.13.1`, `memmap2 ^0.9.0` under the `mmap` feature (on by default) [M, tantivy-0.26.2/Cargo.toml lines 44-72, 275, 454].
- CI: none of the four workflow files in `.github/workflows` (coverage, long_running, scorecard, test) contains the string "windows" [M]. So Windows is not exercised by upstream CI [E: absence of evidence, I did not read every matrix expansion].

Windows-relevant open issues [M, search of open issues containing "windows"]:
- #2847 (opened 2026-03-04, last activity 2026-07-28): `ManagedDirectory` commit fails with `PermissionDenied` (os error 5) on Windows; claimed race between the merge thread and the next commit on `.managed.json` (tantivy 0.25.0, Windows 10/11 NTFS). A commenter on 2026-07-28 could not reproduce in 14 runs (280 commits) against main and argued the lock is already held across `save_managed_paths` (https://github.com/quickwit-oss/tantivy/issues/2847). Status: unreproduced, open.
- #2647 (2025-06-12): `PermissionDenied` when committing tens of thousands of documents from a multi-threaded writer, on tantivy 0.24.1; 0 comments, open.
- #2272 (2023-11-30): `test_managed_directory_gc_while_mmapped` fails on Windows; open.
- Related platform behaviour in source: on Windows, deleting an mmapped file is expected to fail, so GC logs nothing and retries later (`managed_directory.rs:170-175, 402-403`); `MmapDirectory::open` falls back when `canonicalize` returns error code 1 on virtual or network drives (`mmap_directory/mod.rs:253-261`); `open_with_madvice` is `#[cfg(unix)]` only (`mod.rs:222`); the lock doc comment says "On Windows the semantics are again different" (`mod.rs:164`) [M].
- #3100 (2026-09-13): FileWatcher leaks one OS thread per `MmapDirectory` until dropped (reported on 0.22.1, macOS) [M]. Relevant to us if many indexes are opened and evicted in one long-lived `libraryd` process; not Windows-specific.
- #2974 (2026-06-24): `AutomatonWeight`-based queries (`RegexQuery`, `FuzzyTermQuery`, `RegexPhraseQuery`, `TermSetQuery`) materialise every match into a bitset before collection, so `TopDocs::with_limit(N)` is O(matches) [M]. Relevant for broad regexes.

### 2.2 Features we rely on, in 0.26.2 [M, from tantivy-0.26.2 source]
| Feature | Present in 0.26.2? | Evidence |
|---|---|---|
| Phrase queries / positions | Yes. `PhraseQuery`, `PhrasePrefixQuery`; positions stored with `IndexRecordOption::WithFreqsAndPositions` | `src/query/phrase_query/phrase_query.rs:23`, `phrase_prefix_query.rs:25`; used in README examples of tantivy-jieba/cang-jie/lindera-tantivy |
| Custom tokenizers | Yes. `Tokenizer` trait lives in `tantivy-tokenizer-api 0.7.0`; register with `index.tokenizers().register(name, tok)` | tantivy-0.26.2 dep list; tantivy-jieba README |
| N-gram / trigram field | Yes. `NgramTokenizer::new(min, max, prefix_only)`; works on characters (the doc example uses non-ASCII); **every token has position 0**, so adjacency cannot be verified from positions | `src/tokenizer/ngram_tokenizer.rs:4-30, 96-117` |
| Regex query | Yes. `RegexQuery` uses `tantivy_fst::Regex` and matches **terms in the term dictionary**, not text spans; regex syntax in the query-parser grammar added in 0.26 (#2677, #2818); `RegexPhraseQuery` also exists (multi-term regex gaps still a feature request, #2712) | `src/query/regex_query.rs:1-87`, `regex_phrase_query.rs:23`, CHANGELOG 0.26 |
| Merge policy | Yes. `MergePolicy` trait, `LogMergePolicy` (defaults: `min_num_segments_in_merge` 8, `min_layer_size` 10,000 docs, `level_log_size` 0.75), `NoMergePolicy`, `IndexWriter::set_merge_policy` | `src/indexer/log_merge_policy.rs:8-10, 34-76`, `index_writer.rs:470` |
| Warming | Yes. `Warmer` trait with `warm(&Searcher)` and `garbage_collect`, registered through `IndexReaderBuilder::warmers`; also `ReloadPolicy` | `src/reader/warming.rs:11-20`, `reader/mod.rs:21, 128` |
| mmap directory on Windows | Yes, with caveats above (`MmapDirectory`, `mmap` feature on by default, `fs4` file locks) | `directory/mmap_directory/mod.rs` |

### 2.3 Our trigram-prefilter idea against these facts [E, reasoning from the facts above]
- A trigram field is feasible with `NgramTokenizer(3,3,false)`, but `RegexQuery` on that field would match against trigram terms, not text. Regex prefiltering therefore needs our own step: extract required literals from the regex (for example with `regex-syntax`, which `grep-regex` depends on), turn each literal into its trigrams, run a `BooleanQuery` of `TermQuery` (AND), then verify candidates against the canonical text with `grep-regex`/`grep-searcher`. Because ngram positions are all 0, the verification pass is mandatory.
- Literals shorter than 3 characters give no trigram. For Chinese, many words are 2 characters, so a trigram field alone leaves bigram-length literals unfiltered [E]. Lance documents the same fallback for its own NGram index: "literal runs shorter than three characters... fall back to rechecking every row" (https://raw.githubusercontent.com/lance-format/lance/v12.0.0/docs/src/format/index/scalar/ngram.md).
- Lance now ships its own prefilter for exactly this job (section 4.4), which makes a hand-built trigram field in tantivy one of two possible routes. I am not recommending either.

### 2.4 Pin recommendation
`tantivy = "=0.26.2"`. Rationale: the three adapters in section 3 and `tantivy-tokenizer-api 0.7.0` all target the 0.26 line; main is moving to 0.27 with a breaking change. Upgrade to 0.27 only after a release exists and the Chinese adapter has a matching release. Test Windows commit/merge behaviour with the real writer settings before relying on it (issues #2847, #2647 are open and unreproduced upstream).

---------------------------------------------------------------------------

## 3. Chinese tokenizers for tantivy

Compatibility is driven by which tantivy-side crate each adapter depends on [M, crates.io dependency API for the latest version of each]:

| Adapter | Latest | Depends on | Works with tantivy 0.26.2? |
|---|---|---|---|
| tantivy-jieba 0.20.0 | 2026-05-23 | `jieba-rs ^0.10.0`, `tantivy-tokenizer-api ^0.7.0`, `lazy_static` (no direct tantivy dependency) | Yes (tokenizer-api 0.7.0 is what tantivy 0.26.x uses) |
| cang-jie 0.21.0 | 2026-09-23 | `jieba-rs ^0.11.0`, `tantivy ^0.26.1`, `log` | Yes; coupled to tantivy 0.26.x directly, so every tantivy minor needs a new cang-jie |
| lindera-tantivy 5.0.1 | 2026-08-07 | `lindera ^5.0.1`, `lindera-analysis ^5.0.1`, `tantivy ^0.26.1`, `tantivy-tokenizer-api ^0.7.0` | Yes, but pins lindera 5 while lindera is at 6.2.0 |
| vaporetto_tantivy 0.24.0 | 2025-06-14 | `tantivy ^0.24` | No (two minors behind) |
| tantivy-ik 0.7.0 | 2024-01-17 | `tantivy ^0.21.0`, `tantivy-tokenizer-api ^0.2.0`; licence LGPL-2.1-only | No |
| tantivy-analysis-contrib 0.13.1 | 2026-09-06 | `tantivy-tokenizer-api ^0.7`; features include `icu`, `phonetic`, `embedded`; MIT OR Apache-2.0 | Yes (I did not inspect whether its ICU feature needs a system ICU library on Windows) |

### 3.1 tantivy-jieba (https://github.com/jiegec/tantivy-jieba)
- 0.20.0 on 2026-05-23; 21 versions total; repo master last commit 2026-09-29 (merged PR #27 "bump jieba-rs version"; PR text, by a contributor, claims ~36% less time and 1.56x throughput on its benchmark with jieba-rs 0.11 - author claim, not measured by me) [M]. 0 open issues, 0 open PRs, 68 stars, no GitHub releases but tags up to v0.20.0 [M]. MIT, `LICENSE` text "Copyright 2019-2020 Jiajie Chen" [M].
- API facts from `tantivy-jieba-0.20.0/src/lib.rs` [M]: `JiebaTokenizer::new()` defaults to **search mode = true** (jieba `cut_for_search`, which emits overlapping sub-words) and **position = character offset, not sequential ordinal** unless `with_ordinal_position_mode(true)` (lines 56-89, 175-179). For `PhraseQuery` over Chinese, ordinal positions are what phrase adjacency needs [E]; this is a setting to test, not a default to accept. The README snippet `JiebaTokenizer {}` is stale (struct fields are private) [M].
- Windows: pure Rust plus jieba-rs's embedded dictionary; no native build step [M, dependency list].

### 3.2 cang-jie (https://github.com/DCjanus/cang-jie)
- Releases v0.19.0 (2026-05-08), v0.20.0 (2026-07-07), v0.21.0 (2026-09-23) after v0.18.0 (2023-11-04) [M]. 0 open issues, 84 stars, MIT, Rust 2024 and Rust >= 1.88 [M].
- `TokenizerOption::{Default{hmm}, ForSearch{hmm}, Unicode}` (`cang-jie-0.21.0/src/options.rs`, `tokenizer.rs:38-50`); the `default-dict` feature re-exports `jieba_rs::Jieba::new()` [M]. Yanked history: 0.10.0, 0.11.0, 0.12.0 [M]. I did not check how it assigns token positions.
- Because it depends on tantivy itself, a tantivy upgrade is gated on its maintainer; it did track 0.26.1 promptly (0.19.0 already `tantivy ^0.26.1`).

### 3.3 jieba-rs (https://github.com/messense/jieba-rs)
- 0.11.0 (2026-09-16); 0.10.x series 2026-05-22 to 2026-09-15; 7 releases in the last 12 months [M]. 987 stars, 7 open issues, MIT [M]. Active performance work (v0.11.0 notes: "optimize tagging, dictionary loading, HMM lookups and per-call allocations"). v0.10.0 added custom-HMM loading and `byte_start`/`byte_end` on tokens [M, release notes].
- Features: `default-dict` (embeds the dictionary via `include-flate`), `textrank`, `tfidf` [M]. crate tarball is 4.9 MB (dictionary) [M].

### 3.4 lindera and lindera-tantivy (https://github.com/lindera/lindera, https://github.com/lindera/lindera-tantivy)
- lindera 6.2.0 (2026-09-26); majors 3 -> 4 (2026-06-22) -> 5 (2026-08-05) -> 6 (2026-08-25) in under 3 months [M]. MIT. Chinese support is via `embed-cc-cedict` (CC-CEDICT) and `embed-jieba`/`lindera-jieba` features; also embed-cjk bundles [M, lindera-6.2.0/Cargo.toml lines 46-72].
- **Build-time network use:** `lindera-cc-cedict-6.2.0/build.rs` downloads `https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz` when `embed-cc-cedict` is enabled (`fetch_params.download_urls`) [M]. Builds therefore need internet or a pre-seeded cache.
- lindera-tantivy 5.0.1 (2026-08-07), last commit 2026-08-07 "Upgrade lindera to v5.0.1"; 1 open issue (#87 TokenFilter support, 2025-02-22); it has now lagged lindera main by one major for 54 days [M]. Features `embed-cc-cedict`, `embed-ipadic`, `embed-ipadic-neologd`, `embed-ko-dic`, `embed-unidic` [M].
- Position/phrase semantics of its tokens not inspected.

### 3.5 Other options seen [M unless noted]
- `lance-tokenizer 12.0.0` (new crate inside the lance repo) has `jieba-rs` and `lindera` features (`tokenizer-jieba`, `tokenizer-lindera`). Lance 12.0.0 no longer depends on tantivy at all [M, grep of lance, lance-index, lance-tokenizer, lancedb Cargo.toml for "tantivy": no match]. Relevant only if Lance FTS replaces tantivy.
- `seekstorm 3.3.13` (Apache-2.0, 2026-09-12): a separate full engine with a `zh` feature; not a tantivy plugin. Not evaluated further.
- Own adapter: `tantivy-tokenizer-api 0.7.0` is a 3 KB crate [M]; a thin adapter directly on `jieba-rs 0.11.0` is feasible [E] and removes the adapter-release lag. I make no recommendation here.

### 3.6 Pin recommendation for the Chinese path
tantivy-jieba `=0.20.0` is the lowest-coupling choice (only needs tokenizer-api 0.7); cang-jie `=0.21.0` is the choice if jieba-rs 0.11 is needed today from a crates.io release. Either way keep `search_mode` and position mode under our control and test PhraseQuery on Chinese. lindera-tantivy only if Japanese/Korean are added.

---------------------------------------------------------------------------

## 4. lancedb and lance

Sources: crates.io API; https://github.com/lancedb/lancedb ; https://github.com/lance-format/lance ; crate source `lancedb-0.39.0`, `lancedb-0.37.1`, `lance-12.0.0`, `lance-index-12.0.0`, `lance-linalg-12.0.0`, `lance-tokenizer-12.0.0`; docs at tag v12.0.0.

### 4.1 Versions and cadence
- lancedb: 0.39.0 (2026-09-17), 0.38.0 (2026-08-31), 0.37.1 (2026-08-10), 0.33.0 (2026-07-28) on crates.io [M]; GitHub shows `v0.40.0-beta.12` (prerelease, 2026-09-30), 12 betas since 2026-09-17 [M]. 17 crates.io releases in 12 months [M]. Stars 11,566; open issues+PRs 688; Apache-2.0 [M]. MSRV 1.91.0 [M].
- lance: 12.0.0 (2026-09-17), 11.0.0 (2026-08-30), 10.0.0 (2026-08-07), 9.0.0 (2026-07-24), 8.0.0 (2026-07-01), 7.0.0 (2026-05-28) [M]; GitHub has `v13.0.0-rc.1` (2026-09-30) and `v13.1.0-beta.1` [M]. [E] a new major roughly every 2-4 weeks, and the release notes list "Breaking Changes" in each of v10, v11, v12. Stars 7,127; open issues+PRs 1,247; Apache-2.0; MSRV 1.91.0 [M].
- **Lock-step:** `lancedb 0.39.0` depends on `lance =12.0.0` and eleven sibling `lance-*` crates all `=12.0.0`, `arrow ^58.0.0`, `datafusion ^54.0.0` [M]. Mapping: lancedb 0.33.0 -> lance 9.0.0; 0.37.1 -> lance 10.0.0; 0.38.0 -> lance 11.0.0; 0.39.0 -> lance 12.0.0 [M]. So the pin is "lancedb 0.39.0" and lance comes with it.

### 4.2 What changed since lancedb 0.37.1 (the version in the design) [M, release notes + crate diff]
Relevant to a Rust embedded user:
- lancedb 0.37.1 -> 0.38.0 -> 0.39.0 moves lance 10 -> 11 -> 12. arrow (58) and datafusion (54) did not change across these three [M].
- Breaking in lancedb 0.38.0 (release notes): table existence is now manifest-authoritative; `branch merge` renamed `cherry_pick`; listing pagination changes; Python now requires pydantic v2; Node requires >= 22 [M]. Breaking in 0.39.0: job API rename (`get_job`/`job_history` -> `describe_job`/`query_job_events`), materialised-view/function-column changes, namespace-source recording [M]. Most of this is remote/namespace/Python/Node surface; I did not find Rust-embedded API removals in these notes other than those, but I did not diff the Rust public API [E].
- Lance 11.0.0: HNSW construction aligned with the reference implementation; "correct cosine RabitQ query factor"; "fall back to AVX2 f16 kernels on AVX-512 CPUs"; "preserve Windows UNC share roots" (relevant to Stage 2 if the vault is reached over UNC); "recheck contains() with sub-trigram needle instead of dropping rows" (NGram bug fix); FM-Index docs added; many FTS planner/scorer changes [M].
- Lance 12.0.0: "default ivf_rq to 5-bit quantization"; "widen the RaBitQ FastScan accumulator above 1024 dims"; "use substring-safe ngram defaults"; "declare FM index row address results"; "keep every HNSW node reachable after a parallel build"; "perf(index): accelerate RQ scale search"; "share IVF partition scans across batch vector queries" [M].
- Lance 10.0.0 (already in lancedb 0.37.1): "support segmented ngram index", "use ACORN-1 traversal for prefiltered HNSW search", segmented bloom filter and RTree indexes [M].
- lancedb 0.37.1 itself: FTS block-size and custom stop-word options [M].
- Regression risk on `main`, not on releases: lance issue #9506 (2026-09-23) "main does not compile" because of a semantic merge conflict (MinHash scalar index); affects main at 6f843df81, not the tagged release [M].

### 4.3 Vector index features (v12.0.0 / lancedb 0.39.0) [M unless tagged]
- **IVF_PQ:** present; lancedb `IvfPqIndexBuilder`, `num_bits` default 8 and `num_sub_vectors` (`create_index.rs:297-316`); code special-cases `num_bits == 4` and 8 [M, `lance-index-12.0.0/src/vector/pq/storage.rs`], so 4 and 8 are supported [E].
- **IVF_RQ (RaBitQ):** present. lancedb `Index::IvfRq(IvfRqIndexBuilder)` with `num_bits(u32)`; lance `RQBuildParams`; accepted `num_bits` range is **1..=9** (`validate_rq_num_bits`, `lance-index-12.0.0/src/vector/bq.rs:126-132`; docs say "in the range 1..=9"); multi-bit codes store 1-bit sign code plus `num_bits-1` extra bits (docs `vector/index.md` lines 193-290) [M].
- **Default mismatch to watch:** lance's `RabitBuildParams::default().num_bits` is **5** (`bq/builder.rs:36, 968`), while the lancedb 0.39.0 Rust wrapper uses `index.num_bits.unwrap_or(1)` (`lancedb-0.39.0/src/table/create_index.rs:326`; same in 0.37.1 line 303) [M]. Always set `num_bits` explicitly. IVF_RQ requires float vectors (lance issue #9422 text; hamming is rejected) [D].
- **float16 vectors:** accepted. `supported_vector_data_type` allows any Arrow floating type or UInt8 inside a `FixedSizeList` (`lancedb-0.39.0/src/utils/mod.rs:454-461`; Arrow `is_floating()` includes Float16) [M, partly inferred]. Open bug: lance #8607 (2026-08-18): IVF index build over a float16 column loops forever when `vectors per cluster x value magnitude` exceeds 65504; normalised embeddings unaffected [D].
- **Other index types:** IVF_FLAT, IVF_SQ, IVF_HNSW_FLAT/SQ/PQ [M, `lance-index-12.0.0/src/lib.rs:117-139`].
- **Query-side knobs in lancedb 0.39.0:** `nprobes`, `minimum_nprobes`, `maximum_nprobes`, `ef`, `refine_factor`, `distance_range`, `bypass_vector_index`, `fast_search`, `full_text_search`, `only_if(filter)`, `postfilter()` [M, `src/query.rs`].
- Recent open IVF_RQ bugs (all open at pull time): #9422 hamming panics, #8908 u16 distance-table overflow at production dimensions (v12.0.0 notes list "widen the RaBitQ FastScan accumulator above 1024 dims", likely related, I did not verify they close it), #9415, #9395, #9028 [M].

### 4.4 Scalar indexes and filtering [M]
- Types in `IndexType`: BTree, Bitmap, LabelList, Inverted (FTS), NGram, ZoneMap, BloomFilter, RTree, Fm (FM-Index), plus FragmentReuse and MemWal system indexes (`lance-index-12.0.0/src/lib.rs:117-139`).
- **NGram index** (trigrams): accelerates `contains`, `regexp_like`/`regexp_match` and infix `LIKE`; results are "AtMost" (inexact) then rechecked; patterns with no derivable trigram, case-insensitive matches and runs < 3 characters fall back to rechecking every row (docs `scalar/ngram.md` lines 26-38).
- **FM-Index**: substring, prefix and regex search on raw bytes; case-folding left to the caller (docs `scalar/fmindex.md`).
- Filtering: lancedb `only_if("sql predicate")` is pre-filter by default; `postfilter()` switches (query.rs:426, 526). Lance 10 added ACORN-1 traversal for prefiltered HNSW search [M].
- FTS is a Lance-native inverted index (`fts.md`, 19 KB), not tantivy (section 3.5).

### 4.5 Windows [M unless tagged]
- Both repos run native Windows CI. lancedb `rust.yml` job `windows` on `windows-2022` (`x86_64-pc-windows-msvc`) and `windows-11-arm`, running `cargo build --features aws,remote --tests` and tests; lance `rust.yml` job `windows-build` on `windows-latest-4x` running `cargo nextest run` [M, workflow files at tags v0.39.0 and v12.0.0]. lancedb publishes `win32-x64-msvc` and `win32-arm64-msvc` npm packages and Windows wheels (tree listing) [M].
- `lance-linalg` build script prints "fp16 kernels are not supported on Windows. Skipping compilation of kernels." for the optional `fp16kernels` feature (`lance-linalg-12.0.0/build.rs:37`) [M]. On Windows, f16 distance falls back to Rust code paths [E]; the default build still works (CI tests run without that feature on Windows).
- Windows-flavoured open issues: lancedb #2051 "DLL load failed while importing _lancedb" (Python, 2025-01-22, updated 2026-08-06), #3951 no macOS x86_64 wheels; lance #9513 wheel build failure incl. Windows (2026-09-23), #9055 Windows CI flake in Python tests, #8948 `Session::new` panics when a temp dir cannot be created [M]. None is a Rust-crate Windows break that I could identify.
- Lance 11 fixed "preserve Windows UNC share roots" [M].

### 4.6 protoc [M]
- Default build requires `protoc` on PATH or `PROTOC` set: lance `CONTRIBUTING.md` line 23 says protoc "version 3.20 or higher"; CI installs v21.12 on Windows (`protoc-21.12-win64.zip` for lance, `choco install protoc` for lancedb) [M].
- lance 12.0.0 has an optional `protoc` cargo feature that builds protoc from source with `protobuf-src` (`lance-12.0.0/Cargo.toml:83-88`, `build.rs:9-12`; `lance-index/build.rs:13-16`) [M]. I did not verify whether `protobuf-src` builds on Windows MSVC.
- Issue #3073 "Remove protoc requirement" is still open (opened 2024-10-31, 10 comments, last update 2025-10-23) [M].
- Other Windows toolchain needs seen in CI: `VCPKG_ROOT` is set in the lancedb Windows job (purpose not confirmed by me; likely OpenSSL for the `remote` feature) [E].

### 4.7 Pin recommendation
`lancedb = "=0.39.0"` (brings lance 12.0.0, arrow 58, datafusion 54, MSRV 1.91). Pre-release 13.0.0-rc.1 / 0.40.0-beta.x exist; do not pin pre-releases. Plan an explicit upgrade checkpoint after lance 13.0.0 stable, because every major carries breaking changes and fixes in RaBitQ, ngram, HNSW and FTS. Set `num_bits` explicitly for IVF_RQ.

---------------------------------------------------------------------------

## 5. Vector index crates

### 5.1 usearch (https://github.com/unum-cloud/USearch; Rust bindings in `rust/`)
- crates.io usearch 2.26.2 (2026-08-31); 205 versions; 13 in the last 12 months; Apache-2.0 [M]. The repo `main` last commit is the release commit 2026-08-31 [M]. 4,325 stars; 97 open issues+PRs [M]. Rust releases follow the whole project (v2.26.0 2026-07-10, v2.26.1 2026-08-22, v2.26.2 2026-08-31) [M].
- Default feature `numkong` (a separate crate `numkong >=7.5.0,<8` that compiles SIMD kernels; `simsimd` is now an alias) and optional `openmp` [M, Cargo.toml lines 35-39]. Uses `cxx`/`cxx-build` and a C++17 compile [M].
- **Windows build:** `build.rs` has an explicit `target_os == "windows"` branch with MSVC flags (`/std:c++17 /O2 /fp:fast /EHsc ...`) (`build.rs:70-80`); v2.26.0 notes "Build JavaScript Windows x64 prebuilds"; v2.26.1 notes "Respect Cargo-selected Windows CRT #778" [M]. Windows memory mapping implemented with `CreateFileMapping`/`MapViewOfFile` (`include/usearch/index.hpp:2067-2073`) [M]. Not compiled by me. A C++ compiler is needed (cxx) [E]. I did not verify that the `numkong` crate builds with MSVC.
- **Quantization / scalar kinds:** `ScalarKind` includes F64, F32, BF16, F16, E5M2, E4M3, E3M2, E2M3, I8, U8, B1 (1-bit, packed 8 per byte) (`rust/lib.rs:297-323`); Rust `VectorType` is implemented for `f32`, `i8`, `u8`, `f64`, `f16`, `b1x8` (`lib.rs:857-1212`) [M]. So f16, i8 and b1 are available. No RaBitQ.
- **mmap view:** `Index::view(path)`, `view_from_buffer`, plus `save`/`load` and `*_buffer`; README: viewing "does not load the data into memory... using memory-mapping" (`rust/README.md` lines 79-103) [M]. v2.26.1: "Bound file-controlled sizes in `view()`"; v2.26.2: "Retry Linux file mapping without `O_NOATIME`" [M].
- **Filtered search:** `filtered_search(query, count, |key| -> bool)` (`lib.rs:1489-1499`) is predicate-based [M]. Open issue #447 (2024-06-27): "Missing results after save + view + filtered_search" [M]; #672 asks for ACORN-1 filtered search [M].
- Other open Rust-relevant issues: #574 Rust safety/soundness conventions; #691 external vector accessor (vectors cannot live outside the index today [E]); #700 `compact()` (the v2.26.0 notes show "Rust compact binding" was added, so #700 may be stale) [M].
- Pin: `usearch = "=2.26.2"`.

### 5.2 hnsw_rs (https://github.com/jean-pierreBoth/hnswlib-rs)
- 0.3.4 (2026-02-28), 27 versions, 2 releases in 12 months [M]; master last commit 2026-09-26 (so commits are ahead of releases); 254 stars; 11 open issues; license files LICENSE-APACHE and LICENSE-MIT, crates.io says "MIT/Apache-2.0", GitHub reports NOASSERTION because the Apache file has a custom copyright header [M]. Single main author [E].
- README [D]: multi-threaded insert/search, dump/reload with serde, **filtering during search** (sorted id list or callback; "not a post filter"), **mmap on dumped data vectors (not on the graph part)**. Depends on `mmap-rs ^0.7`, `anndists ^0.1`, `rand ^0.9`, `bincode ^1.3` [M]. No quantization (generic over element type) [E]. Windows: no CI or issue evidence either way (open issues for "windows": 0) [M].

### 5.3 hannoy (https://github.com/nnethercott/hannoy) and arroy (https://github.com/meilisearch/arroy)
- hannoy 0.2.0 (2026-08-14), repo pushed the same day; 87 stars; MIT; 398,020 downloads [M]. README [D]: "key-value backed HNSW implementation based on arroy", LMDB (via `heed 0.22.1`) storage, euclidean/cosine/manhattan/hamming plus "quantized counterparts", dynamic insert and delete, Roaring-bitmap edges. Windows: uses LMDB through heed, which targets Windows, but I saw no Windows-specific statement [E].
- arroy 0.8.0 (2026-08-12), 14 versions, 3 in 12 months; 312 stars; MIT; 38 open issues+PRs [M]. Annoy-style random-projection forest on LMDB, "Filter when querying", README says "The code should support Windows, thanks to LMDB" [D]. GitHub `main` last commit 2025-10-16; the 0.8.0 tag is a merge dated 2026-08-12 (`release-v0.8` branch exists) [M]. Both are driven by Meilisearch's needs [E].
- Note: arroy and hannoy store vectors inside LMDB and need a pre-sized memory map (`map_size`) [D]; this is a design input, not evaluated here.

### 5.4 Microsoft DiskANN (Rust, `diskann` crate) and diskann-rs
- `diskann` 0.59.0 (2026-09-11), 18 releases in 12 months, repo last pushed 2026-10-01T01:12Z (UTC), MIT, 1,938 stars [M]. README [D]: "DiskANN3: a composable vector indexing library", real-time updates, quantizers (PQ, MinMax, Scalar, Spherical) for x86 and aarch64, attribute-filter hooks; you implement the storage provider; the older C++ code is "not actively developed". Pre-1.0 with fast churn; Windows status not stated in the README [M/D].
- `diskann-rs` 0.5.0 (2026-02-09) by a different author: 44 stars, 0 open issues, MIT; README [D]: incremental updates, filtered search, PQ, F16/Int8 scalar quantization, mmap single-file storage. Quiet since 2026-02-09 [M]. Low maturity.

### 5.5 instant-distance and hora
- instant-distance 0.6.1 (2023-06-26); GitHub `archived: true`, 344 stars, 5 open issues [M]. Pure-Rust HNSW; pins `rand ^0.8`, `ordered-float ^3.0` [M]. Do not use.
- hora 0.1.1 (2021-08-07), only 2 releases; default-branch last commit 2021-10-23; 26 open issues [M]. Do not use.

### 5.6 faiss (Enet4/faiss-rs)
- `faiss` 0.13.0 (2025-11-15), repo last push 2025-11-15, 21 open issues, Apache-2.0 / MIT [M]. README [D]: it does **not** build FAISS for you; either build the FAISS C API with CMake yourself (fork branch `c_api_head`) or enable the `static` feature, which still needs a C++ compiler and a BLAS library. No Windows statement and no Windows CI evidence in what I read [M]. Not recommended on Windows [E].

### 5.7 Notes across the vector crates
- Only usearch (f16/i8/b1 scalar kinds), diskann-rs (F16/Int8/PQ) and Lance (PQ/SQ/RQ) offer built-in quantization among those inspected; hnsw_rs, hannoy and arroy are float-oriented with the quantized options noted above [D/E].
- Only Lance gives "SQL filter + vector + FTS + scalar index" in one engine; usearch and hnsw_rs give a predicate callback and leave metadata filtering to us [E].

---------------------------------------------------------------------------

## 6. grep-searcher and grep-regex (ripgrep crates)

Sources: crates.io; https://github.com/BurntSushi/ripgrep ; crate source `grep-searcher-0.1.17`, `grep-regex-0.1.14`.
- grep-searcher 0.1.17 (2026-07-15, shipped with ripgrep 15.2.0 of the same day); earlier 0.1.16 (2025-10-22), 0.1.15 (2025-10-16), 0.1.14 (2024-09-09), 0.1.13 (2023-11-28) [M]. Cadence [E]: 1-3 releases per year, tied to ripgrep releases.
- grep-regex 0.1.14 (2025-10-16); previous 0.1.13 (2024-09-09), 0.1.12 (2023-11-26) [M]. No release in the last 349 days; its dependencies are `regex-automata ^0.4`, `regex-syntax ^0.8`, `grep-matcher ^0.1.8` [M].
- Licence: `Unlicense OR MIT` in both crates; `UNLICENSE` and `LICENSE-MIT` are included in the crate tarball (grep-searcher tarball lists `UNLICENSE`, `LICENSE-MIT`) [M]. GitHub's licence detector reports Unlicense for the repo (root file `UNLICENSE` begins "This is free and unencumbered software released into the public domain.") [M].
- Maintenance: ripgrep master last commit 2026-08-04 (ignore-0.4.33, globset-0.4.20) [M]; latest ripgrep release 15.2.0 (2026-07-15) [M]. 201 open issues+PRs overall; grep-searcher-specific open items include #1618 (advance LineStep without iterating), #1634 (expose match offsets in SinkMatch), #1781 (binary files with multiline mode) [M]. A search for open "mmap windows" issues returned 0 [M].
- Edition 2024 for grep-regex 0.1.14 (`Cargo.toml:13`), so Rust >= 1.85 [M, inferred from the edition]. `grep-searcher` has a `searcher/mmap.rs` module (memory-map strategy is opt-in through the searcher builder) [M].
- Features we rely on: line-oriented search with `Searcher` + `RegexMatcher` + `Sink`, binary detection, context lines, multiline option [D, crate README/docs; not exercised].
- Pin: `grep-searcher = "=0.1.17"`, `grep-regex = "=0.1.14"` (grep-matcher resolves transitively).

---------------------------------------------------------------------------

## 7. Cross-cutting notes for the lead (not decisions)

1. **Lock-step constraints:** tantivy 0.26.x + {tantivy-jieba 0.20.0 | cang-jie 0.21.0 | lindera-tantivy 5.0.1}; lancedb 0.39.0 = lance 12.0.0 = arrow 58 = datafusion 54 (so any other Arrow user in `libraryd` must also be on Arrow 58) [M]. MSRV is 1.91 as soon as lancedb is included; tantivy alone needs 1.86; cang-jie and lindera need 1.88 [M].
2. **Chinese tokenizer defaults matter for phrase and trigram features:** tantivy-jieba defaults to search-mode (overlapping tokens) and offset-based positions [M]; tantivy ngram positions are all 0 [M].
3. **tantivy on Windows:** works as a library (it has `cfg(windows)` paths) but is not covered by upstream Windows CI [M], and two Windows `PermissionDenied` reports are open. Stage 1 (this PC) and Stage 2 (AI PC, OS not stated in the brief) both depend on how the writer is configured; test with the real commit/merge pattern before assuming parity [E].
4. **Lance vs tantivy for regex prefiltering:** Lance 12 has built-in NGram (regex-aware) and FM-Index scalar indexes with documented fallbacks [M]. That overlaps with the design's "trigram field in tantivy"; the overlap and the Lance-vs-tantivy split of lexical search are design questions for you.
5. **Abandonment list:** hora, instant-distance (archived), tantivy-ik (LGPL, stale); vaporetto_tantivy is two tantivy minors behind.
6. **Things I could not verify:** actual Windows compilation of lance/lancedb/usearch/numkong/protobuf-src/lindera dictionaries; whether lance #8908 is closed by the v12 accumulator fix; how cang-jie and lindera-tantivy assign token positions; the Windows behaviour of faiss, hannoy and arroy beyond their own statements; tantivy-analysis-contrib's ICU system-library requirement; Rust public-API diffs between lancedb 0.37.1 and 0.39.0 beyond release notes.

---------------------------------------------------------------------------

## 8. Evidence index (primary sources)

crates.io API: `https://crates.io/api/v1/crates/<crate>` and `/<crate>/<version>/dependencies` for every crate named above (queried 2026-09-30).
GitHub (via `gh api`): repos, commits, releases, tags, license, search/issues for the repos in section 1; tantivy issues #2847, #2647, #2974, #2712, #3100, #2272; lance issues #9422, #8607, #9395, #3073, #9506; lancedb issues #2051, #3951; USearch issues #447, #672, #691, #700; ripgrep release notes 15.1.0 and 15.2.0.
Raw files: `https://raw.githubusercontent.com/quickwit-oss/tantivy/main/CHANGELOG.md`; lance docs at tag v12.0.0 (`docs/src/format/index/scalar/{ngram,fmindex}.md`, `docs/src/format/index/vector/index.md`, `CONTRIBUTING.md`, `.github/workflows/rust.yml`); lancedb `.github/workflows/rust.yml` at v0.39.0; READMEs of tantivy-jieba, cang-jie, lindera-tantivy, arroy, hnswlib-rs, hannoy, diskann-rs, DiskANN, instant-distance, faiss-rs, USearch Rust.
Published crate sources inspected (extracted from static.crates.io tarballs): tantivy-0.26.2, tantivy-jieba-0.20.0, cang-jie-0.21.0, lindera-tantivy-5.0.1, lindera-6.2.0, lindera-cc-cedict-6.2.0, lindera-dictionary-6.2.0, jieba-rs-0.11.0, lancedb-0.39.0, lancedb-0.37.1, lance-12.0.0, lance-index-12.0.0, lance-linalg-12.0.0, lance-tokenizer-12.0.0, usearch-2.26.2, arroy-0.8.0, hnsw_rs-0.3.4, diskann-rs-0.5.0, instant-distance-0.6.1, faiss-0.13.0, grep-searcher-0.1.17, grep-regex-0.1.14.
Scratch copies of the raw data (JSON, extracted sources) are in `C:\Users\sethy\AppData\Local\Temp\claude\C--Users-sethy-OneDrive-Desktop-Tutor\ba4722e7-2537-4ced-b4a0-4062c34e0fa8\scratchpad\projsearch\` and may be deleted at any time.
