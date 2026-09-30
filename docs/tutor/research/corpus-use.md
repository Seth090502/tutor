# Tutor: exploiting the library (research round 3 synthesis)

Date 2026-09-30. I re-checked 30 load-bearing items against primary pages (Crossref, Europe PMC, arXiv, Anthropic docs, tantivy source, olmOCR README, the repo's Cargo files). Nothing the reports rest on failed outright; several numbers were corrected and a few stay labelled as carried. The fetch tool returns a small-model summary, so figures are "as reported by that summary".

## 1. Bottom line

1. The biggest measurable gains today are LLM-side over local BM25: query rewriting fused with the raw query, agentic search, evidence packing, machine-checked citations. None needs embeddings or OCR, and only retrieved passages leave the machine.
2. Guided reading, question generation and prerequisite maps read more than retrieved passages if run in bulk. Three reports proposed whole-chapter cloud reads, which is an owner decision (D1). The default below is narrow: only the section the learner opened, on demand; nothing bulk-run until the owner answers.
3. No evidence shows that AI-built concept maps, AI item banks or AI paper tutors improve delayed learning; only retrieval-shaped activity has outcome evidence. Each such feature ships behind a gate and is judged by delayed unassisted accuracy.
4. Scaling to 10-100x is a vector and OCR problem, not a BM25 problem. Keep BM25 complete, embed in tiers, and choose models after the inventory and the second desktop's hardware are known.

## 2. Verification table

Status key: V = confirmed at the source I opened; C = corrected; P = partly confirmed (record exists, the number was not reachable); U = carried, not re-checked.

| # | Claim | Status | What I found |
|---|---|---|---|
| 1 | Yang 2021: testing g 0.499, 222 studies, 48,478 students | V | Abstract via Europe PMC, exact |
| 2 | Karpicke & Blunt 2011: retrieval beat concept mapping, even on a concept-map test | V | Abstract exact |
| 3 | McDaniel 2009: 3R beat rereading; matched note-taking on problems in less time | V | Abstract exact; two experiments, college students |
| 4 | Peters & Chin-Yee 2025: LLM summaries overgeneralise | C | 26-73% applies to DeepSeek, ChatGPT-4o and LLaMA 3.3 70B. Odds ratio 4.85 versus human summaries. Newer models were worse |
| 5 | Pan & Sana 2021, Carpenter 2018, Ponce 2022, Bisra 2018 | P | Records confirmed; effect sizes (g 0.55, 0.36/0.20, n=1,573) not reachable, so they remain priors |
| 6 | BRIGHT: BM25 14.5, best embedder 18.3, GPT-4 rerank of BM25 top 100 gives 17.0 | C | 14.5 and 17.0 in the paper's tables; 18.3 (SFR-Embedding-Mistral) in the abstract. The "Qwen 22.5" reading is dropped |
| 7 | BM25 with GPT-4 reasoning as query | C | Paper says BM25 becomes the leaderboard's best and "below 30". Exact value is in appendix tables I did not read. A summary said 29.0 but the abstract says gains are "up to 12.2 points" (14.5+12.2 = 26.7). Use "about 27" |
| 8 | DIVER 46.8 versus 31.9 | C | 31.9 is on original queries, not "the retriever alone" |
| 9 | Weller 2024: expansion gains fall as retrievers get stronger | V | 24 retrievers, 12 datasets, 11 techniques; expansion generally harms strong retrievers |
| 10 | Isley 2025 field study | C | 91 classes, about 1,700 students. Discrimination 1.3 versus 1.2 (Pr 0.85); difficulty -0.45 versus 0.35, so AI items were easier (about 60% versus 39% correct). Source says "multiple STEM disciplines", not only statistics. No flaw rate or cost |
| 11 | An & Wang 2026: LLM difficulty labels rho 0.06 | C | Correct, but the set is 378 authored questions, 311 deployed, 7,888 responses, one institution. Labels co-generated with Bloom level (rho 0.90) |
| 12 | GPQA experts 65% | P | 65%, non-experts 34%, 448 items confirmed. The 74% "after removing errors" is not in the abstract (U) |
| 13 | HLE-Verified 668 / 1,143 / 689 of 2,500 | V | Exact |
| 14 | MMLU 6.49% erroneous, Virology 57% | V | Exact |
| 15 | Liu 2023: 51.5% sentences fully supported, 74.5% citation precision | V | Four engines: Bing Chat, NeevaAI, Perplexity, YouChat |
| 16 | ALCE: best models lack full support 50% of the time on ELI5; Wallat 2024: up to 57% unfaithful citations | V | Both in abstracts. Wallat's abstract does not name the systems tested |
| 17 | Kolluru 2026: typed graph edges add nothing | C | 690 agent skills (not textbooks), 117 queries; graph substitution -11.2 hit@5 points (p 0.0007); 98.6% of typed edges link skills the ranker already surfaced. Author-written queries overstated performance by up to 44 points |
| 18 | Cui 2026: LLMs violate prerequisites | P | Direction confirmed. The 72.7% versus 48% figures are not in the abstract (U) |
| 19 | NoLiMa: 11 models below 50% of baseline at 32K | V | Abstract states 11; the "of 13" is from R6 |
| 20 | MiniCheck 770M at GPT-4-level grounding checks, 400x cheaper | V | Validity on math, chemistry, non-Latin text unverified |
| 21 | Anthropic prices | V | Opus 5.5 $4/$20, cache write $5, read $0.20. Sonnet 5.5 $2/$10, write $2.50, read $0.20. Batch: Opus 5.5 $2/$10, Sonnet 5.5 $1/$5. 1M context at flat price; tokenizer about 30% more tokens. Reports' arithmetic recomputes correctly |
| 22 | Citations API | V | Valid pointers guaranteed; cited_text not billed as output. Custom-content documents are not re-chunked. Scanned PDFs are not citable. Incompatible with structured outputs (API errors). Covers pointers, not support |
| 23 | R6's retention worry | C | Citations is ZDR-eligible. Only Fable 5/5.1 and Mythos 5/5.1 are "Covered Models" (30-day retention). Opus 5.5 and Sonnet 5.5 are not. Muse Spark and ElevenLabs terms unknown |
| 24 | tantivy 0.26.2 merge defaults | V | 8 segments, 10M docs, min layer 10,000, log size 0.75, delete ratio 1.0 ("not a very sensible default") |
| 25 | Repo facts | V | tantivy 0.26.2, lancedb 0.37.1, pdfium-render 0.9.4; writer 256 MiB; server binds 127.0.0.1; only a test MockOcr implements OcrProvider. Serial ingest and the text chunk-id key not re-read (U) |
| 26 | olmOCR: 82.4, Apache 2.0, 12 GB+, under $200 per million pages | C | Confirmed (paper $176; GPT-4o over $6,240). But the README table lists Chandra 83.1 and Infinity-Parser 82.5 too, PaddleOCR-VL 80.0, Marker 76.1, MinerU 75.2. Authors' own benchmark, so olmOCR is a bake-off candidate, not a finding |
| 27 | Kestin 2025, Keshav timings, Hodds d 0.95, model and OCR licences | U | Not re-checked; keep out of product copy |

Hype, excluded: BRIGHT leaderboard entries above 50, Kosmyna EEG (n=54), Melumad & Yun depth claims (self-report), "90% fewer hallucinations" with no baseline, vendor LanceDB 100M-vector costs.

## 3. Contradictions and how I resolved them

- **Cloud egress.** R3 sends only retrieved passages. R1 (per-section prep), R2 (chapter extraction) and R6 (course builder) send whole sections or chapters. Resolution: Tier A, passage-only, is allowed now; Tier B, anything larger, waits for D1. Its narrowest form, "the section the learner opened, on demand", stays unapproved until the owner says so.
- **Experiment budget.** PEDAGOGY allows two or three n=1 experiments and two are taken (n-back, interleaving). R1, R2 and R6 each propose one; run at most one more, in this order: prequestions, citation display, advisory prerequisites.
- **BRIGHT figures.** R3's summaries conflicted (18.3 versus 22.5; 27-29); resolved in rows 6-7.
- **Graph cost.** R2 ($1-3 per book) and R6 ($0.40 input plus a little output) agree once passes are aligned: $1.10 batched for two passes at 350k in / 40k out.
- **Port.** R5's tunnel example uses 7878; the desktop app configures rag-engine at 8811.
- **Citation path.** Claude custom-content citations cannot be combined with JSON output, so structured generation and cited explanation are separate calls.

## 4. Staged architecture

Data flow, all stages: `library -> pdfium -> chunks (SQLite) + tantivy -> query pipeline -> evidence packer -> lesson LLM -> cited answer -> verifier -> UI`. Question, reading and graph jobs are background queues that read chunks and write only to the question bank, reading sessions and the graph store.

### Stage 0. Now: local BM25 plus LLM-side techniques (passage-only egress)

| Component | What it does | Cost (derived from verified prices) |
|---|---|---|
| Inventory and gold set | Run `survey` for chunk count, scanned share, math-dense share. Build a 50-question gold set from the owner's books, growing to 150-300 | Owner time |
| Index tweaks | LaTeX-aware tokenizer; heading path and book title as separate boosted fields | None |
| Query pipeline | One Sonnet 5.5 call writes 3-5 queries (textbook vocabulary, notation, short HyDE-style passage, prerequisite query). Run all, fuse with RRF, always include the raw query | About $0.005-0.015 per turn |
| Conditional rerank | One listwise call on 30-50 snippets, only when fused scores are flat or the tutor asks for depth. BRIGHT gave only +2.7 for this step | $0.03-0.05 Sonnet 5.5; $0.06-0.10 Opus 5.5 |
| Agentic search | Give the lesson model `search` and `open_chunk`, capped at 4 searches per turn | Tokens only |
| Evidence packer | Hits expand to section, per-source quota, 4-12 chunks (3-8k tokens), strongest first and last, question last, titles and heading paths prepended. The server withholds solution and answer chunks while an attempt is open | None |
| Cited generation and verifier | Claude: one custom-content block per chunk. Muse Spark and ElevenLabs: `[c:ID]` plus quote triples. L0 checks chunk id, quote substring, page range. Failure means "I cannot verify this" | Quotes not billed as output on Claude |
| Source chip | After the attempt closes: title, page, badge, pdfium page crop with the span marked. No numeric confidence. Scanned or garbled pages show the crop and "text not extracted" | None |
| C0 structural extractor | Headings, numbered Definition/Theorem environments, back-of-book index, in-text references, all local | None |

**Gate for Stage 0 (existing `eval-search`, train/test).** Arms: A raw BM25, B tokenizer and fields, C plus rewrites with RRF, D plus rerank, E plus agentic search. Tune on train only, score test once per stage, paired bootstrap, keep a component only if it beats the previous arm by a margin fixed beforehand and meets latency (under 5 s to first token in voice). At about 200 queries, differences under 0.03-0.05 nDCG are unresolvable; report that, not wins. Gold queries come from the owner's attempts or are written without seeing the answer passage (Kolluru: author-written queries overstated results by up to 44 points). Citation gate: L0 pass rate, unsupported-claim rate and abstention on absent answers for Opus 5.5, Sonnet 5.5 and Muse Spark 1.3, thresholds set beforehand. C0 gate: the owner hand-checks 100 random atoms and 100 `uses` edges per book family.

### Stage 1. Now, after D1: on-demand reading, questions, graph

| Component | Design | Cost |
|---|---|---|
| Guided Reading | Sections from heading paths (5-15 pages). One background call per opened section returns 1-3 prequestions, 4-10 idea units, at most 5 candidate atoms and proof warrants, each with chunk ids; keys stay server-side. Order: prequestion guess with confidence, read, closed-book recall, learner summary with 1-4 judgement, idea-unit scoring by two passes, tutor fills only missed units. Paper mode: predict from abstract, evidence table, critique | About 5k in / 1k out per section: $0.02 unbatched on Sonnet 5.5; about $0.20 per 10-section chapter |
| Question generation | Demand queue by skill coverage. Opus 5.5 batch generates; span gate; CAS gates (SymPy, Math-Verify, units, RDKit, re-run each distractor's wrong procedure); two blind solvers, one from another family; MinHash dedup. Verified items feed FSRS and mastery; provisional ones (proofs, essays) do not | About $100 per 1,000 verified at an assumed 60% yield; $78-152 across variants |
| Concept graph | C1 extraction per active book, every edge carrying a verbatim quote checked by substring, two runs, Opus only for disagreements. Machine layer regenerable; curated layer always wins; cross-book `same_as` links are non-destructive | About $1.10 per 500-page book batched |
| Prerequisite use | "Ready" or "not yet" with quote and page, overridable, never a gate and never a retrieval ranker | None |

**Gates.** Reading: idea-unit precision and recall on about 20 hand-annotated sections, proof-warrant accuracy, prequestion answer-leak rate, tutor-summary overgeneralisation. Questions: owner audits 100 random items per generator or prompt version (zero wrong keys bounds the error under about 3% at 95%); any wrong key changes prompt or gates. Graph: owner labels 100 edges per domain and edge type; "not yet" is allowed only at 0.8 precision (my threshold). Outcome gate for all three: delayed 7- and 30-day unassisted accuracy on parallel items.

### Stage 2. Second desktop: local embeddings, reranker, OCR

- **Precondition.** GPU, VRAM, RAM, NVMe (D5) and the Stage 0 inventory. A 24 GB card makes Qwen3-Embedding-8B in bf16 the practical ceiling (derived).
- **Embeddings.** Bake-off on the gold set: Qwen3-Embedding 0.6B/4B/8B, BGE-Reasoner-Embed-8B, EmbeddingGemma-300M (licences carried from R3, unchecked). Then Qwen3-Reranker on the top 100. Vectors in LanceDB, fused with BM25 by RRF, embedded in tiers: active shelves first, the rest when the GPU is idle.
- **Re-test Stage 0.** Expansion often hurts stronger retrievers (Weller), so arms F dense, G dense plus BM25 plus reranker, H plus rewrites decide what stays.
- **OCR.** Tier pages: pdfium text for clean pages; a VLM parser for scans and math-dense pages; a second engine when LaTeX fails a KaTeX render. Bake-off on 200 owner pages: olmOCR-2, dots.mocr, Chandra, PaddleOCR-VL (multilingual). Implement `OcrProvider` as an HTTP client to a local vLLM endpoint; queue order: active shelves, math-heavy, scanned; pause batch jobs during a lesson.
- **Transport.** Engine stays on 127.0.0.1 on the second desktop; the app reaches it by `ssh -L` (untested on Windows OpenSSH) plus a bearer token.
- **Local verifier.** Add an L1 MiniCheck-class check only after validating it on the owner's math, chemistry and non-Latin text.
- **Gate.** Embedding shrink setting accepted only if recall@50 against flat search on a 100k-chunk sample loses under 2 points and `eval-search` does not drop. OCR accepted on owner-page accuracy, pages per second recorded.

### Stage 3. 10-100x growth

One tantivy index with writer heap 1-2 GB, commits per 50-200k chunks, delete ratio about 0.2, and an integer chunk key in SQLite; process-per-worker pdfium extraction with one committer; explicit Lance index retrained when rows roughly double; Matryoshka truncation plus int8 or binary with rescoring; back up sources, SQLite (VACUUM INTO) and Lance but not tantivy, in a separate restic repository; indexes stay outside OneDrive. **Gate:** synthetic 1M, 10M and 100M chunk corpora, replay 5-20k queries, accept only BM25 top-50 p95 under 100 ms warm (my target). No public 10M+ tantivy or 100M LanceDB latency was found.

## 5. Ranked rules

1. **[measurement]** Build the owner gold set and the eval arms before any retrieval change ships; fix keep-margins in advance, tune on train only, and write queries without seeing the answer passage.
2. **[prompt]** Retrieval must not leak answers: the server withholds solution and answer chunks while an attempt is open (extends contract C7).
3. **[feature]** Every lesson citation passes L0 (chunk exists, quote is a substring, pages valid) or is removed and the tutor says "I cannot verify this". Older systems had 51.5% fully supported sentences; measure current models before trusting them.
4. **[feature]** LLM query rewrites fused with the raw query by RRF, plus an agentic search tool capped at 4 per turn; listwise rerank only when scores are flat.
5. **[UI]** Guided Reading wraps the original text; no default summarize button. Learner recall and summary with a 1-4 judgement come first; the tutor shows only missed idea units.
6. **[scheduling]** Only unassisted retrieval feeds FSRS. Candidate atoms are capped near 5 per section and learner-accepted. Provisional items never feed FSRS or mastery.
7. **[question generation]** Generate on demand by skill coverage; deterministic gates, then a cross-family blind solve; owner audit of 100 items per generator version; quarantine and regenerate, never patch.
8. **[question generation]** Difficulty comes only from the learner's unassisted responses at variant-family level with shrinkage. No LLM difficulty labels (rho 0.06), no per-item discrimination.
9. **[prompt]** Default context is a 3-8k-token pack. Escalate to a cached section or chapter (10-50k tokens) when the learner opens a chapter or the first pass falls short; never whole-book by default. Cost does not decide it: a cached 30k-token chapter is $0.006 per turn.
10. **[UI]** Prerequisites are advisory: quote, page, override, and only for edge types that reached 0.8 labelled precision. The graph never gates and never ranks retrieval.
11. **[feature]** Cross-source questions: retrieve per source, extract claims, classify (same, notation, scope, superseded, real disagreement, cannot tell), show quotes side by side, never one merged paragraph. Test on 20-30 seeded conflicts.
12. **[measurement]** At most one new n=1 experiment beside n-back and interleaving: prequestions on alternating chapters, with next-day, 7-day and 30-day unassisted quizzes. Expect to need a few dozen sections.
13. **[infrastructure]** Inventory and benchmark before spending: run `survey`, build synthetic scale corpora, choose embedding and OCR models only by bake-off on the owner's pages and questions.
14. **[infrastructure]** Indexing stays local. Only retrieved passages leave, until D1 is answered.
15. **[UI]** Verified badge plus one-tap page crop; ask the learner to audit one citation per session (citations raise trust even when fake, Ding 2025, abstract-level).
16. **[measurement]** Re-run every rewrite and rerank arm once dense retrieval exists.

## 6. What not to build

- Default summarize buttons, open chat-with-PDF, highlighting or note tools as study features, rereading loops, rigid SQ3R, prequestions on every paragraph, a concept-map editor as the core.
- Library-wide bulk extraction, GraphRAG, or bulk question generation (one candidate per chunk over 1M chunks is about $60,000).
- A graph as gate or ranker; a global ontology; fine-tuned prerequisite classifiers; destructive merges.
- LLM difficulty labelers, LLM-simulated-student schedulers, 2PL or 3PL fits for one learner, or an LLM judge as sole arbiter of key correctness.
- Cloud embeddings, cloud OCR, cloud index-time expansion or contextual retrieval over the whole library (policy).
- A single LLM-judge "faithfulness score" as a gate, numeric confidence, decorative citation chips, a fine-tuned citation model.
- ColPali or vector page retrieval before the second desktop holds the library; Quickwit, Elasticsearch, custom ANN, streaming vector updates, multiple embedding models at once, multi-user auth, OCR on every page.
- Whole-book stuffing by default, or auto-generated courses without diagnostic and verification.

## 7. Owner decisions needed

- **D1.** May a section or chapter the learner has opened go to a cloud LLM for on-demand preparation (prequestions, idea units, extraction)? Bulk batch runs over whole books need a separate yes.
- **D2.** May page images of retrieved PDF pages go to lesson LLMs when math text is garbled?
- **D3.** May small CPU-only embedding models run now for item dedup?
- **D4.** Muse Spark 1.3 price, batch support, retention terms and native attribution, and the same for ElevenLabs agents. The cross-family blind solve depends on this.
- **D5.** Second-desktop GPU, VRAM, RAM and NVMe.
- **D6.** Licence reading for MinerU, Marker and dots OCR.

## 8. Citations and status

V: Yang 2021 (10.1037/bul0000309); Karpicke & Blunt 2011 (10.1126/science.1199327); McDaniel 2009 (10.1111/j.1467-9280.2009.02325.x); Weller 2024 (2309.08541); HLE-Verified (2602.13964); MMLU-Redux (2406.04127); Liu 2023 (2304.09848); ALCE (2305.14627); Wallat 2024 (2412.18004); NoLiMa (2502.05167); MiniCheck (2404.10774); olmOCR paper (2502.18443); tantivy `log_merge_policy.rs`; Anthropic pricing, Citations and data-retention docs.

C: BRIGHT (2407.12883); DIVER (2508.07995); Isley (2508.08314); An & Wang (2609.27063); Kolluru (2608.06196); Peters & Chin-Yee (10.1098/rsos.241776); olmOCR README; R6's retention worry.

P: Pan & Sana (10.1037/xap0000345); Carpenter (10.1037/xap0000145); Ponce (10.1007/s10648-021-09654-1); Bisra (10.1007/s10648-018-9434-x); GPQA (2311.12022); Cui (2609.05245).

U: Kestin, Hodds, Keshav timings, Prinz, Zhai, InstructKG, K12-KGraph, KGGen, RAMDocs, RAPTOR, GraphRAG, LongCite, Qwen3 and BGE cards, MinerU, Marker and dots figures, Muse Spark and ElevenLabs terms, and every cost or latency estimate the reports tagged derived (recomputed here only where Anthropic prices apply).

Limits. Evidence is mostly undergraduates and children on short texts; transfer to a PhD-level adult is assumed. No outcome trial of LLM-generated questions, AI prerequisite graphs, AI paper tutors or retrieval-grounded tutors was found. The 60% yield, wrong-key rate, latencies and scale figures are assumptions until measured.
