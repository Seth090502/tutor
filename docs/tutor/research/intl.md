# Pedagogy of top-performing school systems: synthesis for Tutor

Verified 2026-09-30. Reports C1 to C8 read in full; about 30 citations checked against Crossref, OpenAlex, Semantic Scholar, ERIC, Europe PMC, NBER, Education Next, the NCES PDF and Wikipedia.

## 1. Bottom line

1. The data do not show that East Asian or Nordic classroom pedagogy caused their scores. Culture, selection, shadow education and sampling are confounded with method. Jerrim (2015) finds second-generation East Asian children in Australia outscoring natives by about 100 PISA points. Exported "mastery" programmes gave about +0.06 to +0.15 SD in children.
2. The transferable part is mechanism, not institution. The mechanisms with experimental support (case comparison, interleaving, expertise-faded worked examples, problem-first for learners with prior knowledge, informative feedback, retrieval and spacing) come from trials run in many countries, so "take little from the US" cannot be applied cleanly.
3. Nothing reviewed tested PhD-level or self-directed adult learners. Every rule below is a design hypothesis. The one adult RCT (Kestin 2025) is one lesson at one university.
4. Realistic target: about 0.2 to 0.4 SD on broad delayed measures. Tutoring meta-analyses give 0.37 SD for preK-12 (Nickow 2020), and most education RCTs give 0.06 SD (Lortie-Forgues and Inglis 2019). Bloom's "2 sigma" is not a target.
5. Finland declined, Estonia did well, and Denmark, Sweden, Norway and Iceland are mid-table. "Nordic method" is not a valid label.

## 2. Verification result

Of 29 load-bearing items checked, 24 are verified as stated, 4 corrected and 1 (PISA figures) could not be verified. Search quota was exhausted, so I used abstract APIs.

Corrections and flags:

- **von Hippel 2024**: C8 attributed "0.84 SD narrow vs 0.27 SD broad" to von Hippel. It is from Cohen, Kulik and Kulik 1982, cited by him. His own point: mastery learning (feedback plus testing, no tutoring) scored about 1.1 SD above whole-class, roughly half of Bloom's 2 sigma.
- **Barbieri 2023** (worked examples, g = 0.48): the abstract says self-explanation prompts moderated the effect negatively. This contradicts the C5 and C8 rule "require self-explanation". See section 5.
- **Karaman 2021**: Turkish studies only. C6 used it to argue learner-initiated feedback beats tutor-initiated. The abstract gives student-initiated 1.16, mixed 0.83, adult-initiated 0.69, computer-initiated 0.42. The study cannot carry a causal design rule.
- **Kestin 2025**: C8 left the effect size unverified. Full text gives N = 194 analysed, effect 0.63 SD (linear, underestimated by ceiling) and 0.73 to 1.3 SD (quantile). One 60-minute lesson, immediate post-test, one Harvard physics course. No delayed test.
- **PISA figures**: not verifiable. OECD returned 403. My Wikipedia extraction put Finland at 469 and Singapore at 563 in a "2022" row, which matches C6's "provisional 2025" figures and disagrees with my recall of official 2022 values (Singapore about 575, Finland about 484). Treat every PISA number in C1, C5, C6 and C7 as unverified. TIMSS 2023 grade 8 maths (Singapore 605, Taipei 602, Korea 596, Japan 595, Sweden 517, Finland 504) was confirmed by two independent Wikipedia extractions (C1 and mine). It is secondary but consistent.
- **Leong 2011** (Polya "look back", C4): unconfirmed; my DOI attempt resolved to a different paper.

## 3. Verified evidence table

| Claim | Source (status) | Figure | Strength, applies to |
|---|---|---|---|
| Unguarded LLM access harms later unaided performance | Bastani 2025 PNAS (V) | about 1,000 high-school students. Practice +48% (Base), +127% (Tutor). Later exam -17% (Base), mostly mitigated by Tutor | Moderate. One field experiment, teenagers, short horizon |
| AI tutor beat in-class active learning | Kestin 2025 Sci Rep (V) | 0.63 SD linear, 0.73 to 1.3 SD quantile, N = 194 | Moderate to weak: one lesson, immediate test. Adults, physics |
| Case comparison aids learning | Alfieri 2013 (V) | d = 0.50 [0.44, 0.56], 57 experiments, 336 tests. Larger when learners find similarities and the principle comes after comparison | Strong. Mostly short studies. Needs prior knowledge (Rittle-Johnson 2009, abstract not re-read) |
| Interleaved maths practice | Rohrer 2020 (V) | 61% vs 38% at one month, d = 0.83, 54 grade-7 classes, preregistered | Moderate to strong. Grade 7 only. Both groups took an interleaved review first |
| Problem-first then instruction | Sinha and Kapur 2021 (V) | g = 0.36 [0.20, 0.51]. Reversed for grades 2 to 5 and domain-general skills. Bias-adjusted 0.87 | Moderate, contested. Do not cite 0.87 |
| Worked examples, maths | Barbieri 2023 (V) | g = 0.48. Self-explanation prompts: negative moderator | Moderate. Grades K to postsecondary |
| Expertise reversal | Kalyuga 2003 (V); Tetzlaff 2025 (exists, numbers not read) | Interaction effect, no single ES | Strong as principle. Short studies, induced expertise |
| Unassisted discovery loses, guided wins | Alfieri 2011 (V) | d = -0.38 (unassisted vs explicit), d = +0.30 [0.23, 0.36] (enhanced discovery), 164 studies | Moderate to strong. Mostly school age |
| Formative assessment is about 0.2 SD, not 0.4 to 0.7 | Kingston and Nash 2011 (V) | Weighted 0.20, median 0.25. Only 13 of over 300 studies usable. ELA 0.32, maths 0.17, science 0.09. Computer-based 0.28 | Moderate. K-12 |
| Feedback content matters | Wisniewski 2020 (V) | d = 0.48, 435 studies, strongly heterogeneous | Moderate. Information content is the moderator |
| Tutoring meta-analysis | Nickow 2020 (V) | 0.37 SD pooled, preK-12 RCTs | Strong for school age |
| Bloom's 2 sigma is narrow-test inflated | von Hippel 2024 (V, corrected) | 0.84 narrow vs 0.27 broad (Cohen 1982) | Moderate |
| Singapore-style curriculum, US | Jaciw 2016 (V) | 0.11 to 0.15 SD, 12 schools, grades 3 to 5 | Moderate, small. Curriculum bundle |
| Singapore-inspired mastery, England | Jerrim and Vignoles 2016 (V for "modest positive"; numbers from C2's opened PDF) | +0.10 primary, +0.06 secondary (n.s.), pooled about +0.077 | Moderate, small |
| Word-problem interventions, LD/MD | Lein 2020 (V) | g = 0.56 [0.40, 0.72], larger at elementary than secondary, larger with researcher measures | Moderate. Not typical learners |
| Japanese lessons, TIMSS video | Hiebert 2003, NCES 2003-013 (V, PDF text) | Japan: 54% connection problem statements, 27% problems summarized. Hong Kong: 84% procedure statements. Authors: "no direct inferences" to achievement | Descriptive. 1995 sample, 50 schools |
| Lesson study has no pupil effect at scale | Murphy 2017 EEF (V design; null from C3's opened PDF) | 181 schools, 12,747 pupils, MDES 0.108, null | Strong null. It was the Edge Hill "Talk for Learning" variant |
| Typical education RCT effect | Lortie-Forgues and Inglis 2019 (V) | 0.06 SD mean, CI width 0.30, 141 RCTs | Strong calibration |
| Paper vs screen | Delgado 2018 (V) | g = -0.21, 54 studies, 171,055 participants. Larger under time pressure and for informational text. Null for narrative | Moderate. Trend disputed (Kong 2018, g about -0.25 in Clinton 2019). Not tested on long PhD-level reading |
| Finland's decline partly explained by boys' gap and SES gradient | Ahonen 2020 (V abstract) | "explain it only partly" | Moderate. Cause contested |
| Wellbeing cost | Choi 2026, Jeong 2026, Hu 2026, Zhu 2026 (exist, per Crossref; findings from C5 abstracts not re-read) | Ecological or cross-sectional | Weak to moderate for causation |

## 4. Rules for Tutor, ranked by evidence times impact

Tags: [P] voice/chat prompt, [S] scheduling, [Q] question generation, [F] feature, [U] UI, [M] measurement, [C] corpus.

1. **Attempt first, hint ladder, never hand over the solution on the first request.** Answers and worked solutions stay hidden until the learner commits an answer. Evals run AI-off and closed-book. (Bastani; Kestin's tutor also withheld answers.) [P][U][M] Strength moderate, impact highest.
2. **FSRS at 0.9 stays the retention engine. Add cumulative mixed review after initial learning (start about 20% of reviews) and measure at 7 and 30 days.** Japanese lessons supply no spaced retrieval, so FSRS covers what they lack. (Rohrer; Dunlosky exists, not re-read.) Keep 0.9 as a tunable per-domain default. No source compared 0.9 with other targets. [S][M] Strength moderate to strong, impact high.
3. **Route each concept by prior knowledge.** A 1 to 3 question ungraded diagnostic picks the mode: worked and faded examples for novices, guided problems for intermediates, problem-first then consolidation for advanced. Fade per skill, not per person. Log the mode. (Kalyuga; Sinha and Kapur; Barbieri.) [P][U Plan][M] Strength moderate, impact high. The adult-specific evidence is missing.
4. **Comparison before rule.** Show 2 to 3 varied cases, including a non-example and a flawed solution. Ask what changed, what stayed, and why, then state the rule. Use only when the prior-knowledge check passes; otherwise study cases sequentially first. (Alfieri 2013; Rittle-Johnson.) [P][Q][F Compare card] Strength strong, impact high.
5. **Feedback names the misconception and the next step.** Ask for a 0 to 100 confidence rating before the reveal and show a calibration chart and misconception counts, not a single score. (Wisniewski; Kingston; calibration rationale from screen-reading studies is weaker.) [P][U][M] Strength moderate, impact high.
6. **Question bank as variant families.** Fields: `variation_dimension`, `invariant`, `near_miss_of`, `representation_type`, `relation_type`, `scaffold_level`, item type (recall, near transfer, far transfer, procedural vs application), and an optional 3-level `hint_ladder`. Variants of one seed must not each count as independent FSRS mastery evidence. An LLM-generated variant needs an independent solver pass before it enters the deck. [Q][S] Strength design-derived (Alfieri, Rohrer, Paas), impact high because it protects correctness.
7. **Measure what matters.** Delayed retention, transfer to unseen variants and hint dependence, not minutes, streaks or hours. Run within-learner crossovers on matched topics (explanation-first vs problem-first, paper vs screen, variation-rich vs plain). Treat differences under about 0.2 SD as noise and report intervals. Include an adult-baseline numeracy and literacy track. [M] Strength strong (Lortie-Forgues), impact high.
8. **Load cap and recovery.** Configurable daily cap on new items and reviews, soft stop after about 90 minutes, late-night quiet period, optional weekly wellbeing check-in (sleep, anxiety) used only to adjust load. Korean intensity and sleep data are ecological, so this rests on precaution. [S][U] Strength weak to moderate, cost very low.
9. **One rich problem per unit, then a learner-written summary before the tutor's.** Turn each summary and comparison insight into an FSRS card. Do not use this as the only mode; retrieval and proof-reading need other modes. (TIMSS video is descriptive only; Hong Kong shows it is not necessary.) [P][Q][S] Strength weak, impact medium.
10. **Structure-first representation with adaptive fading.** Diagram or structural sketch before symbols for new structures (part-whole, comparison, change, or a domain analogue such as free-body or reaction-coordinate diagrams). Drop it after 2 to 3 correct unaided attempts and restore it when errors look structural. Evidence is children and learners with difficulties (Lein 2020 g = 0.56; Carbonneau 2013, V-abs, manipulatives help retention more than transfer). Build a drawable canvas only after rules 1 to 7. [P][F] Strength weak for adults, impact medium.
11. **Paper path.** Print-and-read export with a closed-book quiz a day later, self-paced by default, screen reading left available. The average gap is about 0.2 SD and its trend is disputed, so run the crossover (rule 7) before making a claim to the owner. [F][M] Strength small, moderate.
12. **Guided inquiry only.** If a learner is stuck, escalate hints to a worked step. Never leave them in pure discovery. (Alfieri 2011.) [P] Strength moderate to strong, largely covered by rules 1 and 3.
13. **Corpus.** Store citable notes (title, licence, link, own summary), not full reports, for PISA, TIMSS, PIRLS and PIAAC material. No number enters the corpus until a worker verifies it against the OECD or IEA table. Nothing in these reports verifies that Chinese textbooks or papers are openly licensed, so add each through `corpus/manifest.json` with an explicit licence, and do not bulk-ingest. [C]
14. **Solo "lesson study" loop.** Log stalls, hint depth and surfaced misconceptions; revise prompts and item probes weekly. Content QA only. The EEF null says the ritual alone does not raise outcomes. [M][Q] Weak, cheap.
15. **Optional, untested.** A session "board" note saved to the Library, a Danish-style oral-defence voice mode (format unverified), a primer mode before new chapters, a notification-muting focus mode.

## 5. Conflicts and how I resolved them

- **Self-explanation.** C5 and C8 say require it. Barbieri 2023 found prompts lowered the worked-example effect in maths. Barbieri is a meta-analytic moderator, C5 and C8 rely on general principle. Resolution: do not mandate it on every step. Ask "explain why that step works" after an error, at a mode change, or on a transfer item, and test with a crossover.
- **Problem-first vs explicit instruction.** C3 and C8 favour problem-first for learners with prior knowledge; Alfieri 2011 and Rittle-Johnson show novices need guidance; Sinha and Kapur reverse for younger learners. All agree prior knowledge decides, so the diagnostic (rule 3) routes it. The 0.87 bias-adjusted figure is out.
- **Mastery gates.** C2 says no hard gates. C4 wants prerequisite gating. C8 wants mastery with delayed checks. Kulik 1990 reports self-paced college programmes sometimes cut completion (from C8, not re-verified). Resolution: soft gates with a skip, a cold-recall basics check, prerequisite stability before scheduling new items, and at most 2 to 3 new skills per session.
- **Learner-initiated feedback.** C6's claim rests on Turkish K-12 data where adult-initiated feedback (0.69) also beat computer-initiated (0.42). Downgraded: offer hints on request and run the ladder, and test tutor-initiated vs learner-initiated in the crossover. Do not cite Karaman as design evidence.
- **Formative assessment magnitude.** B&W 0.4 to 0.7 against Kingston and Nash 0.20. Kingston's abstract confirms the lower figure and shows only 13 usable studies. Use 0.2 and do not treat either as precise.
- **Practice volume.** C1, C5 and C8 agree hours are a weak lever. This does not conflict with spacing: Macnamara's 4% (V-abs) concerns deliberate-practice hours, not retrieval. C8's 80% first-attempt success target and FSRS 0.9 retrievability are different quantities.
- **Screen vs paper.** Delgado says the gap grows, Kong says it shrinks, and neither tests long PhD-level reading. Kept as a measured option.
- **PISA values differ across C1, C5, C6, C7.** No report has an official table. All set aside. TIMSS 2023 and PIAAC ranks are used only as qualitative context.

## 6. What not to build

- A "Chinese, Japanese, Singapore, Finnish or Nordic method" mode or "proven" copy. The mastery RCTs show +0.06 to +0.15 SD, the Shanghai exchange showed no attainment effect, lesson study showed no pupil effect at scale, and phenomenon-based learning has no controlled evidence.
- Country rankings, leaderboards, percentile competition, streak-shame gamification or norms from B-S-J-Z data (sampling disputed; mainland China absent from PISA 2022).
- Hours-driven drilling, hagwon-style volume targets, Suneung-style timed multiple choice as the main assessment.
- Hard mastery gates, a fixed four-step script for every interaction, a discovery-only default or an always-on productive-failure mode.
- Streaming or exam-gate simulation, a MEXT engine, a teacher network.
- Physical or virtual manipulative modules, a fixed CPA sequence for every topic, or a standalone metacognition module.
- A tutor that hands out solutions on request, or a score-free experience with no calibration signal.
- "2 sigma" or "0.4 to 0.7 formative assessment" claims anywhere in the app. Hard-coded PISA, KI or SEK figures. "Digital native" assumptions. A screen-only or paper-only mandate.
- Scaling AI features ahead of evidence (Korea's 2025 AI-textbook downgrade: headlines only, reasons unverified).

## 7. Citation list and verification status

V = existence and headline figure checked this session. V-abs = checked at abstract level. E = exists (Crossref or OpenAlex), findings not re-read. U = unverified. C = corrected.

- Bastani et al. 2025, PNAS, 10.1073/pnas.2422633122: V-abs
- Kestin et al. 2025, Sci Rep, 10.1038/s41598-025-97652-6: V (full text, Europe PMC)
- Alfieri, Nokes-Malach, Schunn 2013, Educ Psychol, 10.1080/00461520.2013.775712: V-abs
- Alfieri, Brooks, Aldrich, Tenenbaum 2011, J Educ Psychol, 10.1037/a0021017: V-abs (abstract prints CI garbled)
- Rohrer et al. 2020, J Educ Psychol, 10.1037/edu0000367: V-abs
- Sinha and Kapur 2021, Rev Educ Res, 10.3102/00346543211019105: V-abs
- Barbieri et al. 2023, Educ Psychol Rev, 10.1007/s10648-023-09745-1: V-abs, C (self-explanation negative)
- Kalyuga, Ayres, Chandler, Sweller 2003, Educ Psychol, 10.1207/s15326985ep3801_4: V-abs
- Tetzlaff et al. 2025, Learn Instr, 10.1016/j.learninstruc.2025.102142: E
- Jerrim and Vignoles 2016, Econ Educ Rev, 10.1016/j.econedurev.2015.11.003: V-abs ("modest, positive"); SD figures U (C2 preprint)
- Jerrim 2015, Oxf Rev Educ, 10.1080/03054985.2015.1028525: V-abs
- Jaciw et al. 2016, JREE, 10.1080/19345747.2016.1164777: V-abs
- Lein, Jitendra, Harwell 2020, J Educ Psychol, 10.1037/edu0000453: V-abs
- Peng et al. 2025, Educ Psychol Rev, 10.1007/s10648-025-10057-9: E (title: network meta-analysis)
- Hiebert et al. 2003, NCES 2003-013: V (PDF text)
- Murphy, Weinhardt, Wyness 2017, EEF, ERIC ED581145: V (design); null result from C3
- Lewis and Perry 2017, JRME, 10.5951/jresematheduc.48.3.0261: V-abs (no effect sizes)
- Lortie-Forgues and Inglis 2019, Educ Res, 10.3102/0013189X19832850: V-abs
- Kingston and Nash 2011, EM:IP, 10.1111/j.1745-3992.2011.00220.x: V-abs
- Karaman 2021, IJATE, 10.21449/ijate.870300: V-abs, C (Turkey only)
- Wisniewski, Zierer, Hattie 2020, Front Psychol, 10.3389/fpsyg.2019.03087: V-abs
- VanLehn 2011, Educ Psychol, 10.1080/00461520.2011.611369: V-abs
- Nickow, Oreopoulos, Quan 2020, NBER w27476: V-abs
- von Hippel 2024, Education Next: V, C (attribution)
- Delgado et al. 2018, Educ Res Rev, 10.1016/j.edurev.2018.09.003: V-abs
- Clinton 2019, J Res Reading, 10.1111/1467-9817.12269: E
- Kessel, Hardardottir, Tyrefors 2020, Econ Educ Rev, 10.1016/j.econedurev.2020.102009: V-abs
- Abrahamsson 2026, J Hum Resour, 10.3368/jhr.0224-13403r2: E
- Dunlosky et al. 2013, PSPI, 10.1177/1529100612453266: E
- Ahonen 2020, 10.1007/978-3-030-59031-4_6: V-abs
- Choi et al. 2026 (Lancet Reg Health West Pac, 10.1016/j.lanwpc.2026.101924); Jeong et al. 2026 (Medicine, 10.1097/md.0000000000050516); Hu, Lin, Tang 2026 (Front Psychol, 10.3389/fpsyg.2026.1891921); Zhu, Liu, Jia 2026 (Front Psychol, 10.3389/fpsyg.2026.1795875): E
- TIMSS 2023 maths (Wikipedia, two extractions agree): V-secondary
- PISA 2022 and 2025 figures, PIAAC Cycle 2 figures, Loveless 2013 sample counts, hagwon and juku participation: U
- Leong et al. 2011 (Polya look-back): U (DOI mismatch)
- Not re-checked (taken from reports): Gu et al. 2004, Kullberg 2017, Marton, Stigler and Hiebert 1999, Ma 1999, Rosenshine 2012, Black and Wiliam 1998, Kulik 1990, Slavin 1987.

## 8. Before building

1. Have a worker pull PISA 2022 and 2025, TIMSS 2023, PIRLS 2021 and PIAAC Cycle 2 tables from OECD or IEA sources before any figure enters the corpus or UI.
2. Read full texts for Tetzlaff 2025 and the EEF Mathematics Mastery reports.
3. Audit LLM-generated variant families for invariant preservation on the owner's subjects before scaling question generation (rule 6).
