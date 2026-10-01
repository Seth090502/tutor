# Eval and hill-climbing methodology for the vault and tutor

**Sources** (read 2026-10-01):
- Anthropic, "Demystifying evals for AI agents" (engineering blog, 2026-01-09).
- "Quantifying infrastructure noise in agentic coding evals" (2026-02-05).
- The claude-api skill's `build-eval.md`, `eval-hillclimb.md` and `eval-audit.md` (bundled with Claude Code 2.1.285),
  which are the operational versions of the same method.
- If the owner meant a newer post, its link should be added here and this file reconciled with it.

**The rule this whole file serves:** the number that matters is the **held-out test-set delta against baseline,
with a confidence interval**. A score is not taken at face value until someone has read transcripts.

## 1. Terms (from the article)

| Term | Meaning here |
|---|---|
| Task (case) | One question with defined inputs and success criteria, for example a learner question plus the gold addresses and the checkable claims of a correct answer |
| Trial (rep) | One attempt at a task. Outputs vary, so each task runs several times |
| Grader | Logic that scores one aspect. A task has several graders, each with assertions |
| Transcript | Every turn, tool call, tool result and output of a trial |
| Outcome | The final state: the tutor's answer, with its citations |
| Eval harness | Our Rust runner (`vault-eval`): runs tasks concurrently, records everything, grades, aggregates |
| Agent harness | `tutor-runtime`: the locked-down headless Claude Code or OpenCode session plus libraryd |
| Suite | A set of tasks for one capability (retrieval, citation, code navigation, refusal when absent, …) |

## 2. Building the cases

- **Start with 20-50 tasks drawn from real use and real failures,** then grow.
  - **Sources, in order:** real tutoring transcripts, then questions the tutor got wrong, then the owner's 5-10
    hand-written questions, then cases synthesised from the pilot and anchored on those.
- **Quality bar:**
  - Two domain experts would independently reach the same verdict.
  - Everything the grader checks is stated in the task.
  - **Every task has a reference solution:** gold addresses and a correct answer that passes every grader, which
    proves the task is solvable.
- **Balance:** include should-not-fire cases. The answer isn't in the vault, and the correct behaviour is to say
  so.
- **Hard cases** are included only with a stated reason they are hard. Never select cases because the current
  system fails them.
- **Categories** (weighted towards psychology and cognitive science):
  - explain a concept;
  - find the landmark study;
  - exact quote;
  - definition;
  - compare findings across papers;
  - follow a citation chain;
  - maths and notation;
  - code in books;
  - **navigate and synthesise across source repositories** (the "10 state-of-the-art codebases" use case);
  - navigate within a book;
  - Chinese sources;
  - not in the vault.
- **Capability against regression:**
  - **Capability suites** start with a low pass rate (hard synthesis, multi-repository code questions).
  - **The regression suite** (layer A invariants plus simple lookups) stays near 100%.
  - Capability tasks that saturate graduate into regression.
- **A 0% pass rate across many trials usually means a broken task,** not an incapable agent. Investigate it
  before counting it.

## 3. Graders: the cheapest that fits

| What | Grader |
|---|---|
| Citation validity: the address resolves, and quoted text appears verbatim in the cited lines | **Code** (exact). Target 100% |
| Retrieval: gold address recall@10 and @50, nDCG@10, ANN recall against exact search | **Code** |
| Should-not-fire: the answer states the material is absent and makes no uncited claims | Code check for uncited claims, plus a model check of the wording |
| Citation support: the cited text supports the claim it is attached to | **Model**, as checkable claims, calibrated against the owner's verdicts |
| Answer correctness on open questions | **Model**: a list of checkable claims, not a 1-5 scale. Pairwise and blind against the baseline, in random order |
| Teaching quality (if the owner adds the pedagogy rubric) | Model, as checkable claims |

- **Partial credit:** for example, the right document but the wrong section.
- **The judge model is never the tutor's model.**
- **Validation before trusting the judge:**
  - grade a handful of cases and show the owner the grades with their transcripts;
  - run the judge twice on the same outputs and report any verdicts that changed;
  - read the judge's reasoning on the lowest-scoring baseline cases.

## 4. What "97-100% accurate" means, and what it costs to prove

**Targets** (owner, 2026-10-01):

| Metric | Target | Why it is reachable |
|---|---|---|
| Citation validity | **100%** | Structural, not hoped for. `tutor-runtime` checks every citation before the answer reaches the learner (section 7) |
| Citation support | **≥ 97%** | Judge-graded |
| Answer correctness on gold-answer cases | **≥ 97%** | |
| Correct "not in the vault" behaviour | **≥ 97%** | |
| Retrieval recall@10 on gold addresses | **≥ 0.95** | |

**Measured as pass^k as well as pass@1.** The tutor must be right *every* time, so consistency matters. pass^k
is the probability that all k trials succeed; it is reported for k = 3.

**Statistics:**
- The 95% CI half-width for a rate p is 1.96·√(p(1-p)/N).
- At p = 0.97 that is ±1.5 points at N ≈ 500 graded trials, and ±2 points at N ≈ 280.
- **So claiming 97% credibly needs roughly 250 cases × 2 reps** on the test split.
- Every report states the run size (cases × reps × configurations) with the interval.
- Small suites are labelled **directional**.

## 5. Running clean, comparable trials

- **Isolation:** each trial starts clean. That means a fresh locked-down session (`--no-session-persistence`),
  reset workspaces and result caches, and a pinned index snapshot. The eval directory is unreachable
  structurally (canary-tested).
- **Every result line records the full configuration:**
  - deployment stage and machine;
  - runtime and version (Claude Code 2.1.286 / OpenCode 1.18.33);
  - **the model actually served** (from the session's `system/init`, never from config, because Claude Code silently
    falls back);
  - effort, retrieval models and their placement, interface;
  - index version, corpus size, and tool-description hash.
- **Infrastructure noise** (2026-02-05 article):
  - timeouts, rate limits, usage-limit stops, refusals and harness errors are recorded per attempt, and
    classified as refusal, harness or serving error, timeout, or genuine failure;
  - they are excluded from capability denominators by rules written before the scores exist;
  - a round whose error profile differs grossly from baseline is voided and rerun.
- **Performance runs:** warm and cold measured separately, background jobs paused, machine state recorded.

## 6. The hill-climb loop (Phases 5 and 7)

**Step 0: a runnable eval.** No eval, no hill-climb.

**Step 0.5: prove the eval can be climbed, before round 1:**
1. **Noise floor against headroom against the smallest worthwhile gain.** Measure the noise floor with repeats.
   If noise exceeds the smallest gain worth keeping, add reps or cases first, and tell the owner.
2. **Wiring probe:** the vault tools actually reach the session (the self-test), and a known lookup returns its
   gold address.
3. **Recompute the headline from raw per-case rows,** never from an aggregate field.
4. **Spot-check the grader** on the worst baseline cases, and fix wrong gold answers before round 1.
5. **Verify the served model** from responses.
6. **Measure build variance** for anything generated between the lever and the score (contextual enrichment,
   summaries, an index rebuilt with new chunking). Rebuild 2-3 times with no change, and use that spread as the
   floor a change must clear.

**Step 1: goal and scope.**
- **Goal:** quality within latency targets by default; latency and throughput at equal quality only in Stage 2.
- **Tunable:** TUTOR.md, tool descriptions, retrieval parameters (k, RRF constant, rerank depth, chunk size,
  header format, cache and batch sizes), model sizes and placement, vector-index parameters, tutor model and
  effort.
- **Off-limits:** pipeline code changes (those belong in build phases) and the owner's engine.

**Step 2: stopping condition.** Stop at a plateau: fewer than delta for 3 consecutive rounds, with delta above the
noise floor. The budget is a guide; offer to extend when the score is still climbing.

**Step 3: split, reps, baseline.**
- Split at random, stratified by category tag, **never by score**: about 70% train, 30% test.
- Check that the train and test baseline means agree within noise; re-draw if they don't.
- Record IDs in `_state.json`.
- The baseline uses the full set at the chosen reps, with an environment fingerprint.
- When time separates rounds, include a no-change control.

**Step 4: each round is analyse, apply, run, record:**
- **Analyse:** a fresh analyser subagent reads **only train transcripts.** The lead sees scores only, which is the
  data-isolation guarantee. Test transcripts are never read by anyone during tuning.
- **One root-cause change per round.** Rewrite the responsible section or add the missing rule; describe the
  failure *behaviour*. **Never paste failing case content** into prompts, tool descriptions or code.
- Only spend a round on changes that can clear the noise floor.
- **De-fluff every change:** cut platitudes and anything that restates default behaviour.
- **Gates before a full pass:** a premise probe for config levers, a printout of the resolved scope (cases × reps
  × model × cost), and a one-case canary.
- **Keep if** train and test both improve beyond noise. **Revert if** train improves while test stays flat
  (overfit), or anything regresses (including latency targets in Stage 2).
- **Track mechanism engagement** separately from score: tool-call rates, workspace use, how often grep
  verification ran.
- **Treat implausible jumps as suspicious** until hand-checked.

**Step 4.5: a stall** (2-3 rounds inside noise). Make no edit that round. Sort every remaining train failure into:
- an artifact gap (keep going);
- grader disagreement (fix it, then re-grade all variants);
- harness or infrastructure;
- structural: content exists but isn't reached, an **extraction problem**, or retrieval routing;
- variance.

Fix eval and extraction problems first. Only real capability gaps go into further rounds.

**Step 5: report.**
- Test delta against baseline with confidence intervals.
- If it is within noise, say so and recommend against keeping the change.
- Each applied change tagged `[REQUIRED]` or `[TUNE]`.
- A failure taxonomy, 2-3 before-and-after transcript pairs, and what to try next.

## 7. Structural accuracy: the runtime citation check (`tutor-runtime`)

The 100% citation-validity target is guaranteed by construction, not by asking the model nicely.

1. After the tutor drafts an answer, `tutor-runtime` extracts every address and every quoted span.
2. It resolves each address through libraryd (`read`) and checks that each quote appears verbatim, after the
   canonical normalisation, in the cited lines. Code needs an exact line match.
3. If any check fails, the session gets one repair turn naming the exact failures. At most two repair rounds.
4. Anything still failing is shown to the learner marked **unverified**, never silently.
5. All of this is logged and counted in the eval: repair rate, and residual unverified citations, which should
   be 0.

## 8. Files and reports

The Rust runner writes the skill's file contract:
- `_state.json`;
- `vN/results.jsonl` (per case and rep: grade dict, served model, usage, latency, error class);
- `vN/summary.json`, `vN/traces/<id>_rep<k>.json`;
- `vN/change.md`, `vN/change.patch`, `narrative.md`.

This way the claude-api skill's report builder (node is installed) renders `report.html` with per-case
transcripts and diffs, and our own static results page reads the same files.

The eval directory is `C:\Users\sethy\Projects\vault-eval`: outside the vault, never indexed, canary-tested.
