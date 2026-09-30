# learning-core

Pure Rust library for the learner projections in `docs/tutor/CONTRACTS.md` C13. No I/O, no clock, no
global state: every time (epoch seconds) and timezone is an argument, and the same input always gives
the same output.

**Every number here is a provisional local policy default** (`skill-policy/1`, `plan-policy/1`,
evidence ledger EV-023). Nothing in this crate establishes improved learning; `p_mastery` and the
statuses are advisory and reversible, never a lesson gate.

## `skills`

- `SkillPolicy::default()`: `skill-policy/1` (uniform prior, half-life 30 days, `P(p >= 0.70)`,
  mastery at 0.80 with 15 observations, 3 dates, 3 families, a 7-day delayed success).
- `ClosedAttempt`: one `attempt_closed` event joined with its `item_opened` event.
- `SkillFold::new(policy, tz)`, `push(&attempt) -> Result<Vec<Observation>, InputError>`, `states()`,
  `excluded()`. Feed the attempts in `event_seq` order. The fold is resumable and serde-serialisable:
  pushing in batches, or saving and resuming between batches, gives exactly the state of one pass
  (with JSON, enable serde_json's `float_roundtrip`).
- `project(policy, tz, attempts) -> Result<Vec<SkillState>, InputError>`: the fold from scratch.
- `sensitivity(policy, skill_id, observations, &SENSITIVITY_HALF_LIVES)`: `p_mastery` at half-lives
  14 and 60 from the observations `push` returned.
- `prob_at_least(alpha, beta, x)`: `1 - I_x(alpha, beta)` (statrs `beta_reg`).

Choices where C13 is open: dependent observations do not count toward the 15-observation floor; the
delayed criterion holds while the latest delayed observation is correct (so a delayed miss after
mastery gives `needs_review`); `families_available` counts families seen in any attempt; an
ineligible attempt with no reason counts as `unspecified`.

## `plan`

- `plan(&PlanRequest, &[Candidate]) -> Result<Plan, PlanError>`.
- Budget: `min(available, review_budget + time of up to new_item_budget new items, soft stop)`, times
  0.75 when tired (a note says so and that it can be undone). Due work is capped at the review budget.
- Due items (`!is_new`, `due_at <= now`) rank by lowest retrievability (`None` as 0), ties by entry id,
  and are picked in four passes: one item per active track with due work (floor); procedures and
  writing up to `procedure_share_min` of the due budget; per-track shares proportional to due load
  (estimated seconds); then anything that still fits. New items follow, round-robin over active tracks
  in request order, up to `new_item_budget`.
- Default estimated seconds by kind: atom 20, language card 15, procedure 180, writing 900.

## Checks

```
cargo test -j 3 -p learning-core
cargo clippy -j 3 -p learning-core --all-targets
cargo fmt -p learning-core -- --check
```
