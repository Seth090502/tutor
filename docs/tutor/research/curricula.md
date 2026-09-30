# SYNTHESIS: PhD-level curricula and prerequisite maps

Date: 2026-09-30. Inputs: P1-math, P2-physics, P3-biology-earth, P4-chemistry, P5-english. Verifier: Sonnet 5.5. Method: 26 sources re-opened by direct fetch (WebSearch was out of budget). The fetch tool returns a model summary of each page, so figures are as the fetched page states them.

## 1. Bottom line

1. **What is solid.** The teaching method Tutor already uses (attempt-first, hint ladder, delayed unaided retesting, grounded answers) is supported by the best verified evidence in all five reports. The evidence is mostly school or classroom studies, so for one adult self-learner it supports design direction, not predicted effect sizes.
2. **What is not available.** No report could open a non-math PhD qualifying syllabus. Only math has verified official exam pages (Berkeley, Harvard; MIT as reported). Physics, chemistry, biology, earth science and English "PhD level" ladders are the workers' own syntheses. Tutor should ship them as editable, versioned data labelled "proposal", never as a standard.
3. **The biggest practical finding is about the corpus.** OpenStax (the whole library, verified on the Chemistry 2e and Biology 2e prefaces) is CC BY-NC-SA 4.0 and forbids LLM ingestion without written permission. That removes OpenStax from every RAG plan in the reports. MIT OCW, by contrast, explicitly allows AI use with conditions.
4. **Two owner decisions block the corpus work** (Section 6): whether NC-SA material may enter the local index, and which qual target to use per domain.

## 2. Verified table

Status: V = verified as claimed; C = corrected (claim changed); U = could not verify, not relied on.

| # | Claim (source) | What the fetched page says | Status |
|---|---|---|---|
| 1 | Bastani et al. 2025, PNAS, doi 10.1073/pnas.2422633122 | "nearly a thousand high school math students"; GPT-4 access raised grades 48% (GPT Base) and 127% (GPT Tutor); after access removed, GPT Base 17% lower than never-had-access; GPT Tutor's safeguards "largely mitigated" the harm; "crutch" quote | V. Reports' "Turkey, one school" was not on the page |
| 2 | Freeman et al. 2014, PNAS | 225 studies; +0.47 SD on exams (158 studies); odds of failing under lecture 1.95 (67 studies); about 6% exam gain; stronger in classes of 50 or fewer | V. The "failure 32% to 21%" figure in P3 was not on the page; dropped |
| 3 | Kestin et al. 2025, Sci Rep, doi 10.1038/s41598-025-97652-6 | Abstract: learn "significantly more in less time", no numbers. Full text (Europe PMC): 194 of 233 students eligible; effect size 0.73 to 1.3 SD (quantile model), 0.63 (linear); AI median 49 min vs an **assumed** 60 min in class; first-exposure content, no retention test | C. P4 said no effect size: it is in the full text. The in-class time is assumed, not measured |
| 4 | Brunmair and Richter 2019, Psych Bull | Overall g = 0.42; math tasks g = 0.34 ("small"); words g = -0.39 (blocking better); paintings 0.67; expository texts ambiguous | V |
| 5 | Rohrer et al. 2020, J Educ Psychol | Preregistered cluster RCT, 54 grade-7 classes, 4 months, test one month later, 61% vs 38%, d = 0.83 | V |
| 6 | Berkeley math prelim | Two consecutive 3-hour mornings; 5 of 8 questions; 2 per area (real analysis/calculus, complex, linear algebra, abstract algebra); pass = "roughly the level of an A grade in an honors undergraduate course"; passing score "usually between 60% and 70%", about two-thirds pass, "not official policy" | V |
| 7 | Harvard math quals syllabus | "aim to test your ability to solve concrete problems by identifying and applying important theorems"; six areas with named books (Dummit-Foote, Serre, Fulton-Harris; Shafarevich, Harris; Ahlfors; Hatcher, Fulton, Spanier, Greenberg-Harper; Taubes, Lee, Kobayashi-Nomizu; Rudin, Stein-Shakarchi, Lieb-Loss, Evans, plus probability texts) | V. The Real Analysis area also lists PDE and probability texts |
| 8 | MIT OCW terms | CC BY-NC-SA 4.0; "OCW content may be used to train, develop, and improve artificial intelligence and machine learning models" provided attribution, non-commercial use, and ShareAlike on derivative models | V |
| 9 | OpenStax Chemistry 2e | CC BY-NC-SA; "may not be used in the training of large language models or otherwise be ingested into large language models or generative AI offerings without OpenStax's prior written permission" | V |
| 10 | OpenStax Biology 2e | Same licence and same LLM clause | V. Corrects P3 (CC BY from memory) and P2/P3 plans to ingest OpenStax. Astronomy 2e and University Physics not re-opened; assume the same |
| 11 | Graham and Perin 2007 | Grades 4-12; 123 documents, 154 effect sizes; strategy instruction 0.82, summarization 0.82, peer assistance 0.75, product goals 0.70, sentence combining 0.50, process writing 0.32, grammar instruction -0.32 | V |
| 12 | Walters and Wilder 2023, Sci Rep | 636 references, 42 topics; 55% of GPT-3.5 and 18% of GPT-4 citations fabricated; 43% and 24% of real citations had substantive errors | V. 2023 models, no retrieval grounding tested |
| 13 | Miller et al. 2019, Sci Adv | About 1 in 8 US physics PhD students 2000-2010; GPA consistently associated with completion; GRE Physics and Verbal not significant; GRE Quantitative significant in some models; completion differs under 10% between 10th and 90th percentile | V. Nuance: GRE-Q did predict weakly in two of four models |
| 14 | Sinha and Kapur 2021 (productive failure) | 53 studies, 166 comparisons; g = 0.36; 0.37-0.58 at high fidelity; bias-adjusted 0.87 as reported; younger learners favour instruction-first | V as reported; the 0.87 is unusual and is not used |
| 15 | PHYBench (arXiv 2504.16074) | Gemini 2.5 Pro 36.9% vs human experts 61.9% | V. Dated (2025); re-measure on the current model |
| 16 | DeepMind IMO 2025 | Advanced Gemini Deep Think: 35/42, five of six problems, officially graded and certified by IMO coordinators; natural-language end to end | V |
| 17 | Petrov et al. (arXiv 2503.21934) | Best model (Gemini 2.5 Pro) 25% on USAMO 2025; all others under 5% | V |
| 18 | Dodell-Feder and Tamir 2018 | 53 effect sizes from 14 studies; g = .15-.16; "small"; no publication-bias sign | V |
| 19 | Steiss et al. 2024 | 200 human vs 200 ChatGPT feedback comments on secondary essays; humans better on all categories other than criteria-based | V. P5's headline matches its body ("4 of 5") |
| 20 | Yang et al. 2021, Psych Bull | 222 studies, 48,478 students, g = 0.499 for classroom testing | V |
| 21 | Rahman and Lewis 2020 | 99 chemistry studies; trim-and-fill range 0.29 to 0.62; publication bias noted | V |
| 22 | Hodds, Alcock, Inglis (JRME) | Three experiments; a booklet of self-explanation training significantly improved proof comprehension; no effect size | V. Undergraduates, short-term |
| 23 | Lebl, Basic Analysis | v6.3, 15 May 2026; dual CC BY-NC-SA 4.0 and CC BY-SA 4.0 | V |
| 24 | Axler LADR 4e | "Creative Commons BY-NC license" | V |
| 25 | Judson AATA | GFDL | V |
| 26 | Libarkin and Anderson 2005 GCI | Title and year confirmed | V (existence only) |

**Not verified, not relied on:** the 2026 arXiv LLM-proof-grading preprints (P1; never opened); Bisra 2018 g about 0.55 and Barbieri 2023 g about 0.5 (no numbers in the abstracts available); Adesope 2017 g about 0.6; Deslauriers 2019 and Von Korff 2016 (abstract level only, from P2); ChemBench and MaCBench (P4 read the abstracts; not re-opened); RDKit/Ketcher/Open Babel licences (P4 via GitHub API; not re-opened); Latimier 2021 and the "2 of 9 courses" chemistry spacing note; Theobald 2020 figures; TCP and NACLO licences; Princeton's exam format (P1 only).

## 3. Design recommendations, ranked by evidence strength times impact

Evidence grade: A = verified, directly on point; B = verified but by analogy (school or classroom, other subject); C = expert synthesis or analogy only, log and test locally.

| Rank | Tag | Recommendation | Evidence | Why it matters |
|---|---|---|---|---|
| 1 | prompt | **Attempt first, then hint ladder; log every answer reveal and hint level.** No full solution before a written attempt. | B (rows 1, 3) | The only verified harm result: unguarded GPT cut later unaided grades 17%. Already Tutor's method; this confirms it and adds the logging. |
| 2 | prompt | **Ground every fact.** The tutor may not state a quote, date, constant, citation, pKa, mechanism outcome or stereochemical result unless it came from a chunk retrieved this turn or a checker. Otherwise say "I cannot verify this." Label claims textbook-consensus, active-research or contested. | A/B (rows 12, 15) | Fabricated citations 18-55% in 2023 models; best 2025 model 36.9% on hard physics. Models are better now, but unverified. |
| 3 | feature | **Verifier ladder instead of LLM grading.** Tier 1: deterministic (SymPy and Pint for numerics and derivations, RDKit for structures and mechanism steps, Lean optional for math). Tier 2: rubric with at least two independent LLM passes, disagreements flagged to the owner. Tier 3: never a single-pass LLM verdict. The tutor states which tier checked an answer. | B/C (rows 15, 17, 19) | LLM proof and mechanism judgement is the highest-risk failure. The specific 2026 grading preprints are unverified; the rule stands on verified evidence of LLM error and on caution. |
| 4 | measurement | **Delayed, unaided, novel-item retest is the mastery signal.** Retest at 7 and 30 days with fresh parallel items, hints off, plus a transfer set. Report per node: delayed accuracy, transfer accuracy, hint level used. Label results n=1. | B (rows 4, 5, 20) | Rohrer used a one-month delayed test; the four domain reports each proposed 7 days to 1 month. The thresholds are design choices, not validated cut scores. |
| 5 | scheduling | **FSRS only for atomic items** (definitions, theorem statements, "which theorem applies", IR/NMR ranges, paradigms, dates, figures of speech, proof-outline cards). Problem solving is scheduled as schemas with fresh variants. Do not schedule "prove X", "interpret this poem" or a mechanism as a fixed-answer card. | B (row 20) | Testing g = 0.499 in classrooms. Recognising a remembered answer is not skill. |
| 6 | scheduling | **Interleave confusable problem families after one blocked introduction** (SN1/SN2/E1/E2, which-theorem-applies, period or figure identification, ensemble choice). Do not interleave vocabulary or word pairs. | B (rows 4, 5) | Math g = 0.34 (plan for this, not 0.83); words g = -0.39. |
| 7 | question generation | **Computed answers, LLM only for wording and hints.** Parametrised numeric and symbolic families with CAS-computed keys; every generated item blind-solved by a second model family and machine-checked where possible; distractors from documented misconceptions; original items, not copies of licensed inventories (FCI, BEMA, GCA and similar). | B/C | Needed because of the LLM error evidence; concept-inventory reuse terms are unchecked. |
| 8 | corpus | **Licence-tiered manifest** (see Section 5). Record licence text, URL, edition and translator per file. | A (rows 8-10, 23-25) | Avoids an OpenStax terms violation and keeps NC/SA items out of any shareable bundle. |
| 9 | feature | **Prerequisite graph stored as data**: typed edges (hard or soft), per-node evidence state (never attempted, attempted with hints, passed unaided after delay, passed mixed-timed), cross-links so math nodes gate physics, chemistry and astro nodes. Stage opens on prerequisite mastery, with owner override. On a failed physical-chemistry or physics item, run a math-prerequisite diagnostic first. | C | Every report reached the same structure independently, but none is a validated dependency model. Keep it small and hand-edited. |
| 10 | prompt | **Self-explanation before confirmation**: the learner says why each proof inference or mechanism arrow is valid (source, sink, why) before the tutor confirms. | B (row 22) | Undergraduate proof-reading experiments; Bisra's effect size is unverified. |
| 11 | feature | **Mock-qual modes.** Math: Berkeley format (two 3-hour sittings, 5 of 8, four areas), then a Harvard-style theorem-selection set. Oral: voice tutor as MIT-style examiner. Other domains: timed closed-book mixed sets built from the department syllabi the owner chooses. | A for the Berkeley and Harvard format (rows 6, 7); C elsewhere | Only math has a verified exam format. |
| 12 | measurement | **Calibrate any LLM rubric grader against owner-graded work** and report agreement. Blind re-score old drafts to detect drift. | B (row 19) | Humans beat ChatGPT feedback on four of five criteria in a 2023-era study. |
| 13 | prompt | **Confidence rating before feedback; show delayed-retest results, not satisfaction.** | C (Deslauriers 2019 abstract-level, unverified) | Learners rate effortful practice as worse; cheap to implement. |
| 14 | UI | **Structure and curved-arrow editor (Ketcher) wired to the RDKit checker with commit-then-reveal**; Mol* only for biochemistry. | C | Highest-value chemistry build, but it is substantial work; schedule after the checker exists. |
| 15 | feature / UI | **English and linguistics:** reading ledger per field with coverage map against an owner-chosen template list; quote-check button that re-runs retrieval; draft, criterion feedback, owner revision, diff loop with all drafts stored; NACLO-style unseen-language data problems as the linguistics mastery test. | B/C (rows 11, 19) | Writing strategy instruction has school-level ES 0.5-0.8; adult transfer is inference. |
| 16 | prompt | **Predict-then-reveal** for mechanism questions in biology and earth science; faded worked examples as competence grows. | C | By analogy; log and compare with plain practice. |

## 4. Conflicts and resolutions

1. **OpenStax licence (P3 memory: CC BY; pages: NC-SA).** Resolved by fetch: CC BY-NC-SA 4.0 on Chemistry 2e and Biology 2e, both with an LLM-ingestion ban. P2 and P3 proposed ingesting OpenStax; that is withdrawn. A RAG pipeline sending OpenStax text to a cloud LLM is the prohibited use; human reading only until OpenStax grants written permission.
2. **OCW and the project's corpus rule.** AGENTS.md allows full text only for public domain, CC BY or the owner's notes. OCW is CC BY-NC-SA with an explicit AI-use clause (attribution, non-commercial, ShareAlike on derivative models). Resolution: the AGENTS.md rule is stricter than OCW's terms; ingesting OCW for private local use is consistent with OCW's terms but needs the owner to amend the rule.
3. **Interleaving effect size (d = 0.83 vs g = 0.34).** Different designs and populations; 0.83 is one school RCT, 0.34 is the math-specific meta-analytic estimate. Plan and report against 0.34 or lower for an adult learner.
4. **Kestin effect size (P2: 0.73-1.3 SD; P4: none).** Both true: abstract has none, full text has the range and 0.63 linear. The in-class comparison time is assumed. Design direction only.
5. **Mastery thresholds (P1: 60-70%; P2: 70% owner pick; P4: tunable).** Resolution: one owner-set threshold per domain, default 70% on delayed, unaided, novel items, recorded as a design choice. Berkeley's 60-70% is an observed pass band for an exam, not a mastery cut score.
6. **Retest intervals (P1: one month; P2: 1-2 weeks then longer; P3: 2-4 weeks; P4: 7 and 30 days).** Resolution: 7 and 30 days for every domain; matches the verified Rohrer design at the long end.
7. **Proof and answer grading (P1: two LLM passes; P4/P2: deterministic only; P5: calibrate).** Resolution: the verifier ladder in Section 3, rank 3.
8. **Qualifying-exam targets.** P2-P5 found none; unresolved until the owner names departments (Section 6).

## 5. Corpus licence tiers (per source, verified where marked)

| Tier | Rule | Sources |
|---|---|---|
| Ingest freely | Public domain (US, published 1930 or earlier; check edition and translator) and CC BY | Gutenberg via documented mirror, Wikisource (CC BY-SA: attribute, keep SA), Mathematics in Lean (CC BY 4.0, per P1), Earle Physical Geology 2e (CC BY 4.0, per P3), Müller Grammatical Theory (CC BY 4.0, per P5), Lebl under its CC BY-SA option (verified dual licence) |
| Private local index only, owner approval needed | CC BY-NC-SA or NC: attribution stored per chunk, never in a distributed bundle | MIT OCW (AI use verified allowed with conditions), Lebl under NC-SA, Axler LADR (CC BY-NC, verified), Judson (GFDL, verified; GFDL has its own copyleft terms), LibreTexts books chosen one at a time with the per-book licence logged (per P4, not re-verified) |
| Excluded from LLM pipeline | Explicit ban | OpenStax, whole library (verified on Chemistry 2e and Biology 2e) |
| Owner library, metadata and own notes only | In copyright | Hatcher (free download, copyright retained), Lee, Rudin, Ahlfors, Dummit-Foote, Jackson, Sakurai, Kardar, Alberts, Norton Theory and similar |
| Unconfirmed, do not ingest yet | Licence not stated | TCP texts, NACLO problems, Feynman Lectures, Khan Academy, IPCC, PhysPort items, concept-inventory items |

Pipeline note (P2, P4; untested): PDF extraction likely mangles equations and structures; test retrieval on equation-heavy queries and keep SMILES and LaTeX in owner notes.

## 6. Owner decisions needed

1. May CC BY-NC-SA and NC material (OCW, Lebl, Axler, LibreTexts books) live in the private local index? AGENTS.md currently says no.
2. Qual target per domain: math Berkeley-level (Tier 2) or Harvard-level (Tier 3); for physics, chemistry, biology, earth science and English, which departments' published syllabi or reading lists to use as templates.
3. Is Lean acceptable as an optional verifier for math proofs?
4. Will the owner request written permission from OpenStax (their form is on the book page) or drop OpenStax?
5. Is foreign-language reading (Old English, Latin, French) in scope for the English plan?

## 7. What not to build

- A "PhD in N months" pace, a qualifying-exam pass probability, a "PhD-ready" certificate, or a GRE-style score proxy. No validated data exists; GRE Physics did not predict completion (Miller 2019).
- An LLM-only grader for proofs, mechanisms, stereochemistry, spectra or derivations; a single-pass grader of any kind without calibration and owner override.
- A solver that returns final answers first, or LLM-authored answer keys with no CAS, checker or human check.
- Any OpenStax ingester or pipeline that sends OpenStax text to an LLM; a LibreTexts crawler; scraping Gutenberg, Wikisource or NACLO.
- Redistribution of any NC/SA material in releases.
- A home-made concept inventory presented as validated, or copies of licensed inventory items.
- A large custom ontology or an LLM-built knowledge graph; hand-edit the small graph.
- A fixed "PhD reading list" presented as authoritative.
- Streaks, XP or badges as mastery evidence; learning-style personalisation; empathy or "reading makes you better" claims (g about 0.15); "Your Brain on ChatGPT" as evidence (n=54, not peer-reviewed as seen).
- Local reaction-prediction models, heavy ab initio compute, or linking Open Babel (GPL-2.0).
- VLM reading of drawn structures, spectra or figures as authoritative.

## 8. Prerequisite maps

Proposals only; full node lists are in the five reports. Math: five tiers tied to OCW 18.xx courses. Physics S0-S5 and chemistry C1-C7 block on math nodes first. Biology, earth and astro route through chemistry, statistics and the physics map. English uses a per-skill graph.

## 9. Citation list with status

V (verified this pass): every row marked V in Section 2, with DOIs as in the source reports, plus Sinha and Kapur 2021 (doi 10.3102/00346543211019105), Hodds et al. JRME 45(1) (doi 10.5951/jresematheduc.45.1.0062), Libarkin and Anderson 2005 (doi 10.5408/1089-9995-53.4.394).

- U (unverified, not relied on): the 2026 arXiv LLM-grading preprints; Bisra, Barbieri and Adesope effect sizes; Deslauriers 2019, Von Korff 2016, Theobald 2020; ChemBench and MaCBench findings; Latimier 2021; the chemistry and English education papers marked T in P4 and P5; Noy and Zhang 2023; Kosmyna 2025 (do not cite); Princeton and MIT exam formats; canonical textbook lists; tool licences (RDKit, Ketcher, Open Babel); PhysPort, TCP, NACLO, Khan, IPCC reuse terms.
