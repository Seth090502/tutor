# Subject-specific interactive tools and visual identity: synthesis

Verified 2026-09-30 by Sonnet 5.5 (high). Sources: reports G1 (math), G2 (science/chemistry), G3 (English and visual identity). I re-checked 24 papers and 8 library or licence facts against Crossref, OpenAlex, ERIC, Europe PMC, npm, PyPI and GitHub. I could not open some abstracts (publisher pages returned 403, and web search was exhausted). Those items are marked "unconfirmed" and nothing below rests on them alone.

## 1. Bottom line

1. The tool stack is settled and cheap: KaTeX, MathLive, function-plot, JSXGraph, SymPy sidecar, RDKit.js, Ketcher, SmilesDrawer, Harper, Open English WordNet. All render on CSS, SVG or Canvas2D (RDKit.js and SymPy run on the CPU) and work offline.
2. Interactive tools help only when wrapped in guidance: predict, run, explain, then retrieve. Unassisted exploration is worse than explicit instruction (verified, Alfieri 2011).
3. Per-subject looks are for orientation, not memory. The memory effect of context is small and did not replicate in the one replication we verified (Murre 2021, N = 16). Decoration inside learning material costs about -0.16 SD (Cheng 2026, verified). So: accent colour and icons on app chrome, neutral learning panes, and functional colour only where it marks structure (d = 0.38, verified).
4. No study found tests any of this on adult PhD-track learners. Everything is extrapolated from K-12 and undergraduates. The Evals screen should run the owner's own within-person comparisons.
5. The owner's hope that Chinese institutional teaching material is open source was not supported by G2 (no open-licensed Chinese simulation set found). This area did not test it further.

## 2. Verified evidence table

Status key: V = verified against an abstract or record I opened. C = corrected. U = unconfirmed (exists, figure not seen).

| Claim | Source | Verified figure | Status | Limit |
|---|---|---|---|---|
| Digital tools raise secondary maths/science learning; dynamic maths tools and ITS beat hypermedia | Hillmayr 2020 | g = 0.65, k = 92, grades 5-13; larger when added to instruction | V | Secondary school; short, researcher-made tests |
| Active learning beats lecture in STEM | Freeman 2014 | +0.47 SD (158 studies); failure odds ratio 1.95 (67 studies); 225 studies | V | Classroom active learning in general, not visualization |
| Unassisted discovery is worse than explicit instruction | Alfieri 2011 | d = -0.38 (580 comparisons); enhanced discovery d = 0.30 [0.23, 0.36] (360) | V | School-age mostly; the printed CI for the -0.38 figure is garbled in the source |
| Guidance in inquiry works | Lazonder & Harmsen 2016 | Learning outcomes d = 0.50 [0.37, 0.62], 72 studies | V | Age-group conclusions limited |
| Attempt before instruction (productive failure) | Sinha & Kapur 2021 | g = 0.36 [0.20, 0.51], 53 studies; 0.37-0.58 for true productive-failure designs | C | G2 omitted that grade level moderates: grades 2-5 and domain-general skills did better with instruction first |
| Simulation learning in higher education | Chernikova 2020 | g = 0.85 [0.69, 1.02], 145 studies; high-prior-knowledge learners gain more from reflection, low-prior from examples | V | Complex professional skills (medicine, teaching), not conceptual chemistry |
| Spatial training transfers | Uttal 2013 | g = 0.47 (SE 0.04), 217 studies; durable, transfers to untrained spatial tasks | V | Transfer to chemistry grades not shown |
| ITS effects (median) | Kulik & Fletcher 2016 | Median 0.66 SD, 50 evaluations; larger on locally built tests | V | Inflated by test alignment |
| ITS for K-12 maths | Steenbergen-Hu & Cooper 2013 | g = 0.01-0.09, 26 reports | V | Schools |
| ITS for college students | Steenbergen-Hu & Cooper 2014 | g = 0.32-0.37, 35 reports; earlier studies larger than recent | V | Closest to an adult learner; use 0.3-0.4 for planning |
| Step-based tutoring near human tutoring | VanLehn 2011 | The abstract I retrieved was garbled and swapped the two numbers. G1's figures (step-based about 0.76, human about 0.79) match the widely cited result; both about 0.76-0.79 | U (partly) | Direction unconfirmed by me; conclusion "about equal" is consistent either way |
| Dynamic geometry software | Chan & Leung 2014 | SMD 1.02 [0.56, 1.48], 9 quasi-experiments, n = 587 | V | Inflated; short interventions, younger pupils. Do not plan on it |
| Learning styles have no support | Pashler et al. 2008 | "Not adequate evidence base"; matching studies contradict the meshing hypothesis | V | Preferences exist but do not predict outcomes |
| Active learning feels worse than it is | Deslauriers 2019 | "Learn more, but feel like they learn less" (randomized, physics) | V | No numbers seen; single site |
| Seductive details harm learning | Cheng et al. 2026 | g = -0.16 overall; mechanism is extraneous load (via ERIC snippet) | V (partial) | 50 studies / 177 effects not seen |
| Same, earlier meta-analysis | Rey 2012 | 39 effects; small-to-medium retention, medium transfer harm | V | Lab lessons |
| Signalling helps | Alpizar, Adesope & Wong 2020 | d = 0.38; 44 effects from 29 studies, N = 2,726; larger in high-quality studies that control prior knowledge | V | Signals inside multimedia lessons, not app chrome |
| Emotional design helps (contested) | Wong & Adesope | 28 articles; g+ = 0.35 retention, 0.27 transfer, 0.29 comprehension | C | Published 2021 (online 2020) per ERIC; in-lesson graphics only |
| Context-dependent memory does not reliably replicate | Murre 2021 | N = 16 divers, no same-context advantage | V | The F and p values in G3 were not seen; not a clean refutation |
| Animation beats static graphics under conditions | Berney & Bétrancourt 2016 | Paper exists (Comput Educ 101:150-167). g = 0.226, 0.883 (no text) and 0.336 (audio) not seen | U | Directionally supported by Höffler & Leutner 2007 (d = 0.37, 26 studies, via an ERIC snippet) |
| Expertise reversal | Kalyuga et al. 2003 | Exists (Crossref, Educ Psychologist). No effect size extracted | U | Pattern well known; size unknown |
| Environmental context effect d about 0.25 | Smith & Vela 2001 | Title "Environmental context-dependent memory: A review and meta-analysis" confirmed; 0.25 unconfirmed (secondary via Murre) | U | |
| Varied contexts plus retrieval practice | Smith & Handy 2014 | Exists (doi:10.1037/xlm0000019); finding unconfirmed | U | Face-name pairs only |

## 3. Tool facts I re-checked

| Tool | Checked facts | Note |
|---|---|---|
| KaTeX | 0.18.10, MIT, released 2026-09-30 | Matches G1 |
| MathLive | MIT; npm latest is now 0.111.0 | G1 said 0.110.0 (2026-06-09); a newer release landed. Pin a version |
| Mafs | 0.21.0, MIT, published 2024-10-20 | Stale but tiny; fork if needed |
| SymPy | 1.14.0, BSD, 2025-04-27 is still the latest on PyPI | Resolves G1's open question |
| RDKit.js | 2026.3.6, BSD-3-Clause | Matches G2 |
| 3Dmol.js | 2.5.5, BSD-3-Clause | Matches G2 |
| Harper | v2.11.0, 2026-09-16 | Matches G3 |
| Lean 4 | v4.34.1, 2026-09-24 | Matches G1 |
| PhET | Pre-2026-03-29 sims are CC BY 4.0 (PhET historical-licence page). States-of-matter repo is GPL-3.0 (GitHub API) | Current sims being CC BY-NC was read by G2 from a live sim header; I could not reach a page stating current terms. Treat as firm for personal use, unconfirmed for redistribution |

Not re-checked (taken from reports): licences and sizes of Ketcher, SmilesDrawer, JSXGraph, function-plot, OEWN, Wiktionary, CMUdict, LanguageTool, Giac, Symbolica.

## 4. Rules for Tutor, ranked by evidence times impact

1. **Attempt first, reveal second** (voice/chat prompt, UI). Every dictionary, etymology, IPA, hint and library lookup appears only after the learner's attempt. Basis: practice tests beat restudy (Adesope 2017, strong per G3; I did not re-open it), and active practice beats passive (Freeman, verified).
2. **Guided discovery, never "play around"** (prompt, feature). Require a prediction, run the sim or plot, then require an explanation of the gap. Basis: Alfieri and Lazonder, verified. Exception: for a topic the learner has not met, give a worked example first (Chernikova: low-prior learners benefit from examples; Sinha & Kapur: grade and domain moderate the prediction-first benefit).
3. **Machines judge correctness, not the LLM** (question generation, feature). Route algebra and calculus through a cached SymPy sidecar, structures through RDKit.js canonical SMILES or InChI, and units through UnitMath or `uom`. The sim and plot numbers come from code. Basis: engineering rationale plus PAL (unconfirmed by me). Highest impact, lowest evidence risk.
4. **Step-level items with a misconception field** (question generation). Generate each step, check it with SymPy, and verify that each distractor's algebra produces the stated wrong answer. Plan for an effect of about 0.3-0.4 SD, not 0.66 (college ITS evidence).
5. **A sim or viewer session is an encoding event, not an FSRS review** (scheduling). Give each topic 3-5 retrieval cards with randomised parameters, first due after one night. Grade "predict, then manipulate" cards on the prediction. This is a design inference from the testing effect, not a tested result.
6. **Fade scaffolds with mastery** (prompt, UI). After two unhelped correct answers, or once FSRS stability passes a threshold, hide hints and default to symbols first. Sequence: worked plot, blank plane, symbols only. Basis: expertise reversal (pattern confirmed to exist, size unknown).
7. **Theme the chrome only** (UI). Per-subject accent palette and icons on navigation, headers and progress. Question, answer, explanation, sim and plot panes stay neutral and identical across subjects. No mascots, decorative images or animated backgrounds. Basis: Cheng g = -0.16, Rey 2012, Alpizar d = 0.38 for functional colour.
8. **Text-free animation, voice narration** (UI, voice prompt). Animated views carry no paragraph text; the ElevenLabs channel narrates. Basis: Berney (unconfirmed figures, directionally consistent with Höffler & Leutner).
9. **Do not tune from satisfaction** (measurement). Deslauriers (verified) shows felt learning and actual learning diverge. Log accuracy, delayed retention and transfer only.
10. **Within-person experiments in Evals** (measurement). Randomise per item between interactive-first and symbol-first; log prediction correctness, time to first correct step and 7-day transfer. Separately log `theme_id` and `context_variant` per review and compare delayed FSRS recall (expect at most about 0.25, plausibly 0).
11. **Vary retrieval context by default** (scheduling, UI). Rotate voice, text and layout rather than binding items to one look. Evidence is thin (Smith & Handy unconfirmed), so keep it cheap.
12. **English tools** (feature, corpus). Harper on the learner's self-edited draft, logging accepted and ignored suggestions. Close reading with the CSS Custom Highlight API, fixed functional layers, and a required one-line own-words gloss per highlight (the gloss is a design inference). Annotations stored with W3C TextQuote plus TextPosition selectors.
13. **Corpus hygiene** (corpus). Build vocabulary items from OEWN (CC BY 4.0) plus Wiktionary etymology (CC BY-SA 4.0 and GFDL, so store source id and licence per item). Stream kaikki JSONL once into SQLite outside OneDrive. Include untrained-word items to measure transfer.
14. **GPU policy** (UI). CSS, SVG and Canvas2D by default. One WebGL context at a time, 3Dmol only behind a "Show 3D" button, rendering on change, torn down when hidden. Nothing was measured on this machine.
15. **PhET launcher** (feature). Local single-file HTML in an iframe, PhET logo and CC BY-NC attribution visible. Personal use only. Regular builds expose no event data, so measure through Tutor-side prediction logs.

## 5. Conflicts and how I resolved them

| Conflict | Resolution |
|---|---|
| G1 rule 12 styles plots per subject; G2 and G3 say chrome only | Chrome only. Plot and sim panes use one neutral style with functional colour. The evidence supports signalling, not decoration |
| Seductive-details harm (g = -0.16, verified) vs emotional design benefit (g+ 0.27-0.35, verified, contested) | Both are measured on graphics inside lessons, so neither covers app chrome. Take the conservative side for learning panes and allow chrome identity as a design choice with no claimed learning benefit |
| Simulation effect g = 0.85 (Chernikova) vs unassisted discovery d = -0.38 (Alfieri) | Not contradictory. The large figure is complex professional skills with scaffolding. Unassisted exploration is the weak arm. Stay with guided sims |
| ITS effect: 0.66 (Kulik), 0.01-0.09 (K-12 maths), 0.32-0.37 (college), about 0.76 (VanLehn) | The higher figures come from locally built tests and mixed controls. The college meta-analysis is the nearest population. Plan on 0.3-0.4 and let Evals decide |
| Prediction-first (G1, G2) vs grade and prior-knowledge moderation (Sinha & Kapur, Chernikova) | Prediction-first for topics where the learner has prerequisite knowledge; example-first for new ground |
| Per-subject theming as memory aid (owner's assumption) vs context evidence | Not supported. Smith & Vela d about 0.25 is unconfirmed and secondary, the replication failed (N = 16), and varied contexts may beat constant. Identity stays a design choice |
| MathLive version (0.110.0 vs 0.111.0) | Registry shows 0.111.0 today. Pin whichever is tested |
| Interactivity in general vs visualization (Freeman) | Freeman's +0.47 SD is for classroom active learning, not visual tools. Do not cite it for interactive plots |

## 6. What not to build

- Desmos or GeoGebra embeds (key-gated, non-commercial, network-dependent, terms unverified for Desmos).
- A browser-only Lean editor, or Lean as core. Offer an opt-in "Proof Lab" with a 560-810 MB toolchain, flagged experimental (evidence is one qualitative study).
- Default 3D or WebGL plotters, VR or AR labs, a custom physics or molecular dynamics engine.
- A home-grown or Rust CAS (SymPy sidecar instead; symengine crate stale since 2020; Giac is GPL-3).
- Learning-style profiles (Pashler 2008, verified).
- Free-explore sim rooms.
- Decorative per-subject backgrounds, mascots or animated characters in learning panes.
- A fixed theme marketed as a memory aid.
- Satisfaction-driven adaptation.
- Commercial redistribution of PhET, periodic-table data from Wikipedia-derived JSON, bundling eSpeak NG (GPL-3), LanguageTool n-grams by default (8 GB), the Hypothesis client, the 24 GB kaikki file, the stale etymology-db.

## 7. Citation list with verification status

V = verified, C = corrected, U = unconfirmed.

1. Hillmayr et al. 2020, Comput Educ, doi:10.1016/j.compedu.2020.103897. V
2. Freeman et al. 2014, PNAS, doi:10.1073/pnas.1319030111. V
3. Alfieri et al. 2011, J Educ Psychol, doi:10.1037/a0021017. V (OpenAlex returned a wrong abstract for this DOI; figures confirmed via Semantic Scholar)
4. Lazonder & Harmsen 2016, Rev Educ Res, doi:10.3102/0034654315627366. V
5. Sinha & Kapur 2021, Rev Educ Res, doi:10.3102/00346543211019105. C (moderators added)
6. Chernikova et al. 2020, Rev Educ Res, doi:10.3102/0034654320933544. V
7. Uttal et al. 2013, Psychol Bull, doi:10.1037/a0028446. V
8. Kulik & Fletcher 2016, Rev Educ Res, doi:10.3102/0034654315581420. V
9. Steenbergen-Hu & Cooper 2013, doi:10.1037/a0032447. V
10. Steenbergen-Hu & Cooper 2014, doi:10.1037/a0034752. V
11. VanLehn 2011, Educ Psychol, doi:10.1080/00461520.2011.611369. U (partly; abstract garbled)
12. Chan & Leung 2014, J Educ Comput Res, doi:10.2190/ec.51.3.c. V
13. Pashler et al. 2008, Psychol Sci Public Interest, doi:10.1111/j.1539-6053.2009.01038.x. V
14. Deslauriers et al. 2019, PNAS, doi:10.1073/pnas.1821936116. V (no numbers)
15. Cheng et al. 2026, Educ Psychol Rev, doi:10.1007/s10648-025-10099-z. V (g only)
16. Rey 2012, Educ Res Rev, doi:10.1016/j.edurev.2012.05.003. V (via ERIC snippet)
17. Alpizar, Adesope & Wong 2020, ETR&D, doi:10.1007/s11423-020-09748-7. V
18. Wong & Adesope, Educ Psychol Rev, doi:10.1007/s10648-020-09545-x. C (year 2021 in ERIC)
19. Murre 2021, R Soc Open Sci. V
20. Berney & Bétrancourt 2016, Comput Educ, doi:10.1016/j.compedu.2016.06.005. U (figures)
21. Kalyuga et al. 2003, Educ Psychol, doi:10.1207/s15326985ep3801_4. U (size)
22. Smith & Vela 2001, Psychon Bull Rev, doi:10.3758/bf03196157. U (0.25 is secondary)
23. Smith & Handy 2014, J Exp Psychol LMC, doi:10.1037/xlm0000019. U (finding)
24. Ponce, Mayer & Méndez 2022 (learner-generated vs instructor-provided highlighting), Educ Psychol Rev. Exists per ERIC; g 0.36/0.20/0.44 not seen. U
25. Adesope, Trevisan & Sundararajan 2017, Rev Educ Res, doi:10.3102/0034654316689306. Not re-checked (taken from G3)
26. Höffler & Leutner 2007, doi:10.1016/j.learninstruc.2007.09.013. Seen only as an ERIC snippet (d = 0.37, 26 studies, 76 comparisons). Partly V
27. Gao et al. 2023, PAL, arXiv:2211.10435. Not re-checked
28. Not re-checked (cited by reports, no load-bearing use here): Finkelstein 2005, Olympiou & Zacharia 2012, Stieff 2011, Goodwin & Ahn 2013, Colenbrander 2024, Shen 2025, Shin 2021, Fyfe 2014, Rau 2017, Ainsworth 2006, Fiorella & Zhang 2018, Thoma & Iannone 2021, Flynn & Featherstone 2017.

## 8. Open items for the owner or a later run

- Measure real GPU load of WebView2, 3Dmol and PhET on this machine.
- Runtime-check the CSS Custom Highlight API in the installed WebView2.
- Test whether Ketcher supports curved-arrow mechanisms before promising mechanism items.
- Read the full text of Berney & Bétrancourt, Kalyuga 2003 and Smith & Handy before quoting numbers.
- Confirm PhET's current CC BY-NC terms on a rendered page if the app might ever be sold.
- Check China's national education platform terms separately; nothing here supports "all open source".
