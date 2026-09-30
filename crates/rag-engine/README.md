# rag-engine

Rust retrieval engine for the tutor's library of textbooks, papers and notes. It has four
commands:

- `survey` measures a corpus (files, duplicates, pages needing OCR, tokens per GB).
- `serve` runs the HTTP service of contracts C1 and C9 (`docs/tutor/CONTRACTS.md`).
- `ingest` builds and updates the indexes incrementally.
- `search` queries them from the command line.

It is built for a 50-250 GB library (up to about 10M chunks) on a machine with about 20 GB of
usable RAM. Documents are extracted and chunked in parallel with a bounded number in memory,
stored in order, and committed in batches. Indexes live on disk, and the corpus is only ever
read.

## Setup

1. Rust stable (built and tested with 1.98.1 on Windows x64, MSVC toolchain).
2. **PDFium**, bound at runtime. Download the Windows x64 build from
   [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases)
   (tested: `chromium/8076`, asset `pdfium-win-x64.tgz`) and unpack it into `vendor/pdfium/`
   so that `vendor/pdfium/bin/pdfium.dll` exists. `vendor/` is gitignored.

   ```powershell
   Invoke-WebRequest https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8076/pdfium-win-x64.tgz -OutFile pdfium.tgz
   New-Item -ItemType Directory -Force vendor/pdfium | Out-Null
   tar -xzf pdfium.tgz -C vendor/pdfium; Remove-Item pdfium.tgz
   ```

   Another location can be given with `--pdfium <file-or-dir>` or the
   `PDFIUM_DYNAMIC_LIB_PATH` environment variable (the flag wins).
3. **protoc** (build time only). LanceDB's `lance` crates generate code with prost-build, which
   runs the Protocol Buffers compiler. Put a prebuilt one in `vendor/protoc/`:

   ```powershell
   Invoke-WebRequest https://github.com/protocolbuffers/protobuf/releases/download/v36.2/protoc-36.2-win64.zip -OutFile protoc.zip
   Expand-Archive protoc.zip vendor/protoc; Remove-Item protoc.zip
   ```

   The workspace's `.cargo/config.toml` points `PROTOC` at `vendor/protoc/bin/protoc.exe`.
   A `PROTOC` already set in the environment takes precedence, which is how other platforms
   or a system-wide protoc can be used.

The first build compiles LanceDB with Arrow and DataFusion, which takes about 5 minutes (debug)
on this machine.

## Commands

```powershell
cargo build -p rag-engine --release
$exe = "target/release/rag-engine"

# Measure a corpus (JSON report to a file, or stdout without --out).
& $exe survey --root corpus --out data/rag/survey.json

# The service on http://127.0.0.1:8811 (--port 0 picks a free port; the address is logged).
& $exe serve --corpus corpus --notes content/notes --data data/rag

# On the library machine, reachable from the LAN: a token is required.
$env:RAG_ENGINE_TOKEN = "<long random secret>"
& $exe serve --corpus corpus --notes content/notes --data data/rag --bind 0.0.0.0

# One ingest in the foreground; prints a JSON summary. --root limits the walk to one folder.
& $exe ingest --corpus corpus --notes content/notes --data data/rag [--root corpus/math] [--rebuild]

# Search (same JSON as POST /search).
& $exe search --data data/rag --query "one-sided limits" --k 5 --domain math --source-type book --mode bm25
```

Logs go to stderr. Only one process may ingest into a data directory at a time, because
tantivy's writer lock refuses a second one. Stop `serve` before running `ingest` from the
command line on the same `--data`.

## Service (contracts C1 and C9)

`serve` binds `127.0.0.1` unless `--bind <ip>` names another address. It allows CORS for
exactly the app's four origins: `http://localhost:1420`, `http://127.0.0.1:1420`,
`http://tauri.localhost` and `tauri://localhost` (methods `GET` and `POST`, headers
`Content-Type` and `Authorization`).

**Token.** With `RAG_ENGINE_TOKEN` set (surrounding whitespace is ignored; empty means unset),
every route except `GET /health` needs `Authorization: Bearer <token>`, including unknown
routes, or it answers `401 {"error": ...}`.

- The token is compared as BLAKE3 digests, which is constant-time and does not reveal its
  length.
- It is never logged. The log says only whether a token is required.
- CORS runs before the check, so browser preflights, which carry no token, still succeed, and
  a 401 still carries the CORS headers.
- A `--bind` that is not a loopback address (for example `0.0.0.0` or a LAN address) refuses to
  start without a token.
- The token is the only protection: the service speaks plain HTTP, so on an untrusted network
  put it behind TLS (a reverse proxy or a VPN such as Tailscale).

| Endpoint | Behaviour |
|---|---|
| `GET /health` | `{"ok": true, "version": "0.1.0"}` |
| `GET /status` | counts by domain, source type and page class, chunks and how many are embedded, OCR queue, error documents, the embedding configuration, the active job, and spend |
| `POST /survey {"root"?}` | starts a survey job (default root: `--corpus`); `202 {"job_id"}` |
| `GET /survey/latest` | the last survey report (`survey` JSON), or 404 |
| `POST /ingest {"root"?, "rebuild"?}` | starts an ingest job; `root` must be a folder inside the corpus or notes root |
| `GET /jobs/{job_id}` | `{"job_id", "kind", "state", "progress": {"done", "total"}, "started_at", "finished_at", "error"}` |
| `POST /search` | `{"query", "k"?, "domains"?, "source_types"?, "mode"?}`; see Search |
| `GET /documents` | `?domain=&source_type=&q=&sort=title\|added\|pages&limit=50&offset=0`; `{"total", "documents": [Doc]}` |
| `GET /documents/{doc_id}` | `Doc` plus `outline: [{"heading_path", "page_start", "page_end", "chunk_id"}]` |
| `GET /documents/{doc_id}/chunks` | `?from=<ordinal>&limit=20`; `{"chunks": [{"chunk_id", "ordinal", "text", "page_start", "page_end", "heading_path"}], "next"}` |

### Library browsing (`src/library.rs`)

- `Doc` is `{doc_id, path, title, domain, source_type, pages, chunks, bytes, added_at,
  ocr_pending}`.
  - `pages` is 0 for EPUB, Markdown and text.
  - `added_at` is when the document's current content was indexed (epoch seconds).
  - `ocr_pending` is the number of its pages waiting for OCR.
- Only indexed documents are listed. Unreadable ones are counted in `/status` `errors`, and a
  document being ingested appears once its batch is committed.
- `GET /documents` filters and sorts:
  - `domain` and `source_type` filter exactly. The domain ignores case.
  - `q` keeps documents whose title or stored path contains it, ignoring ASCII case; `%` and
    `_` are literal.
  - `sort=title` (the default) sorts A to Z ignoring ASCII case, `added` newest first, and
    `pages` most pages first. Ties are broken by title, then `doc_id`, so pages are stable.
  - `limit` is 1-200 (default 50) and `offset` counts from 0. `total` counts every match.
  - Empty values mean "not given".
- `outline` lists each distinct, non-empty heading path of the document's chunks, in reading
  order. Each entry gives the first chunk under the path, and pages from that chunk to the end
  of its run. Chunks before the first heading are not listed, and a path that recurs later
  keeps its first entry.
- `/chunks` returns chunks with `ordinal >= from` in reading order. `limit` is 1-200 (default
  20). `next` is the `from` for the next page, or `null` after the last chunk.
- These routes use the indexes `documents(state, title COLLATE NOCASE)`,
  `documents(state, indexed_at)`, `documents(state, pages)` and `chunks(doc_id, ordinal)`. The
  last one replaces `chunks(doc_id)`, and is built once when an existing database is first
  opened.

One job runs at a time. A second `POST /survey` or `POST /ingest` gets
`409 {"error": "job <id> is still running"}`. Jobs are stored in SQLite, so finished jobs stay
readable after a restart, and a job that was running when the service stopped is recorded as
failed ("interrupted"). An ingest's `progress.total` grows as it learns about more work: first
the documents, then any OCR pages, then the chunks to embed.

Errors are `{"error": "<message>"}`. The statuses are:

- 400: invalid JSON, unknown fields or query parameters, an empty query, `k` outside 1..100,
  a `limit` outside 1..200, an unknown `sort` or `source_type`, or a `root` outside the roots.
- 401: a token is configured and the request does not carry it (every route but
  `GET /health`).
- 404: an unknown job, route, survey or document.
- 409: a job is already running, or `mode: "vector"` was asked for without vectors.
- 502: the embedding provider failed on the query.
- 500: anything else.

An empty request body for `/survey` and `/ingest` means the defaults.

### Paths and metadata

Stored and reported paths start with `corpus/` or `notes/`, whatever the root folders are
called. Folder names match case-insensitively and domains are lowercased.

| Location | domain | source_type |
|---|---|---|
| `corpus/<domain>/books/**` | `<domain>` | `book` |
| `corpus/<domain>/papers/**` | `<domain>` | `paper` |
| `corpus/inbox/**` | `unsorted` | `unknown` |
| `corpus/<domain>/<other>/**`, `corpus/<domain>/<file>` | `<domain>` | `unknown` |
| `corpus/<file>` | `unsorted` | `unknown` |
| `notes/<domain>/**` | `<domain>` | `note` |
| `notes/<file>` | `unsorted` | `note` |

Byte-identical copies are one document (`doc_id` is the BLAKE3 hash of the content). Its
canonical path, which decides the metadata, is the first of its paths in sorted order.

## Environment variables

| Variable | Meaning |
|---|---|
| `RAG_EMBED_PROVIDER` | `voyage` or `openai_compat`. Unset: no embeddings, and search runs BM25 only. |
| `RAG_EMBED_MODEL` | Model name; required with a provider. |
| `RAG_EMBED_API_KEY` | API key. `voyage` also reads `VOYAGE_API_KEY`; `openai_compat` sends no key when unset. |
| `RAG_EMBED_BASE_URL` | API base (requests go to `{base}/embeddings`). Required for `openai_compat`; `voyage` defaults to `https://api.voyageai.com/v1`. |
| `RAG_EMBED_PRICE_PER_MTOK` | USD per million tokens, used for spend accounting. Unset: tokens are recorded and spend shows $0 (a warning is logged). |
| `RAG_OCR_PROVIDER` | OCR provider. None is implemented yet, so any value is rejected at startup. Unset: pages stay queued. |
| `PDFIUM_DYNAMIC_LIB_PATH` | PDFium library or folder (same as `--pdfium`). |
| `RAG_ENGINE_TOKEN` | Bearer token that `serve` requires on every route but `GET /health`. Needed for a non-loopback `--bind`. Read directly, so it never appears in `--help`. |

A configuration error, such as a provider without a model, stops the service at startup
rather than being ignored. Corpus text goes only to the configured provider. Check its
data-retention terms before the first ingest; Voyage trains on customer data unless you opt
out.

## How ingest works (`src/ingest.rs`)

1. **Walk** the roots, or the requested folder. A file whose size and modification time match
   the `files` table keeps its recorded hash. Other files are hashed in parallel.
2. **Remove** documents that no file points to any more: from BM25, from the vector store,
   then from SQLite. Nothing is removed in a run whose walk reported errors, so an unreadable
   or unplugged folder never empties the index. A scoped ingest only notices deletions inside
   its folder.
3. **Move**: a document whose canonical path changed keeps its chunks and embeddings. Its
   domain and source type are rewritten in both indexes, so sorting a book out of `inbox/`
   costs no re-embedding.
4. **Index** new documents in a pipeline:
   - Extract the PDF text layer (with page triage), the EPUB spine documents, or Markdown or
     plain text. PDFs are extracted in path order on one thread, because PDFium is not
     thread-safe. Everything else is extracted on worker threads.
   - Chunk (and tokenise) the text on the worker threads. By default there is one worker per
     core, minus two for the writer and the PDF thread, up to 16.
   - At most two documents per worker are in memory. A document takes a permit before it is
     extracted and returns it once it is stored.
   - On the ingest thread, in path order, store the document, its chunks and its OCR-queue
     pages in one SQLite transaction. The result is the same as a serial ingest: a test
     compares every stored chunk and document row.
   - A parser or chunker panic becomes that document's error.
   - Add the chunks to the BM25 batch. The batch is committed every 64 documents or 20,000
     chunks.
   - A document stays `indexing` until its batch is committed and then becomes `indexed`. An
     interrupted batch is undone and redone by the next ingest.
   - Encrypted or unreadable files become `error` documents (counted in `/status` `errors`).
     They are not retried until they change or `rebuild` is used.
5. **OCR**: with a provider, pending pages are recognised and their documents re-indexed with
   the OCR text in place of those pages' text layer. Without one, the pages stay pending, and
   whatever text layer they have is indexed.
6. **Embed**: with a provider, chunks that are not in the vector store yet are embedded in
   requests of 128 and appended to LanceDB in groups of 1,024. They are marked in SQLite only
   after the append succeeds, so an interrupted run repairs itself. Throttling (429) and 5xx
   responses are retried with exponential backoff: 1 s doubling, six attempts, and the
   server's `Retry-After` when it sends one. Every request's billed tokens are recorded as
   spend. A different provider or model than the stored vectors fails the job with a request
   to run `rebuild`.
7. **Maintain** the vector store: compact it, and build the IVF-PQ index (cosine) once it
   holds 100,000 vectors.

`rebuild: true` clears the database's documents, chunks and files, the BM25 index and the
vector table, then ingests everything again. Finished OCR text is kept, because it is paid for
and keyed by content.

### Chunks and headings

- **Chunking** (`src/chunk.rs`): `text-splitter` with the cl100k tokenizer (`tiktoken-rs`).
  - Chunks are at most 400 tokens, and Markdown files are split with its Markdown splitter.
  - Consecutive chunks overlap by up to 40 tokens, in whole sentences or words at the level the
    text was split. A chunk that ends at a paragraph or page break longer than that is not
    overlapped.
  - PDF pages are joined with a blank line. A chunk can span a page break, and then
    `page_start` and `page_end` differ.
  - EPUB, Markdown and text chunks have `null` pages.
  - `chunk_id` is `<doc_id>:<ordinal>`.
- **Heading paths** (`src/headings.rs`) are the headings in force where a chunk starts.
  - Markdown: ATX headings (`#` to `######`) outside code fences.
  - EPUB: the table-of-contents path of each spine document (chapter titles).
  - PDF uses a documented heuristic. A heading is a line of at most 80 characters and 12 words,
    followed by a body line of at least 40 characters. It must be one of:
    - `Chapter|Part|Section|Appendix|Unit|Lesson|Lecture <n>` (level 1);
    - a numbered heading `3`, `3.2` or `3.2.1` followed by a capitalised word (the level is the
      number of parts);
    - a title-cased line, which nests under the last numbered heading.

    Run-in labels such as "Proof" or "Example" are not headings. Running headers are ignored:
    lines at the top or bottom of 3 or more pages, compared without page numbers.

## Search (`src/search.rs`)

| `mode` | What runs | `score` |
|---|---|---|
| `bm25` | tantivy, English stemming analyzer (`en_stem`), top `k` | BM25 |
| `vector` | LanceDB, cosine distance, top `k` | cosine similarity (1 - distance) |
| `hybrid` (default) | top 50 of each, reciprocal rank fusion `sum 1/(60 + rank)` | fused score |

- `hybrid` runs as `bm25` when no provider is configured or no vectors exist for the configured
  model, and `mode_used` says what ran.
- `ranks` gives each hit's 1-based rank in each list, or `null` where the hit was not in that
  list.
- `domains` and `source_types` filter inside both engines (tantivy term filters and LanceDB
  pre-filtering). An empty list means no filter.
- Query text is not parsed as query syntax. It goes through the analyzer, and every distinct
  term becomes an optional clause, so input like `f(x) = a:b` cannot fail to parse.
- Query embeddings are billed as spend too.

## Data directory (`--data`)

| Path | What |
|---|---|
| `rag.sqlite3` | files, documents (with page counts by class), chunks (text, pages, heading path), OCR queue, jobs, spend, meta (WAL mode) |
| `bm25/` | tantivy index: chunk text (frequencies, no positions) and filter fields |
| `lancedb/` | LanceDB table `chunks` (chunk_id, doc_id, domain, source_type, vector). It is created only when an embedding provider has produced vectors. |
| `survey/latest.json` | the last survey report from the service |

The service refuses a `--data` inside `--corpus` or `--notes`.

## Scale and memory

- Memory holds the documents in flight (at most two per worker thread, each with its text and
  chunks), one BM25 batch, and up to 1,024 vectors. The tantivy writer has a 256 MB budget,
  and SQLite has a 64 MB page cache. The file list (one small entry per file) is the only
  structure that grows with the corpus.
- `/status` sums per-document counters instead of counting chunk rows, so it stays fast at
  10M chunks. Chunks waiting for embeddings are counted through a partial index.
- There is no per-page table. Only pages that need OCR get a row.
- BM25 stores no term positions, and the chunk text lives once, in SQLite.
- Vectors stay on disk in LanceDB. With the IVF-PQ index, searches do not scan all vectors.
- PDFium is not thread-safe, so PDF text extraction runs on one thread. Other extraction,
  chunking and hashing are parallel. For a PDF-heavy library, that one thread is the likely
  limit; this has not been measured on real books yet.
- **Throughput** (release build, 16-core Windows 11 machine, BM25 only, no embeddings). The
  corpus is `bench/synthetic_corpus.py`: 200 Markdown books, 33.7 MB, 38,648 chunks. Each
  ingest started from an empty `--data`, and the numbers are two runs each:

  | Build | Ingest time | Throughput | Peak working set |
  |---|---|---|---|
  | Serial (before C9) | 23.3-23.4 s | 1.44 MB/s | 149-164 MB |
  | Parallel (14 workers) | 5.0-5.1 s | 6.6-6.7 MB/s | 159-174 MB |

  Both builds stored the same 38,648 chunks.

## What survey measures

- **Discovery**: a recursive walk. `.pdf`, `.epub`, `.md` and `.txt` (any case) are
  documents. Every other file is counted by extension from its metadata and never read.
- **Exact dedup**: a BLAKE3 hash of every document, hashed in parallel. Each group of identical
  files is measured once. The first path in sorted order is kept, and the other copies get
  `duplicate_of` and copied measurements.
- **PDF**: the text layer of each page, via PDFium, one document at a time (PDFium is not
  thread-safe). Encrypted PDFs are reported as errors and never opened with a password. This
  includes PDFs protected only by an owner password (permission restrictions) and those using
  DRM security handlers.
- **EPUB**: the text of every spine document via `rbook`, with markup stripped: tags removed,
  `script` and `style` dropped, entities decoded.
- **MD/TXT**: read as UTF-8, with invalid bytes replaced.
- **Sizing**: `characters` is the text length after collapsing whitespace runs to one space.
  `whitespace_tokens` counts whitespace-separated tokens, and `tokens_est = characters / 4`.

A failure in one file (including a parser panic) is recorded on that file's record, and the
run continues.

### Page triage (`src/triage.rs`)

| Class | Rule |
|---|---|
| `scanned` | fewer than `MIN_TEXT_CHARS_PER_PAGE` (100) meaningful characters, and the page has an image object (including inside form XObjects) |
| `empty` | fewer than 100 meaningful characters and no image object |
| `math_heavy` | at least 100 meaningful characters, and either math signals are at least `MATH_CHAR_RATIO_MIN` (3%) of them, or 1-2 character tokens are at least `SHORT_TOKEN_SHARE_MIN` (40%) of all tokens |
| `born_digital` | every other page with at least 100 meaningful characters |

"Meaningful" excludes whitespace, control and zero-width characters, U+FFFD and Private Use
Area code points. Math signals are `= + - / ^ _`, `± × ÷ ¹ ² ³`, Greek and Coptic,
super/subscripts, letterlike symbols, the mathematical operator and symbol blocks, and
mathematical alphanumerics. An OCR'd scan with an invisible text layer counts as
`born_digital`. The thresholds are starting points and have not been validated on real books
yet. The values used are written into each report under `triage_thresholds`. Ingest uses the
same triage: `scanned` and `math_heavy` pages go to the OCR queue.

### Report

- `totals`: `files`, `bytes` and `by_type` count every copy, and `other_by_extension` counts
  the unread files. From `unique_files` down, figures are per unique content: unique
  files and bytes, duplicate groups, files and bytes, pages and `pages_by_class`, characters,
  tokens, and errors with their bytes. Directory-walk errors are included in `errors`.
- `extrapolation`: `tokens_est_per_gb` and `pages_per_gb` over the bytes of unique documents
  measured without error (GB = 10^9 bytes), and projections for 50 GB and 250 GB. Pages come
  from PDFs only, but the denominator includes EPUB and text bytes.
- `duplicate_groups`, `walk_errors`, and `files` (one record per document: path, kind, bytes,
  blake3, duplicate_of, pages, triage, characters, whitespace_tokens, tokens_est, error).

## Tests

```powershell
cargo test -p rag-engine
cargo clippy -p rag-engine --all-targets
```

- Unit tests (in `src/`) cover:
  - path-to-metadata rules, heading heuristics, chunk token limits, overlap and page ranges;
  - RRF fusion and request validation;
  - incremental ingest: unchanged, changed, moved, deleted, duplicate, scoped, interrupted,
    rebuild and unreadable documents;
  - parallel ingest stores the same chunk, document and OCR-queue rows and BM25 hits as serial
    ingest, over every fixture type plus Markdown of uneven sizes;
  - the pipeline stores in order and stops promptly when storing fails;
  - library listing filters, sorts, paging and escaping, the outline, and chunk pages;
  - embedding batching, retries and spend, including the Voyage and OpenAI-compatible request
    shapes against a local fake server;
  - the OCR queue with a mock provider, the job lifecycle, LanceDB operations, and triage and
    survey helpers.
- `tests/survey.rs` runs the binary's `survey` on `tests/fixtures/`.
- `tests/service.rs` starts `serve` on a free port against `tests/library/`, a corpus-shaped
  tree made from the fixtures (see the file header), and checks:
  - `/health`, `/status` counts, the survey and ingest job lifecycle, and the 409 for a second
    job;
  - that every chunk of the distinct-text documents ranks first for a query built from its own
    text;
  - hit fields, filters in both engines, and error statuses;
  - `/documents` (filters, sort, paging and 400s), `/documents/{doc_id}` with its outline, and
    `/chunks` followed page by page through a whole document;
  - CORS for allowed and refused origins, and that the library files are unchanged;
  - with `RAG_ENGINE_TOKEN`: 401 on every route but `GET /health` without it, or with a wrong
    token or scheme; success with it; preflights allowed; and the token absent from the log;
  - that a non-loopback `--bind` without a token exits before creating anything;
  - hybrid and vector search against a local fake OpenAI-compatible embeddings server, which
    holds its first request to keep a job running.

  The tests remove every `RAG_*` and `VOYAGE_API_KEY` variable (including `RAG_ENGINE_TOKEN`)
  from the server's environment, so they cannot reach a paid API.

PDFium must be in place for the binary tests. The fixtures are generated by
`tests/fixtures/make_fixtures.py` (reportlab and Pillow).

## Dependencies and licenses

Direct dependencies are MIT, Apache-2.0 or BSD-licensed. mupdf (AGPL), epub (GPL-3.0) and
kreuzberg 5.x (Elastic-2.0) are not allowed.

| Crate | Version | License | Used for |
|---|---|---|---|
| axum | 0.8.9 | MIT | HTTP service |
| blake3 | 1.8.7 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception (used under Apache-2.0) | content hashing |
| clap | 4.6.7 | MIT OR Apache-2.0 | command line (`derive`, `env`) |
| futures | 0.3.34 | MIT OR Apache-2.0 | stream and boxed-future helpers |
| lancedb | 0.37.1 | Apache-2.0 | vector store (default features off) |
| pdfium-render | 0.9.4 | MIT OR Apache-2.0 | PDFium bindings (`pdfium_latest`, default features off) |
| rayon | 1.12.0 | MIT OR Apache-2.0 | parallel hashing |
| rbook | 0.7.10 | Apache-2.0 | EPUB reading (default features off) |
| reqwest | 0.13.5 | MIT OR Apache-2.0 | embedding API client (`json`, `native-tls`: Windows SChannel, no C build) |
| rusqlite | 0.40.2 | MIT | SQLite (`bundled`; SQLite itself is public domain) |
| serde | 1.0.229 | MIT OR Apache-2.0 | serialization |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | JSON |
| tantivy | 0.26.2 | MIT | BM25 index |
| text-splitter | 0.33.0 | MIT | chunking (`markdown`, `tiktoken-rs`) |
| tiktoken-rs | 0.12.1 | MIT | cl100k tokenizer |
| tokio | 1.53.1 | MIT | async runtime |
| tower-http | 0.7.1 | MIT | CORS |
| walkdir | 2.5.0 | Unlicense OR MIT (used under MIT) | recursive walk |

lancedb is pinned to 0.37.x because 0.38.0 and 0.39.0 do not compile without their `remote`
feature: `src/job.rs` uses `Error::Http`, which only exists with `remote`.

The full tree has 519 packages, all MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0,
CC0-1.0, 0BSD or MIT-0, alone or as one of the alternatives offered. Two exceptions come in
through lance:

- `option-ext` 0.2.0 is **MPL-2.0**, via `dirs` in `lance-index`. MPL-2.0 is file-level
  copyleft: it is used unmodified, and changes to its own files would have to be shared.
- `xxhash-rust` 0.8.19 is **BSL-1.0**, via `lance-encoding`. SPDX `BSL-1.0` is the permissive
  Boost Software License, not the Business Source License (`BUSL-1.1`).

`lance-namespace-impls` pins `time = "=0.3.47"`, so the whole workspace, including the
desktop app, resolves `time` 0.3.47.

Not crates:

- **PDFium** is a runtime binary from bblanchon/pdfium-binaries. The build scripts in that
  repository are MIT. PDFium itself is BSD-3-Clause and Apache-2.0, and the bundled
  third-party licenses (FreeType, ICU, libjpeg-turbo, libpng, zlib, Little CMS, OpenJPEG,
  Abseil, and others) ship in `vendor/pdfium/licenses/`.
- **protoc** 36.2 (Protocol Buffers, BSD-3-Clause) is a build-time tool only and is not linked
  into the binary.
