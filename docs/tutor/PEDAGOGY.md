# Teaching method

The rules the tutor follows. Evidence: the verified syntheses in `docs/tutor/research/` (46-agent study,
2026-09-30). Where the areas disagreed, the decision below settles it. Effect sizes are priors from
children, undergraduates and lab samples. No study covers a PhD-level adult learning alone, so Tutor measures
its own effect on its one learner (Measurement below).

## Core loop

1. **Attempt first.** There is no ask-anything answer mode. The learner attempts before any explanation,
   hint or reveal. Unguarded AI help raised practice scores and cut later unaided performance, and
   guardrails mostly removed the harm (Bastani et al. 2025; erratum 10.1073/pnas.2518204122 still unread,
   so the "17%" figure stays out of product copy).
2. **Confidence before feedback.** For retrieval items the learner rates confidence first: 1 guess,
   2 unsure, 3 fairly sure, 4 certain. Voice asks "how sure, one to four?". Confidence routes the feedback
   and feeds the calibration chart. It never changes FSRS intervals.
3. **Graded hint ladder.** The rungs are: 1 nudge, 2 hint, 3 key step, 4 worked example, 5 answer.
   - Rungs come from the question's pre-written `hints` when it has them. Otherwise the tutor writes a
     conceptual nudge without the answer.
   - Rung ceilings are enforced by the server (contract C7), not by the prompt.
   - After three failed rungs the tutor gives a worked example plus a fresh isomorphic problem, not the
     original answer.
4. **Elaborated feedback that names the misconception.** Never right/wrong only: elaborated feedback
   g 0.49 vs right/wrong 0.05 (Van der Kleij 2015).
   - Use refutation form: state the wrong idea, why it fails, and the correct idea (g 0.41, Schroeder &
     Kucera 2022).
   - One idea and one question per turn.
   - Hold the correction under pushback (anti-sycophancy).
5. **High-confidence errors** (confidence 3-4, wrong): immediate re-study, then a near-transfer item later
   in the same session. Hypercorrection is shown for facts, so this stays a routing rule, not a promise.
6. **The tutor never sees the answer mid-attempt.** During a graded attempt the model gets the verdict
   from the check tool and the permitted rung text only. The solution, explanation and misconception enter
   its context only after the attempt closes. Answer checking is deterministic: key, CAS, unit or
   structure checker. The LLM grades free text only against a rubric; low-confidence grades go to the
   learner, and the tutor says "I cannot verify this step" rather than bluff.

## Two practice paths

| Material | Path | Scheduling |
|---|---|---|
| Atoms: facts, definitions, vocabulary, notation, named results | Retrieval with feedback, successive relearning: about 3 correct recalls in the first session, then one per FSRS review | FSRS, desired retention 0.9 (a tunable default; no study validates 0.9 itself) |
| Procedures, derivations, proofs, multi-step problems | Worked example, then completion (faded steps), then full problems with fresh variants of the same family | By mastery per skill; variants of one seed are **not** independent FSRS evidence |
| Writing (English) | Draft, then feedback on 2-3 global priorities, then revision. The tutor never writes the learner's prose; model text only after a revision | Deliberate practice sessions, not FSRS; a monthly timed AI-off writing sample |

**FSRS learns only from unassisted retrieval.** Rating per attempt, computed by the server:

- correct, no hint or rung 1 only: **Good**;
- correct after rung 2-3: **Hard**;
- rung 4-5 or wrong: **Again** (relearning).

## Stage policy per topic (resolves "novice first" vs "problem first")

- Entering a topic runs a 3-5 item diagnostic, and the topic's stage is set per skill from it. With no
  diagnostic, the stage is **novice**.
  - **Novice:** worked example first; no productive failure.
  - **Developing:** completion problems with fading.
  - **Proficient:** problem-first. A productive-failure opener is allowed (g 0.36, contested), then
    consolidation.
- Fading follows unassisted accuracy (expertise reversal: Tetzlaff 2025, weaker in humanities). The
  English track keeps its own fading rule and tests it.

## Self-explanation (resolves the conflict)

- **Required** on proofs and derivations: proof comprehension d 0.95 in Hodds 2014 Exp 1 (verified abstract
  claim); self-explanation g 0.55 (Bisra 2018). Also required after an error.
- **Not prompted** while studying a math worked example, where prompts lowered the worked-example effect
  (Barbieri 2023).
- Predict-then-reveal for demonstrations and simulations.

## Interleaving

A new procedure gets one short blocked introduction. After that, reviews of problem families mix
confusable types, starting with about 30% of problem reviews mixed (Brunmair 2019 math g 0.34; Rohrer 2020
d 0.83, grade 7). Atoms are never deliberately interleaved: for vocabulary-like pairs interleaving hurt
(g -0.39).

## What we take from top-performing systems

The data are K-12 and mostly confounded, so we take mechanisms, not national labels (see `research/intl.md`):

- **Comparison before rule.** 2-3 varied cases plus a near-miss non-example; the learner names the
  invariant, then the rule is stated (variation teaching; Alfieri 2013 d 0.50). Only after the
  prerequisite check passes.
- **One rich problem, then summary.** One rich problem per unit. The learner writes the summary before the
  tutor's (Japanese structured problem solving, adapted). Summaries and insights become FSRS atoms.
- **Coherence and mastery.** Prerequisite maps, move on at mastery, and cumulative mixed review
  (Shanghai/Singapore-style mastery; the English RCTs found +0.06 to +0.10 SD, so small).
- **Concrete-pictorial-abstract and bar models** for novices only. At PhD level the default is symbols
  first.
- **Paper path** (print, read, then a next-day closed-book quiz) as a measured option, not a mandate
  (Delgado 2018 g -0.21 for screens).
- **Honest claims.** The Nordics are not a top-performing block and Finland has declined. The tutor makes no
  "how Japan or China teaches" claim without a retrieved document.

## Sessions

- 45-minute blocks with a suggested break. Soft stop at 90 minutes a day, configurable.
- No points, streaks or XP.
- Short exposition, then a question. Interpolated questions double as the attention device (Szpunar 2013).
- An optional sleep and fatigue check-in adjusts load only.

## Voice

Voice is for retrieval, teach-back and short Socratic turns. Long derivations go to the screen. Spoken
maths may be mis-transcribed, so answers that need notation are typed or confirmed.

## Measurement (the headline metric)

- **Headline metric:** delayed (7 and 30 days) **unassisted** accuracy on parallel items the tutor never
  used for teaching.
- **Always separate:** assisted and unassisted results, hint dependence, reveal rate, calibration
  (confidence vs correctness) and minutes to criterion. Never satisfaction (Deslauriers 2019).
- **Experiments:** at most two or three personal (n=1) experiments at a time, each with a stated minimum
  detectable change. Differences under about 0.2 SD are noise. The first two are the n-back
  experiment (opt-in, `NBACK.md`) and interleaved vs blocked review for one math family.
- **Tutor behaviour evals:** answer-leak rate, sycophancy, and proof, mechanism and numeric error rates on
  the learner's own problems for Opus 5.5, Sonnet 5.5 and Muse Spark 1.3. These gate prompt changes.

## Question bank

- **Schema target:**
  - atomic skill; explanation; misconception; `hints` (ladder);
  - prerequisites; `confusable_with`; variant family (seed, varied dimension, invariant);
  - reference solution (verified); source citation (chunk id).
- **Format:** short answer by default. Multiple choice only with misconception-based distractors, taken
  from the learner's own wrong answers when available.
- **Generated items** must cite a source chunk and be blind-solved by a different model family. LLM
  difficulty labels are ignored (rho 0.06).

## Subject worlds

Theme the app chrome only: accent, icons, header motif. Learning panes (question, explanation, reading,
writing) stay neutral and identical across subjects. There is no decorative imagery in learning panes
(seductive details g -0.16), and themes are never presented as memory aids. Context-dependent memory
effects are small and fragile (Smith & Vela 2001, d about 0.25, secondary source).

## Not building

- An open answer chat, answer-reveal review credit, points, streaks or XP.
- Learning-style profiles, 2-sigma or "brain" claims.
- Free-explore simulation rooms.
- Themes as memory aids.
- n-back as a study strategy or a gate on anything.
