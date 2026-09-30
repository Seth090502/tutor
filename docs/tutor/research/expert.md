# Expert-level learning and AI tutoring: verified synthesis

Scope: five reports (D1 AI tutor trials, D2 tutoring and ITS, D3 graduate learning, D4 LLM tutor design, D5 assessment), all read in full. I re-checked 23 load-bearing citations on 2026-09-30 by direct fetch (web search was out of quota). No citation failed outright; three were corrected and several carry caveats.

## Bottom line

1. The only robust, replicated finding is a negative one: unguarded LLM chat raises practice scores and lowers later unassisted performance, and learners do not notice. Guardrails remove the harm. They do not, by themselves, produce a gain.
2. Gains show up when the tutor's pedagogy is the intervention (attempt-first, expert solutions, scaffolding). The evidence is one immediate-test undergraduate physics crossover and one small MBA trial. Nothing is tested at PhD level or on delayed retention.
3. So build Tutor as a measured system: enforce the guard, log assisted and unassisted performance separately, and let the owner's delayed closed-book results decide what stays.

## Verified evidence table

Status key: V = verified as claimed; V* = verified with a caveat; C = corrected; U = not verified by me (taken from a report).

| # | Claim | Source | Status and what I saw |
|---|---|---|---|
| 1 | Unguarded GPT-4: practice +48%, later unassisted exam -17% (-0.054, p<.05). Hint-only tutor: practice +127%, exam -0.4% (-0.004, n.s.). | Bastani et al., PNAS 2025, doi:10.1073/pnas.2422633122 | V (Europe PMC full text). About 1,000 students in grades 9-11, 4 x 90 min, 3 arms. The tutor prompt held correct solutions and common mistakes. GPT-4 answered correctly 51% of the time. GPT Base users did not notice the loss; GPT Tutor users thought they did better. A correction exists and was not read. |
| 2 | Designed AI tutor beat in-class active learning, 0.63 SD (linear), 0.73-1.3 SD (quantile) | Kestin et al., Sci Rep 2025, doi:10.1038/s41598-025-97652-6 | V (full text). N=194 crossover, two intro-physics lessons, immediate post-test, no delayed test, median 49 min. Authors caution it may not transfer to complex synthesis. |
| 3 | Structured AI tutor, graduate adults: +6.63 points (p=.007), voice = text for mastery, voice 2.8x cost | Yang, Van Alstyne, Dellarocas, arXiv 2609.23958 | V* (abstract). Preregistered, N=86 MBA finance. Instructor-graded gain about 2.6 points; gains concentrated in written reasoning. The d=0.53 figure was not seen. |
| 4 | Substituting LLM for exercises lowers understanding per topic; LLMs widen the prior-knowledge gap | Lehmann, Cornelius, Sting, arXiv 2409.09047 | V* (abstract). No aggregate effect. Sample sizes not in the abstract. |
| 5 | Human-AI copilot: +4 pp mastery, +9 pp for weaker tutors | Wang et al., Tutor CoPilot, arXiv 2410.03017 | V. Preregistered, 900 tutors, 1,800 K-12 students. |
| 6 | Expert-supervised LearnLM: transfer 66.2% vs 60.7% | LearnLM Team and Eedi, arXiv 2512.23633 | C. Numbers confirmed, N=165. The abstract says 76.4% of drafts had zero or minimal edits. D1 said 74.4% unedited. Use 76.4% minimal edits. The study is exploratory and every message was human-approved. |
| 7 | GPT-4 reveals the answer about 47% of the time; Claude 3.5-era Sonnet about 5%; expert human tutors about 9% | Maurya et al., MRBench, NAACL 2025, arXiv 2412.09416 | V* (full text). Table 3 lists the share that does not reveal (GPT-4 53.12, Llama-3.1-405B 80.73, Sonnet 94.79, expert 90.62), so D4's readings hold. Older models. The dataset has 192 conversations and 1,596 responses. |
| 8 | Pooled tutoring effect 0.37 SD | Nickow, Oreopoulos, Quan, NBER w27476 | V for 0.37 SD, PreK-12 only. The "96 RCTs" count is not on the abstract page (U). |
| 9 | Human tutoring d=0.79, ITS d=0.76, versus a believed 2.0 and 1.0 | VanLehn 2011, Educ Psychol | V* . The two numbers were confirmed, but the tool paraphrase swapped the labels. The original abstract gives human 0.79 and ITS 0.76. The step/substep breakdown stayed unverified (U). |
| 10 | ITS median 0.66 SD over 50 evaluations; smaller with standardized tests and flawed implementations | Kulik and Fletcher 2016, RER | V (abstract). |
| 11 | Self-explanation prompting g=0.55 (69 effects, 64 studies, 20 moderators) | Bisra et al. 2018, Educ Psychol Rev | V (ERIC record). |
| 12 | Self-explanation training on proof line-to-line logic d=0.95; effect persisted | Hodds, Alcock, Inglis 2014, JRME | V (verbatim abstract). UK undergraduates, transition-to-proof level. |
| 13 | Active learning beats lecture: 0.47 SD over 225 studies | Freeman et al. 2014, PNAS | C. 0.47 SD and 225 studies confirmed. The abstract says lecture students were 1.5 times as likely to fail. D3's "odds ratio 1.95" was not confirmed. Use 1.5x. |
| 14 | Unassisted discovery d=-0.38 versus explicit instruction; enhanced discovery d=+0.30 | Alfieri et al. 2011, J Educ Psychol | V (abstract, 164 studies). Mostly school-age samples. |
| 15 | Problem-first-then-instruction g=0.36 [0.20, 0.51]; reverses in grades 2-5 and for domain-general skills; bias-adjusted 0.87 is not a planning number | Sinha and Kapur 2021, RER | V (abstract, 53 studies). |
| 16 | LLM distractors are valid but miss real student errors | Feng et al., NAACL Findings 2024, arXiv 2404.02124 | V (abstract). K-12 maths. |
| 17 | LLM difficulty labels vs empirical difficulty rho=0.06 (378 items, 7,888 responses) | An and Wang, arXiv 2609.27063 | V. Single data-science class, preprint. |
| 18 | LLM proof judge: human double-grading agreement 90.4%, GPT-5 89.3% single, 90.8% with 5 votes; only 59.5% of o3's correct answers had valid proofs | Dekoninck et al., Open Proof Corpus, arXiv 2506.21621 | V (HTML full text). Competition-style proofs. |
| 19 | Generative search engines: 51.5% of sentences fully supported, 74.5% citation precision | Liu, Zhang, Liang, EMNLP Findings 2023, arXiv 2304.09848 | V. Older systems, not textbook RAG. |
| 20 | Active learning feels worse and works better | Deslauriers et al. 2019, PNAS | V (abstract). Intro physics. |
| 21 | Andragogy research "inconclusive" | Rachal 2002, Adult Educ Q | V (abstract). |
| 22 | Pooled ChatGPT effect g=0.867 vanishes after publication-bias adjustment | Bartoš, Martinková, Wagenmakers 2025, PsyArXiv | V for the preprint and its abstract. D2's claim that the original Wang and Fan meta-analysis was retracted is U (OpenAlex rate-limited me). |
| 23 | "Cognitive debt" from LLM essay writing | Kosmyna et al. 2025, arXiv 2506.08872 | V for existence: N=54, 18 in session 4, EEG, preprint. Contested. Design must not rest on it. |

Citations I did not re-check are listed at the end as U. Rules resting only on them are tagged "reported."

## Conflicts between reports and how I resolved them

1. **Does the tutor model see the solution?** D1, D2 and D4 put the reference solution and misconception text into every tutor turn (Bastani's protective condition). D5 says the tutor model must never see the key, and that a separate check-answer tool returns only a verdict, error category and hint level. Bastani's tutor had solutions in the prompt, the only trial evidence. D5's argument is robustness: a prompt-only "do not reveal" rule can be broken, and the owner can jailbreak their own tutor. Its supporting evidence (Zhao 2026) is weak.
   - Resolution (design inference, untested): during a graded attempt, the tutor model gets the question, the learner's attempt, and a verdict from the deterministic check tool. It also gets pre-written, pre-validated hint text for each rung. It does not get the final answer. After the attempt is closed (correct, given up, or revealed), the full solution and misconception text enter context for explanation. This keeps Bastani's ingredients without giving the model the ability to leak in the middle of an attempt. Run the leak-rate eval on the real models to see whether the stricter split is needed.
2. **Guarded tutors: gain or no gain?** Kestin (+0.63 SD and up) versus Bastani (exam null). Both are valid. Kestin measured an immediate test after a first-exposure lesson. Bastani measured an unassisted exam later. Stronger evidence for "no harm after removal" is Bastani. Stronger evidence for "can speed up first learning" is Kestin. Neither shows a delayed gain. Plan: treat the guard as a floor and get the gain evidence from the owner's own delayed tests.
3. **Voice.** D1 found equal mastery and 2.8x cost in one small preregistered trial; D4 found no outcome evidence. I use D1 for the claim and D4 only for spoken-turn rules, marked as inference.
4. **When a hint is used, how is FSRS graded?** D1: hint above rung 1, explanation or reveal gives Again or Hard only. D4: not Easy after hint 3. D5: revealed items reschedule as new. No study tested any of this. I adopt D1's strict rule because it keeps FSRS inputs to unassisted retrieval, which is what the algorithm assumes.
5. **Mastery thresholds.** D2: advance at about 80-90% on a parallel retest. D5: select items at 60-75% predicted success, and label skills with under about 10 scored items unverified. Different jobs (gate versus selection), so both stand. All numbers are assumptions.
6. **Self-explanation versus worked examples.** Self-explanation prompts have g=0.55 (verified), but D3 also reports that adding prompts to worked examples can reduce the effect (Barbieri 2023, U). Use self-explanation on proofs and derivations the learner works through, not as a mandatory layer on every worked example.
7. **Meta-analytic hype.** D1 says "reportedly" erased, D2 says "retracted." I can confirm only the Bartoš preprint. Cite no pooled AI effect.
8. **LearnLM edit rate.** See table row 6.
9. **Expertise reversal.** D2 verified Kalyuga 2003 and D4 could not open it. Lehmann's prior-knowledge gap points the same way. Treated as moderate, with a within-learner check in Evals.

## Rules for Tutor, ranked by evidence strength times impact

Tags: [P] voice/chat tutor prompt, [S] scheduling, [Q] question generation, [F] feature, [U] UI, [M] measurement, [C] corpus.

1. **No open "ask anything" mode as the main surface. Attempt first, always.** Require a learner attempt or prediction before any hint. Offer free chat only as a labeled lookup mode that never feeds mastery. [P][F][U] Evidence: strong for the harm (row 1), moderate for the benefit (rows 2, 3). Impact: highest.
2. **Headline metric is delayed, unassisted, AI-off performance on parallel items (3 days, 2-4 weeks).** Show assisted and unassisted scores separately everywhere, plus hint-dependency rate and reveal rate. Do not use chat time, satisfaction or self-rated learning as outcomes. [M][U] Evidence: rows 1, 20, strong. Impact: highest, because every other rule is tested by it.
3. **FSRS learns only from unassisted retrieval.** Hint above rung 1, an explanation, or a reveal caps the grade at Again or Hard. A revealed item reschedules as relearning. Keep FSRS (0.9) for item memory, and a separate per-skill mastery estimate for "ready to move on." [S] Evidence: design inference from row 1 (practice inflation). Impact: high, cheap.
4. **Every item ships a verified reference solution, steps and a misconception tag. Answer checking is deterministic where possible** (stored key, CAS, unit checker). An LLM grades free text only against a rubric and low-confidence grades go to the owner. [Q][P][M] Evidence: row 1 (51% base correctness, older model), rows 16, 18. Impact: high.
5. **Server-side hint ladder.** Nudge, hint, step, worked example, answer, with the maximum rung set by policy code, not the LLM. Add a deterministic leak filter on every tutor turn and log the leak rate. After about 3 failed rungs, show a worked example and immediately give an isomorphic problem. [P][F][M] Evidence: rows 1, 7; ladder details are design inference. See conflict 1 for what the model may see.
6. **Self-explanation as a required move on proofs and derivations** ("what justifies this line?"), with predict-then-reveal for results. [P][F] Evidence: rows 11, 12 (strong for g=0.55; moderate for proofs; not tested at graduate level).
7. **Feedback is task-level, specific, and correction-first.** Name the error in one sentence and require a redo. Give no praise unless the check says correct. Hold the correction under pushback. One idea and one question per turn. [P] Evidence: Wisniewski and Kluger and DeNisi (reported), Sharma on sycophancy (reported). Moderate.
8. **Confidence rating before each answer; calibration chart.** [U][M] Evidence: rows 1 and 20 (felt learning misleads). Moderate; cheap.
9. **Adapt to prior knowledge.** If prerequisites are present, pose the problem first and teach after. If they are absent, use a worked example first. Fade scaffolds as unassisted accuracy rises. Default to guided discovery. [P][S] Evidence: rows 14, 15 (moderate, mostly school-age), expertise reversal (moderate). Validate within-learner.
10. **Question-generation pipeline.** Generate from a cited source chunk. Blind-solve with a different model family and quarantine key disagreements. Take misconceptions and distractors from the owner's own wrong answers, not from the model's imagination. Do not use LLM difficulty labels. Prefer short-answer, worked and proof items at advanced level. Add a twin item and a transfer item per skill. [Q] Evidence: rows 16, 17 (moderate); key-error rate (Gema, reported).
11. **Grade proofs with multiple judge passes.** Use 3-5 passes with majority vote, step labels (valid, gap, error), an owner override, and a gold set of the owner's adjudicated answers. Track judge-versus-owner agreement per domain and stop using the judge where agreement is poor (the 0.7 floor is an assumption). [M][Q] Evidence: row 18 (moderate; competition proofs).
12. **Weekly closed-book, AI-off checkpoints per domain** on the Plan screen. [F][M] Evidence: follows from rows 1 and 20.
13. **Grounding with quote-level citations.** Show the exact span, verify programmatically that it exists in the retrieved text, retrieve few passages with the best first and last, and have the tutor say when retrieval is weak and label model-memory statements. [P][C][U] Evidence: row 19 (strong, but older, non-textbook systems); "lost in the middle" (reported).
14. **Voice.** Use it for oral explanation, teach-back and low-friction retrieval. Expect equal mastery to text and about 2.8x cost per engaged minute. Keep about two sentences and one question per turn, put equations on screen, acknowledge quickly while tools run, and allow interruption. [P][F] Evidence: row 3 (moderate-weak) for cost and equivalence; spoken-turn rules are inference.
15. **Knowledge tracing: start simple.** Use BKT or a PFA/IRT-style logistic model with hint and time logging. Adopt a deep model only if it beats the baseline on time-ordered splits of the owner's own data with no leakage. A Bayesian Rasch estimate per skill for item selection is fine, with credible intervals and no single global "mastery %." [S] Evidence: D2 (verified by D2, reported here); D5 (inference).
16. **Paper mode and Proof mode in Library.** Three passes: skeleton and claim/evidence, figures and methods, reconstruct the argument and name a weakness. Proof mode shows one line at a time. [F][C] Evidence: weak (Keshav, apprenticeship, CREATE are frameworks without effect sizes). Label as hypotheses and measure them.
17. **Evals screen.** Per-turn rubric (MRBench's 8 dimensions, LearnLM's 25 items, reported), headline answer-leak and false-praise rates, a regression set of the owner's real mistakes plus pushback prompts, LLM judge as a screen only. Run it on the actual models (Opus, Sonnet, Muse Spark) before trusting any 2023-2025 leak or sycophancy rate. [M] Evidence: rows 7, 18; moderate.
18. **Within-learner crossovers** (guarded tutor versus free chat, or AI versus AI-off blocks, by topic) scored on delayed tests. [M] Design only; with n=1 this is monitoring, not proof.
19. **Corpus: every generated item carries `source_chunk_id`; worked solutions come from the owner's notes or public-domain/CC-BY text.** [C][Q] Serves rule 4 and the project's licensing rule.

## What not to build

- Promises of "2 sigma" or "PhD in X months." Bloom's figure was bundled with mastery learning on narrow tests; modern pooled tutoring is about 0.37 SD (row 8), and I found no adult graduate tutoring trial.
- A reveal-answer button that counts as a successful review.
- Unrestricted chat as the default mode (Bastani's harmful arm).
- LLM-only grading or LLM-only solution checking in maths or chemistry. No proof grading without an owner override.
- An LLM difficulty labeler feeding the scheduler (row 17).
- Same-model generate, answer and grade. A single global mastery percentage.
- Deep-learning knowledge tracing as a core dependency.
- Designs justified by pooled "AI boosts learning" numbers or by "cognitive debt" (rows 22, 23).
- EEG or neuro-adaptive features. NeuroChat had N=24 and measured engagement only (reported).
- Andragogy-based personalization, learning-styles adaptation, a practice-hours mastery proxy, a threshold-concept detector (row 21; Pashler, Macnamara, Barradell reported).
- Engagement, streak and satisfaction as primary outcomes (rows 1, 20).
- Decorative citations, a custom fine-tuned tutor, "sounds human" as a goal, claims that Tutor replaces research apprenticeship.

## Open questions

- No adult or PhD-level RCT of LLM tutoring appears in these reports; the owner's target extrapolates from undergraduate physics, high-school maths and one MBA course.
- No delayed-retention data for the two strongest positive trials.
- Whether current frontier models still leak or flatter at 2023-2025 rates is unmeasured.
- Voice transcription errors on spoken mathematics could confound voice-graded answers.
- No ablation shows which guardrail component carries the effect.

## Citation list with verification status

V = verified as claimed, V* = verified with caveat, C = corrected, U = not verified by me.

- Bastani et al. 2025, PNAS, doi:10.1073/pnas.2422633122 (V)
- Kestin et al. 2025, Sci Rep, doi:10.1038/s41598-025-97652-6 (V)
- Yang, Van Alstyne, Dellarocas, arXiv 2609.23958 (V*)
- Lehmann, Cornelius, Sting, arXiv 2409.09047 (V*)
- Wang et al., Tutor CoPilot, arXiv 2410.03017 (V)
- LearnLM Team and Eedi, arXiv 2512.23633 (C)
- Maurya et al., arXiv 2412.09416 (V*)
- Nickow, Oreopoulos, Quan, NBER w27476 (V, study count U)
- VanLehn 2011, doi:10.1080/00461520.2011.611369 (V*)
- Kulik and Fletcher 2016, doi:10.3102/0034654315581420 (V)
- Bisra et al. 2018, doi:10.1007/s10648-018-9434-x (V)
- Hodds, Alcock, Inglis 2014, doi:10.5951/jresematheduc.45.1.0062 (V)
- Freeman et al. 2014, doi:10.1073/pnas.1319030111 (C)
- Alfieri et al. 2011, doi:10.1037/a0021017 (V)
- Sinha and Kapur 2021, doi:10.3102/00346543211019105 (V)
- Feng et al. 2024, arXiv 2404.02124 (V)
- An and Wang, arXiv 2609.27063 (V)
- Dekoninck et al., arXiv 2506.21621 (V)
- Liu, Zhang, Liang 2023, arXiv 2304.09848 (V)
- Deslauriers et al. 2019, doi:10.1073/pnas.1821936116 (V)
- Rachal 2002, doi:10.1177/0741713602052003004 (V)
- Bartoš, Martinková, Wagenmakers 2025, doi:10.31234/osf.io/8vs32_v1 (V)
- Kosmyna et al. 2025, arXiv 2506.08872 (V, contested)
- Reported only (U): Henkel 2024 (arXiv 2402.09809); World Bank WP 11125; Nie et al. (2407.09975); Chung et al. (2608.16907); Pisan (2608.12292); Wisniewski 2019; Kluger and DeNisi 1996; Chi 2001; Chi and Wylie 2014; Kalyuga 2003; Khajah (1604.02416); pyKT (2206.11460); Jurenka (2407.12687); LearnLM arena (2505.24477); Bridge (2310.10648); Wallat (2412.18004); Magesh (2405.20362); Gema (2406.04127); Shimmei (2505.05815); Zheng (2306.05685); Panickssery (2404.13076); Hofmann (2509.11106); Macnamara 2014; Barbieri 2023; Wang and Fan 2025 retraction claim.
