# Using the library

How Tutor takes full advantage of the owner's library as it grows by orders of magnitude. Evidence:
`docs/tutor/research/corpus-use.md` (round 3) plus `engineering.md` and `corpus.md`. Interfaces: C1, C9
and C11.

## Owner policy (2026-09-30)

- **Indexing stays on this machine** (tantivy BM25, pdfium text). Embeddings and OCR run later on the
  owner's second desktop (Stage 2).
- **The lesson models may read the library:** retrieved passages, the section or chapter the learner has
  opened (on-demand preparation), and page images of retrieved pages when extracted maths text is
  garbled. This is the research's decision D1 in its on-demand form, plus D2.
- **Bulk runs over whole books or shelves** (batch question generation, graph extraction) wait for a
  monthly spend ceiling from the owner.

## Stage 0: better retrieval now (no embeddings)

1. **Query pipeline** (DeepTutor side, contract C11):
   - One small-model call writes 3-5 queries: textbook vocabulary, notation, a short HyDE-style passage,
     and a prerequisite query. The raw query always runs too.
   - Results are fused by RRF (k 60).
   - A listwise rerank runs only when fused scores are flat or the tutor asks for depth (BRIGHT: rewriting
     is the big win, reranking about +2.7).
   - Each step is a setting, off by default until its eval arm wins.
2. **Agentic search:** the lesson model gets `search` and `open_chunk`, capped at 4 searches per turn.
3. **Evidence packer:**
   - Hits expand to their section.
   - A quota per source; 4-12 chunks (3-8k tokens), strongest first and last, the question last.
   - Titles and heading paths are prepended.
   - Escalation to a cached section or chapter (10-50k tokens) happens only when the learner opens a
     chapter or the first pass falls short. Never whole-book by default.
4. **No answer leaks:** while an attempt is open (C7), library search excludes chunks that contain the
   attempt's correct answer, and chunks under solution or answer-key headings. The model cannot look the
   answer up.
5. **Citations, verified (level L0):**
   - Every cited quote must be a substring of the cited chunk, and the pages must lie in its range.
   - A failed citation is removed, and the tutor says "I cannot verify this".
   - Claude uses the Citations API with one custom-content block per chunk. Other models cite as
     `[c:<chunk_id>] "quote"`.
6. **Source chip** after the attempt closes: title, page, a verified badge, and a one-tap page crop
   rendered locally by pdfium with the quote marked. The tutor asks the learner to audit one citation per
   session.
7. **Index quality:**
   - Book title and heading path become separate boosted fields.
   - Unicode maths normalisation, since pdfium emits glyph text, not LaTeX.
   - tantivy tuned for growth: writer heap 1-2 GB, commits every 50-200k chunks, delete ratio about 0.2.
8. **Gate:** the `evals/` harness with train/test arms:
   - A: raw BM25; B: plus fields and tokenizer; C: plus rewrites and RRF; D: plus rerank;
     E: plus agentic search.
   - Gold questions come from the owner's own attempts, or are written without seeing the answer passage.
   - Keep-margins are fixed beforehand. The latency bound is under 5 s to first token in voice.

## Stage 1: reading, questions and maps (on-demand now; bulk after a spend ceiling)

- **Guided Reading** (a section is 5-15 pages, taken from the heading paths):
  1. Opening a section triggers one preparation call. It returns 1-3 prequestions, 4-10 idea units, at
     most 5 candidate atoms and proof warrants, each with chunk ids. Keys stay on the server.
  2. The learner guesses the prequestions with confidence, then reads the original text in a clean
     reader.
  3. Closed-book recall.
  4. The learner writes a summary and judges it 1-4.
  5. Idea units are scored by two passes. The tutor fills in only the units that were missed.
  6. The learner accepts atoms, which become cards.
  - There is no summarize button.
  - Paper mode: predict from the abstract, fill an evidence table, write a critique.
- **Question generation (on demand by skill coverage):**
  - Opus generates. Then come span grounding, CAS/unit/RDKit gates, two blind solvers (one from another
    model family) and MinHash dedup.
  - Verified items feed FSRS and mastery. Provisional ones (proofs, essays) do not.
  - The owner audits 100 random items per generator version. Items are quarantined and regenerated,
    never patched.
  - Difficulty comes only from the learner's unassisted responses.
- **Concept graph (per active book):**
  - Every edge carries a verbatim quote, checked by substring match. Two runs, with Opus only for
    disagreements.
  - Advisory "ready" or "not yet" with quote and page. Never a gate, never a retrieval ranker.
  - A curated layer always beats the machine layer.
- **Cross-source questions:** retrieve per source, extract claims, then classify each pair: same,
  notation, scope, superseded, real disagreement, or cannot tell. Quotes are shown side by side, never
  merged.
- **Gates:**
  - Reading: idea-unit precision and recall on about 20 hand-annotated sections, plus the prequestion
    answer-leak rate.
  - Questions: a wrong-key audit.
  - Graph: 0.8 precision on 100 labelled edges per type.
  - All of them: delayed unassisted accuracy.
  - At most one new n=1 experiment, prequestions on alternating chapters, alongside n-back and
    interleaving.

## Stage 2: the second desktop

- **Local embeddings** (a bake-off on the gold set: Qwen3-Embedding 0.6B/4B/8B, BGE-Reasoner-Embed,
  EmbeddingGemma), plus a local reranker.
  - Fused with BM25 by RRF.
  - Embedded in tiers: active shelves first.
- **OCR:** an `OcrProvider` that calls a local vLLM endpoint.
  - Candidates, by bake-off on 200 owner pages: olmOCR-2, dots.mocr, Chandra, PaddleOCR-VL.
  - Queue order: active shelves, math-heavy pages, then scanned pages.
- **Transport:** the engine stays on 127.0.0.1 there, reached over an SSH tunnel plus the bearer token
  (C9).
- **Re-test:** every Stage 0 arm is re-run once dense retrieval exists (expansion can hurt stronger
  retrievers).

## Stage 3: growth by 10-100x

- **Ingest and storage:**
  - Process-per-worker pdfium extraction with one committer.
  - An integer chunk key.
  - The Lance index retrained when rows roughly double.
  - Matryoshka truncation plus int8 or binary vectors with rescoring.
- **Backups:** sources, SQLite (VACUUM INTO) and Lance, but not tantivy.
- **Gate:** synthetic corpora of 1M, 10M and 100M chunks, with BM25 top-50 p95 under 100 ms warm.

## Not building

- Summarize buttons or chat-with-PDF; highlighting as a study tool.
- Library-wide bulk extraction or GraphRAG.
- A graph as a gate.
- LLM difficulty labels.
- A single LLM-judge faithfulness score as a gate; numeric confidence badges.
- Whole-book stuffing by default.
- Auto-generated courses without a diagnostic and verification.
