# Existing retrieval code audit for the vault (Phase 0, 2026-09-30)

Written by a Phase 0 worker for the lead. Scope: what the owner's existing retrieval code offers the new vault (`libraryd` over a Markdown store addressed as `{doc_id}#L{start}-L{end}`), and what the vault must not repeat. I read code and documents only. I ran nothing that builds or tests: no `cargo` (it is not on the Git Bash PATH, and building would write into `target/`), no tests, no service. Every other action was a read, a listing, a read-only `git log`/`git rev-list`, or a short read-only Python snippet that parsed a JSON file.

**Number tags.**
- **(M)** measured in this session by reading, listing or counting. Test counts are counted from source, not from a test run.
- **(S: where)** documented by the named source (a repo document, a baseline log, or a crate's source in `~/.cargo/registry`). Behaviour I infer from reading code is tagged `(S: file:line)`.
- **(E)** my estimate or inference.

Paths: `T` = `C:\Users\sethy\Projects\tutor`, `RE` = `T\crates\rag-engine`, `D` = `C:\Users\sethy\OneDrive\Desktop\Tutor`, `V` = `C:\Users\sethy\Projects\vault`.

---

## 1. Summary

1. **rag-engine is a working, tested page/chunk-ordinal retrieval service, not a base for the vault.** It is a binary-only crate (no `src/lib.rs`, no `[lib]`; M), 8,796 lines including inline tests (M, `wc -l`). Its text store (the `text` column of the `chunks` table, `store.rs:56`), chunk ids (`<doc_id>:<ordinal>`, `ingest.rs:896`), page-based provenance and REST-only server all conflict with a Markdown-file store addressed by line ranges. It cannot be linked into `libraryd` as it stands; its parts have to be copied or ported.
2. **Worth lifting (verdict "reuse as-is" or "port with small changes")** is in section 3: exact-dedup hashing, Markdown heading parsing, RRF, the job runner, the tantivy writer/merge-policy/rebuild-from-store discipline, the maths normaliser, the PDFium open/encryption-refusal/panic guards, the page-render and quote-locate code, the bearer-token and bind rules, the retrying embedding client, and the black-box service test harness.
3. **Confirmed defects (section 4):**
   - CJK: tantivy's `SimpleTokenizer` makes a whole run of CJK characters one token, and `RemoveLongFilter(40)` removes tokens of 40 bytes or more. So CJK runs of 14 or more characters are dropped, and shorter runs are indexed as one unsplittable token. Neither case supports substring search (`tokenize.rs:24`, tantivy source).
   - English stemmer on every language (`tokenize.rs:26`).
   - Chunk text stored in every SQLite row, overlap included.
   - One mutex-guarded SQLite connection serves every search.
   - A process-wide PDFium lock, so PDF extraction is single-threaded.
   - No stopword removal and no term cap, so a 150-term rewrite becomes about 450 optional clauses.
   - The query-side instruction prefix that Qwen3-Embedding needs cannot be sent.
   - Titles fall back to the file name.
   - The Rust and Python quote normalisers disagree.
4. **None of the adoption plan's 18 increments (I0-I17) is present in code.** No `bench.rs`, `snapshot.rs`, `route.rs`, `lang.rs` or `modelhost.rs` (M, `src/` listing), and the files I read for I2/I3 (`store.rs`, `bm25.rs`, `tokenize.rs`) are unchanged in the way the plan describes. `git log 26041759..HEAD -- crates/rag-engine` prints nothing (M), and the five `wt/rag*`/`wt/corpus-search`/`wt/evals` branches have 0 commits ahead of `custom` (M, `git rev-list`). The plan's caveat about unreconciled worktrees no longer applies.
5. **The plan's central constraint reversed.** `T\docs\tutor\PLAN.md:31` says "no local models, and the RTX 5090 is off-limits". The new design allows the 5090 for embedding. Several plan conclusions flip (section 7). `PLAN.md` and `CORPUS.md` Stage 0 will need amending when the vault becomes the retrieval backend.
6. **Biggest design hazards the audit surfaces for line addressing** (decisions for the lead, not made here; section 6.3):
   - `text-splitter` returns byte ranges inside lines, so a chunk is not a line range unless the canonical Markdown guarantees short lines.
   - rag-engine's `doc_id` is the hash of the source file, not of the extracted text. Re-extraction would silently shift line numbers under an unchanged `doc_id`.
   - Python's `str.splitlines()` and Rust's `str::lines()` disagree on what a line is.

---

## 2. Inventory, pinned versions, licences, build requirements

### 2.1 Pinned versions

Source: `RE\Cargo.toml` and `T\Cargo.lock` (M). Rust edition 2024. The README says it was built with Rust 1.98.1 on Windows x64 MSVC (S: `RE\README.md:18`). `~/.rustup/toolchains/stable-x86_64-pc-windows-msvc` exists (M); I did not read its version.

| Crate | `Cargo.toml` | Locked | Licence (S: `T\docs\tutor\qa\licenses\licenses-rust.csv`) | Notes |
|---|---|---|---|---|
| tantivy | 0.26.2 | 0.26.2 | MIT | Provides `NgramTokenizer`, `RegexTokenizer`, `RegexQuery`, phrase queries, and Snowball stemmers for 18 languages including Spanish, French and German (S: tantivy-0.26.2 `src/tokenizer/`, `src/query/`). No Japanese or Chinese tokenizer. |
| lancedb | 0.37.1 (default features off) | 0.37.1, `lance` 10.0.0, `arrow` 58.4.0, `datafusion` 54.1.0 | Apache-2.0 | README: 0.38 and 0.39 do not compile without the `remote` feature (S: `RE\README.md:535`; not re-verified). Their sources and `lance` 11/12 are in the registry (M), so a bump was tried. Exposes `IvfPqIndexBuilder` and `IvfRqIndexBuilder` with `num_bits: Option<u32>`, and default nprobes 20/20 (S: lancedb-0.37.1 `src/index/vector.rs:267-353`, `src/query.rs:1103`). |
| pdfium-render | 0.9.4 (`pdfium_latest`, default features off, no `thread_safe`) | 0.9.4 | MIT OR Apache-2.0 | Runtime-bound to a PDFium DLL. Without `thread_safe`, so calls are serialised by one global lock (`extract.rs:27`). |
| PDFium binary | n/a | 156.0.8076.0 (S: `RE\vendor\pdfium\VERSION`) | BSD-3-Clause/Apache-2.0 plus bundled third-party licences in `vendor\pdfium\licenses\` (M) | bblanchon/pdfium-binaries `chromium/8076`. The 182-test baseline passed against it (S: `T\docs\tutor\qa\baseline-checks-2026-09-30.md`). |
| text-splitter | 0.33.0 (`markdown`, `tiktoken-rs`) | 0.33.0 | MIT | Other features: `tokenizers` (Hugging Face), `code` (S: crate `Cargo.toml`). |
| tiktoken-rs | 0.12.1 | 0.12.1 | MIT | cl100k sizes chunks. The embedding model's own tokenizer is not used. |
| rusqlite | 0.40.2 (`bundled`) | 0.40.2, `libsqlite3-sys` 0.38.2 | MIT (SQLite is public domain) | `bundled` builds SQLite from C, which needs a C toolchain. |
| axum | 0.8.9 | 0.8.9 | MIT | Used for REST only. Neither `tonic` nor `rmcp` appears anywhere in the repo's manifests (M). |
| tokio | 1.53.1 | 1.53.1 | MIT | |
| tower-http | 0.7.1 (`cors`) | 0.7.1 and 0.6.11 | MIT | Two versions in the lock (E: the other is probably pulled by the desktop shell; not traced). |
| reqwest | 0.13.5 (`native-tls`) | 0.13.5 and 0.12.28 | MIT OR Apache-2.0 | Uses SChannel on Windows. |
| rbook | 0.7.10 (default features off) | 0.7.10 | Apache-2.0 | EPUB spine and table of contents only. |
| blake3 | 1.8.7 | 1.8.7 | CC0-1.0 OR Apache-2.0 | |
| others | clap 4.6.7, rayon 1.12.0, png 0.18.1, serde 1.0.229, serde_json 1.0.151, walkdir 2.5.0, futures 0.3, unicode-normalization 0.1.25 | as listed | MIT or MIT/Apache-2.0 | |

**Licence-relevant transitive dependencies** (S: `RE\README.md:538-545`, and the workspace-wide `licenses-rust.csv`):
- `option-ext` 0.2.0 is **MPL-2.0**, via `dirs` in `lance-index`. It is file-level copyleft and used unmodified.
- `xxhash-rust` 0.8.19 is **BSL-1.0** (the permissive Boost licence, not the Business Source License), via `lance-encoding`.
- `htmlescape` is triple-licensed MIT, Apache-2.0 and MPL-2.0.
- `openssl` crates appear in the workspace-wide list. rag-engine's own `reqwest` build uses SChannel on Windows (S: `RE\README.md:523`), so I did not trace where `openssl` comes from.
- No AGPL or GPL code is linked. The crate's own README names mupdf (AGPL), the `epub` crate (GPL-3.0) and kreuzberg 5.x (Elastic-2.0) as excluded.
- The workspace crate's own licence is `MIT OR Apache-2.0` (`RE\Cargo.toml:5`). The owner's own code can be lifted into the vault repository without licence friction.

**Not in rag-engine, relevant to the vault's sidecars** (S: sweep, `T\docs\tutor\research\rag-sota\2026-09-30-sweep.json`, parsing finding 3 and frameworks finding 5): MinerU is "Apache-2.0 plus additional terms", with a separate licence above 100M MAU or US$20M monthly revenue. GROBID is not listed with a licence in the findings I read, so I leave it unrated. llama.cpp and Qwen3 licences are outside this audit.

### 2.2 Build requirements (Windows 11, as of this session)

- **Toolchain:** Rust stable MSVC (edition 2024), installed under `C:\Users\sethy\.cargo\bin` (M). `cargo` is not on the Git Bash PATH. The legacy workspace at `D` points its target to `C:/Users/sethy/AppData/Local/tutor-cargo-target` (S: `D\.cargo\config.toml`). The `T` workspace builds into `T\target`, which was 22 GB (S: the adoption plan, section 6, which measured it earlier today).
- **protoc (build time):** `T\crates\rag-engine\vendor\protoc\bin\protoc.exe`, 13,269,764 bytes (M; protoc 36.2 per README). `T\.cargo\config.toml` sets `[env] PROTOC = { value = "crates/rag-engine/vendor/protoc/bin/protoc.exe", relative = true, force = false }` (M). An existing `PROTOC` in the environment wins. `vendor/` is gitignored (`RE\.gitignore:1`), so a fresh clone must download it. It is needed because `lance` generates code with prost-build. The vault's `tonic` will need it too, so the same `.cargo/config.toml` pattern can be reused.
- **PDFium DLL (run time and tests):** `T\crates\rag-engine\vendor\pdfium\bin\pdfium.dll`, 7,381,504 bytes (M). The default path is baked in at compile time from `CARGO_MANIFEST_DIR` (`main.rs:36`), so a copied binary will not find it. Override with `--pdfium <file|dir>` or `PDFIUM_DYNAMIC_LIB_PATH`. The test helpers use the same default (`pages.rs:223`, `ingest.rs:1295`).
- **Test fixtures:** committed PDFs and an EPUB under `RE\tests\fixtures` and `RE\tests\library`. They were generated by `make_fixtures.py` with reportlab and Pillow (S: `RE\README.md:504`). None is CJK and none is a real book (M, file list).
- **Build time:** `cargo build --release` of the whole workspace, desktop shell included, took 25 min 13 s at `-j 3` (S: baseline-checks, commit d48dce1f). The README says a first debug build of LanceDB with Arrow and DataFusion takes about 5 minutes on that machine (S: `RE\README.md:44`). The dependency tree is 519 packages (S: `RE\README.md:538`).
- **Process cost:** the machine rules say every process launch costs about 300 ms. rag-engine's own tests spawn the binary per test (S: `tests/service.rs`). Plan builds and test runs as single batched commands.

---

## 3. Module-by-module audit

Verdicts are relative to the vault's source of truth: one Markdown file per document, addressed by line ranges.
- **reuse** = lift as-is.
- **port** = lift the logic and change it.
- **replace** = write fresh, keeping at most the idea.

Test counts are `#[test]`/`#[tokio::test]` attributes counted by `Grep` (M): 125 in `src/`, 5 in `tests/service.rs`, 1 in `tests/survey.rs`, 131 in all. The baseline run of the earlier commit d48dce1f reported 182 passing workspace tests, of which `rag_engine` bin 121, `service` 4, `survey` 1 (S: baseline-checks). The later commits added 4 unit tests and 1 service test. I did not run them.

| Module | Lines | What it does | Tests (M) | Known defects (evidence) | Verdict |
|---|---|---|---|---|---|
| `main.rs` | 208 | clap CLI: `survey`, `serve`, `ingest`, `search` | none (covered by `tests/`) | Default PDFium path is baked in at compile time (`:36`). Every command opens an `Engine` with env-configured embedder and OCR. | **port**: keep the subcommand/env pattern. The vault CLI is different. |
| `error.rs` | 48 | `Error(String)` plus `From` for library errors | none | Errors lose their type. A gRPC server needs status-code mapping. `search::SearchError` and `server::ApiError` re-derive it by hand. | **replace** with typed errors. |
| `engine.rs` | 127 | Shared state (`Bm25`, `Vectors`, `Embedder`, SQLite) and read-only `Roots`; `Roots::check` refuses a `--data` dir inside the corpus | 2 | **One `Mutex<Connection>` serves every search, listing and job read** (`:69`, `:91`; `search.rs:193`, `server.rs:407`), so concurrent queries serialise on the SQL step. The agentic loop and rewrites fan out 3-5 parallel searches (`T\deeptutor\services\library\search.py:80`). (E) impact. | **port** the `Roots` check (the vault must never write into the corpus). **Replace** the connection handling (a read-only pool, or one connection per reader). |
| `discover.rs` | 166 | `walkdir`, `.pdf/.epub/.md/.txt` only, counts other extensions | 5 | The format list is fixed. Symlinks are not followed. `docs` is held fully in memory (fine to millions of files (E)). | **port**: the vault's intake does format detection and quarantine. |
| `dedup.rs` | 96 | BLAKE3 content hash in parallel (`rayon`), exact-duplicate groups | 4 | Exact duplicates only. No near-duplicate detection (preprint vs published version, scanned vs born-digital copies). | **reuse** the hashing. Near-duplicate detection is separate new work. |
| `meta.rs` | 130 | Domain and source type from the folder path (`corpus/<domain>/books|papers/**`) | 6 | No bibliographic metadata and no language. The folder is the only metadata, so it cannot carry a journal, year, DOI or language. | **replace**: the vault's metadata comes from intake and sidecars. |
| `text.rs` | 206 | Character/token estimate (`chars/4`), a hand-rolled `strip_markup` for EPUB XHTML | 9 | `strip_markup` is "only used for counting" by its own comment (`:46`). It turns every tag into a space, drops MathML structure, and decodes only six named entities (`&copy;` stays literal). | **port** `measure()` for survey-like sizing. **Replace** `strip_markup` with a real XHTML to Markdown path. |
| `extract.rs` | 279 | PDFium text per page and cover metadata title, EPUB spine text (`rbook`), plain text. Encryption/DRM refusal. Panic catcher | 3 (rest via fixtures in `service.rs`) | **A process-wide PDFium mutex** (`:27`) makes PDF extraction single-threaded, and the README says the likely limit has "not been measured on real books" (`RE\README.md:393`). PDFium text order is raw (no layout), no de-hyphenation, ligatures (`ﬁ`) stay in the stored text. EPUB MathML is flattened to text. No CJK PDF fixture. | **port** the guards: `open_pdf` (password/DRM/handler-revision refusal, `:73-95`), `catch_panics` (`:13`), `has_image` form-XObject recursion (`:146`), `clean_pdf_text`. As a vault component PDFium becomes a probe, triage and fallback path, not the canonical text. **Replace** the EPUB text path. |
| `triage.rs` | 259 | Per-page class (`born_digital`, `scanned`, `math_heavy`, `empty`) from character counts and ratios | 13 | **Thresholds unvalidated** (S: `RE\README.md:438`: "starting points ... have not been validated on real books"). `math_heavy` pages (3% math signals, or 40% tokens of 1-2 characters) go to the OCR queue and the OCR text *replaces* the text layer (`ocr.rs:44`, `ingest.rs:610`), which could discard a good TeX text layer. Full-width math forms (`＝`, `＋`) are not math signals. A hidden OCR text layer reads as `born_digital`. No test for CJK (see 4.9). | **port** as a cheap CPU routing feature, with the thresholds re-measured and a class set that matches the vault's extractor routing. |
| `tokenize.rs` | 199 | Analyzer `en_stem_math`: NFKC, Greek and operators to LaTeX names, `SimpleTokenizer`, `RemoveLongFilter(40)`, lowercase, English Snowball stemmer | 4 | CJK runs dropped or unsplittable, English stemmer for all languages, no stopword filter (4.1-4.3). `→` becomes the token `to`, `∈` becomes `in` (`:110`, `:72`): they collide with real words and add noise. | **port** `latex_name()` and `normalise_into()` as a reusable maths character filter (the logic is sound and tested: `greek_letters...`, `operators...`, `mathematical_alphanumerics...`, `plain_text_and_latex_queries...`). **Replace** the analyzer chain with per-language analyzers. |
| `chunk.rs` | 219 | `text-splitter` (`MarkdownSplitter` or `TextSplitter`) at 400 cl100k tokens with 40 overlap, page ranges, heading path at chunk start | 7 | Offsets are **byte** offsets into one joined string (pages joined by `"\n\n"`, `:19`). A chunk spanning a page break contains separator text that is in no page. Chunk text includes the overlap, so adjacent chunks duplicate text. A chunk records only the heading in force at its start (`:104`). Sizing uses cl100k, not the embedding model's tokenizer. No CJK sizing test. | **port**: keep `text-splitter` as the sizing engine, but its output must be snapped to line boundaries (6.3). The page logic goes away. |
| `headings.rs` | 381 | Heading paths: Markdown ATX headings outside code fences; PDF text heuristic (chapter words, numbered, title-case) with running-header suppression | 11 | `pdf()` is English-only (`CHAPTER_WORDS`, `SMALL_WORDS`, uppercase test) and guesses. `markdown()` ignores setext headings and HTML `<h1>` (documented, `:3`). Offsets are byte offsets. | `markdown()`: **reuse** (the offset converts to a line number trivially). `pdf()`: **drop** (the vault's extractors supply headings). It can remain a fallback for the PDFium-only path. |
| `pages.rs` | 286 | PDFium page render to PNG with the quote's rectangles highlighted. Quote normalised and looked up in the page's character stream | 6 | Its normaliser (`:103-128`) and Python's `normalize_quote` (`T\deeptutor\services\library\normalize.py`) are documented as "the same" but differ: Python also maps `ʼ « » ‑ ﹘ ﹣ －`, Rust does not. Neither applies NFKC, so `ﬁ` vs `fi` fails. | **port** for "show the source PDF page with the quote marked" (works from a quote, so it does not need page/offset provenance). Unify the normaliser with Python by one spec and a shared test vector set. |
| `bm25.rs` | 506 | tantivy index: fields `chunk_id` (STORED), `doc_id`, `domain`, `source_type`, `text`, `title`, `heading`. Frequencies only, no positions. Writer heap 1 GiB, commit every 100k chunks, log-merge with `del_docs_ratio_before_merge` 0.2. Schema versioning: an index with another schema is deleted and rebuilt from SQLite | 7 | One analyzer for all fields. `chunk_id` is a stored string, so every hit does a stored-field read and a SQL `IN` lookup. Every query term becomes `SHOULD` clauses in 3 fields with no stopword removal or term cap (`:213-221`). `ReloadPolicy::Manual` (`:180`): a second process (e.g. CLI `ingest` beside `serve`) is not seen until a reload. No positions, so no phrase queries and no trigram field. | **port**: keep `Tuning`, merge policy, single-writer discipline, schema-version rebuild, and the "query is never parsed as syntax" rule. Change: `chunk_key` as a u64 fast field, per-language fields, trigram field for regex, and a bounded query builder. |
| `search.rs` | 298 | `/search` modes `bm25`, `vector`, `hybrid` (RRF k=60, 50 candidates each); `load_hits` joins SQLite for text and metadata | 4 (RRF ties, duplicates, validation) | `load_hits` takes text from the `chunks.text` column (`:195`). Single query per request. No offset. Max `k` 100 (`:26`). | **reuse** `rrf()` (`:91`, small, tested, deterministic ties). **Port** the rest: the line slicing replaces the SQL text read. |
| `vector.rs` | 306 | LanceDB table `chunks` (`chunk_id`, `doc_id`, `domain`, `source_type`, f32 `vector`). Cosine. IVF-PQ with defaults at 100k rows (`:205`). `optimize()` | 3 | f32 only (`:106`). String keys, deletes by `IN (...)` in batches of 500 (`:29`). No scalar indexes. Filter columns duplicated and updated in place. No `nprobes` control. No recall check. | **port** if LanceDB wins the vault's vector-index benchmark (`PROGRESS.md` "candidates A and B"). Otherwise **replace**. Either way the id scheme changes. |
| `embed.rs` | 524 | `Provider` trait with `voyage` and `openai_compat` HTTP providers, batches of 128, retry with backoff and `Retry-After`, spend accounting | 9 (retries, request shapes against a local fake server) | `openai_compat` ignores `InputType` (`:160`), so a Qwen3-style query instruction prefix cannot be added. No L2 normalisation, no Matryoshka truncation. One batch in flight. `assert!` panics on an oversize batch (`:235`). | **port**: keep the retry/validation/test-server pattern. Drop spend. Add normalise, truncate, prefixes, concurrency. The `openai_compat` shape probably matches llama.cpp's server (E, not verified). |
| `ocr.rs` | 254 | Durable per-page queue (`ocr_queue`), `OcrProvider` trait, no real provider | 3 | `RAG_OCR_PROVIDER` is rejected at startup, since none is implemented (`:33-42`). The queue stores OCR text in SQLite rows. | **replace**: MinerU is an external sidecar. Keep the idea of a durable per-page state and text keyed by content. |
| `ingest.rs` | 1474 | Incremental ingest: walk, size+mtime pre-check, hash, remove only after a complete walk, move without re-embedding, bounded-memory parallel pipeline with in-order commit, crash repair, schema-version re-index from the database, OCR, embeddings | 14 (incl. parallel==serial row comparison) | Extraction, chunking and indexing are one phase, so re-chunking or re-indexing is only possible from the SQLite chunk text. Only two staleness keys exist (BM25 schema version, embedding model; `:55-57`, `bm25.rs:26`). PDF extraction on one thread. Failed files are not retried until they change. | **replace** as a whole. **Reuse the patterns:** size+mtime pre-check (`:229`); removal only on a complete walk (`:246`); `indexing` to `indexed` commit protocol with repair (`:564-596`); bounded permits with in-order store (`:727-793`); `reindex_if_stale`; the parallel==serial test (`:1341`). |
| `store.rs` | 247 | SQLite schema (`files`, `documents`, `chunks`, `ocr_queue`, `jobs`, `spend`, `meta`), WAL, 64 MiB cache, `user_version` refusal of newer schemas | 3 | `chunks.text TEXT NOT NULL` (`:56`) stores all chunk text. `heading_path` is a JSON string. | **port**: the pragmas (`WAL`, `busy_timeout`, `synchronous=NORMAL`, `cache_size`), the `meta` key-value table, the `user_version` guard. **Replace** the schema. |
| `jobs.rs` | 367 | One active job at a time, states persisted in SQLite, interrupted jobs marked failed at startup, progress counters | 3 | One global job slot: an ingest blocks a survey. Polling progress only. | **reuse** nearly as is, then extend for gRPC streaming progress. |
| `library.rs` | 408 | `GET /documents`, `/documents/{id}` with an outline built from chunk heading paths, `/documents/{id}/chunks?from=<ordinal>` | 2 | Ordinal paging. The outline is derived from chunk heading paths, not a document's heading tree, so it depends on chunk boundaries. | **replace** (`outline` and `read` over the Markdown by line). The listing/filter/sort SQL is reusable for `find_docs`. |
| `survey.rs`, `report.rs` | 203, 157 | Survey a corpus: discovery, hashing, dedup, page triage, size, projections to 50 and 250 GB | 3 (+ `tests/survey.rs`) | Sizes by `characters/4` (`text.rs:7`), a rough estimate. Single PDFium thread. | **reuse** as the measuring tool for Phase 0 and intake characterisation (`rag-engine survey --root <dir> --out <json>`). I did not run it. |
| `server.rs` | 579 | axum REST: routes, bearer token (BLAKE3 digest compare), non-loopback bind needs a token, CORS, `Host`/`Origin` guard against browser pages (C3) | 4 (+ 4 service tests that exercise it) | REST/JSON only. The `Host`/`Origin` guard and CORS exist for a browser client the vault will not have on gRPC. | **port** `require_token`, `check_bind` and the token-absent-from-logs test. The route set is **replaced** by MCP and gRPC tools. |
| `tests/service.rs` | 785 | Black-box tests: spawn the binary on a free port, strip `RAG_*` env, run a job, check search, filters, page images, token and origin guards, and that the library files are unchanged | 5 | Synthetic tiny fixtures only. | **reuse the pattern** for vault integration tests. |
| `bench/synthetic_corpus.py` | 1.9 KB | Generates 200 synthetic Markdown books (34 MB) | n/a | Random vocabulary understates real term distributions (the adoption plan says the same, I0 risks). | **reuse** for plumbing checks. The vault's 6M-chunk benchmark needs real text. |

### 3.1 Tests: what they do and do not cover

- **Well covered:** BM25 stemming/filter/delete/boost behaviour, maths normalisation, RRF, heading heuristics, chunk token limits and page ranges, incremental ingest (unchanged, changed, moved, deleted, duplicate, scoped, interrupted, rebuild, unreadable), serial vs parallel ingest producing identical rows, embedding retries against a fake server, the job lifecycle, token/origin guards, page rendering with highlights.
- **Not covered:**
  - Any non-English text or any CJK text at any layer (analyzer, chunking, triage, headings, rendering).
  - Any real book or paper. All fixtures are tiny and synthetic.
  - Any scale test: no benchmark exists (I0 was never built).
  - PDF extraction throughput.
  - The vector index at its 100k-row threshold. The test uses 3 dimensions and 4 rows (`vector.rs:258`).
  - Concurrent searches against one engine.
  - A real OCR provider (none exists).

---

## 4. Defect register

Severity is for the vault's goals. Where a defect needs a number I could not measure, it is marked (E) and "to measure".

| # | Defect | Evidence | Severity | What the vault must do |
|---|---|---|---|---|
| 4.1 | **CJK runs are dropped or unsearchable.** `SimpleTokenizer` splits only on non-alphanumeric characters, and `char::is_alphanumeric()` is true for Han, kana and Hangul, so a run between punctuation or whitespace is one token. `RemoveLongFilter::limit(40)` keeps a token only if its UTF-8 length is **below** 40 bytes. At 3 bytes per CJK character, runs of **14 or more characters are dropped**, and runs of 13 or fewer are indexed as one whole-run token that matches only the identical run. 光合 would not match a chunk containing 光合成 (the adoption plan's acceptance test, section I3). | `tokenize.rs:24`; tantivy-0.26.2 `src/tokenizer/simple_tokenizer.rs:34,46`, `remove_long.rs:36` (`token.text.len() < token_length_limit`) (S). Not executed: no CJK test exists. The adoption plan marks this as inferred, and my source reading confirms it, with the byte-length nuance above. | **High** (Chinese and Japanese papers are in scope) | Per-language analyzers. A bigram field for Han/kana runs first. A trigram field catches substrings but is not a CJK analyzer. Dictionary segmenters (lindera, jieba) are an eval arm (S: adoption plan I10). |
| 4.2 | **English Snowball stemmer on every language.** Spanish, French, German and others are stemmed with English rules. tantivy ships Snowball stemmers for 18 languages (S: `stemmer.rs:12-31`) and none is used. | `tokenize.rs:26`; one analyzer name `en_stem_math` for all fields (`bm25.rs:113-125`). | Medium | Language per chunk or per document. Per-language fields (IDF stays separate). A stemming-free exact field keeps quote checks exact. |
| 4.3 | **No stopword removal and no term cap.** A 150-term query rewrite creates 3 `SHOULD` clauses per distinct term (about 450), including "the", "of". The maths filter even adds the words `to` and `in` (`→`, `∈`). | `bm25.rs:213-221`; `tokenize.rs:72,110`. | Medium (latency at 6M chunks (E), to measure; the adoption plan's I0 bench was never built) | Bound the query builder (minimum-should-match, or a stopword list per language, or a term-weight cut). |
| 4.4 | **Chunk text stored in every SQLite row**, including 40-token overlaps. The adoption plan estimates about 10 GB at 10k books versus about 3 GB with compressed per-document text (S: plan section 5, tagged E there). tantivy does not store text (only `chunk_id` is STORED), so the text lives once (in SQLite). | `store.rs:56`; `ingest.rs:856-871`; `bm25.rs:117`. | Medium (disk; the 1 TB vault has room, but the Markdown files already hold the text) | The Markdown file is the only text store. Rows hold `(doc, line range)` only. This is the new design. |
| 4.5 | **One mutex-guarded SQLite connection** serves searches, listings, status and job persistence. | `engine.rs:69,91`; `search.rs:193`; `server.rs:407`; `jobs.rs:216`. | Medium (E; to measure under parallel agentic search) | Read-only pool or per-reader connections (WAL allows concurrent readers). |
| 4.6 | **Process-wide PDFium lock.** `pdfium-render` built without `thread_safe`. Every PDFium call (survey, ingest, rendering) holds one global mutex, and ingest extracts PDFs in path order on one thread. A page render during an ingest waits at most one page. | `extract.rs:27-32`; `ingest.rs:666-696,746`; `Cargo.toml:11-13`; README says this "has not been measured on real books yet" (`:393`). | Medium | Extraction in worker processes (adoption plan I4), or accept it as a fallback path only, since MinerU/GROBID do the heavy extraction. |
| 4.7 | **Embedding client cannot express Qwen3-Embedding's query-side instruction** and does not L2-normalise or truncate Matryoshka dimensions. | `embed.rs:160` (`_input` unused for `openai_compat`); no normalisation in `embed_batch` (`:234-266`). | Medium | Add per-model prefix configuration, normalisation, truncation, and a dimension check against the index. |
| 4.8 | **Titles fall back to the file stem** when the PDF has no metadata Title, and the Title metadata is often wrong in scholarly PDFs. `title` is indexed with a 2x boost in every chunk. | `ingest.rs:631,854`; `bm25.rs:213`; `meta.rs` has no bibliographic data. | Medium (150k papers; names like `1-s2.0-S...-main.pdf`) | Titles come from GROBID or equivalent intake metadata, not from folders or file names. |
| 4.9 | **Triage is unvalidated and English-biased.** Thresholds are "starting points" (S: README). `math_heavy` pages are queued for OCR and the OCR text would *replace* the text layer. The adoption plan claims CJK pages "trip the short-token rule and are classed `math_heavy`" (section 2.2). My reading of `triage.rs:113-128` does not support that: CJK prose has few whitespace tokens, so the share of 1-2-character tokens would be low, not high. The real CJK risk is different (e.g. PDFs that emit one token per glyph, and full-width math forms not counted as signals). Unverified either way, since no CJK page has been run. | `triage.rs`, `ocr.rs:44`, `ingest.rs:606-614`. | Medium | Measure on real CJK and TeX pages before trusting any class. |
| 4.10 | **Chunk ids are `<doc_id>:<ordinal>` and unstable.** `doc_id` is 64 hex characters (BLAKE3 of the source file). The ordinal changes whenever the chunker, its parameters or the extractor changes. Python parses ids with `rpartition(":")` (`engine.py:32-37`), and ids are persisted in practice-card tags (`reading/storage.py:211`) and in reading sessions (`reading/service.py:131,180`). | `ingest.rs:896-898`. | High for the swap (section 8) | Line-range ids are stable only while the Markdown is unchanged. See 6.3. |
| 4.11 | **Silent index loss on a schema change.** `Bm25::open` deletes an index with another schema and creates an empty one, so BM25 search "finds nothing" until the next ingest (documented). | `bm25.rs:166-176`; `README.md:270-279`. The adoption plan already proposes a `degraded[]` flag. | Low-Medium | Rebuild from the Markdown store, and report `reindexing:<percent>` rather than returning empty results. |
| 4.12 | **Two normalisers that claim to be the same** (see the `pages.rs` row in section 3). Neither applies NFKC, and the BM25 analyzer does. | `pages.rs:103-128` vs `normalize.py:13-27`. | Low-Medium (false "cannot verify" results) | One written spec with shared test vectors in Rust and Python. |
| 4.13 | **`text.rs::strip_markup` is a counting helper used as an EPUB extractor.** It turns tags into spaces, ignores MathML, and decodes six named entities. | `text.rs:48`; `extract.rs:197-226` (`BLOCK_ENDS` string replacement). | Medium for EPUB | Real XHTML parsing and MathML-to-LaTeX (the adoption plan proposes xberg's approach, S: its section 2.3). |
| 4.14 | **The crate has no library target.** All modules are declared in `main.rs` (`:1-24`). Nothing else can depend on it. | `RE` file list (no `lib.rs`), `Cargo.toml` (no `[lib]`). | Medium (reuse cost) | Copy modules into the vault workspace, or extract a library first. |
| 4.15 | **README drift.** The README's endpoint list and C3 text do not mention the `Host`/`Origin` request guard that `server.rs:130-168` implements (it was added in commit f587c0f0). | `README.md:74-95` vs `server.rs`. | Low | Treat the code as authoritative. |

---

## 5. Python and evaluation code

### 5.1 `evals/` (T\evals)

Baseline: 66 passed (S: baseline-checks, `pytest evals/tests`, earlier today). I did not rerun them.

- **Metrics** (`tutor_evals/metrics.py`): recall@5, recall@10, mrr@10, ndcg@10 through `ir_measures`, with binary relevance. Each gold entry is credited at most once, by the highest-ranked matching hit (`credit_hits`, `:42-53`).
- **Gold matching** (`metrics.py:25-39`, `datasets/README.md:26-41`): a hit matches a gold entry if the `chunk_id` is equal, or the `doc_id` (or `path`) is equal and the gold `page` lies in `[hit.page_start, hit.page_end]`. Page-less sources (Markdown) can only match by `chunk_id`.
- **Splits** (`split.py:44-62`): by source document, hashed with a seed; a question that has gold in several documents ties them into one group.
- **Harness** (`run_retrieval.py:54-72`): `POST {url}/search` with `{query, k, mode}`, or `POST /api/library/search` for arms A-E. `generate.py:495-507` samples chunks through `/search` and writes gold with `chunk_id`, `doc_id`, `path`, `page`.
- **Status:** no dataset exists yet (`evals/datasets/` holds only a README; M). D7 says the existing test split is untouched and will be renamed *dev*, with a frozen *audit* set to be created.

**Reuse verdict:** keep the bootstrap/CI/compare/split machinery and the one-change-per-round rules. **Port** the matching rule to line ranges: a hit matches a gold entry when `doc_id` is equal and the line ranges overlap (or when the hit's text contains the gold quote). That survives re-chunking, which the adoption plan already proposes (I1). Gold items that carry `chunk_id` or `page` must be regenerated (there are none yet).

### 5.2 `T\deeptutor` library code

The Python side does the intelligence: rewrites, RRF over several lists, the leak guard, the evidence packer, listwise rerank, citation checking. Details and every touch point are in section 8. Of note: `search.py:32` asks for 30 hits per query in `hybrid` mode, and `packer.py` expands a hit to its section by ordinal arithmetic over `/documents/{id}/chunks`.

### 5.3 The legacy `D` workspace (`C:\Users\sethy\OneDrive\Desktop\Tutor`)

A small stdlib-Python tutor scaffold (V1.0/1.1) plus two small Rust crates. It is **not** a base for the vault, and it overlaps nothing the vault needs beyond what the main repo already does better.

- **`search-rs/` (`tutor-search`)**: BM25 with the `bm25` crate (MIT), English stemming, loads every `.md`/`.txt` under `corpus/` plus paper metadata, one JSON line per query line over stdin/stdout (`lib.rs`, `main.rs`). 6 unit tests plus 2 protocol tests (M). No chunking and no addressing: a hit is a whole file.
- **`ui-rust/`**: a std-only terminal menu that shells out to the Python CLI.
- **`tutor/`** (about 1,100 lines of Python): quiz, SM-2, a 60-item question bank, a JSON progress store, an API on `127.0.0.1:8797`, and `eval.py` (hit@k of a topic note for each bank prompt).
- **`corpus/`**: 4 starter notes, a 12-source manifest, `index.json` (12 documents), and empty per-domain `papers/` and `books/` shelves (READMEs only) (M).
- **State:** a baseline run earlier today reported 15 unit tests passing and `eval-search --k 3` at 54/60 = 90.0% (S: baseline-checks; the handoff prompt reports 49/60 = 81.7% from an earlier measurement). I did not rerun them.
- **`OPUS_5.5_PROMPT.md`** is a handoff prompt for that workspace, with its rules: stdlib-only core, all new code in Rust, reuse-first, no vendored copyrighted books, never commit `data/`. Those rules apply to the `D` workspace only, not to the vault.
- **Reusable ideas:** the stdin/stdout batch protocol exists to avoid per-query process launches (machine rule), and `eval.py`'s deterministic split. Nothing to lift as code.

---

## 6. What the vault needs that rag-engine does not provide

### 6.1 Missing capabilities (M, from the module list)
- A canonical Markdown store with a line index, and `read`/`outline`/`grep` by line range.
- A trigram field for regex and corpus-wide grep. tantivy has `NgramTokenizer`, `RegexTokenizer` and `RegexQuery` (S: tantivy source). rag-engine uses none of them and stores no positions.
- Per-language analyzers and language identification.
- Bibliographic metadata, near-duplicate detection, intake quarantine and an integrity manifest.
- MCP (`rmcp`) and gRPC (`tonic`) front ends. Only axum REST exists.
- Embedding and reranker sidecar management (llama.cpp). Only HTTP embedding clients exist, and no reranker.
- A library target, typed errors, and per-machine configuration (the baked-in PDFium path breaks portability).

### 6.2 What rag-engine already does that the vault should keep (the "reuse" list)
1. `dedup::hash_all` (BLAKE3 + rayon).
2. `headings::markdown` (fence-aware ATX parse).
3. `search::rrf` and its tie-break rule.
4. `jobs.rs` (single active job, persisted, interrupted becomes failed).
5. The tantivy writer discipline: one writer, 1 GiB heap, commit every N chunks, merge on 20% deletes, schema-version rebuild from the store.
6. The "query text is never parsed as query syntax" rule (`bm25.rs:4-8`).
7. The maths normaliser as a character filter.
8. PDFium open guards: encryption/DRM refusal without ever supplying a password, panic catching, and the form-XObject image check.
9. `pages.rs` page rendering with a quote highlight, from a quote rather than offsets.
10. `server.rs::require_token` (BLAKE3 digest compare) and `check_bind` (non-loopback requires a token).
11. The retrying embedding client (429/5xx/`Retry-After`) with tests against a local fake server.
12. The ingest patterns listed in section 3 (`ingest.rs` row).
13. The black-box service test harness.
14. `survey` as the measuring tool.

### 6.3 Line addressing: what the existing code tells the lead (decisions not made here)

1. **Chunks are not lines.** `text-splitter` returns `(byte offset, &str)` and splits inside a line (`chunk.rs:89-93`). A vault chunk must be a line range, so either the canonical Markdown is produced with short lines (for example one sentence per line), or chunking snaps to line boundaries and a very long line becomes one oversize chunk. I did not measure line-length distributions of MinerU output. That is a Phase 1 pilot measurement.
2. **What `doc_id` hashes.** rag-engine hashes the source file bytes. If `doc_id` stays a source hash while extraction is re-run (a new MinerU or GROBID version), the line numbers change under the same id, and old `#L..` citations point at the wrong text with no error. The adoption plan's answer is an extraction-version suffix (`@x3`, section 2.6) plus re-anchoring by quote (TextQuoteSelector semantics), neither of which exists in code. The citation checker has to be able to say `re_anchored` or `orphaned`.
3. **What a "line" is.** `headings.rs` and `extract.rs::plain_text` use `'\n'` only and normalise `\r\n` to `\n` (`extract.rs:231`). Rust's `str::lines()` agrees. Python's `str.splitlines()` also splits on `\x0b \x0c \x1c-\x1e \x85 \u2028 \u2029`. A line index shared by Rust and Python needs one definition (split on `\n` only), a BOM rule, and a shared test vector set. This is the same class of bug as the two quote normalisers (4.12).
4. **Overlap.** The 40-token overlap duplicates text across adjacent chunks. With line ranges, overlapping ranges are natural, but BM25 then counts the overlap twice. Whether the vault keeps overlap is a design question.
5. **Section expansion becomes simpler.** The packer's ordinal-neighbour walk (`packer.py:107-121`) and its "same heading path" test (`:86-104`) can become "read the lines of the enclosing heading". The heading tree comes from the Markdown itself.

---

## 7. The prior adoption plan against the new design

Source: `V\docs\phase0\prior-rag-adoption-plan-2026-09-30.md` (read fully; it is a 23-repository study written earlier today). Statuses:
- **Applies**: still holds as written.
- **Applies, changed**: holds with the change noted.
- **Conflicts**: contradicts the new design.
- **Superseded**: the new design answers it differently.
- **Unverified**: not checked by this audit.

| Plan section | Claim | Status | Reason |
|---|---|---|---|
| 1 | Keep tantivy as the lexical engine | **Applies** | Source confirms tantivy 0.26.2 has what the vault needs except CJK segmenters. The vault PROGRESS.md already commits to it. |
| 1 | Disk, not CPU, is the binding constraint | **Applies, changed** | The vault has a 1 TB SSD, not the plan's "about 100 GB free". The plan's per-item budgets (section 5) stay useful; the constraint it names was the free space on the main PC's C: and D: drives (S: `PLAN.md:33`). |
| 1, 2.10 | "On this PC no ML model runs unless the owner relaxes PLAN's no-local-models rule"; models belong on the GPU desktop | **Conflicts** | The new design allows the RTX 5090 for embedding in Stage 1 and a Vulkan llama.cpp on the AI PC (RX 7800 XT 16 GB) in Stage 2. `PLAN.md:31` ("no local models, and the RTX 5090 is off-limits (Cinference owns it)") and CORPUS.md's owner policy (embeddings and OCR "later on the owner's second desktop") say the opposite and will need the owner's amendment. |
| 1, 4 | Dense embedding of everything on the CPU is rejected (30-215 days for 10M chunks, sweep) | **Applies, but moot** | The reasoning is about CPU. With the 5090 available the plan's own GPU figure applies (about 10 hours for 6M chunks with Qwen3-Embedding-0.6B at 60 TFLOPS effective), which is an estimate from the plan and not measured. The vault's embedding benchmark (`PROGRESS.md`) will replace it. |
| 2.1 | Keep size+mtime pre-check, BLAKE3 `doc_id`, removal only after a complete walk, crash repair | **Applies** | Confirmed in code (`ingest.rs:229`, `:246`, `:564-596`). |
| 2.1 | Per-stage fingerprints (`extract_fp`, `chunk_fp`, `index_fp`, `embed_fp`, `ocr_fp`), a failure ledger, tombstones, source-version lineage | **Applies** | Confirmed that only the BM25 schema version and the embedding model are tracked today. Matters more with a Markdown store: re-extraction changes lines. |
| 2.2 | Page routing with pdf-inspector, four triage holes | **Applies, changed; partly unverified** | Routing is still needed to decide MinerU vs GROBID vs PDFium text. The claim that CJK pages trip the short-token rule is contradicted by my reading (4.9). pdf-inspector's version, licence and detector claims were not checked. |
| 2.3 | PDFium extraction in long-lived worker processes (6 by default, cores minus 2) | **Applies, changed** | The plan sized this for 8 cores. The AI PC has 6 cores (Ryzen 5 5600X), so cores minus 2 gives 4 there. Whether PDFium is still the main extractor or only a probe depends on MinerU/GROBID doing the canonical conversion. |
| 2.3 | EPUB: rbook plus MathML to LaTeX, DRM refusal, archive limits | **Applies** | Needed if EPUB is a vault input. The code today drops MathML (4.13). |
| 2.4 | Semantic blocks with a block schema, labelled environments, proof end markers | **Applies, changed** | The vault gets headings, tables and LaTeX from MinerU Markdown. Block detection becomes Markdown-syntax-driven (fences, `$$`, pipe tables), which the plan lists in its own v1. |
| 2.5 | text-splitter 0.33 with cl100k at 400/40 | **Applies, changed** | Sizing engine stays, but its output must be snapped to lines (6.3). The sizing tokenizer should be checked against the embedding model's tokenizer. |
| 2.5 | Block-aware pre-pass, parent expansion via heading tree, `GET /chunks/{id}?expand=` | **Applies, changed** | Expansion is "read the enclosing heading's lines". The route disappears into MCP/gRPC tools. |
| 2.6 | **Provenance by page and code-point offsets; chunk id = span id `d3f9...:p12.1043-p13.220@x3`; canonical text = PDFium page text stored once in SQLite `doc_text`** | **Conflicts / superseded** | The vault's text is Markdown on disk addressed by lines. The *principle* ("text stored once; chunk rows hold spans only; ids survive re-chunking; resolve a stale span by quote") still applies and fits better. The page/offset format, the `doc_text` SQLite blob and PDFium char bounds as the primary provenance do not. `pages.rs`-style quote location remains useful for showing the source PDF page. |
| 2.6 | Integer `chunk_key` as the internal key (AUTOINCREMENT, never reused) | **Applies** | Needed for tantivy fast fields and the vector store. It must be derived from or mapped to `(doc, line range)`. |
| 2.6 | Re-anchoring by quote with exact/prefix/suffix (TextQuoteSelector), status `exact`, `re_anchored`, `orphaned` | **Applies** | Essential once line ranges can drift (6.3.2). |
| 2.7 | Rights record per document and purposes (`index`, `display_quote`, `lesson_llm`, ...) | **Unverified for the new design** | The vault brief I was given does not mention it. D1 (owner policy) still stands in the tutor. The lead should decide whether rights live in the vault or stay in the tutor. |
| 2.8 | Per-language analyzers, language per chunk, `cjk2` bigram field, dictionary arms later | **Applies** | This is the most directly reusable plan section. The vault's trigram field is for regex and does not replace it. |
| 2.9 | LanceDB 0.37.1 with IVF_RQ, f16, MRL, scalar indexes | **Applies, changed** | The vault benchmarks "vector index candidates A and B" on synthetic 6M data. Do not presuppose LanceDB. The plan's arithmetic (1024d f16 about 12 GB for 6M rows) is independent of that choice. |
| 2.10 | TEI/vLLM/Python sidecar, `modelhost.rs` with capability negotiation, `embed_fp` | **Conflicts, with a reusable idea** | The new design uses llama.cpp (Qwen3-Embedding, Qwen3-Reranker), GROBID and MinerU. Capability negotiation and fingerprinting the model as part of the index key remain good, and `embed_fp` should include model, dimensions, dtype and normalisation. |
| 2.11 | Several rewrites per request, fusion arms, leak guard, rerank | **Applies** | The vault is called by an agent over MCP, so the rewrite/leak/packer logic stays in the tutor (`deeptutor/services/library`). The vault's `search` should accept a bounded multi-query request. |
| 2.12 | Degraded states reported in every response | **Applies** | Cheap and prevents the silent empty-index behaviour (4.11). |
| 2.13 | Eval v2: gold with doc, page, quote, span; dev vs frozen audit; slices; `bench` at 1M and 10M; `parse-snapshot` | **Applies, changed** | Gold becomes `{doc_id, line range, quote}` and matching becomes line-range overlap or quote containment (5.1). None of `bench` or `parse-snapshot` exists yet. |
| 3 | The 18-increment work plan I0-I17 | **Superseded** | The vault has its own phases (`V\PROGRESS.md`). Many increments become Phase 1-5 tasks of the vault. None exists in code. |
| 4 | Rejected: whole-library graph extraction, full ColBERT, LEANN, SeekStorm, Lance FTS, A VLM on every page, restricted licences | **Applies** | Nothing in the new design reopens these. "A VLM on every page" is now a cost question for MinerU on the 5090. The plan's estimate (8-25 GPU-days at 2-6 pages/s) uses vendor figures and predates the 5090's availability. |
| 5 | Disk and time budgets | **Applies, changed** | The vault's target is 150k+ papers and 10k+ books, about 2.5B tokens and about 6M chunks. The plan's 10k-book budget used about 2B tokens and about 6M chunks, so it is the right order of magnitude. Time figures assumed an 8-core extraction host and CPU-only models. Redo them for the 5090 and the AI PC. |
| 6 | Open question 2: where the GPU desktop sits | **Answered** | The SSD moves from the main PC (Stage 1) to the AI PC (Stage 2). |
| Verified by code | `load_hits` joins on `state = 'indexed'` | **Confirmed** | `search.rs:195-198` |
| Verified by code | A CJK run is one token that `RemoveLongFilter(40)` then drops | **Confirmed, with nuance** | Bytes, not characters; shorter runs stay as one whole-run token (4.1). |
| Verified by code | Writer heap is 1 GiB | **Confirmed** | `bm25.rs:51` |
| Verified by code | Today's design stores chunk text per row | **Confirmed** | `store.rs:56` |
| Verified by code | Eval gold matches on `chunk_id` or doc plus page | **Confirmed** | `metrics.py:25-39` |
| Verified by code | `math_heavy` pages go to the OCR queue | **Confirmed** | `ocr.rs:44-46` |
| Unverified | pdf-inspector 1.25.2 behaviour, jieba/lindera claims, xberg ports, Lance defaults ("IVF_RQ silently defaults to 1 bit") | **Unverified** | Outside this audit. I only confirmed `num_bits` is an `Option<u32>` in lancedb 0.37.1. |

---

## 8. DeepTutor integration points a later swap to `libraryd` would touch

The swap changes the engine behind `RAG_ENGINE_URL` (default `http://127.0.0.1:8811`, `RAG_ENGINE_TOKEN` bearer). Every point below depends on one or more of: the REST shape (`/search`, `/documents`, `/documents/{id}/chunks`, `/documents/{id}/pages/{n}.png`, `/status`, `/ingest`, `/survey`, `/jobs`), the chunk id shape `<doc_id>:<ordinal>`, or the fields `page_start`, `page_end`, `heading_path`.

### 8.1 Python backend (T\deeptutor)

| # | File:line | Dependency | What changes |
|---|---|---|---|
| 1 | `services/rag/pipelines/tutor_library/pipeline.py:30-66, 171-180, 201-248` | URL and token env vars; `POST /search` with `{query, k, mode:"hybrid", domains, source_types}`; response must contain a `hits` list (`:245`); 401 maps to `not_configured` | Transport: REST to gRPC or MCP. Request: `mode` and `source_types` semantics. Response mapping. |
| 2 | same, `:104-142` (`_page_label`, `_source`, `_citation`) | Builds the citation header as "p. {page}" plus heading path | Pages become line ranges (or a derived page label if the vault keeps one). |
| 3 | `services/rag/factory.py:192`; `knowledge/manager.py:1197-1221`; `knowledge/kb_types.py:106`; `knowledge/manifest.py:51,77`; `api/routers/knowledge.py:2650-2661, 3524` | KB type `tutor_library`: a pointer KB with optional `domains` and `source_types` filters | Filter vocabulary (`SOURCE_TYPES`: book/paper/note/unknown from `meta.rs`). |
| 4 | `services/library/engine.py:32-37, 48-104` | `split_chunk_id` (`rpartition(":")`), `RagEngine.chunks()` via `GET /documents/{id}/chunks?from&limit`, `RagEngine.chunk()` | Replace chunk-by-ordinal reads with line-range reads. The id parser changes with the id format. |
| 5 | `services/library/search.py:32, 71-127` | `CANDIDATES_PER_QUERY = 30`, hybrid, 3-5 rewrites in parallel, leak guard per hit, RRF in Python, agentic `open_chunk` (`OPEN_CHUNK_CHARS`) | Candidate size, multi-query batching (the vault could take the rewrites in one request), `mode_used`. |
| 6 | `services/library/packer.py:54, 86-121` | Expands a hit by ordinal neighbours that share a heading path; `[c:<chunk_id>]` tags | Re-implement section expansion over line ranges. Tags carry the new id. |
| 7 | `services/library/leak.py` (232 lines) | Reads hit text and `heading_path` for solution headings in six languages | Needs the heading path (or heading text) per hit. |
| 8 | `services/library/verify.py:19-29`; `citations.py:22`; `normalize.py:13-27` | L0 check: normalised quote must be a substring of the chunk text; page must lie in `[page_start, page_end]`; the citation regex accepts `[c:<id>]` with `[^\]\s]+` | Quote check reads the cited lines; `page_out_of_range` becomes a line-range check. Add re-anchoring (6.3.2). The regex accepts `#` and `-`, so `{doc}#L1-L9` parses. |
| 9 | `api/routers/library.py:65-108` | `POST /api/library/search`, `/verify`, `/settings`; C11 contract (`CONTRACTS.md:368-423`) | The outward C11 contract can stay if the Python layer adapts. Hits are "C1 hits", so C1 (`CONTRACTS.md:17-72`) and C11's hit fields change. |
| 10 | `agents/chat/agentic_pipeline.py:62-97` | A plain chat round is buffered until citations are verified through `RagEngine().chunk` | Latency of citation verification (one engine read per distinct cited chunk). |
| 11 | `services/reading/engine.py` (92 lines), `service.py:98-131,170-180`, `section.py:21-68`, `payloads.py`, `storage.py:176-234`, `api/routers/guided_reading.py:46-101` | `GET /documents/{id}` (outline) and `/chunks?from=<ordinal>&limit=` with a `next` cursor. A reading session stores `chunk_ids` and rejects a resume when the section's chunk ids differ (`service.py:180`). Atoms accepted into practice cards carry `chunk_id` and the tag `chunk:{chunk_id}` (`storage.py:211`) | **Persisted ids.** Existing sessions and practice cards (none if no library is ingested yet) would need a migration or an alias table. Section selection by pages (`section.py`: 5-15 pages) must be redefined without `page_start`. |

### 8.2 Evals (T\evals)

`tutor_evals/run_retrieval.py:54-72` (REST `/search`), `metrics.py:25-39` (gold on `chunk_id`/page), `generate.py:248-275, 495-507` (samples via `/search`; reads `chunk_id`, `path`, `text`, `page_start`), `split.py:44-62` (groups by `doc_id` and `path`), `datasets/README.md`. Changes are listed in 5.1.

### 8.3 Desktop shell (T\apps\desktop)

| File:line | Dependency |
|---|---|
| `src-tauri/src/services.rs:256-279` | Spawns `rag-engine.exe serve --corpus <T>\corpus --notes <T>\content\notes --data <T>\data\rag --port 8811`; supervises it as service `rag_engine`; passes `VOYAGE_API_KEY` from Credential Manager (`:52`) |
| `src-tauri/src/lib.rs:293-294`, `tauri.conf.json:34-35` | CSP `connect-src` for `http://127.0.0.1:8811` and `ws://127.0.0.1:8811` |
| `src/lib/backend.ts:6` | `RAG_ENGINE_URL = "http://127.0.0.1:8811"` |
| `src/features/library/useLibrary.ts:24-95` | `/status`, `/survey/latest`, `/jobs/{id}`, `POST /ingest`, `POST /survey` |
| `src/features/reading/api.ts:47-71` | `/status`, `/documents?...`, `/documents/{id}`, `/documents/{id}/chunks?from&limit`, `pageImageUrl` = `/documents/{id}/pages/{n}.png?w=&highlight=` (an `<img>` request, with no `Origin`) |
| `src/features/voice/tools.ts:278-282` | `search_library` goes through the backend `/api/library/search`, not to the engine directly |
| `src/features/reading/*.tsx` | Use `page_start`, `page_end`, `heading_path` and `chunk_id` in UI types (`types.ts`, `ReaderPane.tsx`, `reader.ts`) |

The Library screen's status, job and ingest controls assume rag-engine owns ingestion. With the vault, intake is a separate pipeline. That UI has no counterpart yet.

### 8.4 Contracts and documents that name the current shape
`T\docs\tutor\CONTRACTS.md` C1 (`:17-72`), C9 (`:290-315`), C11 (`:368-423`, including "Chunk ids are `<doc_id>:<ordinal>`" at `:407`); `T\docs\tutor\CORPUS.md` Stage 0 (the packer, leak guard, citation check), Stage 2 (second desktop), Stage 3; `T\docs\tutor\PLAN.md:31`, D1, D7-D9, D13; `T\AGENTS.md` if it repeats these.

---

## 9. Open items for the lead (recorded, not decided)

1. **Line granularity and canonical Markdown form** (6.3.1): what guarantees a maximum line length, if anything.
2. **What `doc_id` hashes and how extraction versions are named** (6.3.2).
3. **One definition of a line, a normaliser and an NFKC rule**, with shared Rust and Python test vectors (6.3.3, 4.12).
4. **Whether rag-engine modules are copied into the vault workspace or first extracted as a library crate** (4.14).
5. **The `PLAN.md:31` / CORPUS owner-policy amendment** for the 5090 and the second desktop's role.
6. **Rights per purpose** (adoption plan 2.7): vault feature or tutor feature.
7. **Whether the 125 + 6 existing tests run clean today**: not run in this audit. The baseline shows 182 passed at d48dce1f for the whole workspace. A single batched `cargo test -p rag-engine` run would confirm it (the PDFium DLL and protoc are in place; M).
8. **A real-data PDFium throughput and a CJK triage measurement** are the two numbers the audit could not produce and the plan relies on (pages per second per process; classes on CJK pages).
9. **Check what happens to `data/knowledge_bases` entries and any reading sessions or practice cards** that reference chunk ids before the swap. `T\data\rag` does not exist and `T\corpus` holds only `.gitkeep` files (M), so nothing has been ingested yet and nothing persisted from a library ingest. Practice cards and reading sessions from the owner's use of the Python backend were not inspected.
