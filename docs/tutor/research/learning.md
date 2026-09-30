# Learning science and cognitive neuroscience: synthesis for Tutor

Scope: eight reports (A1 to A8). I read all of them in full, then re-checked 33 load-bearing sources against abstracts retrieved this session from Europe PMC, Crossref, OpenAlex, Semantic Scholar, publisher pages and the srs-benchmark README. Full texts were not read.

## 0. Bottom line

1. The strongest, best-replicated levers are retrieval with feedback, spacing with relearning, and feedback that explains. Together they fit Tutor's current design (FSRS, question bank with explanation and misconception).
2. Retrieval practice is not the engine for multi-step problem solving. The 2025 math meta-analysis (corrected below) found the testing effect not robust in math. Procedural skill needs its own worked-example, fading and interleaved path.
3. Scaffolding must depend on the learner's per-topic knowledge (expertise reversal is robust), not on a global pedagogy.
4. An LLM that answers on demand harms later unassisted performance; a guardrailed one largely does not. This is the single most important prompt constraint.
5. Neuroscience supplies explanations, not new methods. The planned Dual N-Back section has no support as a learning aid.
6. Nearly all evidence is from children, undergraduates or lab samples. No verified PhD-level evidence exists for any rule below. Treat effect sizes as priors and have Tutor measure the owner's own data.

The owner's East Asian and Nordic pedagogy idea is outside this area. A4 could not verify a primary source for Chinese variation teaching, so nothing here relies on it.

## 1. Verified evidence table

Status: V = abstract matched this session; C = corrected versus the report; R = report-level only (not re-checked by me).

| Claim | Figure (as verified) | Status | Strength and limits |
|---|---|---|---|
| Classroom testing effect | g = 0.499, 222 studies, N = 48,478; moderated by feedback, repeats, format (Yang 2021) | V | Strong. School and college. |
| Testing transfers | d = 0.40 [0.31, 0.50], 122 experiments; weakest for rearranged items, untested material, worked-example problems; bias-corrected intercepts often near zero without moderators (Pan & Rickard 2018) | V | Moderate. Transfer is conditional. |
| Testing vs restudy at delay | 5-min test favoured restudy; 2-day and 1-week favoured testing; students felt restudy was better (Roediger & Karpicke 2006) | V | Strong. Prose passages. |
| Retrieval in math | Testing vs restudy g = 0.18, CI crosses zero, 7 studies; spacing g = 0.28, 27 studies (course-embedded 0.24) (Murray, Horner & Goebel 2025) | C | Report A1 quoted preprint values 0.22 and 0.26; published abstract differs. Weak for testing. |
| Spacing ridgeline | Optimal gap about 20-40% of a 1-week delay, about 5-10% of a 1-year delay; N > 1,350 (Cepeda 2008) | V | Strong. Facts, up to 1 year. |
| Successive relearning | Relearning vs single session ds = 1.52 to 4.19; initial-lag effects ds = 1.07-1.87 fall to -0.20-0.38 after relearning (Rawson 2018). "3 correct recalls then relearn" prescription, 533 students (Rawson & Dunlosky 2011) | V | Strong in lab. Vocabulary and concept definitions. |
| FSRS prediction | 9,999 Anki users, 349.9M evaluated reviews; log loss FSRS-6 0.346, FSRS-7 about 0.336-0.340, DASH 0.368, ACT-R 0.403, HLR 0.469 | C | A2 said "350M reviews" loosely; README says 727M raw, 349.9M evaluated. Measures prediction, not learning. Community-run. |
| Interleaving | g = 0.42 overall; math g = 0.34 ("small"); paintings 0.67; words -0.39 (blocking better); expository text n.s. (Brunmair & Richter 2019) | V | Moderate. Needs confusable, similar-between categories. |
| Interleaved math RCT | 61% vs 38%, d = 0.83, 54 classes, unannounced test 1 month later (Rohrer et al., J Educ Psychol, online 2019 / issue 2020) | V | Moderate. 7th grade, near transfer. |
| Elaborated feedback | EF 0.49, KCR 0.32, KR 0.05; delayed timing negatively affected effects but no significant interaction with outcome level (Van der Kleij 2015) | V | Moderate. Computer-based item feedback. Resolves A3's garbled timing note. |
| Unguarded LLM harm | GPT Base +48% practice, then -17% when access removed; GPT Tutor +127%, harm largely mitigated; about 1,000 students (Bastani 2025) | V | Moderate. One field study, high-school math. A3 reports a correction notice; I did not read it. |
| Structured AI tutor | Students learned significantly more in less time than active-learning class (Kestin 2025) | V | Moderate. No effect size in abstract; A5's "twice as much" is from a preprint and is not used. |
| Worked examples, math | g = 0.48, 55 studies, 181 effect sizes; adding self-explanation prompts moderated the effect negatively; correct examples alone best (Barbieri 2023) | V | Strong for novices. Math only. |
| Expertise reversal | Low knowledge d = 0.505 for high assistance; high knowledge d = -0.428; weaker for humanities and language (Tetzlaff 2025) | V | Strong. Asymmetric. Matters for the English track. |
| Productive failure | g = 0.36 [0.20, 0.51], 53 studies; 0.37-0.58 when faithful; grades 2-5 and domain-general skills favoured instruction first; abstract also states bias-adjusted g = 0.87 (Sinha & Kapur 2021) | V | Moderate, contested. The 0.87 is unexplained; do not use. |
| Self-explanation prompts | g = 0.55, 69 effect sizes (Bisra 2018) | V | Moderate. Lab-heavy; shrinks in digital and delayed settings (Tan 2025, R). |
| Learning by teaching | g = 0.27 overall; 0.48 with teaching expectancy; -0.02 without (Kobayashi 2024) | V | Moderate. Needs preparing-to-teach expectancy. |
| Case comparison | d = 0.50 [0.44, 0.56], 57 experiments; best when learners find similarities first and principle comes after (Alfieri 2013) | V | Strong. |
| Pretest vs posttest | d = 0.35 vs 0.74, transfer only after posttesting (Latimier 2019); pretest >= posttest, 5 experiments, n = 1,573 (Pan & Sana 2021) | V both | Contested (see section 3). |
| Sleep and memory | g = 0.44; g = 0.28 after selective-reporting correction; larger with multiple study trials (Berres & Erdfelder 2021). Sleeping between sessions halved relearning practice, n = 40 (Mazza 2016) | V | Moderate; vocabulary only. |
| Cognitive-training far transfer | No convincing far transfer vs treated controls (Melby-Lervåg 2016; 87 publications). n-back: medium on untrained n-back, very small elsewhere, single vs dual and dose did not moderate (Soveri 2017; 33 RCTs) | V | Strong (null). |

## 2. Rules for Tutor, ranked by evidence strength times impact

Tags: [PROMPT] voice or chat tutor prompt; [SCHED] scheduling; [QGEN] question generation; [FEAT] feature; [UI]; [MEAS] measurement; [CORPUS].

1. **Attempt before reveal, then elaborated feedback.** For every scored item, the tutor collects the learner's committed answer before any hint or explanation, then gives feedback that explains (names the misconception), never right/wrong alone. Escalate hints: nudge, partial step, worked step. Log hint level. [PROMPT] [UI] [FEAT]. Basis: Yang 2021, Van der Kleij 2015, Bastani 2025. The "check answer before explain" ordering is an inference from these, not a tested protocol.
2. **Successive relearning on FSRS for declarative atoms.** New item: retrieve to about 3 correct recalls in session 1, then one correct recall per later FSRS-scheduled session. Do not over-drill session 1. Log item, time, grade, latency, mode (voice or chat) on every review. Always show the answer after a miss. [SCHED] [MEAS]. Basis: Rawson 2011 and 2018, Cepeda 2008. Lab vocabulary evidence; FSRS itself has no learning-outcome RCT.
3. **Two separate paths: declarative and procedural.** Atoms (definitions, theorem statements, mechanisms, vocabulary) go through FSRS recall. Derivations and multi-step problems go through worked example, completion problem, then full problem, with fresh isomorphic variants each time. Report the two separately. [SCHED] [QGEN] [UI]. Basis: Barbieri 2023, Murray 2025, Pan & Rickard (worked-example problems weak).
4. **Per-topic stage policy (novice, developing, proficient).** Novice on a high-interactivity item: example first, then fade backward. Proficient: problem first, no hints by default, example only after a committed attempt. Recompute after each attempt from stability, correct rate, latency and hint use. Treat the owner as a novice in every new topic. [PROMPT] [SCHED]. Basis: Tetzlaff 2025. Stage thresholds (for example 80% accuracy) are assumptions to tune. English track: do not port math fading rules untested.
5. **Guardrail the LLM.** No answer-on-demand in practice flows; hints graded and logged; track an AI-reliance gap (hint-free accuracy vs guided accuracy). [PROMPT] [MEAS]. Basis: Bastani 2025.
6. **Question-bank schema.** Keep one skill per item, explanation, misconception. Add prereqs, element_interactivity (1-5, an LLM-seeded estimate to calibrate), confusable_with, typical_confident_error, near-transfer and structural-transfer variants tagged separately, and a corpus passage citation for each explanation. Validate generated worked solutions with a second pass or a computer algebra system. Default format: cued recall or short answer; multiple choice only for discrimination, with misconception-based distractors and per-option feedback. [QGEN] [CORPUS]. Basis: Pan & Rickard, Little et al. 2012 and Greving & Richter 2018 (R), Chen, Paas & Sweller 2023 (R).
7. **Confidence before feedback.** Four levels (guess, unsure, fairly sure, certain), asked after the answer and before feedback, on retrieval attempts only. Route feedback: high-confidence error gets a misconception-naming explanation, a same-session retest and a retest at the next FSRS interval; low-confidence correct gets a short reason. Confidence does not drive FSRS parameters. [PROMPT] [UI]. Basis: Butler 2008, Butler 2011, Metcalfe 2017 (all R; hypercorrection is established for facts, unproven for problems). Design inference.
8. **Interleave confusable problem types.** Block briefly when a procedure is new, then mix confusable types (integration techniques, equilibrium vs kinetics, similar mechanisms). Do not interleave vocabulary-like pairs or long expository reading. [SCHED]. Basis: Brunmair 2019, Rohrer 2019/20; part of the benefit may be spacing (Foster 2019, R).
9. **Selective generative prompts.** Self-explanation after errors or for proficient learners, including "why is this misconception wrong". Comparison of two same-structure cases, principle stated afterwards, for chemistry and physics. Fade prompts as unprompted quality rises. [PROMPT] [QGEN]. Basis: Bisra 2018, Alfieri 2013, Barbieri 2023.
10. **Teach-back with announced expectancy.** Announce before study that the owner will explain it; the tutor plays a naive student; owner explains from memory, then may open sources for a second pass. Schedule after first successful recall. [FEAT] [PROMPT]. Basis: Kobayashi 2024; open-book finding is R (Sibley 2022).
11. **Measure the owner, not the population.** Randomize about 20% of eligible items to restudy vs retrieval, hold out about 10% from extra review, test at 7-30 days, keep a separate transfer set. Report Brier score, calibration curve, delayed accuracy, minutes to criterion, reliance gap. State that one learner is low-power. Never use satisfaction as the outcome. [MEAS] [UI]. Basis: Deslauriers 2019 (felt vs actual diverge); Roediger & Karpicke 2006.
12. **Short exposition, retrieval as the attention device.** At most 2-3 minutes of talk before an answerable question. Default 40-50 minute sessions with a pre-set, overridable 5-10 minute break, labelled an A/B default. Voice never interrupts mid-derivation. Feedback informational, not praise. Offer 2-4 choices; no points or streaks. [PROMPT] [UI]. Basis: Szpunar 2013; break values are not from a verified trial.
13. **Desired retention as a tunable default.** Start at 0.90; offer tiers (0.95 near exam dates, 0.85 bulk, 0.80 breadth) with daily workload shown. No source validates 0.90 as optimal; tiers are design judgment. [SCHED] [UI].
14. **Sleep as a logged covariate.** First relearn naturally falls next day under FSRS; log sleep window and test a sleep-crossing effect on own data; expect g near 0.3. [SCHED] [MEAS]. Basis: Berres 2021, Mazza 2016. Weak.
15. **Progressive multimedia rules for new material only.** Signal key terms, keep text beside figures, segment long derivations, no decorative images; offer expert mode that turns them off. [UI]. Basis: Noetel 2022 (R), expertise reversal.
16. **Neuromyth guard.** Fixed adversarial prompt set confirms the tutor never asserts learning styles, 10% of the brain, dopamine-hit or brain-training claims. [MEAS] [PROMPT].

## 3. Conflicts and how I resolved them

- **Retrieval practice as universal engine (A1 headline) vs complex material (A1, A4, Murray).** Resolved toward conditional use. The general effect is strong (g = 0.499), but the math-specific effect is g = 0.18 with a CI crossing zero, and van Gog & Sweller 2015 is an editorial hypothesis (not a pooled effect). Use retrieval for atoms; step back one stage on a lapse of a high-interactivity item.
- **Attempt-first everywhere (A1, A5, A8) vs worked examples first for novices (A4).** These are two different rules. "Commit an answer before the tutor reveals it" is a guardrail that applies to every scored item (Bastani). "Problem before instruction" (productive failure) is a sequencing choice with g = 0.36, reversed in children and contested at high element interactivity (Ashman 2020, R). Resolved: the guardrail is universal; productive failure is gated by prerequisites and a conceptual target and always followed by consolidation. Stronger evidence: expertise reversal (d = 0.505 and -0.428, 60 studies) over productive failure (mostly non-adult).
- **Self-explanation prompts (A5 recommend, A4 caution).** Bisra g = 0.55 compares prompts vs none broadly; Barbieri found prompts added to worked examples reduced the effect in math. Resolved: prompts are selective (after errors, proficient stage), not always on. Barbieri is the more specific evidence for Tutor's math use.
- **Pretesting (A1, A6 positive vs Latimier).** Both verified. Pan & Sana found pretest >= posttest on expository text; Latimier found post 0.74 vs pre 0.35 with transfer only after posttesting. Materials and formats differ. Resolved: optional short prequestions only when the target is taught immediately, never core loop.
- **Confidence format (A1 0-100, A3 four-level, A6 1-5).** All agree it comes before feedback. Resolved: four levels (works by voice, avoids spurious precision). A3's warning against per-item JOL sliders on study material stands (reactivity g = 0.054, R).
- **Feedback timing (A1 immediate implied; A3 adaptive).** Verified Van der Kleij: delayed timing had a negative effect overall, without a significant interaction. Classroom-versus-lab mixed evidence and Mullet 2014 (record verified only) favour delay for transfer. Resolved: default immediate elaborated feedback; randomize delay for proficient topics and read the owner's own 7-day retest.
- **Interleave by default (A2) vs block first (A8).** Math g = 0.34 is small. Resolved: brief block for new procedures, then interleave confusable types (design inference).
- **Scheduling after sleep (A6) vs FSRS-decided intervals (A2).** Compatible: first FSRS interval is at least a day. Keep criterion learning in session 1 and log sleep. Mazza is one n = 40 vocabulary study.
- **FSRS "20-30% fewer reviews" (docs claim).** Not independently verified; do not display.
- **Stale or unverifiable numbers.** Murray values corrected; Sinha & Kapur 0.87 flagged; Kestin "twice as much" dropped.

## 4. What not to build

- Dual N-Back as a learning aid, or any score feeding Plan or scheduling. Verified meta-analyses show task-specific gains only. If built, label it an optional working-memory game and keep it lowest priority.
- A retrieval-only engine for derivations and multi-step problems.
- An answer-on-demand or hint-on-demand LLM mode in practice.
- One global Socratic, explain-first or struggle-first tutor.
- Learning-styles, hemisphere or brain-age features; "learning pyramid" retention percentages.
- Points, XP, streaks, leaderboards, dopamine-themed rewards, hours-counter gamification.
- Growth-mindset or grit modules; willpower bars; exercise-boost messaging; phone-presence shaming; face or voice emotion detection.
- Sleep-learning, targeted memory reactivation, or reconsolidation "memory editing".
- Custom HLR, DASH or ACT-R schedulers, neural schedulers (revisit above about 100k reviews), forced expanding intervals.
- Real-time cognitive-load dashboards; germane load as a KPI.
- Per-item JOL sliders; confidence-weighted scoring as a headline; "unskilled and unaware" messaging.
- Curiosity teasers before unrelated material; deliberately induced confident errors.
- UI copy presenting productive failure or cognitive load theory as settled.

## 5. Open questions

1. Retrieval, spacing, interleaving and feedback effects at PhD level on high element-interactivity material: no verified evidence.
2. Whether hypercorrection holds for adult problem solving (only a 2026 dissertation, not peer reviewed).
3. A correction notice for Bastani 2025 exists per A3; read it before quoting 17% in product copy.

## 6. Citations and verification status

**V: abstract matched this session (33).** Yang 2021 (doi:10.1037/bul0000309); Pan & Rickard 2018 (10.1037/bul0000151); Roediger & Karpicke 2006 (10.1111/j.1467-9280.2006.01693.x); Rawson & Dunlosky 2011 (10.1037/a0023956); Rawson et al. 2018 (10.1037/xap0000146); Cepeda et al. 2008 (10.1111/j.1467-9280.2008.02209.x); Lindsey et al. 2014 (10.1177/0956797613504302); Kang et al. 2014 (10.3758/s13423-014-0636-z); Brunmair & Richter 2019 (10.1037/bul0000209); Rohrer et al. (10.1037/edu0000367); Van der Kleij et al. 2015 (10.3102/0034654314564881); Bastani et al. 2025 (10.1073/pnas.2422633122); Kestin et al. 2025 (10.1038/s41598-025-97652-6); Barbieri et al. 2023 (10.1007/s10648-023-09745-1); Tetzlaff et al. 2025 (10.1016/j.learninstruc.2025.102142); Sinha & Kapur 2021 (10.3102/00346543211019105); Bisra et al. 2018 (10.1007/s10648-018-9434-x); Kobayashi 2024 (10.1007/s10648-024-09871-4); Alfieri et al. 2013 (10.1080/00461520.2013.775712); Pan & Sana 2021 (10.1037/xap0000345); Latimier et al. 2019 (10.1038/s41539-019-0053-1); Berres & Erdfelder 2021 (10.1037/bul0000350); Mazza et al. 2016 (10.1177/0956797616659930); Hu et al. 2020 (10.1037/bul0000223, sleep TMR g = 0.29, not a software feature); Szpunar et al. 2013 (10.1073/pnas.1221764110); Deslauriers et al. 2019 (10.1073/pnas.1821936116); Melby-Lervåg et al. 2016 (10.1177/1745691616635612); Soveri et al. 2017 (10.3758/s13423-016-1217-0); Macnamara et al. 2014 (10.1177/0956797614535810; deliberate practice explains 26% of variance in games, 21% music, 18% sports, 4% education, <1% professions); Macnamara & Burgoyne 2023 (10.1037/bul0000352; growth mindset d = 0.05, n.s. after bias correction); Ciria et al. 2023 (10.1038/s41562-023-01554-4; exercise-cognition d falls 0.22 to 0.05 after bias correction); srs-benchmark README (github.com/open-spaced-repetition/srs-benchmark).

**C: corrected.** Murray, Horner & Göbel 2025 (10.1007/s10648-025-10035-1): published abstract gives testing g = 0.18 (CI crosses zero) and spacing g = 0.28, not the preprint's 0.22 and 0.26. srs-benchmark: 349.9M evaluated reviews from 9,999 users (raw 727M); FSRS-7 about 0.336 in the table variant I saw (A2 said 0.340).

**T: record or title confirmed, content not read.** Kluger & DeNisi 1996 (10.1037/0033-2909.119.2.254); Mullet et al. 2014 (10.1016/j.jarmac.2014.05.001, title only).

**R: report-level only, not re-checked by me.** All other sources cited in reports A1 to A8 and mentioned above, including Cepeda 2006, Latimier 2021, Vaughn 2016, Hartwig 2022, Foster 2019, Wisniewski 2020, Fyfe 2015, Butler 2008 and 2011, Metcalfe 2017, Rhodes & Tauber 2011, Greving & Richter 2018, Little 2012, van Gog & Sweller 2015 (editorial), Ashman 2020, Loibl 2017, Tan 2025, Noetel 2022, Sibley 2022, Shields 2017, Smith 2016, Albulescu 2022, Biwer 2023, Vohs 2021, Sievertsen 2016, Deci 1999, Vaci 2019, Macnamara & Maitra 2019, Sala 2019, Pashler 2008, Nielsen 2013. Treat their numbers as unconfirmed and do not quote them in product copy.
