# Completeness critique, research round 2

Basis: I read all four round-2 SYNTHESIS.md files in full, the round-1 critique.md, and grepped PEDAGOGY.md, corpus.md, domains.md and expert.md. I did not re-check primary sources. The arithmetic in gap 1 is mine.

## 1. Gaps, most serious first

1. **No owner time budget across areas.** Round 2 adds 7/30-day retests on every domain, a monthly no-AI language hold-out, forms A/B/C batteries every 6-8 weeks, calibration, mock quals, coverage-meter reading, and three card types per language word. Nobody sums it. Using engineering's simulated 7.9 reviews per card-year at 0.90 retention (workload, assuming FSRS is the true model), an 8,000-word vocabulary at three cards per word is about 190,000 reviews a year, roughly 520 a day, before any math or science. Split cards per skill is the languages area's own design choice, not a cited result. A workload cap and a per-word rather than per-skill default need deciding.
2. **The owner's own library vs the corpus rules.** Engineering plans to parse and embed 50-250 GB with hosted engines. Curricula puts Rudin, Hatcher, Dummit-Foote and similar at "metadata and own notes only". AGENTS.md forbids vendoring copyrighted books. The round-1 gap (may owned copies go to Anthropic, ElevenLabs or a parser, and on what terms) is only half answered: engineering covers Mathpix's 30-day retention and says other vendors are unverified. Nobody checked Anthropic or ElevenLabs terms for owned copies, or how the OpenStax LLM ban interacts with a user's own copy. Owner decision plus a terms check, not more literature.
3. **"PhD level" is still undefined outside math.** Only Berkeley and Harvard math formats were opened. Physics, chemistry, biology, earth and space science, and English ladders are worker syntheses. Also absent: research skills (reading and writing papers, experimental design, data analysis, scientific programming), lab and field work, and first-round gap 6 (tutorial, seminar, oral-defence practice). Coursework mastery is not research readiness.
4. **English and linguistics remain thin even though the owner is an L1 speaker (which closes one round-1 question).** The only English evidence is Graham & Perin (grades 4-12), Dodell-Feder (g about 0.15) and Steiss. Nothing on doctoral writing, rhetoric or formal linguistics pedagogy. Literary corpus is restricted to pre-1930 public domain, so modern criticism is reachable only through the owner's copies (see gap 2).
5. **Language track has no curriculum or goal.** Language order follows speech-recognition tiers, not the owner's purpose. Missing: reading-only classical languages (Latin, Greek, Hebrew; the languages area itself found no pronunciation API for them), scripts and morphology-heavy languages, time-to-target ranges (FSI figures failed verification), and whether any language is tied to the English PhD track (curricula open question 5).
6. **Voice for technical subjects is untested.** Round 2 tested ElevenLabs only for language. Spoken math and chemistry transcription, latency, real per-minute agent cost (only "Speech Engine $0.08/min" seen), and spoken vs typed retrieval are still open from round 1.
7. **Model reliability is still unmeasured.** Curricula's 36.9% (PHYBench) and 18-55% fabrication figures are 2023-2025 models. Every area defers to an in-house eval on current Opus and Sonnet. It has not been built, and it gates every relaxation of the verifier ladder.
8. **Initial reading of dense chapters and papers** (round-1 gap 7) got no round-2 coverage. Cognition's staged paper reading is tagged weak.
9. **Adherence for a solo adult** (round-1 gap 11): streaks are ruled out everywhere; nothing replaces them.
10. **Question-bank authoring cost** (round-1 gap 12) is worse: each item now needs CAS key, two-family blind solve, span check, second-pass validation. No per-item owner-time or API cost estimate.
11. **Chemistry structures, and equation-heavy retrieval through pdfium plus 400-token chunks,** are untested. The gold set is designed but not built.

## 2. Contradictions

1. **Prerequisite gating.** Curricula ranks "stage opens on prerequisite mastery, with owner override" and math nodes "gate" physics and chemistry. Engineering lists hard gating under "do not build" and makes edges advisory. Pick advisory with a diagnostic (engineering's version, since no validated dependency model exists).
2. **Two mastery definitions.** Curricula: 70% delayed, unaided, novel-item accuracy at 7 and 30 days. Engineering: Beta posterior with about 15-20 unassisted attempts over several days, assisted attempts never counted. Cognition adds stable batteries every 6-8 weeks. These can disagree for the same skill.
3. **Testing effect in math.** Cognition calls retrieval practice strong (Yang g 0.499, classroom, all subjects) and makes it the main schedule. Engineering cites Murray 2025 (preprint): math testing vs restudy g 0.22 with a confidence interval crossing zero, spacing g 0.26, "may be smaller than other domains". Math is the largest domain; the plan should use the smaller figure.
4. **Experiment budget.** Engineering caps personal experiments at 2-3, about 160-320 items per arm, effects under about 6 points unresolvable. Languages proposes several (3 voices vs 1, alternating-week recasts vs prompts), cognition allows worked-example vs attempt-first, PEDAGOGY has n-back and interleaving. Total exceeds the cap and overlaps skills.
5. **Confidence scale.** PEDAGOGY uses 1-4; cognition says 0-100 with a Brier dashboard. Round-1 critique flagged this; round 2 adds a second voice rather than resolving it.
6. **Worked examples vs "no answer before attempt".** Cognition allows a labelled worked-example mode; languages make attempt-first a hard rule; curricula says no full solution before a written attempt. Needs a per-item-type rule and a log field.
7. **Interleaving parameters.** Curricula: after one blocked intro, confusable families. Engineering: about 30% mixed-family queue. Round-1 said about 20%. Languages: separate card states. FSRS for vocabulary (interleaving g = -0.39) must stay blocked-neutral.
8. **Bastani extrapolation.** Three areas rely on the -17% (unguarded arm only, high-school math). The languages synthesis applies it to language and the correction is unread in all four areas. Keep as design direction, labelled.
9. **FSRS-6 vs FSRS-7 and FSRS for families/vocabulary.** Languages build on FSRS for vocabulary; engineering says calibration for family-level skills and language vocabulary is untested, and FSRS evidence is recall prediction only.
10. **Sleep rule.** Cognition ships a weak-inference default that changes scheduling while also saying covariates are descriptive only.

## 3. Follow-ups, most valuable first

1. Owner-decision memo: owned-library processing (vendors, terms, retention), NC-SA in the private index, OpenStax, Lean, target programs, first languages.
2. Build the in-house eval: current models on the owner's real problems (proof, mechanism, numeric, leak, sycophancy), including spoken-math transcription.
3. Corpus inventory plus 150-200 page gold set, then the parser bake-off (measured before any spend).
4. One integrated design doc: mastery definition, single confidence scale, hint and review schema (engineering C4 fields), workload cap, experiment register limited to 2-3.
5. Open real target syllabi (the owner names programs) and write one prerequisite-graph pilot for math; add a research-skills track.
6. Small recorded test of ElevenLabs Scribe and agent behavior on the owner's accented beginner speech, plus a native-checked 50-sentence gold set for the first language.
7. Read the unread items: Bastani correction, Murray full text, Xu 2024 erratum, OpenStax Psychology copyright page.
8. Adherence design and the English/linguistics doctoral-writing literature sweep.
