# Phase 0: delivered material on D:\ (what the vault will ingest, as of now)

Measured 2026-09-30 22:04 to 22:07 local (EDT, UTC-4; manifest timestamps are UTC). Read-only: nothing on D:\ was changed, `oa-forge.exe` was not run, the only process launched against D: content was one `tar -tzf` and one `tasklist`. Labels: **[M]** measured by my scripts, **[D]** documented by source (cited), **[E]** my estimate. Scripts are in the session scratchpad `p0deliv\` (fmt_pdf.py, manifests.py, peek.py, recheck.py), not kept in the repo.

## 0. Bottom line

1. **Almost nothing real has been delivered yet.** All the listed locations hold 237 files and 12,701,445 bytes (12.7 MB) [M]. Of that, only **17 unique real PDFs, 10.06 MB** [M] (3 further copies are exact duplicates). Everything else is metadata (about 1.1 MB of JSON/JSONL/CSV plus a 363 KB OCW sitemap) or **synthetic mock PDFs of 23 to 409 bytes** written by oa-forge's own test harness [M].
2. **Only three folders contain real harvested content:** `D:\Corpus-pilot2` (15 PDFs, 9.71 MB), `D:\oaf-v-1991c` (2 PDFs, 0.35 MB), and `D:\oaf-v-1991` (3 PDFs, 1.19 MB, byte-identical copies of three Corpus-pilot2 arXiv papers). `oaf-v-1991b` is a real arXiv run that stored nothing. The other 17 `oaf-v-*` folders, `t-out`, `t.tar.gz`, `oaf-b.tsv`, `oaf-px.txt` are oa-forge verification fixtures (mock documents, `example.org` hosts, 127.0.0.1 proxies) [M, inference from content].
3. **Zero psychology/cognitive-science corpus yet, zero books, zero Chinese.** The `cognition` domain only holds mock stubs. The one in-domain real item is an MIT DSpace paper (Gweon, Jara-Ettinger, Schulz, Tenenbaum, "The Naive Utility Calculus ... Commonsense Psychology", 4.07 MB, CC BY-NC-ND) filed under domain `inst`.
4. **Formats:** the only real format is PDF (born-digital). No EPUB, JATS (except one 387-byte mock .nxml), TEI, arXiv LaTeX source, HTML, DOCX or Markdown anywhere [M].
5. **The owner's manifest columns are not what oa-forge writes.** `domain, level, type, title, license_class, url, provenance, mode` is oa-forge's *batch input* TSV (`--batch`), not its delivered manifest. The delivered manifest has different columns (section 4). The ingest adapter must read `_meta\manifest.jsonl` plus the `.pdf.meta.json` sidecars.
6. **Nothing is still growing.** Newest file mtime is 21:59:45, no `oa-forge.exe` process at 22:07, file count and bytes identical between two passes (my first pass 236 files + the 415-byte t.tar.gz = 237 files, 12,701,445 bytes on the second) [M].

## 1. Where oa-forge lives and what it is

- Source: `C:\Script Developement\oa-forge` (found by one directory walk; Cargo.toml, README.md, instruction.md, ARCHITECTURE.md, TASKS.md, src/). Package `oa-forge` 1.0.0, Rust edition 2024 (`Cargo.toml`). The binary `D:\oaf-bin\oa-forge.exe` is 6,709,248 bytes, mtime 2026-09-30 21:58:21 [M].
- It harvests open-access PDFs under a license gate (default-deny) into `<root>\<domain>\<source>\<item>\<item>.pdf` plus `<item>.pdf.meta.json`, with `<root>\_meta\` holding manifest, state, requests and robots caches (`README.md` line 7; `instruction.md` section 3) [D].
- Sources implemented: arXiv, Internet Archive, Harvard DASH, MIT DSpace, Europe PMC, ERIC, MIT OCW, PLOS (opt-in), curated list, CORE, Google Books, and batch/bulk clients (OAI-PMH, FTP/tarball, API, GitHub mirror) (`src\sources\*.rs`) [D].
- D:\CLAUDE.md read first; its rules (never delete/move/create on D:) were followed. No file was created on D:.

## 2. Inventory per folder

Totals include all files under each root (PDFs plus `_meta`). Time columns are file mtimes (local). "Role" is my classification from content (section 3).

| Folder | Files | Bytes | PDFs (real / stub) | Newest write | Role |
|---|---:|---:|---|---|---|
| Corpus-pilot2 | 46 | 10,403,731 | 15 / 0 | 18:30:06 | real live pilot (arxiv, mit, ocw) |
| oaf-v-1991 | 12 | 1,255,044 | 3 / 0 | 19:33:04 | real arXiv; duplicates of pilot2 |
| oaf-v-1991b | 4 | 47,197 | 0 / 0 | 19:39:02 | real arXiv run, 22 quarantined, 0 stored |
| oaf-v-1991c | 10 | 366,911 | 2 / 0 | 19:41:23 | real arXiv, 1992/1993 papers |
| oaf-v-ag | 11 | 13,547 | 0 / 1 | 21:59:45 | mock (CORE, Google Books) |
| oaf-v-ax | 10 | 5,800 | 0 / 2 | 18:47:58 | mock arXiv |
| oaf-v-cap | 12 | 42,467 | 0 / 2 | 18:47:58 | mock IA, license capture |
| oaf-v-dd | 12 | 65,204 | 0 / 2 | 21:24:19 | mock dedupe, stale lock |
| oaf-v-dd2 | 6 | 41,647 | 0 / 1 | 21:30:04 | mock, stale lock, no manifest |
| oaf-v-dd3 | 6 | 41,694 | 0 / 1 | 21:33:08 | same |
| oaf-v-dd4 | 6 | 41,662 | 0 / 1 | 21:37:43 | same |
| oaf-v-dd5 | 10 | 43,605 | 0 / 2 | 21:41:19 | mock dedupe |
| oaf-v-dd6 | 6 | 41,677 | 0 / 1 | 21:42:17 | mock, stale lock, no manifest |
| oaf-v-dd7 | 11 | 70,404 | 0 / 2 (+2 dedup rows) | 21:45:37 | mock dedupe |
| oaf-v-dd8 | 11 | 70,359 | 0 / 2 | 21:46:28 | mock dedupe |
| oaf-v-e19 | 9 | 8,108 | 0 / 1 | 20:09:12 | mock PLOS |
| oaf-v-ftp | 11 | 5,642 | 0 / 2 | 20:56:58 | mock bulk FTP/tarball |
| oaf-v-iarelax | 10 | 41,292 | 0 / 2 | 18:47:34 | mock IA relaxed |
| oaf-v-iastrict | 9 | 39,619 | 0 / 1 | 18:47:34 | mock IA strict |
| oaf-v-plos | 9 | 6,933 | 0 / 1 | 18:47:27 | mock PLOS |
| oaf-v-q | 11 | 47,772 | 0 / 2 | 21:16:01 | mock PDF quality check |
| t-out | 2 | 506 | 0 / 1 | 2023-11-14 (tar mtime) | extracted fixture (PMC100) |
| t.tar.gz | 1 | 415 | n/a | 20:52:40 | fixture archive |
| oaf-b.tsv, oaf-px.txt | 2 | 209 | n/a | 20:51 / 20:09 | batch input fixture, proxy list |
| **Total** | **237** | **12,701,445** | **20 real / 27 stub** | 21:59:45 | |

Folder mtimes of the "last few hours" folders: oaf-v-ag 21:59:45, dd8 21:46:28, dd7 21:45:37, dd6 21:42:17, dd5 21:41:19, dd4 21:37:43, dd3 21:33:08, dd2 21:30:04, dd 21:24:19, q 21:16:01, ftp 20:56:58 (all older than 22:00; the 'Newest write' column above gives each folder's newest file).

**Corpus-pilot2 cross-check:** my recount (15 rows, 15 PDFs, 9,708,484 bytes, every sha256 and size matches its manifest row) equals the figure TASKS.md reports for `--verify` ("15 rows, 15 files, 9708484 bytes", `C:\Script Developement\oa-forge\TASKS.md` PART A) [M vs D].

## 3. Formats by content (magic bytes and markers, not extension)

All 237 files classified by head bytes / archive members (script `fmt_pdf.py`):

| Detected format | Files | Bytes | Notes |
|---|---:|---:|---|
| PDF (>= 1 KB) | 20 | 11,247,923 | 17 unique; 3 duplicated into oaf-v-1991 |
| PDF stub (< 1 KB) | 27 | 3,720 | all begin `%PDF-1.4` + "oa-forge mock document" (or similar); not real papers |
| JSONL (metadata) | 63 | 996,913 | manifest, state, requests, quarantine |
| JSON (metadata) | 104 | 74,059 | sidecars, robots caches, sha_index |
| XML other | 1 | 363,047 | `_meta\ocw-sitemap.xml` (2,588 `<loc>` entries) |
| CSV / TSV | 16 / 1 | 14,597 / 161 | manifest.csv mirrors / oaf-b.tsv |
| JATS XML | 1 | 387 | `t-out\PMC100.nxml`, a mock `<article>` with a CC BY licence link |
| plain text | 3 | 223 | one skipped log, proxy list, one quality note |
| EPUB / TEI / arXiv LaTeX archive / HTML / DOCX / Markdown | 0 | 0 | none present |

`t.tar.gz` (415 bytes, listed with one `tar -tzvf`): two members, `PMC100.nxml` (387 B) and `PMC100.pdf` (119 B, mock). It is **not** an arXiv LaTeX source archive (no .tex). `t-out` is its extraction; `PMC100.pdf` there is byte-identical to `oaf-v-ftp\...\PMC100.pdf`. Its mtime 2023-11-14 is just the tar member time.

### Real versus mock evidence
- Mock: PDF bodies like `%PDF-1.4 % oa-forge mock document: pair deposit A`; titles "Mock PMC Paper", "Mock Attention Study", "CORE Nine"; hosts `ftp.example.org`, `cdn.example.org`; `D:\oaf-px.txt` lists `http://127.0.0.1:18080` and `:18081` (same pair as `tests\evals.rs` lines 988 and 1024, no credentials in it); batch row URL `https://ftp.example.org/pub/oa_file_list.csv` [M].
- Real: hosts in `Corpus-pilot2\_meta\requests.jsonl` are ocw.mit.edu (81 requests, HTTP 200), openstax.org (17), dspace.mit.edu (19), arxiv.org (11), cf003.cdn.4science.cloud (5), plus cnx.org, loc.gov, winstonchurchill.org, dni.gov [M].

## 4. Manifest formats: how a row maps to a file, and how that differs from the owner's columns

### 4.1 What oa-forge writes (delivered manifest)
Defined in `C:\Script Developement\oa-forge\src\manifest.rs` (struct `Row`, lines 14-42; CSV header line 168) [D], confirmed on disk [M]:

`manifest.jsonl` keys: `domain, source, category, title, id, version, timestamp, bytes, sha256, file, license` always; `authors, year` (only on newer-build rows), `dedup_of, dedup_note, license_class` (only when set).
`manifest.csv` mirror has **three different headers across the 16 roots** (schema drift between oa-forge builds) [M]:
- 12 columns (1 root, Corpus-pilot2): `domain,source,category,title,id,version,timestamp,bytes,sha256,file,license,dedup_of`
- 13 columns (10 roots): the above + `license_class`
- 16 columns (5 roots): `domain,source,category,title,id,version,authors,year,timestamp,bytes,sha256,file,license,dedup_of,license_class,dedup_note`

Sidecar `<item>.pdf.meta.json` keys [M]: always `authors, bytes, category, domain, fetched_utc, id, license, sha256, source, title, url, version, year`; sometimes `pdf_pages, text_extract, tiny_bytes, pdf_title, src_title, meta_conflict, pdf_authors, pdf_dois, pdf_year` (extracted from the PDF itself), `license_class, license_seen, license_source_url, publisher, source_host, license_relaxed`, and for bulk rows `bulk_provenance, channel, channel_ref`. Sidecars on the real pilot2 rows have authors and year even though the pilot2 manifest rows do not.

Other `_meta` files: `sha_index.json` (sha256 to first stored file), `state.jsonl` (cursors, stage done records, skip counters), `requests.jsonl` (every HTTP request, including `robots_exempt`), `quarantine.jsonl` (id, source, url, license_seen, reason), `skipped-paywalled.log` (6 tab fields), `ocw-sitemap.xml`, `robots\<host>.json`, `manifest.lock` (`README.md` item 7) [D, M].

### 4.2 How a row links to its file
- **`file`** = root-relative path with forward slashes, e.g. `math/arxiv/2609.37839v1/2609.37839v1.pdf` [M]. Layout is `<domain>/<source>/<item>/<item>.pdf`; `<item>` is the sanitized id, with an 8-hex hash suffix when sanitization or length changes it (`alg-geom/9205001` becomes `alg-geom_9205001v2_aa78d406`; OCW ids become `1-00-introduction-to-computers-and-engineering-problem-_2e577fea`) [M].
- Unique row key is `(source, id, version)` (`manifest.rs` `insert`, line 83). Content identity is `sha256` plus `bytes` on every row. Sidecar path is `file + ".meta.json"`. Paths are relative to the root, so **each root is its own namespace**: the same paper can occur in two roots (3 papers do).
- `dedup_of` rows: file and `dedup_of` point at the surviving file, while `bytes` and `sha256` stay those of the dropped duplicate "for the audit trail" (`manifest.rs` `repoint_to_dedup`, lines 117-136) [D]. Consequence measured: 43 of 45 rows resolve to an existing file; 40 match size and sha; 3 "mismatches" are exactly dedup rows; 2 dedup rows point at files that no longer exist because a newer arXiv version replaced them [M]. An ingester must skip `bytes`/`sha256` checks on rows with `dedup_of` set.
- Source-specific ids: arXiv `2609.37839`, MIT DSpace UUID, OCW `<course>/<file>.pdf`, PLOS DOI, PMC id, IA identifier, CORE DOI.

### 4.3 Batch-complete marker or checksum
- **No batch-complete marker exists on disk** (no `.done`, `COMPLETE`, `SUMS`, no per-batch checksum file) [M].
- Closest signals: `state.jsonl` records `{"k":"stage","key":"<stage>","v":{"done":true,"at":...}}` (present in 10 roots; Corpus-pilot2 shows only `curated` done); the run's stdout `== oa-forge report ==` and final `REPORT-JSON {...}` line (stdout only, **not persisted** to D:); and the offline `oa-forge --verify` (exit 0/1) that re-hashes every row (`README.md` item 7) [D]. I did not run it; my own re-hash of all 45 rows agrees with section 2.
- Integrity: sha256 per row in manifest.jsonl/csv, in each sidecar and in `sha_index.json`. Manifest/state writes are atomic (temp name suffix `.oaf-tmp` then rename, `src\atomic.rs` lines 8-36) [D], so a reader should ignore `*.oaf-tmp`; none present now.
- `_meta\manifest.lock` (JSON `{"at":<unix>,"pid":N}`) is the single-writer lock; it is removed on clean exit and reclaimed when stale (`manifest.rs` 256-302) [D]. **Five roots still carry one** (dd, dd2, dd3, dd4, dd6; pids 11836, 41468, 40876, 24608, 32428), and no oa-forge process is running [M]: these are killed or abandoned runs. dd2, dd3, dd4, dd6 also have no manifest.jsonl at all. Their locks and write times (21:24 to 21:42) are not evidence of a live writer.
- Practical rule for ingest: treat a root as complete only when no `manifest.lock` exists and the producer reports a verify pass; otherwise it is a snapshot. An explicit "batch finished" marker would have to be added (not mine to decide).

### 4.4 The owner's stated columns versus oa-forge
Owner columns: `domain, level, type, title, license_class, url, provenance, mode`. This is **exactly** the header of `D:\oaf-b.tsv` and of `struct BatchRow` in `src\sources\batch.rs` lines 1-30 (8 tab-separated columns; header row skipped when first column is "domain"; `mode` is `targeted` or `bulk`) [D, M]. It is the harvest *request* list, not a record of delivered files. Mapping to what is stored:

| Owner column | Equivalent in delivered data | Gap |
|---|---|---|
| domain | `domain` (folder and manifest) | same name; values are folder names (`math, textbook, inst, cognition, works`) |
| level | **none** | `BatchRow.level` is parsed but never read (grep of `src` finds no use) [M]; it is not carried into any manifest or sidecar |
| type | `category` | for targeted rows `category = type` (`batch.rs` line 131); for the one bulk row observed it became `category: "ftp"` (the channel), so the type "dump" is lost |
| title | `title` | same |
| license_class | `license` (free-text string) and, separately, `license_class` | **name collision, different meaning.** Input `license_class` is the *stated licence* or the keywords `open` / `captured` (`batch.rs` 115-123). Output `license_class` is only the accession class `open` or `captured`, present on 3 of 45 rows. The licence string is in output `license` |
| url | sidecar `url` (and `requests.jsonl`) | not in manifest.jsonl/csv; for bulk rows the sidecar `url` is a local `file://...\_meta\parts\...` path, the remote one is in `channel_ref` |
| provenance | sidecar `bulk_provenance` (bulk) | not in the manifest row; for targeted rows only an internal `batch_provenance` field in the in-memory item, which I did not see in any sidecar |
| mode | sidecar `channel` (`ftp`) | `mode` itself is not stored |
| (none) | `source, id, version, timestamp, bytes, sha256, file, authors, year, dedup_of` | not present in the owner's list |

Not observed: any **targeted** batch row's output. No delivered manifest has `source = "batch"` (distinct sources are arxiv, ia, mit, ocw, plos, core, bulk), so how targeted rows land is only known from code, not from data.

`oaf-b.tsv` content (161 bytes, one data row): `cognition | 1 | dump | PMC slice | (empty) | https://ftp.example.org/pub/oa_file_list.csv | [via NCBI OA Subset FTP] | bulk` [M].

## 5. Distinct values in the delivered manifests (45 rows, 16 manifests)

These are inflated by mock rows; the real-only line follows each [M].

- **domain:** math 18, textbook 18, inst 5, cognition 3, works 1. Real only: math 10, inst 5, textbook 5, cognition 0.
- **source:** arxiv 18, ia 13, mit 5, ocw 5, plos 2, core 1, bulk 1. Real only: arxiv 10 rows (7 unique ids), mit 5, ocw 5.
- **category:** `cognition` 13, `math.AC` 10, `math.AG` 8, `calculus` 5, OCW course slugs 5, `attention` 3, `ftp` 1. The MIT items are labelled `calculus` because that was the DSpace query term, so `category` is a search term, not a subject taxonomy. Note: the psychology paper is in `calculus`.
- **license** (free text, 9 distinct strings): CC BY 4.0 26, CC BY-NC-SA 4.0 6, arXiv nonexclusive license 5, CC BY-SA 4.0 2, CC BY-NC-SA 2, CC BY-NC-ND 1, CC BY-NC-SA 3.0 1, "Copyright 2020 University of X. All rights reserved." 1 (mock, captured), "Open (relaxed: This article is freely accessible)" 1 (mock). Real rows contain NC, ND and the arXiv nonexclusive licence, so no normalised SPDX class exists and a licence field with those semantics will have to be derived.
- **license_class:** absent 42, open 2, captured 1 (all mock). **version:** empty 27, 1 13, 2 5. **dedup_of:** set on 5 rows (`version:1->2` 3, `fuzzy:dedupe pair paper||` 2), all mock. **year:** present on 7 manifest rows only.
- **level / type / mode** (owner columns): only oaf-b.tsv has them: level `1` (1), type `dump` (1), mode `bulk` (1), license_class empty (1), domain `cognition` (1).
- **Quarantine (licence gate)** [M]: Corpus-pilot2 8 MIT items (2 `missing-license`, 6 `unrecognized-license`); oaf-v-1991b 22 arXiv items `unrecognized-license`, all with `license_seen = http://arxiv.org/licenses/assumed-1991-2003/` (pre-2004 arXiv papers). 1991c, run about 7 minutes later, stored two of those same ids (`alg-geom/9205001`, `alg-geom/9305010`) as "arXiv nonexclusive license", so the gate behaviour for 1991-2003 arXiv changed between runs; I did not investigate why. Pilot2 skip counters also show 15 and 16 `robots-disallowed` and 1 `no-pdf` [M]. TASKS.md documents that DASH and ERIC items are overwhelmingly quarantined and that Harvard DASH 3,341 and MIT ~54 items were quarantined in the earlier live pilots (those scratch roots are not on D:, not measured here) [D].

## 6. Born-digital versus scanned, and CJK (PDF sample)

Sample: all PDFs present (47, below the 300 cap): 20 real (17 unique after sha dedup) and 27 stubs. Method: raw bytes, zlib-decompress of every stream (so fonts and operators inside object streams are visible), count of `/Type /Font`, `/FontFile*`, `/Subtype /Image`, `BT` blocks, `Tj`/`TJ` operators, invisible-text render mode (`3 Tr`, an OCR-layer marker), image filters (DCT, CCITT, JBIG2), `/Producer` and `/Creator`, page counts from `/Count`. No OCR/ink/text extraction was run.

**Result for the 17 unique real PDFs [M]:** all 17 are **born-digital** (hundreds to thousands of text operators, embedded font files, `3 Tr` count 0, `/Subtype /Image` count 0 for 13 of them, 1 to 6 for the rest). The one flagged "mixed" is an MIT OCW Word 2010 lecture handout (10 pages, 6 images, 454 text operators): still born-digital, figure-heavy. **Scanned: 0 of 17.**
- Producers seen: arXiv GenPDF (tex2pdf) via pikepdf (the 5 arXiv papers from 2026), dvips + Ghostscript (the 2 arXiv papers from 1992/93 and 1 MIT paper), matplotlib, Ghostscript/pdftk, OmniGraffle/Quartz (MIT papers), PowerPoint/Word/Acrobat Distiller (OCW).
- Pages: 304 pages over 17 PDFs (mean 17.9, range 4 to 49); mean 33 KB per page [M].
- 27 stub PDFs have no pages, fonts or images and are excluded from the sample; one stub (`oaf-v-q\ia-q-meta`) is deliberately built with 2 `BT` pages to test oa-forge's PDF meta extractor, and one (`ia-q-bin`) deliberately has none.

**CJK signal [M, low confidence for the absence claim]:**
- Zero CJK characters in any manifest, sidecar, quarantine, sitemap or text file (0 of ~1.1 MB metadata, including 363 KB OCW sitemap). All titles are English; two have diacritics (Knörrer, Naïve) and are valid UTF-8 (raw bytes `Kn\xc3\xb6rrer`; the replacement glyph that appeared in my first terminal output was a display artifact, no invalid UTF-8 in any manifest).
- CJK font/CMap names found in only 2 of 17 PDFs: `MS-PGothic-90ms-RKSJ-H` and `ArialUnicodeMS-90ms-RKSJ-H` in two PowerPoint-derived OCW handouts. These are the Japanese Shift-JIS fallback mappings that Office writes for Latin text; I treat them as non-CJK content, but font names alone cannot prove that, and I did not extract page text. `Identity-H` CID fonts occur in 12 of 17 (arXiv tex2pdf output, Office); that is normal for Latin and not a CJK indicator.
- A first pass with a loose font-name regex produced false positives on compressed binary (names like `FZG`); I discarded it and repeated the check only on `/BaseFont` values.
- **Conclusion:** no Chinese material has been delivered, and there is no evidence of CJK glyph content in the 17 real PDFs. A Chinese test set must come from elsewhere (oa-forge's own TASKS.md notes that the Chinese repositories it probed, e.g. ir.pku.edu.cn, oar.tsinghua.edu.cn, were unreachable or failed DNS) [D].

## 7. Duplicates (size + sha256, all 237 files)

- 10 groups, 19 redundant files, 1,206,126 redundant bytes [M]. Real: **3 arXiv PDFs** (2609.37931, 2609.37967, 2609.38068; 1,186,683 bytes) exist in both `Corpus-pilot2` and `oaf-v-1991`. The rest is mock fixtures (stub `ia-dup-a.pdf` 127 bytes x8, `2601.00010v2` x3, PMC100 stub x2, repeated `state.jsonl`, etc.).
- **Near-duplicates not caught by hash:** MIT items `cedb9760-...` and `e4981c29-...` are both titled "Cache Calculus: Modeling Caches through Differential Equations" (355,655 vs 401,801 bytes; CC BY-NC-SA vs CC BY 4.0), i.e. the same work deposited twice. Two OCW PDFs also share a course. oa-forge already has a fuzzy key (title + first-author surname + year, `harvest.rs fuzzy_key`) but it needs authors and year in the row; Corpus-pilot2 rows lack them in the manifest [M, D].
- Version chains: arXiv `2601.00010` v1 replaced by v2 in the mock roots (`dedup_note version:1->2`).

## 8. Language

Titles and metadata: all English; no Chinese titles anywhere in the manifests (45 rows), sidecars (43), quarantine (about 33 rows) or request logs [M]. arXiv categories are math.AC and math.AG only; MIT items are CS/cognitive-science/mathematics papers; OCW items are MIT engineering course handouts [M].

## 9. Totals and cold-tier projection

Measured delivered: 12,701,445 bytes total; real unique PDFs 17 files / 10,061,240 bytes (mean 591,838 B, median 355,655 B) [M].

| Real class (unique) | n | Bytes | Mean | Median |
|---|---:|---:|---:|---:|
| arXiv math (2026 + 1992/93) | 7 | 2,218,221 | 316,889 | 267,968 |
| MIT DSpace papers | 5 | 6,699,594 | 1,339,919 | 401,801 |
| MIT OCW handouts | 5 | 1,143,425 | 228,685 | 194,369 |

n is tiny and unrepresentative (maths and handouts, one 4 MB outlier). Projection for the owner's targets, **my estimates [E], not data**:
- Papers: 150,000 x 0.36 MB (median) to 0.59 MB (mean) = **54 to 89 GB**. Psychology journal PDFs with figures are usually larger than arXiv maths; a plausible band is 0.3 to 1.5 MB, giving **45 to 225 GB**. A cross-check from TASKS.md [D]: 8 Europe PMC articles from the earlier live pilot totalled 86.7 MB, about 10.8 MB each (a scratch root outside D:, not re-measured here); if that were typical biomedical open-access size, 150k papers would be about 1.6 TB, so the paper estimate is highly source-dependent and must be re-measured on a real psychology sample before the SSD is sized.
- Books: no book has been delivered. 10,000 x 5 to 25 MB (text-based EPUB/PDF) = **50 to 250 GB**; scanned page-image books at 50 to 200 MB would reach 0.5 to 2 TB and would not fit a 1 TB SSD with the indexes. The scanned share is the biggest unknown for the cold tier.
- Rough cold-tier (originals) range: **about 100 to 475 GB** for the central assumptions, i.e. fits 1 TB only if scanned books are limited. Delivered-so-far share of the target: 17 of 150,000 papers (0.011%), 0 of 10,000 books.
- Pilot2 per-item pace as a throughput hint [M, from manifest timestamps]: 5 arXiv PDFs between 22:24:13Z and 22:25:28Z (75 s from first to fifth), 5 MIT PDFs 22:25:54Z to 22:26:14Z (20 s), 5 OCW PDFs 22:26:53Z to 22:29:21Z (148 s), i.e. a few seconds to tens of seconds per item. oa-forge is intentionally slow (2.0 s per host) [D, `consts.rs`], so harvest rate, not disk, will gate filling the vault.

## 10. Items for the lead to decide or know (none acted on)

1. The ingest adapter's source of truth should be `_meta\manifest.jsonl` + sidecars, not the owner's 8-column TSV; the owner's `level` and `mode` are not preserved by oa-forge, and `license_class` has two meanings (section 4.4).
2. manifest.csv has three schema variants across roots; manifest.jsonl is the safer reader (optional keys), but mind `dedup_of` rows (bytes/sha belong to the dropped file).
3. No batch-complete marker: five roots hold dead `manifest.lock` files; there is no persisted run report. A completeness rule must be defined by the lead (section 4.3).
4. `level` is accepted in the batch TSV but ignored by the code (batch.rs); if the owner needs a reading-level or tier field in the vault, it must be added upstream or tracked separately.
5. `category` is an arbitrary harvest query term (`calculus`), not a topic: topic metadata for psychology/cognitive science must come from elsewhere (OpenAlex/GROBID), as the delivered data carries none.
6. For test material, the useful real set is small: 17 unique PDFs (7 arXiv math, 5 MIT papers incl. 1 psychology, 5 OCW handouts). The 27 mock stubs and metadata-only roots are fixtures; I would not ingest them as corpus.
7. The OCW and MIT items carry NC/ND licences; the owner's tutor use is personal, but the licence column must be kept for any later sharing decision.
8. Not verified: that nothing outside the listed paths holds delivered material (e.g. `E:\Corpus`, `C:\Corpus`, the other scratch roots TASKS.md mentions); oa-forge's default root is `$env:CORPUS_ROOT`, else `E:\Corpus`, else `C:\Corpus` (`README.md` item 2). TASKS.md says an accidental `E:\Corpus` run was removed, so a larger "operator full harvest" folder may exist elsewhere. I was asked only about the listed D: paths and did not look.

## 11. Sources

- `D:\CLAUDE.md` (drive rules), `D:\oaf-b.tsv`, `D:\oaf-px.txt`, `D:\Corpus-pilot2\_meta\*`, `D:\oaf-v-*\_meta\*`, `D:\t-out\*`, `tar -tzvf D:\t.tar.gz`.
- `C:\Script Developement\oa-forge\README.md`, `instruction.md`, `TASKS.md`, `Cargo.toml`, `src\manifest.rs` (Row, flush, lock), `src\state.rs`, `src\atomic.rs`, `src\sources\batch.rs` (BatchRow, route_channel, targeted_item), `src\consts.rs`, `tests\evals.rs` (proxy fixture lines 988, 1024).
