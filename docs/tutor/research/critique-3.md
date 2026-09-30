# Completeness critique, research round 3

Basis: I read the round-3 SYNTHESIS.md, engineering.md, corpus.md, critique-2.md and PEDAGOGY.md in full. I did not re-check primary sources. The `survey` command exists in the repo (crates/rag-engine/src/survey.rs), so the inventory the synthesis defers could be run now.

## Gaps, most serious first

1. **Who audits correctness?** Round 3 leans on owner audits (100 items per generator version, 100 atoms and 100 edges per book family, 100 edges per domain and type, 20 annotated sections, 200 OCR pages, 20-30 seeded conflicts, 150-300 gold queries, one citation per session). The owner is the learner. Checking a wrong key in PhD-level material they have not yet learned is circular. The plan needs an independent oracle (CAS, a textbook's answer key, a tutor or domain expert) and a summed owner-hours budget. Critique-2 gap 1 (no total owner time) is now worse.
2. **Scanned pages are invisible to Stage 0.** BM25 over pdfium text cannot retrieve a scan. "Not found in the library" (PEDAGOGY grounding rule) is then false for any book with a large scanned share. Coverage per book (indexed-text share) must be tracked, and the tutor's miss message conditioned on it. A local CPU OCR stopgap (D3 covers only embeddings) is not raised.
3. **Foreign languages are absent.** Five languages get no treatment of tokenisers and stemming, CJK or Arabic or Cyrillic BM25, cross-lingual query rewriting, or multilingual OCR (only PaddleOCR-VL is mentioned). The English humanities track is also missing: Stage 0 is maths-shaped.
4. **The LaTeX-aware tokenizer has nothing to work on.** pdfium returns Unicode glyph text, not LaTeX. The tweak helps only LaTeX-source texts (Judson, Stacks) or post-OCR output. Math retrieval over pdfium text needs Unicode math normalisation, and this is untested.
5. **Answer withholding is undefined.** How does the server recognise "solution and answer chunks" in unstructured books (back-of-book answers, worked examples inside sections, solutions manuals)? The evidence packer also "expands hits to the section", which contains the worked solution. Rewrites and agentic search can reach neighbouring chunks. No leak test is specified beyond the "prequestion answer-leak rate".
6. **Ingest time and dedup.** Serial ingest (marked U) is the real bottleneck for 250 GB. There is no estimate of extraction hours, no duplicate-edition or duplicate-file handling (which skews BM25 and per-source quotas), and no source authority, level or edition weighting (intro text versus graduate text for a PhD-level learner).
7. **Which learner surfaces use retrieval?** PEDAGOGY has no ask-anything mode. Round 3 designs cited answers, agentic search and library search without mapping them to attempt-first flows (question generation, post-attempt feedback, guided reading).
8. **Voice.** The 4-search agent loop, rewrites and rerank conflict with the 5 s first-token target. `[c:ID]` quote triples cannot be spoken, and ElevenLabs' own knowledge and retention path is unexamined.
9. **No cost ceiling.** Unit costs are given, but there is no daily or monthly spend estimate or cap. The cache write ($5/M) and the five-minute TTL are ignored in the "$0.006 per turn" claim.
10. **Weak transfer of evidence.** BRIGHT (StackExchange reasoning queries, GPT-4 reasoning as query) may not transfer to textbook-lookup queries where BM25 is strong. "Strongest first and last" packing is unsourced. The evidence for rewriting is 2024 and pre-dates the models used here.
11. **Figures, tables and chemistry structures** are covered only by page crops. Structure recognition is deferred (engineering C5) and not mentioned here. The backup plan has no restore-time target when tantivy must be rebuilt from 250 GB or more.

## Contradictions with earlier rounds

1. **Hosted OCR and embeddings.** engineering.md recommends hosted parsers and embeddings (Mathpix, Mistral, Datalab, voyage) and lists self-hosted OCR and embedding under "do not build", citing no GPU. Round 3 reverses both (owner policy plus the second desktop) without marking engineering.md rec 4, rec 9 and C5 as superseded. The $150-400 tiered plan and C6 are now void.
2. **Grounding check is weaker.** engineering rec 2 and PEDAGOGY require that every number and equation in an answer appear in cited text. Round 3's L0 checks only chunk existence, quote substring and page range. That drops the numeric and equation check.
3. **Proofs are "provisional" but PEDAGOGY puts them on the mastery path.** Round 3 rule 6 says provisional items (proofs, essays) never feed mastery. PEDAGOGY makes proofs and derivations mastery-scheduled, with self-explanation required. How proof skills ever reach "mastered" is undefined (two-pass LLM rubric, Lean?).
4. **Experiment budget.** PEDAGOGY's round-2 rule says at most two running at a time; round 3 adds a third by citing the looser "two or three" line. "Alternating chapters" also breaks engineering rec 17 (randomise over items or days, never periods) and confounds chapter difficulty. At the MDE limits in engineering (under about 6 points is unresolvable), a prequestion effect may be unmeasurable.
5. **Graph.** PEDAGOGY: "small, hand-edited data". Round 3 builds LLM-extracted per-book graphs (and cites Kolluru's null result for retrieval) with no outcome evidence, plus owner labelling.
6. **Licence gate.** corpus.md rule 1 (`ai_use` per item, indexer refuses to send non-allowed items to an LLM) and rules 9-10 (short quotes only from non-open items, no long quotation in items) are not reconciled with 3-8k-token packs, 10-50k-token chapter caches and stored verbatim quotes. Owner-copy terms remain unchecked (critique-2 gap 2 is still open).
7. **Cross-family blind solve** (PEDAGOGY) depends on Muse Spark terms (D4) with no fallback if they are unacceptable.
8. **Open schema gap from engineering C4** (source_span, page, parse_id, span_verified) is not carried into Stage 1 item design.

## Most valuable follow-ups

1. Run `survey` on the real library now. Report chunk count, scanned and math-dense shares, pages per GB, ingest throughput and duplicates. This answers D5 and gaps 2 and 6.
2. Decide D1, D2 and D4, and add a cost-ceiling decision. Write one owner-hours budget and name the independent key oracle per domain.
3. Write one integrated design that resolves contradictions 1-8 and marks superseded lines in engineering.md and corpus.md.
4. Build the gold set (50 queries) and the leak test: queries whose answers sit in same-section solutions, and a solution-chunk classifier check.
5. Research and prototype multilingual tokenisation and cross-lingual retrieval, plus a CPU-local OCR stopgap for scans.
6. Specify the voice path: latency budget, how citations are spoken or shown, and ElevenLabs retention terms.
7. Decide the proof-mastery path before shipping provisional items.
