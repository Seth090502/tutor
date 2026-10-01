# Phase 1 specification: intake, canonical store, exact access

The contract every Phase 1 worker builds against. The lead owns this file, `Cargo.toml` at the root and
`crates/vault-api`. Workers propose changes to them; they don't make them.

Owner decisions in force (2026-10-01):
- Build state of the art now, on the stand-in vault at `E:\Stand-In Vault`.
- **Copy** originals into the store; do not move them until the owner approves moving.
- GROBID waits for the owner's hosting choice.
- GPU extraction (MinerU, OCR) waits for owner GPU windows. Documents needing it wait in state `waiting_gpu`;
  they are not quarantined.
- Source code folders are a first-class delivery type (section 9).

## 1. Crates

| Crate | Kind | Owns |
|---|---|---|
| `vault-api` | lib | Addresses, request and response types, the `VaultApi` trait (lead) |
| `vault-core` | lib | Config, layout and path confinement, canonical normalisation, write-once store, SQLite schema, job queue, integrity manifest, and the Phase 1 tool implementations (`read`, `outline`, `grep` with trigram, `find_docs` over metadata, `vault_info`) |
| `vault-extract` | lib | Format detection and converters from bytes to canonical-ready Markdown plus structure; client stubs for sidecars (GROBID, MinerU, pandoc). No storage, no SQLite |
| `vault-code` | lib | Source trees: language detection, symbols, repository document, exclusion and secret rules, licence, SWHID (after the code research lands) |
| `vault-intake` | lib | The pipeline: inbox scan, readiness, routing, quality, identity, categories, dedup, publish, batch reports |
| `libraryd` | bin | MCP (rmcp, streamable HTTP) and gRPC (tonic) over a `VaultApi`, bearer-token auth, `/healthz` |
| `vault-client` | lib | tonic client for the gRPC API, which the desktop app and the eval link |
| `vault-cli` | bin `vault` | `status`, `intake run\|watch`, `verify`, `quarantine list\|show\|retry`, `classify`, `info` |

**Rules for all crates:**
- Rust 2024, MSRV 1.92, `cargo build` with `-j 4`, and `CARGO_TARGET_DIR` per worktree.
- Pin new dependencies exactly in the workspace `[workspace.dependencies]` (propose additions to the lead).
- Use `thiserror` for errors, `tracing` for logs, and no `unwrap`/`expect` on production paths.
- Every public behaviour has a test.

## 2. Identity and addresses

- **Grammar:** `vault-api/src/address.rs` is normative.
- **`hash16`** is the first 16 hex characters of BLAKE3 over the **canonical text bytes** (section 3).
- **`slug`:**
  - Built from the best title at first publication: ASCII-folded with `deunicode`, lowercased, runs of
    non-alphanumerics become `-`, trimmed to at most 48 characters at a hyphen, `untitled` if empty.
  - Code files use `<repo>-<path tail>`.
  - The slug is frozen at first publication.
- **Collision:** a new text whose `hash16` exists with *different* bytes is quarantined as `id_collision`. The
  same bytes mean the same document; only provenance is added.
- **File names:** prose is `text/{h2}/{doc_id}.md`. Code is `text/{h2}/{doc_id}.{ext}`, where `ext` is the
  original extension lowercased from an allowlist of letters and digits up to 8 characters, else `.txt`.
- **Immutability:** a text file is written once (temp file, fsync, rename) and never modified or deleted.
  - A re-extraction producing different text is a **new document**. `lineage(old, new, reason)` links them,
    and `documents.superseded_by` points from old to new.
  - `read` on the old address returns the old text plus `superseded_by`.

## 3. Canonical text

**Prose** (everything except code):
1. Decode to UTF-8:
   - BOM and UTF-16 detection first;
   - otherwise UTF-8;
   - otherwise detect with `chardetng` and decode with `encoding_rs`, recording the encoding in the sidecar.
2. NFC (`unicode-normalization`).
3. Line breaks:
   - CRLF and CR become LF.
   - U+2028, U+2029, U+0085, VT and FF become LF. FF also records a page break when the converter didn't
     already supply pages.
4. Remove: BOM and U+FEFF anywhere, U+200B, soft hyphen U+00AD, and C0 and C1 controls except TAB and LF.
   Keep ZWJ and ZWNJ.
5. In prose lines: tabs become spaces, runs of spaces become one, and no trailing whitespace.
6. **Blocks kept verbatim, one source line per line,** never sentence-split:
   - fenced code (```` ``` ```` or `~~~`), including its language tag;
   - display maths (`$$` on its own line through the closing `$$`);
   - tables (consecutive lines starting with `|`);
   - HTML blocks;
   - headings (`#`…);
   - entries under a `References`, `Bibliography` or `Index` heading (one entry per line).
7. **Sentences per line:**
   - Each paragraph (a run of non-blank prose lines) is joined with single spaces, then split with
     `unicode-segmentation` UAX #29 sentence boundaries.
   - A boundary is suppressed after common abbreviations (`e.g.`, `i.e.`, `et al.`, `cf.`, `vs.`, `Fig.`,
     `Figs.`, `Eq.`, `Eqs.`, `Ref.`, `No.`, `Vol.`, `pp.`, `p.`, `ch.`, `Ch.`, `Sec.`, `ca.`, `approx.`, `Dr.`,
     `Mr.`, `Mrs.`, `Ms.`, `Prof.`, `St.`, initials such as `A.`, and `U.S.`).
   - A boundary is also suppressed inside inline `$…$` maths and inline code spans.
   - Chinese and Japanese split after `。！？`.
   - Lines over 4,000 characters are split at the last whitespace before 4,000.
8. **List items and block quotes:** their sentences are split the same way. Continuation lines are indented to
   the item's content column (list) or keep the `> ` prefix (quote), so the Markdown still renders identically.
9. Paragraphs are separated by exactly one blank line. There are no leading blank lines, and the file ends with
   exactly one LF.

**Code** (section 9): decode as in step 1, CRLF and CR become LF, the BOM is removed. **Nothing else changes**:
tabs, trailing spaces, NFC form and a missing final newline all stay, so line numbers equal the original's.

**Required properties (tests):**
- `normalize(normalize(x)) == normalize(x)`.
- No text is lost: the multiset of non-whitespace characters is unchanged, except the removals in step 4.
- A Python `splitlines()` and a Rust `lines()` on the output agree.

## 3a. Sidecar (`hot/sidecars/{h2}/{doc_id}.json`)

- **Structural fields are immutable:** `doc_id`, `kind`, `lines`, `bytes`, `headings`, `pages`, `blocks`,
  `code`, `extraction` and `sources`.
- **Metadata fields may be rewritten** by intake through an atomic replace: `title`, `authors`, `year`,
  `language`, `identifiers`, `categories`, `identity`, `quality`, `provenance`. SQLite is authoritative for
  them; the sidecar mirrors it.

```json
{"format":1,"doc_id":"…","kind":"paper","lines":1234,"bytes":56789,
 "title":"…","authors":["…"],"year":2021,"language":"en",
 "identifiers":{"doi":"…","arxiv":"…","isbn":["…"],"pmcid":"…","openalex":"W…"},
 "headings":[{"level":1,"text":"…","start":1,"end":1234,"page":"iii"}],
 "pages":[{"label":"214","index":230,"line":5012}],
 "blocks":[{"kind":"table","start":10,"end":30}],
 "code":null,
 "extraction":{"path":"jats","tool":"vault-extract 0.1.0","extract_fp":"…","encoding":"utf-8"},
 "quality":{"score":0.97,"garbled_ratio":0.0004,"empty_pages":0,"ocr_confidence":null,"flags":[]},
 "sources":[{"original":"<blake3>","ext":"pdf","bytes":1234567}],
 "provenance":[{"batch":"…","received":"2026-10-02","path":"papers/x.pdf","manifest":{}}],
 "categories":{"primary":"psych-cogsci","secondary":[],"openalex_topic":"T10001","areas":["memory"],"source":"openalex","confidence":0.9},
 "identity":{"confidence":0.95,"source":"manifest+openalex","flags":[]},
 "created":"2026-10-02","supersedes":null}
```

## 4. Layout, config and path confinement

- **Layout:** see `docs/vault/INSTRUCTIONS.md` section 2. `vault.toml` at the root gives `format_version`,
  `vault_id` and the default tier names.
- **Machine config,** TOML, outside the vault. Lookup order: `--config`, then `VAULT_CONFIG`, then
  `%ProgramData%\vault\libraryd.toml`, then `%APPDATA%\vault\libraryd.toml`.

```toml
stage = "local"                      # local | networked
machine = "main-pc"                  # label for logs and result lines; never written into the vault
[vault]
root = 'E:\Stand-In Vault'           # holds vault.toml
hot = ""                             # optional absolute overrides for each tier
cold = ""
inbox = ""
[server]
mcp_bind = "127.0.0.1:7341"
grpc_bind = "127.0.0.1:7342"
token_file = 'C:\ProgramData\vault\token'
allowed_hosts = ["127.0.0.1", "localhost"]
[intake]
originals = "copy"                   # copy | move (move needs owner approval)
stability_seconds = 30
loose_quiet_minutes = 15
workers = 6
[sidecars]
grobid_url = ""                      # empty = disabled
mineru = ""                          # command or URL; empty = documents wait in waiting_gpu
pandoc = ""
openalex_key_env = "OPENALEX_API_KEY"  # value read from the environment; never logged
```

**Confinement:**
- Every stored path is relative to a tier root, uses `/` separators, and passes validation: no `..`, no
  absolute or drive-prefixed parts, no reserved Windows names (`CON`, `NUL`, `COM1`…), no trailing dot or
  space.
- Resolution joins to the root, then canonicalises, then checks that the result is under the canonicalised
  root (so a junction or symlink cannot escape).
- Files are opened with `\\?\` long paths on Windows.
- Tools never accept paths; they accept doc ids and addresses only.

## 5. Store operations

- **Originals:** `cold/originals/{h2}/{blake3}.{ext}`.
  - Copy to a temp name in the same directory, fsync, verify BLAKE3, then rename.
  - Never overwrite. An existing file with the same name means a duplicate; verify it and skip.
  - In `move` mode, a rename within the volume, with hash verification before and after.
- **Quarantine:** `cold/quarantine/{h2}/{blake3}.{ext}` plus `{blake3}.reason.json`, which holds the reason
  code, detail, batch, original path, and every attempt with its tool and error. Suspected secrets are never
  copied (section 9).
- **Integrity manifest:** append-only JSONL, `hot/manifests/originals.jsonl` and `hot/manifests/text.jsonl`.
  - One line per stored file: `{"path":"…","blake3":"…","bytes":n,"at":"…"}`.
  - `vault verify` re-hashes every file and reports missing, extra and mismatched files.
  - The files are line-buffered and fsynced per batch.

**Defence in depth (wave 2):**
- After writing a text, sidecar-structure, original or quarantine file, the store sets the file's read-only
  attribute, so a careless agent or editor gets "access denied" instead of silently corrupting an address.
- Integrity verification still catches anything that bypasses it.
- This changes no ACLs or system settings.

## 6. SQLite (`hot/db/vault.sqlite`)

- **Settings:** WAL, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`.
- **Schema version** in `PRAGMA user_version`, with forward-only migrations.
- **One writer process** (intake) and any number of readers (libraryd).
- **Never over a network share.**

**Tables (key columns; workers add indexes as needed):**

| Table | Key columns |
|---|---|
| `meta` | `key`, `value` (`vault_id`, `taxonomy_version`, `index_version`) |
| `batches` | `batch_id`, `kind` (`batch`/`oaforge`/`loose`/`code`), `inbox_name`, `detected_at`, `ready_at`, `completed_at`, `state`, `files_expected`, `report_path` |
| `deliveries` | `delivery_id`, `batch_id`, `rel_path`, `bytes`, `blake3`, `sha256`, `received_at`, `manifest_json`, `hint_category`, `outcome` (`pending`/`published`/`duplicate`/`near_duplicate`/`quarantined`/`excluded`/`left_in_place`/`listed_missing`/`waiting_gpu`), `reason`, `rule`, `detail`, `doc_id`. Unique on (`batch_id`, `rel_path`) |
| `originals` | `blake3`, `ext`, `bytes`, `format`, `stored_rel_path`, `first_batch`, `stored_at` |
| `documents` | `doc_id`, `hash16` (unique), `slug`, `rel_path`, `kind`, `title`, `authors_json`, `year`, `language`, `lines`, `bytes`, `extraction_path`, `extract_fp`, `quality`, `identity_confidence`, `primary_category`, `level`, `license`, `created_at`, `superseded_by` |
| `doc_originals` | `doc_id`, `blake3` |
| `doc_identifiers` | `doc_id`, `scheme`, `value`, `source` |
| `doc_categories` | `doc_id`, `category`, `role`, `source`, `confidence` |
| `doc_areas` | `doc_id`, `area` |
| `lineage` | `old_doc`, `new_doc`, `reason`, `at` |
| `near_dups` | `group_id`, `doc_id`, `is_primary`, `similarity` |
| `repos` | `repo_id`, `name`, `url`, `commit`, `license`, `snapshot_at`, `doc_id`, `batch_id` |
| `repo_files` | `repo_id`, `path`, `doc_id`, `language`, `swhid` |
| `quarantine` | `id`, `delivery_id`, `blake3`, `reason`, `detail`, `stored_rel_path`, `at` |
| `jobs` | `job_id`, `stage`, `key`, `state` (`queued`/`running`/`done`/`failed`/`waiting`), `attempts`, `fingerprint`, `last_error`, `updated_at`. Unique on (`stage`, `key`) |
| `doc_fts` | FTS5 over `doc_id` (unindexed), `title`, `authors`, `abstract`, `headings`, `keywords` (for `find_docs` in Phase 1) |

## 7. Intake outcomes and reason codes

- **Every delivered file reaches exactly one outcome** (the `deliveries.outcome` values above). The batch
  report `inbox/_done/<batch>/_INTAKE_REPORT.md` lists every file.
- **Reason codes:** `unsupported_format`, `protected`, `corrupt_file`, `empty_file`, `extraction_failed`,
  `low_quality_text`, `incomplete_copy`, `archive_limits`, `id_collision`, `path_too_long`, `suspected_secret`.
- **Exclusion rules (code only):** `vcs-metadata`, `dependencies`, `build-output`, `lockfiles`, `generated`,
  `binary`, `oversize-text`.
- **Readiness:** INSTRUCTIONS.md section 3.

## 8. Formats and routing (Phase 1 availability)

Detection is by content: magic bytes, then container inspection.

| Detected | Route now | Later |
|---|---|---|
| Markdown, plain text | native normalisation | |
| JATS (`<article` with the JATS DTD or namespace) | `vault-extract` JATS converter (roxmltree, `allow_dtd`); MathML and `tex-math` become LaTeX | |
| TEI | `vault-extract` TEI converter | GROBID output reuses it |
| HTML and XHTML, including arXiv (LaTeXML) | `vault-extract` HTML converter; MathML `alttext` becomes LaTeX | |
| EPUB, DOCX, ODT | xberg if it builds and passes the sample on Windows; otherwise our converters (EPUB is XHTML in a zip) | |
| PDF, born-digital | xberg native or PDFium text with page labels, whichever wins the 20-document sample | GROBID for papers |
| PDF, scanned, or maths-heavy | `waiting_gpu` | MinerU |
| arXiv LaTeX source | `waiting_tool` (pandoc not installed yet) | pandoc or xberg `.tex` |
| Jupyter notebook | cells become Markdown, code in fences | |
| ZIP or TAR | unpack within limits; members are processed individually | |
| Code tree | `vault-code` (section 9) | |
| Other binary | quarantine `unsupported_format` | |

**PDF classification:**
- Per-page text-layer density and image coverage.
- Maths-heavy from maths font names (CMMI, CMSY, MSAM, STIX Math, Cambria Math) and the symbol ratio.
- CJK ratio.
- Book versus paper from page count over 120 and metadata.

## 9. Source code (summary; full design after the code research)

**Scope (owner, 2026-10-01):** not only game code. Many state-of-the-art open-source codebases, organised in
collections such as `gamedev` and `rust-exemplars`. An agent mid-development can look at how they solve the same
problem, compare approaches across repositories, and read exact lines, seamlessly.

**Correction from the repo sweep (`docs/phase0/repos-*.md`):**
- **Classify files by content and markers, never by directory name alone.** Several tools exclude `bin/`,
  `build/`, `out/` and `docs/` by name, which drops real engine source.
- **Use these signals:**
  - `.gitignore` and `.gitattributes` (`linguist-vendored`, `linguist-generated`) through the `ignore` crate;
  - `@generated` headers;
  - build-artifact signatures (object files, PDBs, `CMakeCache.txt`);
  - lockfiles;
  - minified-content heuristics.
- **Vendored third-party code** (Godot's `thirdparty/`) is kept and tagged `vendored`. It is de-duplicated across
  repositories by content.
- **Translations and similar bulk data** (`.po`) are indexed lightly or excluded by a named, reported rule.

- Each kept file is its own document, canonicalised as code (section 3) and addressed by its real line
  numbers.
- One repository document per repository snapshot holds: the README first, then the file tree with addresses,
  then the ranked symbol map.
- Exclusion rules are listed in section 7.
- **Secret scanning** runs before anything is stored. A hit is recorded as quarantined with reason
  `suspected_secret` and **left in place**: never copied, indexed or served.
- **Identity:** the repository's remote URL and HEAD commit come from `.git` files (read, never by running git).
  Each file gets a SWHID content id.

## 10. Phase 1 tools (`vault-core` implementing `vault-api`)

- **`read`:** resolves by `hash16`. `expand` is either lines or section (the innermost heading containing the
  range). It returns `page` and `section`, and per-item errors for unknown ids or out-of-range lines.
- **`outline`:** headings and pages from the sidecar; symbols for code.
- **`grep`:** scope is docs, category (with area), or the whole vault; workspace arrives in Phase 5.
  - The vault-wide scope uses a tantivy trigram field over line-aligned blocks of about 16 KiB. The query is
    planned from the regex (Cox and Zoekt method) with `regex-syntax`.
  - Candidates are then verified with `grep-regex` and `grep-searcher` over the canonical text, and real line
    numbers are reported.
  - Measure document-level against block-level trigrams and keep the better one.
- **`find_docs`:** FTS5 over metadata and headings in Phase 1; BM25 and dense search arrive in Phases 3-4.
- **`vault_info`:** as typed. `tools_available` lists what works now.
- **Not yet built:** `search`, `workspace`, `citations` and `define` return `ApiError::NotYetAvailable` with
  the phase.

**Every response:**
- respects `max_tokens` (default 8,000, hard cap 20,000, at 4 characters per token);
- fills `truncated` with the reason and how to get the rest;
- MCP text output wraps every passage as
  `<vault_passage address="…" page="…" id="<random 4 alphanumerics>">…</vault_passage id="…">`,
  with the same structured JSON in `structuredContent`;
- gRPC returns the structured form.

## 11. libraryd

- MCP over streamable HTTP at `/mcp` (rmcp, pinned), and gRPC (tonic) on its own port.
- `/healthz` returns `{"ok":true,"stage":…,"vault_id":…}` without auth.
- **Auth:** everything else requires `Authorization: Bearer <token>`, compared in constant time. The token
  file holds 32 random bytes, hex.
- `allowed_hosts` follows the config.
- Response compression (gzip or zstd) on gRPC.
- Requests in a batch run concurrently.

## 11a. Any agent, not just the tutor (owner, 2026-10-01)

The vault is built for the tutor, but any agent (Claude Code, Codex, OpenCode, the owner's own agents) must be
able to reference chosen material seamlessly.

**Target use case:** an agent asked to study 5-7 folders of game source code can map them, find anything, follow
definitions and references across files and repositories, and read exactly what it needs, in few calls.

- **Collections** (planned `vault-api` addition at the wave-1 merge):
  - Named, persistent sets of documents and repositories that the owner or an agent curates, for example
    `gamedev` = 6 repositories + 4 books.
  - `Filters.collection`, plus `Scope::Collection { name }` for grep.
  - `vault collection create|add|remove|list`.
  - Workspaces stay per session; collections persist.
- **Agent guidance travels with the server:**
  - libraryd sends MCP server `instructions` at initialisation: the tool loop, batching, addresses and budgets.
  - A Claude Code skill (`vault`) carries the longer playbook ("study a codebase", "find the landmark paper",
    "cite exactly").
  - The tutor's own rules stay in TUTOR.md.
- **Code navigation is first-class** in the same nine tools:
  - `outline` returns a ranked repository map and per-file symbols.
  - `define` finds symbol definitions.
  - `citations` follows reference and import edges.
  - `grep` has symbol-aware ranking.
  - `find_docs` routes to repositories and files.
  - The details land in section 9 from the code research (`docs/phase0/research-code-*.md`, `repos-*.md`).
- **Everything is Rust.** Specialist non-Rust tools (MinerU, GROBID, llama.cpp, pandoc) run only as sidecar
  processes called from Rust.

## 12. Layer A invariants for Phase 1 (`cargo test` plus an integration harness against a temp vault)

| Invariant | Check |
|---|---|
| Needle | 500 random lines from random documents are each found exactly by vault-wide grep and returned exactly by read |
| Round-trip | Every address any tool returns resolves through read |
| Stability | Reclassify a document and re-run intake on an unchanged file: addresses are unchanged |
| Portability | Copy the vault to another root under a different drive letter, point a config at it, and the whole suite passes with no rebuild |
| Parity | MCP and gRPC give identical structured results for the same requests |
| Completeness | Every delivered file has exactly one outcome |
| Canary | A random string planted in a file outside the vault (the eval directory) is unfindable through every tool |
| Resumability | Kill intake mid-batch and restart: final state and text bytes are identical to an uninterrupted run |
| Integrity | `vault verify` passes; flipping one byte in a stored file is detected |
