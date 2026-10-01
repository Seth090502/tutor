# rag-engine adoption plan: retrieval for 10k books now and 10-50M chunks later (2026-09-30)

**What I read.**
- The sweep in `docs/tutor/research/rag-sota/2026-09-30-sweep.json`: 81 findings across 6 dimensions.
- The deep-dive results: 9 groups with content. The "platform" group was empty.
- `crates/rag-engine` on `custom` at 26041759: ingest, chunk, extract, triage, tokenize, bm25, search, vector, store, pages, headings, ocr, embed, meta and discover.
- `docs/tutor/CORPUS.md`.
- `CONTRACTS.md` sections C1, C2, C9 and C11.
- `PLAN.md` decisions D1, D7-D9 and D13.
- Task P3 in `tasks.json`.
- The gold-matching rules in `evals/tutor_evals`.

**What I did not read.** The worktrees `wt/rag`, `wt/rag-pages`, `wt/rag-remote` and `wt/corpus-search` may already change some of these files. Reconcile them before assigning work.

**What I ran.** I edited no files and ran no code. The only commands were `ls`, `du` and `df`, in this session.

**The state that decides the order.** No corpus or index exists in this checkout yet. `corpus/` holds only empty domain folders, and there is no `data/rag` (M: ls, this session). So any change to the storage format costs nothing today. After the first bulk ingest, the same change means re-extracting the whole library. That is why this plan puts format work first.

**Number tags:**
- **(M: source)**: measured, and I name who measured it.
- **(V)**: a vendor's or author's claim that nobody has reproduced independently.
- **(E)**: my own estimate or arithmetic. Every threshold I propose is also (E).
- **(req)**: a requirement from the brief, PLAN or a contract.

Version numbers are not tagged.

---

## 1. Verdict

**Keep the current architecture and harden it; do not adopt a platform.** The current architecture is:
- a Rust rag-engine with tantivy BM25 as the retrieval path that always runs;
- SQLite as the one authority on what exists and what may be shown;
- LanceDB as an optional vector path;
- the Python backend for LLM query rewrites, packing evidence, the answer-leak guard and showing citations.

**Why not a platform.** None of the frameworks reviewed documents a corpus near 10-50M chunks. None gives offset-level provenance (sweep, frameworks findings 0 and 8). Every model-based parser, embedder and reranker worth having needs a GPU.

**What matters now needs only the CPU and no models:**
- **A provenance core based on spans.** Page text is stored once, compressed. A chunk's id is its page-and-offset span, so it survives re-chunking. Bounding boxes are computed from PDFium when needed.
- **Lexical analysis per language.** Japanese and Chinese are effectively unsearchable today. Spanish, French and German go through the English stemmer.
- **PDFium extraction in several processes, with page routing.** This should cut the extraction part of the first 10k-book ingest from about 6-22 hours to about 1-4 hours (E). Only the pages that need OCR or LaTeX go to the GPU desktop.

**Disk, not CPU, is the binding constraint.**
- The 10k-book index needs about 9-12 GB at peak without vectors (E).
- A 50M-chunk index needs about 75-95 GB without vectors and 110-130 GB with them (E).
- The source PDFs alone, about 250 GB (E, PLAN figure), already exceed this PC's free space. The large tier belongs on the GPU desktop or a dedicated drive.

**Models belong on the GPU desktop.** Dense vectors, OCR, VLM parsing and cross-encoder reranking run there, behind an adapter that is off by default, negotiates capabilities with the model host, and never falls back to the cloud. On this PC no ML model runs unless the owner relaxes PLAN's "no local models" rule.

**Rejected** on cost, disk and evidence: graph extraction over the whole library, a full ColBERT index, and dense embedding of everything on the CPU.

---

## 2. Target pipeline

This is the order of stages in the target design:
1. Discover the files and hash them.
2. Route each page.
3. Extract text with N PDFium worker processes.
4. Store a durable parse record (page texts, classes, bookmarks, blocks).
5. Chunk into block-aware spans.
6. Index in tantivy (one field per language), and in LanceDB where vectors exist.
7. Search in Rust: several rewrites per request, fusion, a rights check for the purpose of the request, and flags for degraded results.
8. In Python: pack evidence, apply the leak guard, rerank, cite.
9. Locate the quote, then render the page crop.

### 2.1 Discovery and source versions
- **Keep** the current approach: walkdir, a size and mtime pre-check, a BLAKE3 content hash as `doc_id`, removal only after a complete walk, and repair after a crash (M: ingest.rs). Dependencies: blake3 (CC0 or Apache-2.0) and walkdir (MIT or Unlicense).
- **Port the ideas, not the code, from CocoIndex** (Apache-2.0). Do not depend on it: it is pre-1.0, keeps its state in LMDB, has no tantivy target, and assumes a Python component model. The four ideas:
  - **Fingerprints per stage for each document:**
    - `extract_fp`: PDFium build, cleaning rules and routing version.
    - `chunk_fp`: chunker version, size, overlap and block rules.
    - `index_fp`: `bm25::SCHEMA_VERSION`, the analyzers and dictionary versions.
    - `embed_fp`: model, dimensions, dtype and normalisation.
    - `ocr_fp` per page: the model id.

    When a fingerprint is stale, only that stage re-runs, starting from the previous stage's stored output. Today only the BM25 schema version and the embedding model are tracked (M: ingest.rs).
  - **A failure ledger** records the reason and the `extract_fp`. A failed file is retried only when the extractor changes or the owner asks.
  - **Tombstone-first deletes** that are retried at startup. SQLite stays the authority. Search already drops unknown or unindexed chunks, because `load_hits` joins on `state = 'indexed'` (M: search.rs).
  - **A source-version lineage table:** path, old `doc_id`, new `doc_id`, time replaced. With it, an old citation resolves to "superseded source" and is re-anchored by its quote.
- **Why:** this gives incremental ingest with invalidation per source version, and needs no new framework.

### 2.2 Page routing
- **Today** there are four classes, decided from PDFium text length, the share of maths symbols, the share of 1-2 character tokens, and whether the page has image objects (M: triage.rs). The deep dives found four holes; none has been reproduced here yet:
  - Type3 or vector-outlined text with no images is classed `empty`.
  - A hidden OCR text layer is classed `born_digital`.
  - Garbled ToUnicode maps pass.
  - Japanese and Chinese written without spaces trip the short-token rule and are classed `math_heavy`.
- **Target.** Each page gets a class, a list of reasons, a text source (`native`, `sidecar`, `ocr`, `vlm` or `none`) and a trust level.
  - Classes: `born_digital`, `sidecar`, `scanned`, `garbled`, `vector_text`, `math_heavy`, `table_heavy` (this one needs geometry, see I17) and `empty`.
- **Depend on pdf-inspector 1.25.2 (MIT)**, detection only, with `default-features = false` and `ScanStrategy::Full`.
  - It detects invisible text layers, vector-outlined text, Type3 fonts or Identity-H fonts without ToUnicode, and tiled scans.
  - Its detector is tangled with its font code, so depending on it is cheaper than porting it.
  - Four conditions:
    - Our encryption refusal runs first, because pdf-inspector retries the empty password.
    - Files over a size cap (300 MB, E) are skipped and recorded as `routing: partial`.
    - Page counts from lopdf and PDFium must agree.
    - The version is pinned exactly.
- **Port three things:**
  - The pure-string garble checks from pdf-inspector's `text_quality.rs` (MIT): replacement runs, private-use and C1 control runs, dollar-as-space, and the substitution-cipher statistic.
  - A CJK variant of those checks: the share of U+FFFD, private-use and Kangxi-radical characters.
  - xberg's idea of a language-plausibility check, using whatlang (MIT).
- **Hidden text layers:** adopt the layer when it passes the garble gate. pdf-inspector's rule is at least 40 alphanumerics (V). Otherwise queue the page for OCR.
- **Why:** routing is the only way the GPU time fits. Running a VLM on all 4M pages at 2-6 pages/s (V) would take 8-25 GPU-days (E, sweep).

### 2.3 Extraction
- **Keep PDFium** through pdfium-render 0.9.4 (MIT or Apache-2.0; PDFium itself is BSD-3-Clause). In our stack it is the only source that gives exact page-and-character provenance on born-digital pages, and it handles scans and malformed files well.
- **Add long-lived worker processes.** This ports the idea from liteparse (Apache-2.0); no code needs copying.
  - A hidden `rag-engine extract-worker` subcommand. Each worker loads its own PDFium.
  - The parent talks to it over stdio with length-prefixed JSON.
  - The parent sets a deadline per document, kills and respawns a stuck worker, and records the file in the failure ledger.
  - Use 6 workers by default (cores minus 2, E).
  - Starting a process on this PC costs about 300 ms (M: machine rules, 2026-09-28). That is paid once per worker, not per document.
  - The current in-process path stays as the fallback and as the test oracle.
- **Split ingest into two phases.** Workers produce durable parse records, and one committer writes them to SQLite. Chunking and indexing read those records, so re-chunking or re-indexing never re-extracts.
- **EPUB:**
  - Keep rbook 0.7 (Apache-2.0).
  - Port MathML-to-LaTeX from xberg (MIT), preferring an embedded `application/x-tex` annotation, using roxmltree (MIT or Apache-2.0).
  - Refuse DRM'd members listed in `encryption.xml`.
  - Apply archive limits: 500 MB, 100:1 compression and 10,000 entries (V: xberg defaults).
- **Not now.** pdf_oxide becomes a bake-off candidate only for document classes where it matches PDFium's text and is at least 2× faster per core (E threshold). liteparse and xberg are not dependencies.

### 2.4 Semantic blocks
- **Our own schema.** Its shape follows MinerU's MiddleJson and liteparse's LayoutBlock, as design only; no MinerU code. A block has:
  - `block_id`, `kind`, `spans`;
  - `bbox`, optional, as page-relative [0,1] coordinates;
  - `source`: `heuristic`, `epub`, `markdown`, `layout:<model>` or `vlm:<model>`;
  - `latex` and `html`, both optional.
- **Kinds:** paragraph, heading, display_equation, theorem-like (theorem, lemma, proposition, corollary, definition, example, exercise, remark), proof, solution, table, code, caption, footnote, list.
- **Version 1 on the CPU, from text only:**
  - Labelled environments in six languages, for example Theorem, Satz, Théorème, Teorema, 定理 and Proof, Beweis, Démonstration, Demostración, 証明, 证明.
  - Proof end markers: ∎, □ and QED.
  - Lines that end in an equation number.
  - Runs of lines with a high share of maths symbols.
  - EPUB MathML and `<table>` elements.
  - Markdown code fences, `$$…$$` blocks and pipe tables.
- **Version 2, on the GPU desktop or after a measured geometry pass:**
  - Regions from a layout model, with the native text inside each region. This is MinerU's text-layer-first policy, and pdf-inspector's region API is the template.
  - LaTeX from a VLM is stored next to the native text and never replaces it.
- **A side benefit:** `exercise` and `solution` blocks feed the answer-leak guard in C11.

### 2.5 Chunking and parent expansion
- **Keep** text-splitter 0.33 (MIT) with tiktoken-rs cl100k: 400 tokens with a 40-token overlap (M: chunk.rs).
- **Add a block-aware pre-pass:**
  - Atomic blocks are never split unless they exceed 1,200 tokens (E).
  - An oversize table is split at row boundaries, with the header repeated in each piece (xberg's RepeatHeader).
  - A long proof is split at paragraph boundaries; each piece keeps the same `block_id` and is marked "part i of n".
  - A chunk never crosses a section boundary at level 2 or above (E).
- **Parents:** block, then section, then chapter. Each chunk stores its `block_ids` and `section_id`.
  - `GET /chunks/{id}?expand=block|section` returns the parent's spans and text, capped at 8k tokens (E).
  - The Python packer (CORPUS Stage 0 step 3) uses this instead of stitching chunks together by ordinal.
- **Heading paths:** use PDF bookmarks first, classed FULL, SKELETON or IGNORE (an idea ported from PageIndex, MIT). Fall back to the existing text heuristic. Record which one was used as `toc_source`.

### 2.6 Provenance model
- **Canonical text.** Each page's canonical text is the cleaned PDFium text for one `extract_fp`: today's `clean_pdf_text` output, without NFKC.
  - It is stored once per document: zstd-compressed page texts plus a page index and a BLAKE3 `page_text_hash` per page, in SQLite (`doc_text`).
  - Chunk rows keep spans only; they no longer store text.
  - This keeps store.rs's rule that there is no row per page.
- **Offsets** count Unicode code points, which is Python's native unit:
  - PDF: code points into the canonical page text.
  - EPUB: spine item plus code-point offset, and the print page from the nav page-list when the book has one.
  - Markdown and plain text: offset into the file text.
- **Chunk id = span id**, for example `d3f9a1c07b2e4d55:p12.1043-p13.220@x3`:
  - The first part is a 16-hex prefix of `doc_id`, lengthened on a collision.
  - `x3` is the extraction version.
  - The id describes itself, stays the same under re-chunking, and resolves without an alias table.
  - Internally, tantivy (as a u64 fast field) and Lance use an integer `chunk_key`: SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`, so keys are never reused and a stale index entry can never point at a different chunk.
- **Bounding boxes are not stored for native pages.**
  - `POST /quotes/locate` takes a chunk id (or a doc and page) plus a quote.
  - It returns the page, start, end, quads, and a status: `exact`, `re_anchored` or `orphaned`.
  - It uses PDFium's loose character bounds and the same normalisation as the C11 verify route; pages.rs already does this for highlights.
  - OCR and VLM pages cannot be recomputed, so for them we store line polygons, confidence and the model id.
- **Re-anchoring after an extractor change.** Callers keep the quote (DeepTutor already does). Re-anchoring uses W3C Web Annotation TextQuoteSelector semantics: the exact text plus a 32-character prefix and suffix (E), searched on the same page and one page either side. A `re_anchored` or `orphaned` result is always reported, never hidden.
- **Citation strength stays as D9 says.** An exact span match proves only that the text came from that place. `text_source` lets the UI say "quoted from OCR text" when that is the case.

### 2.7 Rights for each purpose
- **A rights record per document:**
  - the licence: an SPDX id, `owned_copy` or `unknown`;
  - the purposes it allows: `index`, `display_quote`, `page_image`, `lesson_llm`, `bulk_generation`, `export`;
  - `decided_by`: a rule or a manual decision;
  - `rights_version`.
- **Where the rules come from:** path globs in `corpus/rights.toml` (the format is my proposal, E), plus the owner's defaults from D1. They are resolved at ingest. `unknown` is shown as a state of its own; it is never silently allowed or denied.
- **Enforcement is in Rust at the search boundary:**
  - `/search` takes a `purpose`.
  - The SQLite join in `load_hits` is the authority.
  - A `rights` STRING field in tantivy and a column in Lance only pre-filter large exclusions.
  - Revoking `index` deletes the document's chunks.
  - Revoking any other purpose re-indexes that document's chunks from stored text, which is cheap.
- **DRM'd EPUBs and encrypted PDFs** are refused, as today, and recorded as a rights state.

### 2.8 Lexical index and analyzers per language
- **Today:**
  - One English analyzer serves every field.
  - A run of CJK characters becomes one token, which `RemoveLongFilter(40)` then drops (M: tokenize.rs). That consequence is inferred from how tantivy's SimpleTokenizer behaves; no test has shown it yet.
  - Spanish, French and German text goes through the English stemmer.
- **Target: keep tantivy 0.26 (MIT).** It is the only mature Rust BM25 with incremental segments and deletes. SeekStorm lacks compaction, Japanese, and analyzers per field.
- **Language per chunk.** Try, in order:
  1. The document's default language, from a manifest or its folder.
  2. Its script: kana means Japanese; Han without kana means Chinese.
  3. whatlang (MIT), for Latin-script text in English, Spanish, French or German.
  4. Otherwise `x`.
- **Fields:** `text_`, `title_` and `heading_` for each of en, es, fr, de, ja, zh and x. Only the chunk's own language is filled.
  - tantivy binds one analyzer to each field, and IDF stays separate per language.
  - Every analyzer shares the existing maths normaliser (NFKC plus LaTeX names) as a character filter.
  - English keeps `en_stem_math`. Spanish, French and German use tantivy's Snowball stemmers. `x` is not stemmed.
- **A `cjk2` field.** Our own overlapping-bigram tokenizer runs over Han and kana runs only. It needs no dependency and never meets an out-of-vocabulary word, so it comes first.
- **Dictionary analyzers come later, as an eval arm (I10).** Keep whichever wins on the ja and zh dev slices, or fuse it with the bigrams.
  - **Japanese:** lindera (MIT), pinned by version. Start with IPADIC and compare UniDic, both with licences we can use. Load dictionaries from disk with mmap and use Decompose mode. Use our own adapter; lindera-tantivy unwraps errors inside indexing threads and is pinned to lindera 5.
  - **Chinese:** jieba-rs 0.11 (MIT), with `cut_for_search` at index time and `cut` at query time, through our own 60-line adapter.
  - **Alternative:** icu_segmenter from ICU4X (Unicode-3.0), one crate for both languages.
- **Queries.** Analyse the query with each candidate language's analyzer, chosen from the query's script and the learner's current language, and OR the field clauses together. CJK queries always also search `cjk2`.
- **Index settings:**
  - frequencies stored, no positions, no stored text;
  - `chunk_key` as a u64 fast field, replacing the stored `chunk_id` string;
  - `rights`, `lang`, `domain` and `source_type` as STRING fields.
- **Arm to measure:** during bulk ingest, use NoMergePolicy and commit every 100k chunks, then run one merge (an idea from SeekStorm).

### 2.9 Optional vector tier
- **Keep LanceDB 0.37.1 (Apache-2.0)** and change seven things:
  - Build IVF_RQ with `num_bits` set explicitly. The pinned version silently defaults to 1 bit (M: lancedb 0.37.1 source, deep dive). Compare 1, 3, 4 and 5 bits against today's IVF_PQ.
  - Store the vector column as float16.
  - Truncate dimensions (Matryoshka, MRL) for models that support it.
  - Make `nprobes` configurable. The pin defaults to 20 (M: lancedb source).
  - Add BTree indexes on `chunk_key` and `doc_key`, and Bitmap indexes on `domain`, `source_type`, `lang` and `rights`.
  - Retrain the index when the row count has roughly doubled since training (E).
  - Store unit-normalised vectors. Lance computes RQ cosine as L2 on normalised vectors (V).
- **Settings I recommend:**
  - Up to 10M chunks: 1024 dimensions in float16, plus IVF_RQ at 4 bits.
  - From 10M to 50M chunks: 256-384 dimensions via MRL, in float16, plus IVF_RQ at 4 bits.
  - Rerank candidates from the stored float16 vectors, not by embedding them again. The arithmetic is in section 5.
- **turbovec (MIT)** is a reference only. Revisit it only if IVF_RQ misses its recall or latency target.
- **On this PC** the tier stays empty unless the owner allows models on the CPU (I14). The plumbing can be tested now with synthetic vectors.

### 2.10 GPU-desktop tier
- **Topology, as C9 says:** rag-engine runs where the library lives. Models run as local services on the GPU desktop:
  - TEI (Apache-2.0; confirm the current licence) for embeddings and cross-encoder reranking;
  - vLLM (Apache-2.0) for VLM parsing;
  - a thin Python sidecar that gives OCR and parsing one uniform API.
- **The adapter, `modelhost.rs`:**
  - It stays off unless `RAG_MODEL_HOST_URL` is set, and it requires a token for any address that is not loopback.
  - At startup it asks the host for its capabilities: which tasks it runs, model id and revision, languages, dimensions and dtype, maximum tokens, and what it outputs (spans with polygons, LaTeX, HTML).
  - A fingerprint of those capabilities feeds `embed_fp` and `ocr_fp`. If the model changes, the affected stages are marked stale. They are re-run only when a job is started explicitly.
  - If the host cannot be reached, work stays pending with the reason recorded. There is never a cloud fallback (D8).
- **Workloads, routed pages only:**
  - OCR for pages classed scanned, garbled or vector text: PP-OCRv6 (confirm the model card) or PaddleOCR-VL-1.6 (Apache-2.0).
  - LaTeX for maths regions, and tables as HTML.
  - Embeddings, chosen by a bake-off on the dev split: Qwen3-Embedding 0.6B or 4B (Apache-2.0), BGE-M3 (MIT), Harrier-0.6B (MIT), Granite-311M-r2 (Apache-2.0).
  - Reranking: Qwen3-Reranker-0.6B (Apache-2.0) or bge-reranker-v2-m3 (Apache-2.0).
- **OcrProvider output is extended**, following the shape of pdf-inspector's contracts: spans with polygon, confidence, model identity, render DPI and warnings. A page's `text_source` becomes `ocr`, `vlm` or `fused`.
- **Not used here:**
  - Marker, Surya and Chandra weights, because their OpenRAIL-M licences limit revenue.
  - Nougat, MonkeyOCR v1 and every Jina model, because they are non-commercial.
  - MinerU, except as an external worker we never ship, and only if the owner accepts its custom licence.

### 2.11 Query pipeline
1. **Python (C11).**
   - Write 3-5 rewrites: textbook vocabulary, notation, a HyDE-style solution sketch used only for the lexical leg, and a prerequisite query.
   - Run the leak guard.
   - Set the `purpose`.
2. **Rust `/search`.**
   - It accepts `queries: [{text, legs, weight}]`, `fusion: rrf | bm25_sum_minmax` and `purpose`, and analyses each query per language.
   - BM25 returns the top 200 per rewrite (E).
   - RRF with k = 60 stays the default (M: search.rs).
   - The arm to test sums BM25 scores across rewrites, min-max normalises them, then blends them with the dense scores:
     - BGE-Reasoner uses 0.75 dense and 0.25 BM25 (V).
     - Bruch et al. 2022 found that a convex blend beats RRF (V).
     - On BRIGHT, BM25 went from 14.5 to 30.5 nDCG@10 with 5 rewrites (V).
   - The vector leg sees only the plain question.
   - Each hit carries its spans, `text_source` and a `degraded[]` list.
3. **Reranking, only when fused scores are flat or the tutor asks for depth.**
   - Now: the LLM listwise rerank CORPUS already plans. D1 lets retrieved passages reach lesson LLMs.
   - Later: a cross-encoder, once the GPU host advertises one.
   - A CPU cross-encoder only if the owner allows it. With bge-reranker-v2-m3, top-10 at 256 tokens each takes about 1.7 s (E, sweep).
4. **Expansion, packing and citation.** Expand hits to their block or section, pack them, cite by span id, and call `/quotes/locate` for boxes.
   - Latency budget: under 5 s to the first voice token (req: CORPUS).
   - BM25 target: p95 under 300 ms at 10M chunks (req: C9).
   - A second target, in CORPUS Stage 3, is under 100 ms warm for the top 50 (req). The two targets differ; see section 6.

### 2.12 Degraded states, stated honestly
- **Per page:** `text_source` (`native`, `sidecar`, `ocr:<model>`, `vlm:<model>` or `none`), the class with its reasons, and `untrusted_text`.
- **Per document:** its state; any failure, with the reason and extractor version; stale stages; rights `unknown` or `revoked`; OCR pages pending; embeddings absent, partial or stale.
- **Per hit and per response:** `mode_used`, the legs that ran, and `degraded[]`. Examples: `ocr_pending_page`, `garbled_text_layer`, `cjk_bigram_only`, `vector_leg_skipped:<reason>`, `reindexing:<percent>`.
  - The last one replaces today's silent empty index after a schema change (M: PLAN known issues).
- **`/status`:** the model host (`disabled`, `unreachable`, or `ok` with its models) and counts of every state above.

### 2.13 Evaluation
- **Retrieval, in the existing Python harness.**
  - A gold item becomes `{qid, question, lang, domain, strata, gold: [{doc_id, page, quote, span?}]}`.
  - A hit matches when its span overlaps the gold span or its text contains the normalised quote. Today it matches on `chunk_id`, or on doc plus page (M: metrics.py). The new rule survives re-chunking.
  - Splits follow D7: **dev**, split by source document, for hill-climbing; and a frozen **audit** set, split by document and opened only at release checkpoints, each opening recorded.
  - The primary metric, fixed before looking at results, is nDCG@10. Also report recall@10, recall@100 (how many candidates reach the reranker) and MRR.
  - Slice every metric by language, domain, page class and block kind. Record p50 and p95 latency with each run.
- **Parsing.**
  - A new Rust tool, `rag-engine parse-snapshot`, writes each page's status, `page_text_hash`, class and reasons (from xberg's pdf_oracle idea). Run it before and after every extractor or routing change.
  - Build a gold set of 150-300 of the owner's own pages, in JSON shaped like OmniDocBench's. Its code is Apache-2.0; its dataset is research-only and is not used.
  - Score in Python:
    - page edit distance, strict and loose;
    - precision and recall of routing;
    - the chunk-integrity rate for equations, proofs and tables;
    - heading-path accuracy;
    - F1 on numeric tokens;
    - later, the character error rate of OCR in each language.
- **Scale.** `rag-engine bench` at 1M and 10M chunks: p50, p95 and p99; index bytes per chunk; build time; peak memory. Measure with and without filters, and with long 5-rewrite queries.
- **Licences.** A cargo-deny licence policy in QA denies AGPL, GPL, LGPL-only and non-commercial licences.

---

## 3. Work plan, in dependency order

**Effort** (E): S is about 1-2 days, M about 3-5 days, L about 1-3 weeks.

**About the dev metrics.** Until a corpus exists (I11), increments are accepted on unit and property tests plus a pilot shelf of about 100 owner books covering all six languages (E). Their dev metrics are measured on that shelf and checked again after I11.

**Contract amendments the lead must approve:**
- C1 and C11: the chunk-id format, `Hit.spans`, `text_source` and `degraded`, `/search` taking `queries[]`, `fusion` and `purpose`, and the new routes `/quotes/locate` and `/chunks/{id}`.
- C9: the outline built from sections, and `/status` reporting states and capabilities.
- C2: the dev and audit splits, and slices.

| # | Increment | Depends on | Rust or Python | Effort | Dev metric it should move |
|---|---|---|---|---|---|
| I0 | Baselines: `bench`, `parse-snapshot`, licence gate | none | Rust | M | none (records the baseline) |
| I1 | Eval v2: quote and span gold, dev and audit splits, slices | none (parallel with I0) | Python | M | none (first BM25 baseline) |
| I2 | Provenance core: stored page text, span ids, integer keys, `/quotes/locate`, stage fingerprints | I0 | Rust (+ small Python adapter) | L | neutral nDCG@10; bytes per chunk down |
| I3 | Analyzers per language, CJK bigram baseline, language id | I2 | Rust | M | nDCG@10 and recall@10 on ja, zh, es, fr, de up |
| I4 | PDFium extraction in worker processes, failure ledger, two-phase ingest | I2 | Rust | M | none (pages per second) |
| I5 | Page routing v2: pdf-inspector, garble checks, CJK-aware rules | I4 | Rust | M | routing precision and recall |
| I6 | Sections from bookmarks, parent expansion | I2 | Rust | M | nDCG@10 neutral or up |
| I7 | Semantic blocks v1, block-aware chunking, EPUB MathML | I6 | Rust | L | recall@10 on definition, theorem and proof queries |
| I8 | Rights per purpose, lineage, tombstones, degraded states | I2 | Rust | M-L | leak count 0 (policy) |
| I9 | Several rewrites per request, fusion arms | I3 | Rust + Python | M | nDCG@10 and recall@100, arm C vs C' |
| I10 | CJK dictionary analyzer arm: lindera, jieba, ICU4X | I3, I1 | Rust | M | ja and zh slices |
| I11 | First bulk ingest (10k books), check scale | I2-I8 (I9 and I10 may come after) | Rust (ops) | S + run time | C9 p95 on the real index; freeze the audit set |
| I12 | Vector tier hardening: IVF_RQ, float16 and MRL, scalar indexes | I2 | Rust | M | ANN recall@10 vs exact search |
| I13 | Document- and section-level routing arm | I6 | Rust | M | recall@100 and nDCG@10 |
| I14 | CPU models, only if the owner allows them | I12, owner | Rust | M each | recall@100, OCR CER, rerank nDCG |
| I15 | Model-host adapter with capability negotiation, off by default | I2, I5, I8 | Rust + Python sidecar | L | none (plumbing) |
| I16 | GPU bake-offs and full runs | I15, GPU desktop | Python (eval) + ops | L | nDCG@10 per language; CER; formula F1 |
| I17 | Geometry features: display equations, font-size headings, XY-cut, tables | I4, I7 | Rust | L | reading order; integrity of equations and tables |

### Details

**I0. Baselines, benchmark, snapshot and licence gate. No behaviour change.**
- **Files:** new `src/bench.rs` and `src/snapshot.rs`; `main.rs`; `deny.toml` at the workspace root; QA notes.
- **Acceptance:**
  - `bench --chunks 1000000` writes p50, p95 and p99 for queries of 1, 5 and 150 terms, with and without a domain filter, plus bytes per chunk, build seconds and peak memory.
  - It refuses to run when free disk is less than 3× its own estimate (E).
  - `parse-snapshot` run twice on a 200-PDF sample gives identical `page_text_hash` values.
  - Pages per second for a single PDFium process are recorded.
  - `cargo deny check licenses` passes.
- **Risks:**
  - Synthetic text understates vocabulary; prefer public-domain text.
  - The 10M bench needs about 5 GB of temporary disk (E).

**I1. Eval v2 (Python, in `evals/`).**
- **Files:** `dataset.py`, `metrics.py`, `split.py`, `runfile.py`, `compare.py` and `datasets/README.md`.
- **Acceptance:**
  - A quote-based gold item matches only hits that contain the quote.
  - The old doc-plus-page rule stays available as a looser `page` metric.
  - The audit file is write-once. `compare` refuses to read it without `--checkpoint`, and the run file records that it was opened.
  - The existing tests stay green.
- **Risks:**
  - No questions exist yet.
  - Generating them needs an LLM key and approved spend.
  - Synthetic questions lean towards English.

**I2. Provenance core. Do this before the first bulk ingest.**
- **Files:**
  - `store.rs`: schema v2 with `doc_text` blobs; chunks keep spans but no text; `chunk_key` AUTOINCREMENT; ordinal kept; stage fingerprints.
  - `chunk.rs`: spans per chunk.
  - `ingest.rs`: `reindex_if_stale` reads the blobs.
  - `bm25.rs`: `chunk_key` fast field; schema version 3.
  - `search.rs`: `load_hits` slices text out of the blobs, through an LRU cache.
  - `library.rs`, `pages.rs`: highlighting by span.
  - `server.rs`: `/quotes/locate` and `/chunks/{id}`.
  - `vector.rs`: `chunk_key`.
- **Acceptance:**
  1. A property test: slicing the canonical page text by a chunk's spans reproduces the chunk's text exactly.
  2. Every span id minted at 400 tokens resolves to identical text after re-chunking at 300.
  3. Locate quads cover exactly the quote's characters, checked against the pages.rs highlight.
  4. Re-extracting the same file gives an identical `page_text_hash`.
  5. When `chunk_fp` changes, only indexing re-runs; when `extract_fp` changes, extraction re-runs. The ingest report counts both.
  6. No `chunk_key` is ever reused after a delete.
- **Metric:** nDCG@10 stays within noise. SQLite should hold at least 50% fewer bytes per chunk (E).
- **Risks:**
  - Python code that parses `<doc_id>:<ordinal>` (search DeepTutor for it).
  - Code-point versus UTF-16 offsets in the webview; the contract must say which.

**I3. Analyzers per language.**
- **Files:**
  - `tokenize.rs`: a reusable maths character filter, a script-run splitter, the bigram tokenizer, and analyzers for es, fr, de and x.
  - New `lang.rs`.
  - `bm25.rs`: fields per language and a query across fields.
  - `chunk.rs`, `ingest.rs` and `store.rs`: a `lang` column.
  - New dependency: whatlang.
- **Acceptance:** write these tests first; they must fail on today's code.
  - A Japanese chunk containing 光合成 is found by the query 光合.
  - 拉格朗日 is found in a Chinese chunk.
  - `reacciones` is stemmed as Spanish.
  - The maths tests pass for every Latin-script analyzer.
  - A mixed-script chunk indexes both scripts.
- **Risks:**
  - Short or mixed chunks may be misdetected; they fall back to `x` plus `cjk2`.
  - IDF is fragmented across language fields.
  - The bigram field makes CJK-heavy shelves larger; measure this with I0.

**I4. Extraction worker processes.**
- **Files:** new `extract_worker.rs`, `main.rs`, `ingest.rs` (a pool and two phases), `store.rs` (the ledger).
- **Acceptance:**
  - Page text hashes are identical to the in-process path.
  - 6 workers reach at least 4× the pages per second of one process (E target).
  - A worker that sleeps past its deadline is killed, recorded as `timeout`, and ingest continues.
  - A worker that aborts is respawned, and its file is retried once.
- **Risks:**
  - Handling pipes and process handles on Windows.
  - Memory for PDFium in each process.
  - Antivirus scanning each new process.

**I5. Page routing v2.**
- **Files:** `triage.rs`, `extract.rs` (the raw PDFium bindings for character mapping errors and text render mode), new `route.rs`, `ocr.rs`, `store.rs`.
- **Dependency:** pdf-inspector `=1.25.2`. Check its dependency tree with cargo-deny.
- **Acceptance:**
  - Test on 300 hand-labelled owner pages: TeX books, scans with a hidden text layer, Type3 books from dvips, CJK, two-column layouts. Precision and recall for "needs OCR" should both be at least 0.9 (E).
  - Japanese prose without spaces is classed born_digital.
  - An encrypted fixture never reaches pdf-inspector.
  - The snapshot diff shows class changes only, never text changes.
- **Risks:**
  - pdf-inspector changes fast: 8 releases in 3 weeks (M: gh, deep dive).
  - lopdf reads whole files into memory.
  - If page counts disagree, routing falls back to PDFium only for that document, with the reason recorded.

**I6. Sections and parent expansion.**
- **Files:** `headings.rs` (bookmark tiers), `extract.rs`, `store.rs` (a `sections` table), `chunk.rs`, `server.rs`, `library.rs`.
- **Acceptance:**
  - On 30 books with hand-checked tables of contents, the heading path is correct for at least 90% of sampled chunks (E).
  - Bookmarks with non-monotonic page targets are classed IGNORE.
  - Expansion is capped.
- **Risk:** bookmarks that point to the wrong page.

**I7. Semantic blocks v1.**
- **Files:** new `blocks.rs`; `chunk.rs`; `extract.rs` (EPUB MathML and tables); `store.rs`; `tokenize.rs`, so EPUB LaTeX is indexed through the maths normaliser.
- **Acceptance:**
  - On the gold pages, at least 95% of theorem statements, proofs of 1,200 tokens or fewer, and display equations sit inside one chunk (E).
  - Spans cover all page text that was not deliberately dropped.
  - Split tables repeat their header.
  - An EPUB MathML fixture produces LaTeX.
  - Exercise and solution blocks are visible to the leak guard (a Python test).
- **Risks:**
  - False positives from label heuristics in six languages.
  - Proofs that run across pages.

**I8. Rights, lineage and tombstones.**
- **Files:** `store.rs`, `meta.rs` (`rights.toml`), `ingest.rs`, `bm25.rs`, `vector.rs`, `search.rs` (`purpose` and the authority check), `server.rs`.
- **Acceptance:**
  - A document that denies `lesson_llm` never appears in results for that purpose, even when the tantivy field is deliberately stale.
  - Revoking `index` survives a crash between the two index deletes.
  - Replacing a file at the same path records lineage, and an old span id resolves as `superseded`.
- **Risk:** the owner's default purposes are not known yet.

**I9. Several rewrites per request, and fusion arms.**
- **Files:** `search.rs`, `bm25.rs`, `server.rs`; the Python DeepTutor library adapter.
- **Acceptance:**
  - Fused output is identical to the Python RRF path on fixtures.
  - p95 for five 150-term queries at 10M synthetic chunks stays within the budget I0 measures.
- **Risks:**
  - Tuning on anything other than the dev split.
  - Rewrites that sketch a solution can pull in answers, so the leak guard runs after fusion.

**I10. CJK dictionary analyzer arm.**
- **Files:** `tokenize.rs` and `bm25.rs`. Dictionaries live on disk, and their versions are part of `index_fp`.
- **Acceptance:**
  - Golden tokenization tests pass.
  - A fuzz run over 1 GB of CJK text causes no panics.
- **Risks:**
  - The verdict needs at least 100 ja and 100 zh questions to rise above noise (E).
  - The provenance of jieba's bundled dictionary is undocumented.

**I11. First bulk ingest.**
- Needs the owner's library location first.
- **Acceptance:**
  - The C9 p95 target is checked on the real index.
  - `/status` shows plausible counts for each degraded state.
  - The audit set is frozen from this corpus.

**I12. Vector tier.**
- **Files:** `vector.rs`, `embed.rs` (MRL truncation, normalisation, dimension check), `ingest.rs` (`embed_fp`), `search.rs`.
- **Acceptance:**
  - With 1M synthetic normalised vectors of 1024 dimensions, IVF_RQ recall@10 against flat search is at least 0.95 at the chosen nprobes (E).
  - Bytes per row are within 10% of the Lance formula.
  - The query plan shows deletes by key using the BTree index.
- **Risks:**
  - The lancedb pin: 0.38 and later need the `remote` feature.
  - The build needs protoc.
  - Float16 support in the pinned version is unconfirmed.

**I13. Document- and section-level routing arm.** A second small tantivy index, or a document-type field, restricts the chunk search to the top sections or documents. It is judged on recall@100 and latency.

**I14. CPU models, only if the owner allows them.** Each gets a 5-minute throughput test first: 2,000 chunks or 200 pages. Then it runs as an eval arm.
- Static vectors: potion-multilingual-128M through model2vec-rs (MIT).
- mE5-small (MIT) on a prioritised subset, through ort.
- PP-OCRv6 small through ort, for scanned pages.
- A CPU cross-encoder over the top 10.

**I15. Model-host adapter.**
- **Files:** new `modelhost.rs`; `ocr.rs` (polygon storage); `embed.rs` (host capability and dimensions); `store.rs`; `server.rs`; `ingest.rs`.
- **Acceptance:**
  - With the default configuration, `/status` reports `model_host: disabled` and no outbound connection is made (a test listener must never be hit).
  - A fake host with capabilities drains the OCR queue, and pages come back with the model id and polygons.
  - A change in capabilities marks stages stale but does not run them.
  - When the host is down, pages stay pending with the reason.
  - A URL that is not loopback is refused without a token.

**I16. GPU bake-offs.**
- Embedders and rerankers on dev, per language.
- OCR and VLM on 200 owner pages per language, using olmOCR-bench-style unit tests and F1 on formula tokens.
- Then the full embedding run and OCR on the routed pages.

**I17. Geometry features.**
- Character geometry, only on flagged pages and only after its cost per page is measured: centred display equations, heading maps by font size, and XY-cut for multi-column pages (ported from liteparse, Apache-2.0, with attribution).
- Also check whether vertical Japanese text (tategaki) comes out in the right order. If not, port pdf_oxide's column clustering (MIT or Apache-2.0).

---

## 4. What we reject, and why

- **Graph extraction over the whole library** (GraphRAG, LightRAG, HippoRAG).
  - It needs an LLM call per chunk. At our size that is extrapolated to cost $7k-$500k (E, sweep).
  - Sending book text to a cloud LLM for indexing breaks D1's local-indexing rule.
  - On textbooks, the best graph or tree method beats BM25 by only about 2 points: 73.58 vs 71.66 accuracy (V: GraphRAG-Bench authors). Several graph methods scored below using no retrieval at all.
  - We keep only structure from each book's own table of contents; graphs, if ever, only per curated shelf.
- **A full ColBERT or PLAID index.**
  - It costs about 76 bytes per token, so about 30 KB per chunk (E): about 180 GB at 6M chunks and about 1.5 TB at 50M.
  - next-plaid's ecosystem has no multilingual ColBERT model.
  - next-plaid bundles rusqlite 0.38 against our 0.40, which cannot link (E, from the manifests).
- **Dense embedding of everything on the CPU.** With transformers of 0.3B parameters or more it takes 30-215 days for 10M chunks (E, sweep). The GPU desktop does it in about 10 hours for 6M chunks (E).
- **LEANN's recompute index.** It cannot update or delete, building it needs more than 64 GB of RAM, and it puts a model in the query path. Quantised codes reach similar bytes per vector.
- **Replacing tantivy with SeekStorm.** SeekStorm never reclaims deleted documents, has no Japanese, and binds one tokenizer to a whole index. Its benchmarks are self-reported images.
- **Replacing tantivy with Lance FTS.** It would store the text twice, has no boolean operators, and needs dictionaries downloaded.
- **A VLM or layout model on every page.** That is 8-25 GPU-days (E), plus the risk of hallucinated text. Layout on the CPU costs 150-300 ms per page (V: xberg).
- **These projects as linked dependencies:**
  - pdf_oxide as the main extractor, for now: an audit found 144 defects, it changes often, and it holds about twice the file size in RAM.
  - xberg, liteparse, MinerU and PaddleOCR as linked dependencies: each is a monolith, needs a patched PDFium, has a custom licence, or is Python.
  - lindera-tantivy and tantivy-jieba adapters: they unwrap errors inside indexing threads and lag behind the tokenizer versions. We write our own.
- **Whole-library contextual retrieval with an LLM, as Anthropic describes it.** The cloud is excluded and the CPU is too slow. Heading paths and titles serve instead.
- **RL-trained search agents.** They need GPU training and target web question answering.
- **turbovec for now.** It is a flat scan held entirely in RAM, and it has been tested only at 100K vectors (V).
- **Positions in the tantivy index.** Phrase queries are not needed, because exact quotes are checked against the stored text.
- **Float32 or 1024-dimension float16 vectors at 50M chunks,** and caching embeddings apart from the vector store.
- **Anything we would ship under an excluded licence:**
  - AGPL: PyMuPDF, pymupdf4llm, mupdf-rs, Chunkr, ParadeDB, BookRAG, kemingy/rabitq.
  - GPL: the `epub` crate, LinearRAG, Typesense.
  - Non-commercial: Jina models, SPLADE-v3, Nougat, ReasonIR, Reason-ModernColBERT.
  - Revenue-limited OpenRAIL-M: Marker, Surya, Chandra.
  - CC BY-SA dictionaries, such as CC-CEDICT, stay unbundled.
  - The OmniDocBench and GraphRAG-Bench textbook datasets are never ingested; they are non-commercial or unlicensed.

---

## 5. Disk and time budgets

**Workload assumptions (all E):**

| | 10k books (now) | 50M chunks (later) |
|---|---|---|
| Source files | ~250 GB (PLAN figure) | ~2 TB |
| Pages | ~4M | ~33M |
| Tokens (cl100k) | ~2B | ~17B |
| Raw page text | ~8.8 GB | ~73 GB |
| Chunks | ~6M | 50M (req) |

**Disk (E unless tagged):**

| Item | 10k books | 50M chunks |
|---|---|---|
| Page text, zstd at about 3× | ~3 GB | ~24 GB |
| Today's design instead: uncompressed text in every chunk row, overlap included (I2 avoids this) | ~10 GB | ~80 GB |
| SQLite: spans, sections, blocks, parse records | ~1 GB | ~6-8 GB |
| tantivy: frequencies, no positions, no stored text (I0 measures this) | ~1.5-3 GB | ~12-25 GB |
| Temporary space during tantivy merges | +3 GB | up to +25 GB |
| OCR and VLM text plus polygons, for 10-20% of pages | ~1-2 GB | ~5-10 GB |
| Vectors, optional | 1024d f16 12.3 GB + RQ 4-bit 3.2 GB = 15.5 GB | 256d f16 25.6 GB + RQ 4-bit 7.8 GB = 33.4 GB |
| **Peak without vectors** | **~9-12 GB** | **~75-95 GB** |
| **Peak with vectors** | **~25-28 GB** | **~110-130 GB** |

**Vector arithmetic.** The Lance RQ size formulas are (V). The multiplications are (E).

| Format | Bytes per row | 6M rows | 10M rows | 50M rows |
|---|---|---|---|---|
| 1024d f32 | 4,096 | 24.6 GB | 41.0 GB | 204.8 GB |
| 1024d f16 | 2,048 | 12.3 GB | 20.5 GB | 102.4 GB |
| 1024d int8, or 512d f16 | 1,024 | 6.1 GB | 10.2 GB | 51.2 GB |
| 384d f16 | 768 | 4.6 GB | 7.7 GB | 38.4 GB |
| 256d f16 | 512 | 3.1 GB | 5.1 GB | 25.6 GB |
| IVF_RQ 4-bit, 1024d | 540 | 3.2 GB | 5.4 GB | 27.0 GB |
| IVF_RQ 4-bit, 256d | 156 | 0.9 GB | 1.6 GB | 7.8 GB |
| IVF_RQ 1-bit, 1024d | 148 | 0.9 GB | 1.5 GB | 7.4 GB |
| ColBERT, 128d, 4-bit, 400 tokens | ~30,400 | ~182 GB | ~304 GB | ~1.5 TB |

Lance keeps the raw vector column next to its index, so a row costs raw plus index.

**What fits on this PC** (C: has 142 GB free (M: df, this session); the brief says about 100 GB):
- The 10k-book index fits, with or without vectors.
- 50M chunks, and the source PDFs, do not fit beside anything else.

**Time (E unless tagged):**

| Stage | 10k books | 50M chunks |
|---|---|---|
| Hashing sources, first run: NVMe at 3 GB/s vs USB HDD at 0.15 GB/s | 2-30 min | 11 min-3.7 h |
| PDFium extraction today, one thread, 5-20 ms per page (vendor figure 2-5 ms (V: liteparse)) | 6-22 h | 46-183 h |
| PDFium extraction with 6 workers, assuming close to linear scaling | 1-4 h | 8-31 h |
| Routing signals (pdf-inspector, full scan) | +10-40 min | +1.5-5 h |
| Chunking and language id | 10-60 min | 1.5-8 h |
| tantivy indexing | 15-45 min | 2-6 h |
| **First ingest in total, with workers** | **~2-6 h** | **~12-45 h** |
| Re-index from stored text, after an analyzer change | 30-90 min | 4-12 h |
| GPU: Qwen3-Embedding-0.6B at about 160 chunks/s, assuming 60 TFLOPS effective | ~10 h | ~3.6 days |
| GPU: VLM on 10-20% of pages at 2-6 pages/s (V) | 0.8-4.6 days | 6-38 days, so route more tightly, active shelves first |
| CPU OCR, only if the owner allows: 1-3 pages/s on 8 cores | 37-111 h for 400k pages | not planned |
| CPU static embeddings, only if the owner allows | 10-40 min | 1.5-6 h |
| CPU mE5-small, only if the owner allows: 50-100 chunks/s | 17-33 h | 6-12 days |

**Memory.**
- At 6M chunks the tantivy index (up to 3 GB) fits in the page cache (E).
- At 50M, the 12-25 GB index is mostly cache-resident in 64 GB, and 7.8 GB of RQ codes fits in RAM (E).
- The writer heap is 1 GiB (M: bm25.rs).
- CJK segmentation speed is not a bottleneck: lindera runs at 3.78M characters per second (V: maintainer's measurement).

---

## 6. Open questions for the owner

1. **Where does the library live?** About 250 GB of PDFs (E) do not fit on C:, which has 142 GB free (M), or on D:, which has 61 GB free per PLAN. A USB HDD would add hours of I/O.
2. **How should the GPU desktop be set up?** Either the library and rag-engine move there (C9's plan), or this PC keeps the index and uses the GPU desktop only as a worker.
3. **Does "no local models" (PLAN) also forbid small models on this PC's CPU?** That covers static embeddings, mE5-small on a subset, CPU OCR and a CPU cross-encoder (I14).
4. **Which purposes are allowed by default?** For licence-unknown and restricted-copy PDFs you own: indexing, showing quotes, page images, sending passages to the lesson LLM, bulk generation.
5. **A pilot shelf.** About 100 books covering all six languages (E), plus about 2 hours to hand-label 300 pages for routing and about 150-300 pages for parsing gold (E).
6. **Spend on LLM-generated questions** for the dev set, and who writes the frozen audit set (D7).
7. **Which latency target applies?** C9 says BM25 p95 under 300 ms at 10M chunks; CORPUS Stage 3 says under 100 ms warm.
8. **Japanese dictionaries: how much disk?** IPADIC is about 10 MiB, UniDic about 44 MiB and Sudachi about 570 MB built (V: lindera). How much of the library is Traditional Chinese or vertical Japanese?
9. **Vector precision** (PLAN pending item 7). I recommend 1024 dimensions in float16 up to 10M chunks, and 256-384 dimensions via MRL beyond that.
10. **May a MinerU worker run on the GPU desktop at all?** It is custom-licensed and would never be shipped. The alternatives are licensed Apache-2.0 or MIT.

**Housekeeping for the lead.** The owner asked for old Rust build output to be deleted.
- `C:/Users/sethy/Projects/tutor/target` is 22 GB: `debug` is 16 GB and `release` 4.5 GB (M: du, this session).
- A cargo build was writing to `target/debug` at 19:29, so I deleted nothing.
- Once no build is running, `cargo clean --profile dev` (or deleting `target/debug`) frees about 16 GB.
- The reference clones under `C:/Users/sethy/Projects/refs/rag` take 1.4 GB (M). They can be removed once this plan is accepted.