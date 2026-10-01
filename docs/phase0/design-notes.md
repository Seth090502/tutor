# Phase 0 design notes (lead's working draft; the proposal cites these)

Nothing here is final. Each item says what it depends on.

## 1. Identity and addresses

**Rule: published text never changes.** A file `text/{h2}/{doc_id}.md` is written once and never edited.
- Re-extracting with a better path (for example, a MinerU retry, or an extractor upgrade later) creates a
  **new version** with its own `doc_id`.
- A `lineage` table links each version to its original, and `read` on an old address returns the old text
  exactly, plus a `superseded_by` pointer.
- So every address ever cited resolves to exactly the text that was cited, forever. This is the only way
  "addresses stay valid" and "documents can be re-extracted" can both hold. The brief's
  `doc_id = hash(normalized text)` already implies it.

**doc_id format:** 16 hex characters of BLAKE3(normalized text), then `-`, then a slug, for example
`3f9a1c07b2e4d55e-kahneman-thinking-fast-and-slow`.
- **Why 16 hex:** the brief's example uses 6 hex (24 bits). At 160,000 documents a 24-bit id collides almost
  surely (birthday bound: n²/2^25 ≈ 760 expected pairs). At 64 bits the chance of any collision is about
  7e-10. Intake still checks, and quarantines a colliding document with a stated reason.
- **The slug** is ASCII-folded from the title, at most 48 characters, and frozen when the document is first
  published. Fixing the title later changes metadata, not the id.
- **The resolver** accepts the bare 16-hex prefix. Tools return the full form; citations may use either.

**Address grammar:** `{doc_id}#L{a}-L{b}`, or `{doc_id}#L{a}` for a single line. Lines are 1-based and inclusive.

**Line granularity: one sentence per line** ("semantic line breaks") for prose.
- Markdown renders it identically, because a single newline inside a paragraph is a soft break.
- Paragraphs stay separated by blank lines.
- **Why:** a line address then pins a claim to a sentence, not to a 2,000-character paragraph. That makes
  "does the cited text support the claim" checkable, and keeps `read` responses small.
- Tables, code blocks, display equations, headings and reference entries keep their natural lines.
- Chinese sentences split at 。！？.
- A splitting error never loses text; it only moves a line break.
- Cost: about 100M lines at full scale. Fine for grep, and the line index is in the sidecar.

**Page numbers:** the sidecar holds a page-to-line map. It records the printed page label (PDF /PageLabels,
or a detected running number, or the EPUB page-list) and the physical page index. `read` returns the printed
page for the first line of the range.

## 2. Canonical Markdown conventions (all paths)

- UTF-8, NFC, LF line endings, no BOM, no trailing spaces, no control characters except tab in code.
- Headings: `#` levels follow the document structure. The document title is the only `#`.
- **Maths:** inline `$…$`, display `$$…$$` on lines of their own, with LaTeX kept verbatim from the source
  (LaTeX sources, JATS MathML via conversion, MinerU output). A plain-text rendering is indexed alongside,
  not stored in the text.
- Tables are GitHub-style pipe tables. A table that can't be represented is kept as a fenced `html` block
  (flagged in quality).
- Code is in fenced blocks, with the language tag when known.
- **References:** a `## References` section with one entry per line, numbered as in the source. The sidecar
  holds the parsed fields (from GROBID or JATS).
- **Books:** front matter, TOC, chapters, notes and the back-of-book index (`## Index`, one entry per line)
  are kept as sections.
- Figure and table captions are kept where they appear (`*Figure 3.* …`).

## 3. Indexes and why these choices

- **tantivy, chunk level:** BM25 with positions (for phrase and proximity) on the chunk text, with the
  indexing-only header `Title | Authors | Year | Section path`.
  - Analyzers: English (stemmed, maths-normalised), Chinese (dictionary or bigram, chosen in Phase 3),
    and a no-stemming fallback.
  - Fast fields: `doc_key`, `chunk_key`, `field`, `level`, `year`, `lang`, `source_type`.
- **tantivy, document level: metadata and structure, not full text.** Title, authors, abstract, keywords, TOC
  headings, back-of-book index terms, venue. BM25 over whole books mostly ranks by length and is a poor
  router; structure terms are what `find_docs` needs. *(Disagrees with "document-level BM25" if that meant
  full text.)*
- **Trigram prefilter for regex: test block granularity against document granularity.** Every book contains
  nearly every common trigram, so a document-level trigram index filters poorly for patterns made of common
  characters. Proposal: line-aligned blocks of about 16 KiB (about 800k blocks at full scale), with postings
  that record only which blocks contain each trigram. Measure index size and p95 for a set of regexes in
  Phase 1, and keep the better one. Regex-to-trigram planning follows Russ Cox and Zoekt, using
  `regex-syntax` (ripgrep's parser). Verification uses `grep-regex` and `grep-searcher` over just the
  candidate blocks.
- **Vectors:** chunk, section and document levels. The raw vectors (f16) live in Lance tables as data. The ANN
  index is chosen by the Phase 0 benchmark.

## 4. Intake handoff (proposal)

- **Batch folder plus marker.** The owner copies a batch into `inbox/{batch-name}/`: the files plus an
  optional `manifest.tsv`. When copying is finished, the owner creates `inbox/{batch-name}/_BATCH_COMPLETE`.
  It can be empty, or hold `files=<n>` so intake can check the count.
- **Intake ignores a batch folder until the marker exists.** Then, for each file, it also checks that the size
  and modification time are unchanged across two looks 30 s apart, and that the file opens with
  no other writer (a Windows share-mode check). A copy still in progress, including over SMB, holds the file
  open for writing and fails that check.
- **Loose files without a marker** are accepted after 15 minutes without change and the same open check.
  The batch is then named after the inbox day.
- **Manifest changes needed from the owner.** The stated columns (domain, level, type, title, license_class,
  url, provenance, mode) have no column linking a row to a file.
  - **Required:** `file`, the relative path inside the batch.
  - **Optional, used when present:** `doi`, `arxiv_id`, `isbn`, `pmcid`, `authors`, `year`, `language`,
    `sha256` (lets intake prove the copy is intact), `edition`.
  - Unknown extra columns are kept verbatim in provenance.
- **Originals are moved, not copied.** After hashing, intake moves each delivered file from the inbox into
  `originals/{h2}/{blake3}.{ext}` with one rename on the same volume. Copying would hold every original twice
  on a 1 TB drive.
  - If the cold tier is on another volume, intake copies, verifies the hash, then moves the inbox file to
    `inbox/_done/{batch}/` for the owner to delete.
  - The original file name, batch, date and manifest row are kept in SQLite.
  - **This touches originals, so it needs the owner's approval.**

## 5. GPU windows (the 5090 also hosts the tutor)

- GPU stages (MinerU, OCR, bulk embedding, enrichment) never start a unit of work unless free VRAM is above that
  stage's measured need, read from `nvidia-smi` once per unit.
- The owner can run `vault gpu yield`, which writes a flag file. GPU stages then stop at the next unit
  boundary (within about a minute) and release the GPU.
- The owner reloads the tutor; the queue resumes when VRAM frees again or on `vault gpu resume`.
- Nothing ever stops Cinference or Salad.

## 6. Storage arithmetic

Formulas used in the proposal; inputs come from the delivered-material report.

| Item | Formula (estimate) |
|---|---|
| Text store | 2.5B tokens × ~4.2 bytes per token (English, Markdown) ≈ 10.5 GB, plus ~10% for sentence line breaks and markup ≈ 11.5 GB |
| Sidecars | ~160k × ~30 KB (heading tree, page map, references) ≈ 5 GB |
| SQLite | metadata ~1 GB, citation edges (~6M refs × ~150 B) ~1 GB, job and quality records ~1 GB, glossary ~0.5 GB |
| tantivy chunks | postings with positions ≈ 0.35-0.5 × text ≈ 4-6 GB, plus fast fields ~0.3 GB |
| tantivy trigram | to be measured; estimate 3-10 GB depending on granularity |
| Raw vectors | N × dims × 2 bytes (f16): 6M × 1024 = 12.3 GB; × 2560 = 30.7 GB; × 4096 = 49 GB. Sections and documents add ~25% |
| ANN index | from the vector benchmark |
| Rebuild headroom | one extra copy of the largest index during a rebuild |
| Originals (cold) | measured mean size per format × count |

## 7. Pilot composition (request to the owner; about 7,000 documents)

**By field** (papers and books):

| Field | Papers | Books |
|---|---|---|
| Psychology and cognitive science | 3,150 | 350 |
| Engineering | 630 | 70 |
| Mathematics | 560 | 140 |
| Medicine | 570 | 30 |
| Computer science and AI | 620 | 80 |
| Natural sciences | 450 | 50 |
| Humanities | 220 | 80 |
| **Total** | **6,200** | **800** |

**Minimum count per extraction path** (the categories overlap the field counts):

| Path | Minimum count |
|---|---|
| JATS XML (PMC open access) | 1,000 |
| arXiv LaTeX source archives | 500 |
| arXiv HTML | 200 |
| EPUB books | 250 |
| HTML books or long-form web texts | 50 |
| Born-digital paper PDFs (no structured version delivered) | 2,500 |
| Born-digital book PDFs | 300 |
| Scanned PDFs (old papers and books) | 300 |
| Maths-heavy PDFs | 300 |
| Chinese-language papers | 300 |
| DOCX | 30 |
| Markdown or plain text | 50 |

**Special content:**
- 150 books with a back-of-book index.
- 50 code-heavy programming books.
- Landmark psychology papers covering the eval topics (from your hand-written questions).

**Deliberately messy files (about 150):**

| Kind | Count |
|---|---|
| Bad scans | 30 |
| Odd encodings (Latin-1, GBK, mojibake) | 20 |
| Truncated or corrupt PDFs | 20 |
| Password-protected PDFs | 10 |
| Zero-byte or not-a-document files | 10 |
| Exact re-deliveries | 30 |
| arXiv-preprint and published-version pairs | 30 |
| Pairs of book editions | 10 |

**Manifest coverage:**
- About 5% of files with no manifest row.
- A few manifest rows with no file.

## 8. Pre-transfer schedule (transfer assumed Monday 2026-10-05, morning)

| When | Work | Depends on |
|---|---|---|
| Wed night | Phase 0 audits; vector benchmark; model-benchmark preparation | nothing |
| Thu 10-01 | SSD attached and measured; GPU window (~1.5 h) for the model matrix; Phase 0 report; **approval** | owner |
| Thu-Sat | Phase 1: intake, CPU extraction paths, GROBID, the store, the integrity manifest, vault-core exact tools over MCP and gRPC, tutor bridge, invariants including portability. MinerU and OCR run in GPU windows (estimated 4-10 h for the pilot's scanned and maths-heavy pages) | pilot delivered; GROBID hosting approved |
| Sat-Sun | Phase 2: eval layers A-C, layer D harness; **two owner reviews**; baselines with the tutor loaded | owner reviews; tutor available |
| Sun | Phase 3 (BM25), if Phase 2 is through | |
| Sun evening | Clean checkpoint: jobs drained, writers committed, WAL checkpointed, services stopped, integrity and layer A recorded, full backup of the vault to E: (about 50-80 GB at pilot scale, about 10 minutes at 120 MB/s) | |
| Mon morning | Transfer | owner |

**Note:** bulk GPU work does not need the drive to be local. MinerU runs at about 1-3 pages/s, which is under
1 MB/s of PDF input, far below a gigabit link (to be measured in Stage 2). Embedding sends text, not files. So
the real pre-transfer constraint is GPU time shared with the tutor, not drive locality. The schedule therefore
puts correctness and a verified checkpoint first, and runs GPU extraction in owner-approved windows.
