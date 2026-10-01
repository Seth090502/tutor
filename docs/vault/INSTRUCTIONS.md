# Vault instructions: delivering, ingesting and finding content

This file lives at the root of the vault and travels with it. The canonical copy is in the code repository at
`docs/vault/INSTRUCTIONS.md`. Last updated 2026-10-01 (vault format 1).

> **Status today (Stage 1, stand-in at `E:\Stand-In Vault`):**
> - **Works now:** `/ingest` prepares byte-verified copies in `inbox\`; the intake pipeline is still pending.
>   Wave 1 core/server/converters do not by themselves ingest an inbox delivery.
> - **Arrives with Phase 1:** the intake service that ingests the inbox, and the `vault` command-line tool.
>   Until then, everything marked **(Phase 1)** below is the design those parts are being built to.
> - When the 1 TB SSD arrives, this whole folder, inbox included, is copied to the SSD and verified (section 9).
>   Files you drop here today are not lost.

## Contents

1. [What the vault is](#1-what-the-vault-is)
2. [Layout: what each folder is for](#2-layout-what-each-folder-is-for)
3. [Delivering content](#3-delivering-content)
4. [Source code folders](#4-source-code-folders)
5. [What intake does, step by step](#5-what-intake-does-step-by-step)
6. [Checking results and fixing problems](#6-checking-results-and-fixing-problems)
7. [Categories](#7-categories)
8. [Finding and reading content (traversal)](#8-finding-and-reading-content-traversal)
9. [Moving the vault](#9-moving-the-vault)
10. [Rules that keep the vault trustworthy](#10-rules-that-keep-the-vault-trustworthy)
11. [Quick reference](#11-quick-reference)

## 1. What the vault is

The vault is your library in a form a tutor can search, read exactly and cite.

- **One clean Markdown file per document.** Every book, paper and source file becomes one UTF-8 Markdown file,
  the *canonical text*, addressed by line numbers.
- **Indexes over that text:** keyword (BM25), exact and regex (trigram), and meaning (vectors). They find things
  fast.
- **libraryd serves all of it** to the tutor and to Claude Code.
- **Citations look like `3f9a1c07b2e4d55e#L120-L148`** (document id, then a line range). An address never
  changes once issued: not when the document is reclassified, not when the corpus grows, and not when the drive
  moves to another machine.

**Guarantees:**
- **Nothing you deliver is silently dropped.** Every file ends up in the vault, or in quarantine with a stated
  reason, or (for a few file types in source folders) is excluded by a named rule that the intake report lists.
- **Provenance is kept exactly as you deliver it:** source, URL, licence, manifest fields, plus the date and the
  batch it arrived in.
- **Originals are kept byte for byte,** stored under the hash of their bytes.

## 2. Layout: what each folder is for

```
<vault root>\
  INSTRUCTIONS.md      this file
  vault.toml           vault identity and format version (no machine paths)
  inbox\               THE ONLY FOLDER YOU WRITE TO
    <batch-name>\      a delivery batch (section 3.1)
    loose\<category>\  drop folders for single files (section 3.3)
    _done\             finished batches: what intake did not take, plus each batch's report
    _templates\        manifest and SOURCE.toml templates
  hot\                 fast tier: everything the tutor reads (intake writes it; don't edit)
    text\{h2}\{doc_id}.md        canonical text, one file per document
    sidecars\{h2}\{doc_id}.json  headings with line ranges, page map, metadata, provenance, quality
    db\                          SQLite: metadata, categories, citations, glossary, job queue
    index\tantivy|trigram|vectors\   indexes (all rebuildable from text\)
    meta\taxonomy\               the category map (CATEGORIES.md, categories.json, OpenAlex snapshot)
    manifests\                   BLAKE3 integrity manifests
    reports\                     intake report and status pages (static HTML)
    cache\                       rebuildable caches
  cold\                bulk tier (may move to another disk later)
    originals\{h2}\{blake3}.{ext}   your delivered files, byte for byte
    quarantine\{h2}\{blake3}.{ext}  files that could not be ingested, each with a reason file
```

- `{h2}` is the first two hex characters of the id, which spreads about 160,000 files over 256 folders.
- **Never edit, rename or delete anything under `hot\` or `cold\` by hand.** Intake and the integrity manifest
  track every file there. A hand edit shows up as an integrity failure.
- In `inbox\`, top-level names that start with `_` belong to intake. Everything else there is yours.

## 3. Delivering content

### 3.0 The fast path: `/ingest`

- Put material in **staging**, `E:\Ready to Transfer` on the main PC. Keep repositories as whole folders.
- In Claude Code, Codex or OpenCode, request **`/ingest`**. The skill uses the independent Rust
  `tools/vault-stage` helper, with its own lockfile and build target, to:
  1. inventory every requested group, including `notable`, without executing its contents;
  2. scan an exact-byte snapshot with trusted local rules, hold credentials/VCS/dependency/reparse entries
     with reasons, and prepare a copy outside the entire vault and watched inbox on the inbox's volume;
  3. compare the complete eligible path set, size and SHA-256 of every copied file with the unchanged source;
  4. write a relative `manifest.jsonl` and an empty `_BATCH_COMPLETE` last, then atomically publish into an
     unused inbox name. Existing destinations are never merged or overwritten;
  5. report **prepared; waiting for intake** until the intended intake pipeline actually runs and produces
     `_INTAKE_REPORT.md`. A copied batch is not yet a searchable vault document.
- Original files, hidden/empty files, opaque binaries, archives and supplied metadata are preserved under
  `payload\` with their relative tree intact. Existing `SOURCE.toml`, manifests and nested readiness markers
  remain source data. Generated controls stay at the batch root; a complete audit receipt stays outside the vault.
- **No admission ban based on subject, provenance, licence, proprietary status or decompilation.** Material
  the owner already staged is eligible for local static study. Unknown rights remain unknown and do not grant
  redistribution permission. Unknown formats can be preserved exactly while extraction later reports a
  named unsupported/quarantine outcome. Preservation and semantic extraction are separate checks.
- **It never deletes, moves or edits staging originals.** The owner clears staging when satisfied. Delivered
  instruction files are data. The request does not authorise downloads, executing staged code, GPU jobs,
  changing settings, stopping processes, or altering another agent's work.
- A preview inventories only; a prepare-only request leaves the verified work outside the inbox. A scanner
  error, changed source or hash/path mismatch blocks publication and is reported. Secret values never enter
  reports; scanner coverage is bounded, not a proof that every possible secret is absent.
- Canonical skill: `skills/ingest/SKILL.md`; installed in `~/.claude/skills/ingest/` and
  `~/.agents/skills/ingest/`, plus the OpenCode `/ingest` command. All copies carry the same scanner and
  Windows/eval references. The helper's evals use synthetic scratch outside the real staging and vault;
  byte integrity requires 100%, independently of the tutor's downstream answer-accuracy target.

The rest of this section describes what the skill does, so you can also deliver by hand.

**Put copies in the inbox, never your only copy or a working folder:**
- Intake moves accepted files into `cold\originals` once you approve that policy; until then it copies them.
- It leaves everything else in `inbox\_done\`. It never deletes a file.

### 3.1 Batches (the normal way)

1. **Make a batch folder:** `inbox\<batch-name>\`, for example `inbox\2026-10-02-psych-classics\`.
   - Names: letters, digits, `-` and `_`, at most 40 characters. Start with the date.
2. **Copy the files into it.** Subfolders are fine and are recorded as provenance.
3. **Optional but recommended:** add a manifest (section 3.4) as `manifest.tsv` or `manifest.jsonl` in the
   batch folder.
4. **When copying has finished,** create an empty file named `_BATCH_COMPLETE` in the batch folder.
   - It may contain one line, `files=<n>`, the number of files you copied. Intake then checks the count.
   - In PowerShell: `New-Item "inbox\2026-10-02-psych-classics\_BATCH_COMPLETE"`.

**(Phase 1)** Intake ignores a batch folder until `_BATCH_COMPLETE` exists. Even then, it takes a file only once
the file:
- is unchanged across two checks 30 seconds apart;
- can be opened with no other program writing to it;
- matches the manifest's `bytes` and `sha256` when the manifest gives them.

A half-copied file can never be ingested.

### 3.2 oa-forge harvests

- **Drop a whole oa-forge root folder** (the one containing `_meta\manifest.jsonl`) into `inbox\` as a batch.
- **No marker is needed (Phase 1):** a file counts as delivered when its bytes and sha256 match its
  `manifest.jsonl` row.
- **Intake reads** `_meta\manifest.jsonl` and every `<file>.meta.json` sidecar (which has the URL) as the
  manifest.
  - It ignores `_meta\` housekeeping files (`state.jsonl`, `requests.jsonl`, robots caches, `*.oaf-tmp`).
  - It records them in the batch report.
- **Asks for oa-forge** (your side):
  - carry `level`, `type`, `mode` and `url` into `manifest.jsonl`;
  - write `_meta\_BATCH_COMPLETE` at the end of a clean run.
  - Until then, a root with a stale `manifest.lock` is held back and reported, not ingested half-written.

### 3.3 Single files without a batch

- **Drop them into `inbox\loose\<category>\`.** The folders are `psych-cogsci`, `engineering`, `mathematics`,
  `medicine`, `cs-ai`, `natural-sciences`, `humanities`, `social-other`, and `unsorted` if unsure.
- **The folder name is a category hint, not a decision** (section 7).
- **(Phase 1)** A loose file is taken after 15 minutes with no change and an exclusive-open check. Loose files
  are grouped into a daily batch named `loose-YYYY-MM-DD`.

### 3.4 Manifests

- A manifest describes files you deliver. It is optional per file: files without a row are identified from their
  own content and from OpenAlex.
- **Formats:**
  - **TSV:** UTF-8, tab-separated, with a header row. A template is in `inbox\_templates\manifest.tsv`.
  - **JSONL:** one JSON object per line, with the same keys.

| Column | Required | Meaning |
|---|---|---|
| `file` | **yes** | Path of the file relative to the batch folder. This links the row to a file. |
| `domain` | no | Category slug or hint word (`psych-cogsci`, `mathematics`, `psychology`…; section 7) |
| `type` | no | `paper`, `book`, `chapter`, `thesis`, `report`, `notes`, `code`, `dataset-doc`, `other` |
| `level` | no | `intro`, `graduate`, `landmark`, `recent` |
| `title`, `authors`, `year` | no | Authors separated by `;`. Used to verify identity. |
| `doi`, `arxiv_id`, `pmcid`, `pmid`, `isbn`, `openalex_id` | no | Identifiers. Any one of them makes identification nearly certain. |
| `language` | no | ISO 639-1 code (`en`, `zh`, …) |
| `edition`, `publisher`, `series` | no | Books |
| `url`, `provenance`, `license_class`, `license`, `mode` | no | Where it came from and on what terms. Kept verbatim. |
| `sha256` or `blake3`, `bytes` | no | Proves the copy is complete and intact. Strongly recommended. |
| `version_of` | no | The `file`, DOI or arXiv id of another version (preprint and published, editions) |

- Unknown extra columns are kept verbatim in provenance.
- A row whose file never arrives is reported as "listed but missing". It is never invented.

### 3.5 Formats

Format is detected from the file's content, not its extension.

| Format | How it is ingested |
|---|---|
| JATS XML (PubMed Central `.nxml`) | Structured converter; best quality for papers |
| arXiv LaTeX source (`.tar.gz` or `.gz` containing `.tex`) | LaTeX converter; maths kept verbatim |
| arXiv HTML and other HTML or XHTML | HTML converter (maths from MathML alt-text) |
| EPUB | `vault-extract` EPUB converter; MathML becomes LaTeX; protected files are quarantined |
| DOCX and ODT | `vault-extract` DOCX and ODT converters |
| PDF, born-digital paper | PDFium text/layout; GROBID structure/references when the sidecar is available |
| PDF, born-digital book | PDFium (`vault-extract` feature; requires the matching local PDFium library) |
| PDF, maths-heavy | MinerU (LaTeX for equations) |
| PDF, scanned | MinerU OCR (English and Chinese) |
| TEI XML | Structured converter |
| Markdown, plain text | Normalised directly (encoding detected) |
| Jupyter notebooks | Cells become Markdown, with code in fenced blocks |
| ZIP or TAR archives | Unpacked (limits: 500 MB, ratio 100:1, 10,000 entries); each member is processed as its own file |
| Any text file inside a code folder, including Markdown | Stored verbatim (section 4) |
| Images, audio, video, executables, fonts | Quarantined as `unsupported_format` (re-deliver a PDF if it is a document) |
| Encrypted PDF, DRM EPUB | Quarantined as `protected` |

**Best formats to deliver, in order:**
1. JATS (from PMC) or arXiv source for papers.
2. EPUB for books.
3. Born-digital PDF.
4. Scans, last.

When you have two formats of the same work, deliver both: intake keeps the best as primary and links the other.

## 4. Source code folders

Deliver code with `/ingest` (section 3.0), or by hand: copy each repository as **one whole folder** into a
batch folder, then add `_BATCH_COMPLETE`. A folder of repositories is fine; each repository inside it is handled
on its own.

**(Phase 1)** Intake recognises a repository by a supplied `SOURCE.toml` at its root, or
by its contents: a `.git` folder or build files such as `Cargo.toml`, `CMakeLists.txt`, `package.json`, `go.mod`,
`*.sln`, `SConstruct` or `*.uproject`.

### What happens to a repository

- **Every text file becomes its own vault document, stored verbatim.**
  - The encoding becomes UTF-8, line endings become LF, and a byte-order mark is removed. Nothing else changes:
    tabs, trailing spaces and a missing final newline all stay.
  - So `…#L40-L72` is exactly lines 40-72 as any editor shows them.
  - This includes Markdown, text and other documentation files inside the repository, so their line numbers
    match the repository too.
  - PDFs, EPUBs and notebooks inside a repository go through their normal routes (section 3.5). A notebook's
    line numbers refer to its converted text, not to the `.ipynb` file.
- **Each file is classified by its content and the repository's own markers, never by folder name alone.**
  A `build\`, `bin\` or `docs\` folder that holds real source is kept.

  | Class | Examples | How agents see it |
  |---|---|---|
  | source | `.cpp`, `.rs`, `.cs`, `.gd`, `.hlsl` | Searched and ranked first |
  | build | `CMakeLists.txt`, `Cargo.toml`, `*.Build.cs`, `SConstruct` | Searched; used for the module map |
  | docs | `README.md`, design notes, Godot's `doc/classes/*.xml` | Searched |
  | test | test folders and files (`*_test.go`, `tests\`) | Searched, ranked below source |
  | generated | files with an `@generated` or "DO NOT EDIT" header, `*.gen.*`, `linguist-generated` | Searched, ranked below source |
  | vendored | third-party copies (`thirdparty\`, `linguist-vendored`) | Searched, ranked below source |
  | data | translations (`.po`), large tables, test baselines | Found by exact search (`grep`) and `read` only |

  - The signals are the repository's `.gitignore` and `.gitattributes` (`linguist-vendored`,
    `linguist-generated`, `linguist-language`), Godot's `.gdignore`, generated-file headers, Linguist's list of
    vendored paths, build-artefact signatures, and line length (minified files).
- **Identical files are stored once.** When several repositories carry the same file (a vendored zlib, a
  forked engine), it is one document listed under every repository, with the same address everywhere.
- **A repository document is generated for each delivery:** a metadata block (name, URL, commit, licence,
  languages, collections), the README, the file tree with each file's address and class, binary files listed
  by path, size and type, and a ranked map of the main symbols. `outline` on it shows the tree and the map.
- **Recorded per repository:** name, URL and commit (from `SOURCE.toml`, or read from `.git` if it was
  delivered), the licence, and the language of each file. Each file also gets a Software Heritage identifier
  (SWHID) of the delivered bytes.
- **Symbols:** functions, classes, methods and other definitions are extracted with line ranges for the main
  languages. They power `outline`, `define`, `citations` and symbol-aware ranking. Languages without symbol
  support are still fully searchable with `grep` and `search`; `vault_info` lists the coverage per language.
- **Macro-heavy C++** (Unreal, Source engine, Godot) is parsed with a profile that hides export and annotation
  macros (`ENGINE_API`, `UCLASS(...)`, `GDCLASS(...)`) from the parser only. The stored text is never changed.
  Files the parser cannot fully understand are flagged, and stay searchable.

### `SOURCE.toml`

Optional, at the repository root; a template is in `inbox\_templates\SOURCE.toml`. Every field is optional.
`/ingest` preserves a supplied file byte for byte; it does not insert or rewrite metadata in the verified
payload. Missing metadata remains unknown. Bounded Git observations stay in the external receipt until
a supported intake provenance handoff exists; do not claim intake reads that receipt.

| Field | Meaning |
|---|---|
| `name` | Short repository name, used in results and in `repo` filters |
| `url` | Where it came from |
| `commit` | Owner-supplied verified commit, or unknown. File hashes bind the actual working-tree bytes; a HEAD observation does not prove a clean checkout |
| `license` | SPDX id (`MIT`, `GPL-2.0-only`…), `owned` for your own code, or `unknown`. Licence files are detected anyway |
| `categories` | First one is primary; usually `cs-ai` |
| `collections` | Named sets this repository joins, for example `gamedev`, `compilers`, `rust-exemplars` |
| `level` | `intro`, `graduate`, `landmark` or `recent` |
| `description` | One line on what the code is and why it is in the vault |
| `companion` | DOI or title of a book or paper this code accompanies |
| `profile` | Only if detection gets it wrong: `unreal`, `source-engine`, `godot` or `generic` |

**Licences and provenance:** owner-staged material is admitted for local static study regardless of origin,
including proprietary, reconstructed or decompiled source. Record actual licence/provenance evidence;
public availability does not prove rights. Missing or uncertain licence is `unknown`,
`redistributable=false`; an export must not include it without confirmed permission.

### Collections

- **A collection is a named, persistent set of repositories and documents,** for example `gamedev` = the engines
  and games plus a few books. Agents can search, grep or map a whole collection in one call.
- `SOURCE.toml` `collections` adds a repository at intake. **(Phase 1)** `vault collection create|add|remove|list`
  edits collections later.
- Collections are labels: changing them never changes an address.

### A later commit of the same repository

- Deliver the whole folder again, with the new `commit` in `SOURCE.toml`. It becomes a **new snapshot.**
- Unchanged files keep their documents and addresses. Changed and new files become new documents.
- The old snapshot stays readable, and its addresses keep working. Searches use the newest snapshot of each
  repository unless a request names an older one.
- **History is not ingested,** only the delivered files. Most delivered clones are shallow anyway.

### Excluded by rule

These are never silently dropped. The batch report lists each one with its path, size, BLAKE3 hash and the rule
that matched, and the files stay in `inbox\_done\<batch>\` for you to review. Large exclusions are summarised per
folder.

| Rule | What |
|---|---|
| `vcs-metadata` | `.git\`, `.hg\`, `.svn\` |
| `dependencies` | Package-manager install trees: `node_modules\`, Python virtual environments (a folder with `pyvenv.cfg`), `site-packages\`, `bower_components\` |
| `build-output` | Folders that contain build artefacts (object files, `.pdb`, `CMakeCache.txt`, `.tlog`, `.ninja_log`). Decided by contents, not by the folder's name |
| `gitignored` | Files the repository's own `.gitignore` excludes (usually local build output) |
| `lockfiles` | `Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `poetry.lock` |
| `minified` | Text with very long lines: minified JavaScript or CSS, source maps, packed data |
| `binary` | Non-text files that are not a supported document format: images, models, sounds, compiled files, game data. The repository document still lists them by path, size and type |
| `oversize-text` | Text over 2 MB that is not hand-written source (generated tables, dumps), and any text file over 16 MB. Large hand-written source such as SQLite's single-file `sqlite3.c` is kept |

### Secrets: please remove them before copying

- Intake also scans every file for private keys, tokens, `.env` files and credential files.
- A match is **left in place** in `inbox\_done\<batch>\`: never copied into the vault, never indexed, never
  served. It is recorded as quarantined with reason `suspected_secret`, naming the rule, never the value.
- Open-source projects often carry test keys (TLS test certificates). Those are caught too, and are listed by
  folder so the report stays readable.

### Preservation, holds and extraction limits

- **Opaque/game/build assets and archives:** `/ingest` preserves eligible ordinary files byte for byte.
  Intake may report `binary`, `unsupported_format` or `archive_limits` instead of producing searchable text.
  Never execute or unpack a staged program to make it eligible; bounded extraction is the intake layer's job.
- **Notes and indexes:** ordinary notes are eligible. Hold credential-bearing files by scanner/filename reason,
  not every note by location. The known credential-bearing `E:\Ready to Transfer\INDEX.md` must stay held;
  report its path only. Do not print it in agent context or silently sanitize its bytes.
- **Stage holds versus intake exclusions:** staging accounts for named VCS/dependency/reparse and credential
  holds in its external receipt. Section 4's text-index exclusions are a separate future intake outcome;
  they must not be used as blanket copy bans for entire staging groups or opaque originals.

## 5. What intake does, step by step

Every step is **(Phase 1)** and resumable: stopping and restarting never duplicates or loses work.

1. **Readiness:** the marker, stability and manifest checks above.
2. **Hash:** BLAKE3 of the bytes. If those exact bytes are already in the vault, this delivery's provenance is
   added to the existing original and nothing else happens (outcome **duplicate**).
3. **Store the original** in `cold\originals\{h2}\{blake3}.{ext}`.
4. **Detect format; route to an extractor** (section 3.5).
5. **Normalise to canonical Markdown:**
   - UTF-8, LF line endings, one sentence per line for prose.
   - Headings as `#` levels, maths as `$…$` and `$$…$$` with LaTeX verbatim, tables as pipe tables, code in fences.
   - References as one entry per line.
   - Page numbers kept in the sidecar's page map.
   - Code (section 4) keeps its exact lines.
6. **Quality check:** garbled characters, empty pages, broken encoding, missing sections, OCR confidence.
   - A weak result is retried with available structured/GPU extractors (GROBID, MinerU or OCR) before giving up; unavailable routes are reported, never implied to have run.
7. **Identify** from the manifest, then the document itself (title, authors, DOI, arXiv id, ISBN, and for books
   the Cataloging-in-Publication block), then OpenAlex verification.
   - Confidence is recorded. Documents whose identity can't be established are flagged, not rejected.
8. **Categorise and detect language** (section 7). Chinese text gets a Chinese tokenizer.
9. **Find near-duplicates** (preprint against published version, editions). The best version becomes primary;
   the others are linked, never deleted (outcome **near-duplicate, linked**).
10. **Publish:**
    - `hot\text\{h2}\{doc_id}.md` and its sidecar are written once and never changed.
    - `doc_id` is 16 hex characters of the text's BLAKE3 hash plus a short slug.
    - A better extraction later becomes a new version; the old addresses keep working and point to it.
11. **Index:** keyword, trigram and vector indexes are updated incrementally; there are no full rebuilds.
12. **Report:**
    - The batch folder moves to `inbox\_done\<batch>\` with `_INTAKE_REPORT.md` inside. It lists every file and
      its outcome: **published**, **duplicate**, **near-duplicate (linked)**, **quarantined (reason)**,
      **excluded (rule)** or **left in place (suspected secret)**.
    - What remains in that folder is exactly what the vault did not take. Delete it when you are satisfied.

## 6. Checking results and fixing problems

**(Phase 1)**

- **Intake report:** `hot\reports\intake\index.html` is a static page with no network access. It shows:
  - documents per batch, format and category;
  - extraction paths used and quality scores;
  - duplicates and quarantine reasons.
- **Status page:** `hot\reports\status\index.html` shows service health, disk use per tier (with a warning at
  85%), recent batches and the latest eval scores.

**Commands:**

| Command | What it does |
|---|---|
| `vault status` | One-screen summary |
| `vault intake run` | Process the inbox once |
| `vault intake watch` | Keep processing as batches complete |
| `vault quarantine list` | Every quarantined file with its reason code |
| `vault quarantine show <id>` | Details and the extraction attempts |
| `vault quarantine retry <id>` | Retry after an extractor upgrade |
| `vault verify` | Re-hash every original and text file against the integrity manifest |

**Quarantine reason codes:**

| Code | Meaning |
|---|---|
| `unsupported_format` | Not a document format |
| `protected` | Encrypted or DRM |
| `corrupt_file` | The file is damaged |
| `empty_file` | The file has no content |
| `extraction_failed` | Every extractor failed |
| `low_quality_text` | Below the quality bar after every retry |
| `incomplete_copy` | Did not match its manifest hash before the timeout |
| `archive_limits` | An archive broke the unpack limits |
| `id_collision` | Two different texts share a hash prefix; never expected in practice |
| `path_too_long` | The path is too long to handle |
| `suspected_secret` | Left in place (section 4) |

**To fix a quarantined file:** deliver a better copy (a cleaner PDF, the EPUB, the JATS) in a new batch. Intake
links it to the quarantined one.

## 7. Categories

The full map is `hot\meta\taxonomy\CATEGORIES.md`.

| Priority | Category (slug) |
|---|---|
| 1 | Psychology and cognitive science (`psych-cogsci`) |
| 2 | Engineering (`engineering`) |
| 2 | Mathematics (`mathematics`) |
| 3 | Medicine (`medicine`) |
| 4 | Computer science and AI (`cs-ai`) |
| 5 | Natural sciences (`natural-sciences`) |
| 6 | Humanities (`humanities`) |
| 7 | Social sciences and other (`social-other`) |

- **Categories are metadata, never folders.** Reclassifying a document changes its category, not its address.
- **The backbone is OpenAlex's taxonomy:**
  - 4 domains, 26 fields, 252 subfields and 4,516 research topics.
  - OpenAlex classifies the papers it knows, so most papers are categorised by the metadata lookup itself.
  - Every topic maps to one **primary** vault category and any number of **secondary** ones.
- **The cognitive-science overlay** makes cognition-related topics from other fields (language acquisition,
  intelligent tutoring, dementia, ADHD, visual attention, the learning sciences in education) members of
  psychology and cognitive science too.
- **Study map:** psychology topics also carry an area, for focused searches:
  - perception, attention, memory, language, thinking, emotion and motivation;
  - developmental, social, personality, clinical, biological;
  - methods, applied, computational, foundations.
- **Books and preprints** are mapped through their Library of Congress or Dewey class (often printed in the book
  itself) and their arXiv category.
- **Precedence when sources disagree:** your manifest's `domain`, then OpenAlex's topic, then arXiv category,
  then LCC, then DDC, then the inbox folder hint, then the nearest topic by meaning.
  - Every assignment records its source and confidence.
- **(Phase 1)** `vault classify <doc_id> <category>` overrides a category by hand. The override is recorded and
  survives re-processing.

## 8. Finding and reading content (traversal)

### 8.1 Addresses

| Form | Example |
|---|---|
| Line range | `3f9a1c07b2e4d55e-cowan-magical-number-four#L120-L148` |
| Short form, same meaning | `3f9a1c07b2e4d55e#L120-L148` |
| Single line | `…#L120` |

- Lines are counted in the canonical Markdown. For prose, one line is one sentence; for code, one line is one
  source line.
- Responses also give the printed page ("p. 214") from the sidecar's page map, so you can check the original.
- If a document was later re-extracted, its old addresses still resolve to exactly the cited text, with a pointer
  to the newer version.

### 8.2 The tools

**(Phase 1-5)** These are the same over MCP (Claude Code, the tutor) and the native gRPC interface (the desktop
app).

| Tool | Use it to |
|---|---|
| `vault_info()` | Counts per category and format, coverage against targets, index version, deployment stage |
| `find_docs(queries[], filters, k)` | Find documents: landmark studies, textbooks, a paper you half-remember |
| `search(queries[], filters, k)` | Find passages by meaning and keywords, with addresses and pages |
| `workspace(action, doc_ids[])` | Keep a working set of documents (add, list, clear) |
| `grep(patterns[], scope, context_lines, max_hits)` | Exact or regex search. Scope is a document, the workspace, a category or study area, a repository or a collection, or the whole vault |
| `read(addresses[], expand)` | Exact text at addresses, expanded by lines or to the whole section |
| `outline(doc_ids[])` | Heading tree with line ranges and pages; for a repository, the file tree and a ranked symbol map fitted to the budget; for a code file, its symbols |
| `citations(doc_ids[], direction)` | What these documents cite, or what cites them, within the vault; for code: imports, includes, calls and references, both directions |
| `define(terms[])` | Textbook definitions, from back-of-book indexes and definition passages, and, for code, symbol definitions across repositories |

**Filters** (all optional):
- `category`, `area` (psychology study map), `type`, `level`, `year_from`, `year_to`, `language`,
  `openalex_topic`, `batch`, `collection`, `repo` (a repository name, or `name@commit` for an older snapshot).

**The intended loop:** batch everything you can into one call at each step.
1. `find_docs` and `search` with 2-4 focused queries.
2. `workspace add` the promising documents.
3. `grep` and `read` inside the workspace.
4. `citations` to follow the trail.
5. Answer with addresses.

**Example** (the shape of an MCP call):

```
search(queries=["capacity limit of working memory", "Cowan embedded-processes model", "chunking in short-term memory"],
       filters={"category": "psych-cogsci", "area": "memory"}, k=8)
grep(patterns=["magical number (seven|four)"], scope={"workspace": true}, context_lines=1, max_hits=40)
read(addresses=["3f9a1c07b2e4d55e#L120-L148"], expand="section")
```

**Response format:**
- Every response stays within a size budget (about 8,000 tokens by default). It says when it truncated and how
  to get the rest.
- Returned text is wrapped as `<vault_passage address="…" page="…" id="…">…</vault_passage>`. It is reference
  material, never instructions.

### 8.3 Connecting Claude Code

**(Phase 1)** The port and token location are filled in when libraryd ships.

- **Stage 1 (this PC):**
  `claude mcp add --scope user --transport http libraryd http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <token>"`
- **Stage 2 (AI PC):** the same command with the AI PC's private address in place of `127.0.0.1`.
- The token is in a file only your account can read. Do not paste it into chats or commit it.

### 8.4 Reading files directly

You can open `hot\text\{h2}\{doc_id}.md` in any editor: it is plain Markdown. Its sidecar `hot\sidecars\…json`
has the metadata, headings and page map. Find a document's id with `find_docs` or in the intake report. **Look,
don't edit** (section 2).

## 9. Moving the vault

- **Nothing inside the vault records a machine name, drive letter or absolute path.** Each machine has its own
  config, `%ProgramData%\vault\libraryd.toml`, outside the vault: tier roots, bind address, token file, sidecar
  URLs and model paths. Moving the vault is a config change plus a validation run.
- **Stand-in to SSD (now):**
  1. Copy the whole `E:\Stand-In Vault` folder to the SSD, for example `F:\vault`.
  2. Point the config at it.
  3. Run `vault verify` and the invariant suite. Nothing has been ingested yet, so today this is just a copy.
- **SSD to AI PC (transfer day):**
  1. Bring jobs to a clean checkpoint (writers committed, databases checkpointed, services stopped).
  2. Verify integrity and run the invariant suite.
  3. Safely remove the drive and install it in the AI PC.
  4. Write the AI PC's config, then verify and run the invariant suite again.
  5. Point the tutor and Claude Code at the network address.
  - The full procedure is in the repository README (Transfer phase).
- **The cold tier can move on its own,** for example to the AI PC's hard drive if space gets tight. Copy
  `cold\`, change `cold_root` in the config, and verify.

## 10. Rules that keep the vault trustworthy

- **Everything in the vault is readable by the tutor.** Do not deliver answer keys, eval files, private notes you
  don't want quoted, or secrets. The eval directory deliberately lives outside the vault.
- **Never edit `hot\` or `cold\` by hand.** Deliver a corrected file instead.
- **Keep the drive's own permissions** (Authenticated Users: Modify). They keep working after the drive moves to
  another PC. Don't add permissions for your personal account only.
- **Space:** the status page and `vault_info` warn before either tier passes 85% of its disk.
- **Backups:** originals, `hot\text\`, `hot\sidecars\` and `hot\db\` are the hard-won assets. Indexes rebuild.
  The backup scheme and restore test arrive in Phase 8.

## 11. Quick reference

| I want to… | Do this |
|---|---|
| Deliver papers and books | `inbox\<date-name>\` + files + optional `manifest.tsv` + `_BATCH_COMPLETE` |
| Deliver an oa-forge harvest | Drop the whole root folder into `inbox\` |
| Deliver one file | Put it in `inbox\loose\<category>\` |
| Deliver source code | Copy the repository into a batch folder (remove secrets first), + `_BATCH_COMPLETE` |
| See what happened | `inbox\_done\<batch>\_INTAKE_REPORT.md`, or `hot\reports\intake\index.html` |
| Fix a quarantined file | Deliver a better copy in a new batch |
| Find something | `find_docs` / `search`, then `grep` / `read` |
| Cite something | Copy the address it returned: `doc_id#Lstart-Lend` (+ page) |
| Check integrity | `vault verify` |
