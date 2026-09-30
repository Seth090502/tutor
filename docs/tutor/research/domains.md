# SYNTHESIS: Discipline-based pedagogy for advanced math, English, science and chemistry

Scope: reports E1 (math), E2 (English), E3 (science), E4 (chemistry), E5 (concept inventories and misconceptions). Verified 2026-09-30 by looking up abstracts and records on Crossref, Europe PMC, ERIC, Semantic Scholar and arXiv. No full texts were read, so "verified" means the abstract or record says what the report claims.

## 1. Bottom line

1. All five reports converge on one design principle: the tutor helps only after the learner attempts, and success is judged by delayed, unaided performance on fresh problems. The strongest direct evidence is Bastani et al. 2025 (unguarded GPT-4 lowered later grades 17%; a guardrailed tutor largely removed the harm) and Kestin et al. 2025 (a pedagogically built AI tutor beat in-class active learning).
2. Nearly all the evidence is school-age or introductory undergraduate. For PhD-level adult self-study, no trial in any of the four subjects was found. Every rule below is an inference from mechanisms plus intro-level data, and Tutor should test each rule on the owner's own delayed-retention data (single-learner alternating comparisons).
3. FSRS at 0.9 is well supported for discrete facts (Yang 2021, Latimier 2021) and is not a measure of skill. For problem solving, schedule by skill and serve a new variant each time.
4. Current LLMs are unsafe as sole authority for proofs, calculations and chemical mechanisms. Published failure numbers are for 2023-2025 models, so Tutor needs its own evals on Opus 5.5 and Muse Spark 1.3 before relaxing any verifier rule.

## 2. Verified evidence table

Status: V = abstract or record read this session and figure matches; C = corrected; U = exists but figure not checked here (taken from the report).

| # | Claim | Source | Checked figure | Status |
|---|---|---|---|---|
| 1 | Unguarded GPT-4 harms later unaided grades; guardrails mitigate | Bastani et al. 2025, PNAS 122(26), 10.1073/pnas.2422633122 | About 1,000 high-school students; "17% reduction in grades for GPT Base"; "largely mitigated" in GPT Tutor | V. Caveat: a PNAS erratum exists (10.1073/pnas.2518204122, Aug 2025); its content could not be read. Treat the 17% as provisional |
| 2 | AI tutor beats in-class active learning | Kestin et al. 2025, Sci Rep 15, 10.1038/s41598-025-97652-6 | Crossover RCT, N = 194 of 233; 0.63 SD linear, 0.73-1.3 SD quantile; AI median 49 min vs about 60 min class | V. Limits: first-exposure content, two lessons, no delayed test, authors caution about higher-order synthesis |
| 3 | Active learning beats lecture in STEM | Freeman et al. 2014, PNAS, 10.1073/pnas.1319030111 | 0.47 SD on exams and concept inventories (158 studies); failure odds ratio 1.95 (67 studies); 225 studies total | C. E1 attached 0.47 to "225 studies"; correct is 158 |
| 4 | Active learning feels worse but works | Deslauriers et al. 2019, PNAS, 10.1073/pnas.1821936116 | Active students learned more but perceived less learning (direction only in abstract) | V |
| 5 | Interleaving improves math delayed tests | Brunmair and Richter 2019, Psych Bull, 10.1037/bul0000209 | 59 studies, 238 effects; g = 0.42 overall, g = 0.34 math; blocking better for word-based material | V |
| 6 | Interleaving RCT in math | Rohrer et al. 2020, J Educ Psych 112(1), 10.1037/edu0000367 | 54 seventh-grade classes; 61% vs 38%, d = 0.83, unannounced test 1 month later | V |
| 7 | Interleaved physics homework | Samani and Pan 2021, npj Sci Learn, 10.1038/s41539-021-00110-x | Median +50% and +125% on two surprise tests; students rated it harder and thought they learned less | V. One course, 8 weeks |
| 8 | Self-explanation helps proof comprehension | Hodds, Alcock and Inglis 2014, JRME, 10.5951/jresematheduc.45.1.0062 | Three experiments; Exp 1 d = 0.95 on comprehension; Exp 3 a 15-minute intervention with lasting benefit | V. New figure: d = 0.95, small samples, inflated-effect risk |
| 9 | Self-explanation prompts in general | Bisra et al. 2018, Educ Psych Rev, 10.1007/s10648-018-9434-x | 64 reports, 69 effects, g = 0.55 | V (was recalled [R] in E1) |
| 10 | Productive failure | Sinha and Kapur 2021, Rev Educ Res, 10.3102/00346543211019105 | 53 studies, 166 comparisons, g = 0.36 [0.20, 0.51]; 0.37-0.58 at high fidelity; 0.87 bias-adjusted; grades 2-5 better with instruction first | V. Headline 0.36 only |
| 11 | Retrieval (quizzing) in classrooms | Yang et al. 2021, Psych Bull, 10.1037/bul0000309 | g = 0.499, 222 studies, 48,478 students | V |
| 12 | Spaced retrieval beats massed | Latimier, Peyre and Ramus 2021, Educ Psych Rev | 29 studies; spaced vs massed g = 0.74; expanding vs uniform g = 0.034, not significant | V |
| 13 | Feedback effectiveness depends on information content | Wisniewski, Zierer and Hattie 2020, Front Psychol, 10.3389/fpsyg.2019.03087 | d = 0.48, 435 studies, 994 effects, I2 = 83.4%; about 17% negative; high-information 0.99; corrective 0.46; reward/punishment 0.24 | V |
| 14 | Automated writing feedback | Fleckenstein, Liebenow and Meyer 2023, Front AI, 10.3389/frai.2023.1162454 | 20 studies, k = 84, N = 2,828; g = 0.55; heterogeneous | V. CI not rechecked |
| 15 | Writing instruction, adolescents | Graham and Perin 2007, J Educ Psych, 10.1037/0022-0663.99.3.445 | 123 documents, 154 effects; strategy 0.82, summarization 0.82, grammar -0.32 | V |
| 16 | Writing instruction, grades 6-12, update | Graham et al. 2025, Reading and Writing | 148 comparisons, 22,838 students; overall 0.38; process 0.75; strategy 0.59; transcription 0.54; SRSD 0.84 | V |
| 17 | Refutation text | Schroeder and Kucera 2022, Educ Psych Rev, 10.1007/s10648-021-09656-z | 44 experiments, g = 0.41 | V |
| 18 | Spatial training is durable and transfers to other spatial tasks | Uttal et al. 2013, Psych Bull, 10.1037/a0028446 | 217 studies, g = 0.47; stable over delays; transfer to untrained spatial tasks | V. Transfer to chemistry or geology not shown |
| 19 | LLMs fail at olympiad proofs (time-sensitive) | Petrov et al. 2025, arXiv 2503.21934 | USAMO 2025: best 25% (Gemini 2.5 Pro), all others under 5%; revised Sep 2025 | V. March 2025 models |
| 20 | Calculus Concept Inventory is weak | Gleason et al. 2019, 10.1080/0020739X.2018.1538466 | About 1,800 students, four institutions; "lacks sufficiently strong validity and reliability for its intended use" | V |
| 21 | Testing effect (retrieval vs restudy) | Rowland 2014, Psych Bull, 10.1037/a0037559 | Paper exists; abstract carries no effect size; g of about 0.50 is recalled | U |

Not checked (taken from reports as stated): Kalyuga expertise reversal, Rau 2018, Stieff, Padalkar and Hegarty, Cooper 2012, Houchlei 2021, Danielson 2025, Hake 1998, Madsen 2015, Kortemeyer 2023, Steiss 2024, Thoma and Iannone 2021, Potvin, Sadler, Traxler, Nissen, Melby-Lervag, Bego 2024, ChemBench, VanLehn 2011.

Failed citations: none. Corrections: Freeman study count (row 3); the Bastani erratum flag (row 1). Two items remain unverified and are flagged wherever used: Rowland g and every figure marked U above.

## 3. Rules for Tutor, ranked by evidence strength times impact

Tags: [P] voice/chat tutor prompt, [S] scheduling, [Q] question generation, [F] feature, [UI] interface, [M] measurement, [C] corpus.

1. **Attempt first, hint ladder, no answer dumping.** Order: clarifying question, goal or strategy cue, next-step hint, worked step, full solution only after an attempt or an explicit "show me" that is logged and costed. [P][UI][M] Evidence: Bastani (moderate, erratum caveat), Kestin. Applies to all four subjects. Impact: highest.
2. **Success metric is delayed (7+ days), unaided accuracy on new variants; log assisted and unassisted separately; never use satisfaction, streaks or XP as quality signals.** [M][UI] Evidence: Deslauriers 2019, Samani and Pan 2021, Bastani. Strong on direction.
3. **Retrieval and spacing for discrete items via FSRS at 0.9:** definitions, theorem statements, formulas, reaction facts, vocabulary, rhetorical terms. Production or cued recall, not recognition. [S][Q] Evidence: Yang 0.499, Latimier 0.74 (strong). Expanding versus uniform spacing does not matter. Domain caveat: chemistry spaced-retrieval gains were significant in only 2 of 9 courses (Bego 2024, unchecked), so measure locally.
4. **Predict then reveal with a 1-5 confidence rating; show mismatches; track calibration.** [P][UI][M] Evidence: active learning 0.47 SD (Freeman), hypercorrection (Metcalfe, pattern only). Medium-strong.
5. **Self-explanation aimed at logical or causal links** before the answer: why is this proof line valid, which electron source and sink, why does this paragraph follow. Reject paraphrase. [P] Evidence: Bisra g = 0.55; Hodds d = 0.95 (small). Chemistry-specific evidence is mixed (weak). Moderate.
6. **High-information, criterion-referenced feedback, 2-3 priorities, global issues first**, never praise-only. [P] Evidence: high-information 0.99 versus 0.24, but feedback overall is heterogeneous with 17% negative effects. Moderate.
7. **Interleave confusable categories after the first blocked exposure; always use fresh variants; schedule problem skills by skill or problem family, not by item.** [S][Q] Evidence: g = 0.34 math, d = 0.83 RCT, physics homework. Moderate. Do not interleave vocabulary definitions (blocking was better for word material in Brunmair). Unknown for graduate proof-technique selection.
8. **Verify with deterministic tools, never with the LLM alone:** CAS/Python for numbers and symbols, RDKit-style checks for atom balance, charge and arrow validity, optional Lean for proofs; the tutor states "I cannot verify this step" instead of bluffing. [F][P] Evidence: Petrov, Kortemeyer, ChemBench and Watts (unchecked). Direction moderate, figures dated; impact high.
9. **Misconception catalogue and refutation-form correction.** Name the wrong idea, why it is tempting, why it fails, end on the correct model; say "consistent with misconception X", never "validated". [P][Q][C][UI] Evidence: g = 0.41 (text-reading studies only; not shown for voice or PhD level). Moderate.
10. **Fade support by prior knowledge.** Worked example plus self-explanation for new topics; problem-first for familiar ones; productive-failure opener (5-10 minutes, no help) only when the learner has prerequisites. [P] Evidence: expertise reversal and Rau (unchecked), productive failure g = 0.36 (contested). Moderate-weak.
11. **English: never write the learner's prose.** Require a draft and revision before any model text; exemplars only after a draft; contextual error correction logged to a recurring-errors list; no isolated grammar drills. [P][F] Evidence: grammar -0.32, process 0.75, strategy 0.59 (school-age), Bastani by analogy. Moderate for adults.
12. **Writing is deliberate practice, not FSRS:** fixed weekly drafts, required revision, and a monthly timed AI-off sample scored on a fixed rubric, with revision uptake logged. [S][M][F] Evidence: theory (Kellogg), Farrokhnia (quality of feedback did not improve revisions). Weak-moderate.
13. **Proof workspace:** state goal and proof type first, then line-by-line steps with justifications; proof-critique items with one flawed step; Library proofs become prove-first or fill-the-gap tasks. [F][Q] Evidence: Hodds, Selden and Selden, Weber (qualitative). Moderate-weak. Lean is opt-in (weak evidence).
14. **Chemistry structure and curved-arrow editor** with deterministic checks; withhold the product on early attempts; commit-then-reveal 3D viewer. [F][P] Evidence: quasi-experimental and qualitative; no test of an LLM-plus-tool tutor. Reasoned, weak-moderate, high impact because LLMs fail here.
15. **Question-bank schema:** skill, misconception_id, representation, variant_family, verified solution, a transfer context; two-tier answer-plus-reason for conceptual items; parameterised variants computed in code; 3-4 options; at least one catalogue-mapped distractor; independent solve check; unmapped distractors labelled "generic". [Q] Evidence: LLM distractors diverge from real student choices (Feng 2024, unchecked). Moderate.
16. **Optional spatial drills**, tracked separately. [F] Evidence: g = 0.47 on spatial tasks; chemistry transfer unproven. Low-priority.
17. **Corpus and licensing.** Catalogue misconceptions as concepts with citations and own-words text; write original items; do not ingest FCI, CCI, QMCS or FMCE item text or scrape PhysPort. Record licence per source. [C] Basis: PhysPort access terms and 17 USC 102(b) as reported by E5, not legal advice.

## 4. Conflicts and how they were resolved

1. **Freeman study count** (E1 says 0.47 SD over 225 studies; E3 and E5 say 158). Verified: 0.47 SD comes from 158 exam studies; 225 is the total including failure-rate studies. Use 158.
2. **Interleaving everywhere (E1, E3) versus null or reversed effects for words and expository text (E2, from Brunmair).** Both true. Benefit scales with similarity between categories and with complexity. Interleave confusable problem families in math, physics and chemistry; block vocabulary and definitions.
3. **Attempt-first (productive failure) versus worked examples for novices (E1, E3, E4).** Resolved by prior knowledge: novices on a topic get worked examples plus self-explanation; learners with prerequisites get a capped problem-first opener. Productive failure is g = 0.36, contested, adult samples were a minority, and the 0.87 bias-adjusted figure (verified in the abstract, unusual direction) is not used as evidence.
4. **FSRS scope.** E1 and E3 say FSRS only for discrete items; E4 says one FSRS card per skill. Compatible: a card may represent a skill node, but what is shown on review must be a fresh variant, never the identical problem. Strong general retrieval evidence (Yang, Latimier) outweighs the weak chemistry-specific finding (Bego, 2 of 9 courses), but it should be measured on the owner's data.
4b. **Retest timing.** E5 proposes a retest at about one day after a high-confidence wrong answer; E1 and E3 propose delayed tests at 7 days and 4 weeks. Resolved: the 1-day retest is remediation (a design hypothesis, flagged experimental); the 7-day and 4-week tests are the measurement.
5. **Hint ladder thresholds.** E1 allows a logged "show me"; E3 requires two attempts before a full solution. Resolved: default two attempts, with a logged escape.
6. **Spatial training strength.** E3 calls it strong; E4 calls it moderate for chemistry. Both fit: strong for spatial tasks (verified g = 0.47, durable, transfers to other spatial tasks), unproven for chemistry or geology outcomes. Keep it optional. Note g = 0.47 coincides with Freeman's 0.47 but measures a different thing.
7. **Feedback quality versus learning (E2).** AI feedback can rate as good as teachers' yet not improve revisions (Farrokhnia), while aggregate AWE g = 0.55 (verified) is heterogeneous and likely inflated by short, researcher-made outcomes. Stronger evidence: high-information feedback (0.99) plus the revision gate; do not rely on feedback quality ratings as the metric.
8. **Intro-level inventories versus a PhD-track learner (E5 versus E3).** Inventories are group-level, introductory and ceiling-limited; they serve as floor checks only. Use upper-division instruments' misconceptions as concepts, and never use an FCI score as the learner's level.
9. **Dated LLM-failure evidence.** Petrov (March 2025 models), Kortemeyer (2023) and the JCE chemistry studies (GPT-3.5/4 era) are out of date for Opus 5.5 and Muse Spark 1.3. The stable finding is overconfidence and weak electron-movement reasoning. Keep the verifier rules until Tutor's own evals show low error rates.

## 5. What not to build

- A default answer-giving chat, a "rewrite my essay" button, or an LLM that writes the learner's prose (Bastani; Fan 2025 reported in E2).
- A Lean-first curriculum (weak, self-selected evidence), or LLM proofs, arrow diagrams or stereochemistry presented as authoritative.
- FSRS on whole multi-step problems with repeated identical items.
- Streaks, XP, satisfaction or Hake normalized-gain dashboards (Nissen 2018, unchecked; gain is biased toward strong pretests).
- Learning-styles personalization, a generic critical-thinking module, empathy-from-fiction claims, or EEG "cognitive debt" messaging (n = 54 preprint).
- An essay score shown to the learner as a grade (AWE g ranges 0.43-0.86); keep scores internal for trend tracking.
- Isolated grammar drills (-0.32).
- A Johnstone-triangle mode (no intervention evidence); use it as a prompt pattern only.
- Copies of FCI, CCI, QMCS or FMCE items, a PhysPort scraper, an English multiple-choice concept inventory with invented authority, an LLM-only distractor generator, single-answer automatic misconception diagnosis.
- A large VR module, an unguided simulation sandbox, simulated peers, or Dual N-Back claims for subject learning (no far transfer, Melby-Lervag 2016, unchecked); keep N-Back optional and separate.

## 6. Corpus and ingestion notes

- Seed the misconception catalogue from published literature (concept plus citation, own words). Otero 2024 (arXiv 2412.03765) math-misconception benchmark is reported CC BY-NC-SA 4.0, unchecked here.
- Licences to confirm before ingesting full text: CBE-LSE articles, J Chem Educ, Project 2061, the Eedi dataset, PhET terms. All unverified.
- The owner's question about open Chinese institutional materials was outside E1-E5 and is not answered here. No licences were checked. It needs its own research pass before anything is added to `corpus/manifest.json`.

## 7. Open questions

1. No evidence at PhD level or for adult solo learners in any of the four subjects; run single-learner alternating comparisons (for example predictions on versus off, by topic) on delayed retention.
2. Does refutation correction work in spoken dialogue and with advanced adults? Only text-reading studies found.
3. Does a verifier-backed LLM tutor reproduce human-feedback gains in chemistry? No study.
4. Current Opus 5.5 and Muse Spark 1.3 error rates on proof grading, mechanisms and numerics: needs Tutor's own held-out evals.
5. What did the Bastani erratum change? Read it before quoting the 17% in any product copy.
6. Is the owner a native English speaker? It decides how much EFL corrective-feedback evidence applies.
7. Coverage gaps: biology, earth science, upper-division and graduate physics, physical and quantum chemistry, East Asian curricula, Polya and Schoenfeld (books unopened), bianshi variation teaching.
8. Verify before quoting: Rowland g, VanLehn 2011 (human tutoring about 0.79, ITS about 0.76), Alfieri 2011, Rau, Danielson 2025 pooled effect.

## 8. Citation list with verification status

V = verified this session (abstract or record read). U = exists per report, not checked by me. Full DOIs are in section 2 for V items.

V: Bastani 2025 (plus erratum flag); Kestin 2025; Freeman 2014 (corrected); Deslauriers 2019; Brunmair and Richter 2019; Rohrer 2020; Samani and Pan 2021; Hodds 2014; Bisra 2018; Sinha and Kapur 2021; Yang 2021; Latimier 2021; Wisniewski 2020; Fleckenstein 2023; Graham and Perin 2007; Graham 2025; Schroeder and Kucera 2022; Uttal 2013; Petrov 2025; Gleason 2019.

U (report-only, not checked by me): Rowland 2014 (figure recalled), plus every source in the "Not checked" list in section 2 and all other works cited only in E1-E5 (for example Dunlosky 2013, Kellogg 2007/2009, Steiss 2024, Farrokhnia 2026, Fan 2025, Kosmyna, Hake 1998, Chernikova 2020, Lazonder and Harmsen 2016, Mirza 2025, Watts 2023, Otero 2024). Consult the source reports for their full details.
