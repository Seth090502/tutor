# Engineering synthesis: technical RAG and scheduling design for Tutor

Verified 2026-09-30 by re-opening primary pages (arXiv, GitHub READMEs, vendor pricing pages, Crossref, Europe PMC, Anthropic docs). Inputs: X1 (technical RAG) and X2 (scheduler and mastery). "[sim]" means a simulation by the report author, not literature; I did not re-run any simulation.

## 1. Bottom line

1. Parsing quality and citation discipline matter more than embedding choice. Tier the parse, ground every quantitative claim in a stored span, and show the page image next to any quoted formula.
2. Schedule flashcard-like atoms with FSRS-6 at its published defaults and 0.90 desired retention. Model procedures and proofs with a per-skill decayed Beta posterior plus a sequential mastery test, not FSRS and not "k in a row".
3. Nearly all numbers that are specific to Tutor (cost, pages per GB, power, workload) rest on assumptions or simulations. Build a corpus inventory and a hand-checked gold set first so these become measured.
4. No cited study involves PhD-level adults, and no source shows FSRS improves learning; its evidence is recall prediction on Anki data.

## 2. Verified table

Status: **verified** = page opened, claim matches; **corrected** = claim needed a fix; **partial** = exists, but the specific figure was not confirmed; **failed** = could not be confirmed.

| # | Claim | Source checked | Status | Note |
|---|---|---|---|---|
| 1 | olmOCR-Bench math scores: olmOCR v0.4.0 83.0 (ArXiv) / 82.3 (old scans); Chandra 0.1.0 82.2 / 80.3; Marker 1.10.1 83.8 / 66.8; Mistral OCR API 77.2 / 67.5 | allenai/olmocr README table | **verified** | Overall: Chandra 83.1, olmOCR 82.4, Marker 76.1, Mistral 72.0. Ai2 authors the benchmark. |
| 2 | olmOCR is Apache 2.0, claims under $200 per million pages | same README | **verified** | The cost claim is for self-hosting, which the owner cannot do. |
| 3 | olmOCR 2 paper: Poznanski, Soldaini, Lo, arXiv 2510.19817 | arXiv abstract | **verified** | |
| 4 | Hosted olmOCR at $0.07-0.10 per 1M input tokens, and the per-page estimate of about $0.25-0.45 per 1k | DeepInfra page returned 404 | **failed** | Price and token counts remain unverified. The "$185 OCR everything" cost is dropped from recommendations. |
| 5 | MIRB: Ju and Dong, 12 datasets, 13 models, voyage-3-large 54.54, text-embedding-3-large 49.93 | arXiv abstract and PDF text | **verified** | |
| 6 | MIRB reranking on top 10: voyage-3-large 54.54 becomes 48.36 (bge-reranker-v2-m3) or 50.42 (jina-reranker-v2); text-embedding-3-large 49.93 becomes 44.67 or 46.55 | PDF Table 10 text | **verified** | The paper says reranking "generally leads to a decline", with a few slight gains. It also says reasoning and premise retrieval are hard for all models. Only two older general rerankers were tested. |
| 7 | ALCE (Gao, Yen, Yu, Chen, EMNLP 2023): best models lack complete citation support 50% of the time on ELI5 | arXiv abstract | **verified** | Models of that date. |
| 8 | Claude Citations: `cited_text` not billed as output; citations on `document` or `search_result` blocks plus structured outputs returns HTTP 400; pointers parsed by the API | platform docs (markdown) | **verified** | Pointers are valid by construction; nothing says the passage supports the claim. |
| 9 | Mathpix: $0.005/page (v3/pdf), $0.0015/page (Files API batch), $0.002/image, $19.99 setup fee, 30-day default retention, 24 h opt-out | mathpix.com/pricing/api | **verified** | Accuracy on math is still unbenchmarked independently. |
| 10 | Mistral OCR 4.1 at $4 per 1,000 pages | mistral.ai/pricing/api | **verified** | A general half-price Batch tier is listed; whether it covers OCR is not confirmed. |
| 11 | Datalab convert $4 per 1k (fast/balanced), $10 (accurate) | datalab.to/pricing | **verified** | The "25% off for retention" claim was not on the page. |
| 12 | voyage-4 $0.06 per 1M tokens, first 200M free; voyage-context-4 $0.12; batch 33% off | docs.voyageai.com pricing | **verified** | voyage-3-large is $0.18, with no free tokens. |
| 13 | Anthropic contextual retrieval: failures down 35% (5.7% to 3.7%), 49% with contextual BM25, 67% with reranking (to 1.9%); $1.02 per million document tokens with caching | anthropic.com/news | **verified** | Vendor-reported; not textbook math; 2024 pricing. |
| 14 | FSRS benchmark: about 10k users, about 350M reviews; FSRS-6 log loss 0.3460; FSRS-5 0.3561; FSRS-4.5 0.3625 | srs-benchmark README | **verified** | 9,999 collections, 349,923,850 reviews. |
| 15 | "FSRS-7 0.3370" | same README | **corrected** | 0.3370 is the "FSRS-7 recency" variant. Plain FSRS-7 is 0.3401. Both beat FSRS-6. |
| 16 | FSRS-6 has 21 parameters; R=(1+factor·t/S)^(-w20) with trainable decay w20 | awesome-fsrs wiki, The Algorithm | **verified** | The wiki does not mention FSRS-7. |
| 17 | Licences: fsrs-rs (crate `fsrs`) BSD-3-Clause; py-fsrs MIT | GitHub repos | **verified** | New: fsrs-rs reports that FSRS-6 and FSRS-7 are selected from the parameter count, so both are available in Rust. |
| 18 | Brunmair & Richter 2019: g = 0.42 overall, maths 0.34, words -0.39; 59 studies, 238 effects | Europe PMC abstract | **verified** | Abstract calls the maths effect "small". |
| 19 | Yang et al. 2021: g = 0.499, 222 studies, 48,478 students | Europe PMC abstract | **verified** | Classroom students. |
| 20 | Lindsey et al. 2014: +16.5% vs massed, +10.0% vs one-size spaced | Crossref abstract | **verified** | Middle-school foreign-language course. |
| 21 | Cepeda et al. 2008: more than 1,350 people; optimal gap about 20-40% of a 1-week delay, 5-10% of a 1-year delay | Crossref abstract | **verified** | Fact recall. |
| 22 | Murray, Goebel, Horner 2025: maths spacing g 0.26 (25 studies, 49 effects); testing vs restudy g 0.22 with CI crossing zero (6 studies, 20 effects) | Crossref abstract | **verified** | Crossref type is posted-content, so a preprint. The abstract says the maths effect "may be smaller than in other domains". |
| 23 | Gervet et al. 2020, JEDM 12(3): logistic regression with good features leads on moderate datasets; 9 datasets | Zenodo abstract | **verified** | Abstract adds that BKT-style models underperformed. |
| 24 | Mettler, Massey, Kellman 2016: adaptive spacing beat fixed schedules on immediate and delayed tests | Europe PMC abstract | **verified** | Effect sizes not read. |
| 25 | Beck & Chang 2007, identifiability problem for student modelling | Crossref | **verified (title and authors)** | Content of the argument not re-read. |
| 26 | MolScribe 76-93% accuracy on public benchmarks | arXiv abstract | **verified** | It is a local model. See conflict C5. |
| 27 | Latimier et al. 2021 meta-analysis of spacing retrieval | Crossref | **partial** | Exists; effect sizes unread, so none are quoted here. |
| 28 | Azure Read $1.50 per 1k; formula add-on and Google Document AI prices | not re-checked | **unverified** | Left as unknowns; both are candidates for the bake-off only. |

No claim that a recommendation depends on failed verification. The only failure (row 4) removes an optimistic cost figure, not a design decision.

## 3. Recommendations, ranked by evidence strength times impact

Tags: prompt, scheduling, question generation, feature, UI, measurement, corpus. "Evidence" is how well the source supports the rule; "impact" is how much it changes Tutor.

1. **[prompt] State a formula or quantitative claim only if it sits in a cited span.** Otherwise label it as reasoning beyond the source. On a retrieval miss say "not found in the library" instead of filling from memory. Say when a formula came from OCR and offer the page image. Evidence: strong for the mechanism (Anthropic docs), moderate for the need (ALCE). Impact: highest, because it is the honesty guarantee.
2. **[corpus] Add a deterministic span check after every cited answer.** Cited text must be a normalized substring of a retrieved chunk, and every number and equation in the answer must appear in cited text. Citations only guarantee a valid pointer, not support. Evidence: strong docs, moderate ALCE.
3. **[corpus, measurement] Before any bulk spend, run a library inventory and build a hand-verified gold set of about 150-200 pages.** Classify pages as text-layer or scan, score math density, and count real pages per GB. Every cost figure scales with pages per GB (assumed 10,000, about 3x uncertain either way). Evidence: the need is strong because no independent math benchmark exists for Mathpix, Azure or Google.
4. **[corpus] Tier the parse.** Keep pdfium for clean prose. Send math-dense and scanned pages to a hosted parser, and run a bake-off on the gold set between at least olmOCR 2 (hosted, through a provider whose price I confirm), Mathpix batch, Mistral OCR 4.1 and Datalab. Escalate any page whose extracted LaTeX fails KaTeX rendering to a second engine. Evidence: moderate; the claim that pdfium garbles equations is X1's untested judgement.
5. **[scheduling] Atoms use FSRS-6 with published defaults and desired retention 0.90; never above 0.95.** Use the Rust `fsrs` crate (BSD-3) or py-fsrs (MIT).
6. **[measurement] Mastery for procedures and proofs is a band, not a number.** Keep a time-decayed Beta posterior per skill and a sequential probability ratio test. Assisted attempts never update it. "Mastered" needs roughly 15-20 independent unassisted attempts over several days. Evidence: moderate, rests on Gervet 2020, Beck & Chang 2007 and the author's simulations [sim]. Impact: high.
7. **[question generation] Generate from cited spans and store the evidence.** Each item stores the span, page, parse engine and version, `family_id`, the varied dimension and the invariant. Reject an item if its answer key does not validate against the cited text, and compute numeric keys independently. Citations cannot be combined with structured outputs (HTTP 400), so use a citation-enabled call for grounded text, then a separate schema-constrained extraction call, then the span check.
8. **[scheduling] Variant families are testlets.** Schedule the family as one FSRS card whose review is a fresh unassisted variant; serve at most one fresh variant per family per day; never call variants of one seed independent practice. The FSRS-for-families approximation is unvalidated; monitor calibration.
9. **[corpus] Hybrid retrieval with no default reranker.** tantivy BM25 with a tokenizer that keeps LaTeX commands, hosted dense vectors chosen by bake-off (voyage-4, voyage-3-large, Gemini Embedding 2, text-embedding-3-large), reciprocal rank fusion. Add a reranker only if it wins on the owner's queries. Evidence: moderate and narrow (see C1).
10. **[corpus] Structure-aware chunking.** Split by heading, definition, theorem, proof, example; never inside `$$...$$` or a LaTeX environment; keep parent-child links; keep the 400-token cap but expect fewer words. Add short contextual prefixes only for the current curriculum, where the cost is small.
11. **[UI] "Verify source" page-crop button, KaTeX everywhere, per-page "OCR suspect" flag.** Never show raw OCR LaTeX strings as final.
12. **[scheduling] Personalise FSRS weights only after about 1,000 unassisted graded reviews**, at most monthly, and accept the fitted set only if a chronological holdout beats the defaults by a margin fixed beforehand. Store both parameter sets. The 1,000 figure is X2's judgement; the Anki manual only says that "less than a few hundred" reviews is a common cause of poor fit.
13. **[scheduling] Daily queue:** due atoms, then due families, about 30% mixed-family problems with confusables adjacent, cumulative review of earlier units, 90-minute soft cap. Evidence: interleaving g 0.34 in maths, small.
14. **[scheduling] Prerequisite edges are advisory**, labelled author, LLM or data, used to route the entry diagnostic and suggest review, never to block. Flag edges the data contradict.
15. **[UI, measurement] Show binned predicted-vs-observed recall and a retention-workload table.** [sim] Reviews per card in year 1 at FSRS-6 defaults: 5.5 (R 0.80), 6.7 (0.85), 7.9 (0.90), 13.2 (0.95), 18.3 (0.97). (Assumes FSRS is the true model: workload, not validity.) Headline outcome stays delayed unassisted parallel items.
16. **[measurement] Retrieval evaluation.** nDCG@10 and recall@20 on about 200 queries split into semantic, formula and premise types, using the existing `--split train|test --seed N` discipline and paired bootstrap. Add citation precision and recall in the ALCE sense.
17. **[measurement] At most two or three personal experiments**, each with a preregistered minimum detectable effect, randomised over items or days, never periods. [sim] A retention-setting contrast (0.85 vs 0.92) needs about 320 reviews per arm; a logit-0.8 method effect needs about 160-320 items per arm; effects under about 6 points are unresolvable. Label small-effect tests underpowered and report intervals.
18. **[corpus] Provenance.** Store raw engine output, engine and version, the page image and original PDF, so pages can be re-parsed selectively and cards flagged when a source changes.

## 4. Conflicts and resolutions

- **C1. Rerankers: MIRB says they hurt; Anthropic says they help (67% fewer failures).** Different data (math benchmark with two older general rerankers vs mixed, mostly non-math data) and different rerankers. Resolution: default off, test a modern reranker on the gold set. Do not cite either number as settled for Tutor.
- **C2. FSRS-6 vs FSRS-7.** X2 recommends FSRS-6 and lists FSRS-7 as unchecked. I confirmed FSRS-7 beats FSRS-6 on the benchmark (0.3401, or 0.3370 for the recency variant), and that fsrs-rs supports both. But the wiki page I read documents only FSRS-6, and its 21 parameters have published defaults. Resolution: start with FSRS-6 and revisit FSRS-7 when its algorithm page and default parameters are documented; log enough review data (`fsrs_log`) that switching needs no data loss.
- **C3. olmOCR-Bench ranking.** X1's scores match the README exactly. The one thing to keep in mind is that Chandra's overall score (83.1) is above olmOCR's (82.4) and Marker's ArXiv math (83.8) is above both; olmOCR's case rests on price and open weights, not on leading the table.
- **C4. Schema gap between the two reports.** X2's SQLite schema has `item.chunk_id` but nothing for the cited span, page or parse engine that X1 says each item and card must carry. Resolution: add `source_span`, `page`, `parse_id` (engine and version) and `span_verified` to `item`, and a `parse` table. Keep X2's append-only `attempt` log unchanged.
- **C5. "No local models" vs on-demand MolScribe.** MolScribe and DECIMER are local neural models, and the GPU is unavailable. Resolution: defer chemistry-structure recognition. If needed later, try a cloud vision model on the crop plus an RDKit round-trip check, or confirm Mathpix's SMILES output. Do not bulk-parse.
- **C6. Cost range.** X1's "OCR everything" range ($185 to $5,000 at 50 GB) mixes a verified price list with an unverified olmOCR estimate and an assumed page density. After dropping the olmOCR figure, the verified floor is Mathpix batch at $750 per 500k pages, and the tiered plan ($150-400 at 50 GB) is an estimate, not a quote. Treat all of it as a planning range until item 3 is done.
- **C7. Copyright and privacy.** The project rules forbid vendoring copyrighted books into the corpus; parsed text of purchased books is derived local data and must stay out of git. Sending purchased books to a hosted parser is a separate owner decision. Mathpix retains data 30 days unless you opt out (24 hours); retention for Azure, Google, Datalab and hosted olmOCR is unverified. Check terms before any bulk upload.
- **C8. Power numbers** assume independent items with item-difficulty SD 1.0 logit; generated variants may differ. Treat them as optimistic until the owner's variance is measured. Other open unknowns: Mathpix/Azure/Google math accuracy on the owner's pages, modern reranker and voyage-4 behaviour on math, FSRS calibration for family cards and for foreign-language vocabulary.

## 5. What not to build

- Whole-library OCR with the priciest engine, or any bulk OCR before the inventory and bake-off.
- Self-hosted OCR, embedding or reranking models.
- Bulk chemical-structure extraction.
- A default cross-encoder reranker.
- Reliance on any engine's confidence score (Azure's is hard-coded) or on Mistral's March 2025 94% marketing figure. Third-party olmOCR-Bench puts that Mistral model at 72.0 overall.
- Deep knowledge tracing, transformer or RWKV schedulers, per-skill BKT fitting at n=1.
- "Three correct in a row = mastered" (passes a 0.7-accuracy learner about 34% of the time [sim]).
- Hand-tuned FSRS weights, and a retention dial above 0.95 (X2 forbids only above 0.97; I cap it lower because workload nearly doubles from 0.90 to 0.95 [sim]).
- Hard prerequisite gating, and any efficiency claim for it.
- Crediting assisted attempts or same-session variants as independent evidence.
- Marketing copy citing "20-30% fewer reviews than SM-2" (community-wiki simulation), or claiming FSRS improves learning.
- Several simultaneous experiments on overlapping skills, or p-value verdicts on effects under about 0.3 logit.

## 6. Citation list with verification status

Verified unless stated.
- Poznanski, Soldaini, Lo 2025, olmOCR 2, arXiv 2510.19817; github.com/allenai/olmocr README (benchmark table, licence). Verified.
- Ju & Dong 2025, MIRB, arXiv 2505.15585 (Tables 9 and 10 read from PDF text). Verified.
- Gao, Yen, Yu, Chen 2023, ALCE, EMNLP 2023, arXiv 2305.14627. Verified.
- Anthropic, Citations documentation, platform.claude.com/docs/en/build-with-claude/citations. Verified.
- Anthropic 2024, Contextual Retrieval, anthropic.com/news/contextual-retrieval. Verified.
- mathpix.com/pricing/api; mistral.ai/pricing/api; datalab.to/pricing; docs.voyageai.com/docs/pricing. Verified (details in section 2).
- open-spaced-repetition/srs-benchmark README; awesome-fsrs wiki "The Algorithm"; fsrs-rs; py-fsrs. Verified (FSRS-7 figure corrected).
- Brunmair & Richter 2019, Psych Bull 145(11), 10.1037/bul0000209. Verified.
- Yang, Luo, Vadillo, Yu, Shanks 2021, Psych Bull 147(4), 10.1037/bul0000309. Verified.
- Lindsey, Shroyer, Pashler, Mozer 2014, Psych Sci, 10.1177/0956797613504302. Verified.
- Cepeda, Vul, Rohrer, Wixted, Pashler 2008, Psych Sci, 10.1111/j.1467-9280.2008.02209.x. Verified.
- Murray, Goebel, Horner 2025, OSF preprint, 10.31219/osf.io/wdahf_v1. Verified; preprint.
- Gervet, Koedinger, Schneider, Mitchell 2020, JEDM 12(3), 31-54. Verified.
- Mettler, Massey, Kellman 2016, JEP: General, 10.1037/xge0000170. Verified (direction only).
- Beck & Chang 2007, "Identifiability: A Fundamental Problem of Student Modeling", UM 2007. Verified (title and authors).
- Qian et al. 2023, MolScribe, arXiv 2205.14311. Verified.
- Latimier, Peyre, Ramus 2021, Educ Psych Rev. Partial (existence only).
- Not re-checked this round (quote none of their numbers without opening them): Khajah et al. 2016; Pavlik et al. 2009; Pelánek 2016 and 2017; Choffin et al. 2019; Pelánek & Řihák 2017; Zhang et al. 2025; Wainer et al. 2007; Doignon & Falmagne 1999; Kravitz & Duan 2022; Ye et al. 2022; Anki manual; Azure, Google and Gemini pricing.
