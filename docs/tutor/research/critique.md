# Completeness critique of the Tutor research round

Basis: I read the intl, corpus and expert SYNTHESIS.md files in full and the other four through the structured summaries supplied in the brief. Checks were against the brief and the summaries, not against primary sources.

## 1. Gaps (most serious first)

1. **Owner-copy licence gate vs the main use case.** The corpus area says the indexer must refuse to embed or send to an LLM anything whose `ai_use` is not allowed. The PhD-level content will be the owner's 50-250 GB of all-rights-reserved textbooks and papers, and RAG sends chunks of them to Claude or Muse Spark. As written, the gate blocks the core feature. Nobody researched provider data terms, retention, or the personal-use basis for sending owned copies to a hosted API, or the local-model alternative. This needs an owner decision, not more literature.
2. **RAG engineering for technical libraries is not researched.** The corpus area covers only licensing. Missing: parsing scanned or LaTeX-heavy PDFs, equations, chemical structures and tables; chunking; embeddings and reranking; OCR cost at 250 GB; and a citation-span check (only the 2023 Liu et al. result is cited). Every area assumes quote-level grounding works.
3. **No integrated scheduler design.** The areas add FSRS atoms, problem families whose variants are not independent evidence, per-skill mastery (BKT/Rasch), cumulative interleaved review (about 20%), hint-capped grades, a load cap, 2-3 new skills per session and randomized holdouts. No area specifies how these compose. FSRS cold start on a single user with few reviews is also unaddressed.
4. **Experiment budget is infeasible for n = 1.** Proposed within-learner tests: retrieval vs restudy, interactive-first vs symbol-first, theme vs no theme, paper vs screen, tutor- vs learner-initiated feedback, self-explanation required vs not, plus an n-back n-of-1. No area gives a power calculation or considers carryover. The n-back area itself says the minimum detectable change is about 18 IQ points. Most of these cannot be resolved by one learner.
5. **"PhD level" is undefined.** No area maps the target: qualifying-exam syllabi, prerequisite graphs per subject, placement diagnostics, time-to-goal, or an external yardstick. The Plan screen has no research behind it. "Science" is not specified (domains lists biology, earth science, upper-division physics and physical chemistry as gaps).
6. **International pedagogy is K-12 only.** The owner is an adult working toward doctoral level. Nothing covers doctoral-level practices (Oxbridge tutorials, Russian problem-solving circles, qualifying-exam preparation, research apprenticeship, seminar and oral-defence formats). The intl area's own rule 15 lists a Danish oral-defence mode as unverified. Its findings are mostly null or confounded.
7. **Initial exposure to new material from textbooks.** The areas cover retrieval, feedback and problem solving. How the owner should first read and understand a dense chapter is thin: reading strategy, note-taking, elaboration, mnemonics and imagery. Only a "paper mode" labelled as hypothesis appears.
8. **Neuro-cognitive content is thin.** The brief asks for it. Only sleep (g about 0.3) is covered. Exercise, attention, consolidation, circadian timing and caffeine are not. No deliberate-practice literature (Ericsson; the intl area cites Macnamara's 4% only in passing).
9. **English is thin.** The rules cover AI writing guardrails and a timed monthly sample. Missing: advanced reading, rhetoric, literary analysis and vocabulary depth. It is unclear whether the owner is an L1 speaker or an ESL learner; that changes the whole track. Tetzlaff 2025 says expertise-reversal evidence is weaker for language learning.
10. **Model and voice reliability in the owner's subjects.** No area measured Opus 5.5, Sonnet 5.5 or Muse Spark 1.3 on proofs, mechanisms, numerics, answer leakage or sycophancy; all failure figures come from 2023-2025 models. Spoken-maths speech-to-text errors, ElevenLabs latency and cost, and spoken vs typed retrieval are untested.
11. **Adherence for a solo adult over years.** Gamification is ruled out, but nothing positive replaces it (autonomy support, habit design, plateau handling). The owner's load, sleep and burnout covariates are planned with no evidence on what to do with them.
12. **Question-bank authoring cost.** Every item needs a verified solution, blind solve by a different model family and a source citation. Nobody estimated the cost or owner time for thousands of PhD-level items.

## 2. Contradictions and inconsistent statuses

1. **Self-explanation.** Learning says selective (after errors or at proficiency). Expert says required on proofs and derivations. Domains says before answering. Intl cites Barbieri 2023 (prompts were a negative moderator) against "require it". One rule is needed per item type, and it should be marked as a hypothesis under test.
2. **Novice default vs diagnostic routing.** Learning says treat the owner as a novice in each new topic (worked example first, gate productive failure). Intl and expert say run a short diagnostic, and use problem-first when prerequisites exist. Tools defaults to symbols-first at PhD level and also proposes interactive-first tests. Decide which default applies before the diagnostic exists.
3. **Interleaving definitions differ.** Learning: after a brief block, confusable problem types only. Intl: cumulative mixed review at about 20% of reviews. Domains: after first blocked exposure. All also say FSRS handles vocabulary-like items, where interleaving hurts (g = -0.39). They agree in spirit but not in parameters.
4. **Hint handling in FSRS.** Learning says log every review. Expert caps hinted or revealed items at Again or Hard, and revealed items reschedule as relearning (D5 said new). Intl says variants of one seed are not independent evidence. No area says what the stored review record looks like.
5. **Confidence scale and timing.** Learning: four levels before feedback. Intl: 0-100 before reveal. Domains: 1-5 with predict-then-reveal. Expert: a rating before each answer. Pick one scale and one moment.
6. **Session length.** Learning: 40-50 minutes with pre-set breaks. Intl: soft stop at about 90 minutes. Bastani's sessions were 90 minutes. Both are labelled defaults; settle on one for the build.
7. **Whether the tutor model sees the answer** is resolved only by an untested tiered design (expert). Tools and domains say "hide until tried" for the learner, which is a different question from the model's context.
8. **Status of the same citations differs by area.**
   - Bastani 2025: verified in corpus and expert. Learning and domains say the erratum was not read. The -17% figure is the single most repeated number in all seven areas.
   - Kestin 2025: intl and expert say 0.63 SD verified from full text. Corpus says the abstract has no effect size, so quote none. Learning excluded it.
   - Murray 2025: learning reports g = 0.18 and 0.28 from the abstract. Corpus says findings rest on one report and are unverified.
   - Tetzlaff 2025 and Sinha & Kapur 0.87 are cited with different confidence.
   Nothing should reach prompts or UI copy until one evidence ledger gives each number a single status.
9. **Chinese open-source premise.** Corpus and intl say licences do not support it. Tools, domains and the intl open questions still carry it as open, although smartedu.cn and OECD terms are genuinely unread. The answer is "no" for what was checked and "unknown" for what was not; the areas should say the same thing.
10. **Inquiry vs explicit instruction.** Tools supports guided discovery with sims (d = 0.30 enhanced) and learning supports explicit first for novices. They are reconciled only if the prediction-gate and "never free-explore" rules are written into the sim specs.
11. **N-back scope.** Every area agrees on no transfer claims. The tension left is effort: an optional n-back, four-weekly transfer probes and an n-of-1 card cost owner time against a 60-minute weekly cap, for a result the n-back area expects to be null.

## 3. Follow-ups worth a second round (most valuable first)

1. **Owner-decision memo on corpus licensing and LLM use** (gap 1, corpus conflict 1): provider terms, local-model option, AGENTS.md amendment, country of residence.
2. **Single evidence ledger.** Read the Bastani erratum, Kestin, Murray and Tetzlaff full text, then fix each number's status (contradiction 8). Cheap and unblocks prompt and UI copy.
3. **Empirical model audit on the owner's real problems**: leak rate, sycophancy, proof, mechanism and numeric error for Opus, Sonnet and Muse Spark, plus spoken-maths transcription. Code and evals, not literature.
4. **Scheduler and measurement design spike**: FSRS cold start, problem families, mastery model, review schema, plus a power analysis that limits the experiments to two or three that one learner can resolve.
5. **RAG for technical PDFs**: parsers for math and chemistry, chunking, rerank, citation-span verification, evaluated on a sample of the owner's own books using `eval-search` with the held-out split.
6. **Define the target**: doctoral-level syllabus and prerequisite maps per subject, placement diagnostics, English-track definition (ask the owner), external benchmarks.
7. **Doctoral-level pedagogy and reading strategy**: tutorial, seminar and qualifying-exam practice; textbook and paper reading methods; deliberate practice.
8. **Neuro-cognitive and adherence sweep**: sleep, exercise, attention, autonomy-supportive design for solo adults.
9. **Low priority**: K-12 Chinese variation teaching, PISA tables, Nordic details. The intl area itself finds little transferable beyond mechanisms already covered.
