use std::collections::BTreeMap;

use chrono::TimeZone;
use chrono_tz::America::New_York;
use chrono_tz::{Tz, UTC};
use learning_core::skills::{
    ClosedAttempt, FirstOutcome, SENSITIVITY_HALF_LIVES, SkillFold, SkillPolicy, SkillState,
    SkillStatus, prob_at_least, project, sensitivity,
};
use proptest::prelude::*;

const DAY: f64 = 86_400.0;
/// 2026-09-21T10:13:20Z.
const T0: f64 = 1_790_000_000.0;

fn attempt(id: &str, at: f64, correct: bool, family: Option<&str>, entry: i64) -> ClosedAttempt {
    ClosedAttempt {
        attempt_id: id.to_owned(),
        closed_at: at,
        eligible: true,
        ineligible_reasons: Vec::new(),
        first_outcome: if correct {
            FirstOutcome::Correct
        } else {
            FirstOutcome::Incorrect
        },
        skill_ids: vec!["s".to_owned()],
        family_id: family.map(str::to_owned),
        entry_id: entry,
    }
}

fn ineligible(
    id: &str,
    at: f64,
    reasons: &[&str],
    family: Option<&str>,
    entry: i64,
) -> ClosedAttempt {
    ClosedAttempt {
        eligible: false,
        ineligible_reasons: reasons.iter().map(|r| (*r).to_owned()).collect(),
        first_outcome: FirstOutcome::Other,
        ..attempt(id, at, false, family, entry)
    }
}

fn only(states: Vec<SkillState>) -> SkillState {
    assert_eq!(states.len(), 1, "{states:?}");
    states.into_iter().next().unwrap()
}

fn skill(policy: &SkillPolicy, tz: Tz, attempts: &[ClosedAttempt]) -> SkillState {
    only(project(policy, tz, attempts).unwrap())
}

fn no_decay() -> SkillPolicy {
    SkillPolicy {
        half_life_days: f64::INFINITY,
        ..SkillPolicy::default()
    }
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected} by more than {tolerance}"
    );
}

/// `1 - I_x(a, b)` for integer a, b: `P(Binomial(a + b - 1, x) < a)`, summed term by term.
fn binomial_reference(a: u32, b: u32, x: f64) -> f64 {
    let n = a + b - 1;
    let mut term = (1.0 - x).powi(n as i32);
    let mut below = 0.0;
    for j in 0..a {
        below += term;
        term *= f64::from(n - j) / f64::from(j + 1) * x / (1.0 - x);
    }
    below
}

/// `1 - I_x(a, b)` by composite Simpson integration of the unnormalised density.
fn simpson_reference(a: f64, b: f64, x: f64) -> f64 {
    let integral = |upper: f64| {
        let n = 400_000;
        let h = upper / f64::from(n);
        let f = |t: f64| t.powf(a - 1.0) * (1.0 - t).powf(b - 1.0);
        let mut sum = f(0.0) + f(upper);
        for i in 1..n {
            sum += f(f64::from(i) * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        sum * h / 3.0
    };
    1.0 - integral(x) / integral(1.0)
}

#[test]
fn beta_closed_forms() {
    // Beta(5, 1): the four-success value 1 - 0.7^5.
    close(prob_at_least(5.0, 1.0, 0.7), 1.0 - 0.7f64.powi(5), 1e-13);
    close(prob_at_least(5.0, 1.0, 0.7), 0.83193, 1e-12);
    for a in [1.0, 2.5, 7.0, 13.3] {
        // Beta(a, 1) has CDF x^a.
        close(prob_at_least(a, 1.0, 0.7), 1.0 - 0.7f64.powf(a), 1e-13);
        // Beta(1, b) has CDF 1 - (1 - x)^b.
        close(prob_at_least(1.0, a, 0.7), 0.3f64.powf(a), 1e-13);
    }
    for a in [1.0, 2.0, 3.5, 10.0, 40.0] {
        close(prob_at_least(a, a, 0.5), 0.5, 1e-13);
    }
    // Beta(1/2, 1/2) is the arcsine law: CDF 2/pi * asin(sqrt(x)).
    for x in [0.1f64, 0.5, 0.7, 0.9] {
        let cdf = 2.0 / std::f64::consts::PI * x.sqrt().asin();
        close(prob_at_least(0.5, 0.5, x), 1.0 - cdf, 1e-13);
    }
}

#[test]
fn beta_integer_parameters_match_binomial_tail() {
    for (a, b) in [
        (1, 1),
        (4, 2),
        (5, 1),
        (16, 1),
        (15, 3),
        (40, 25),
        (200, 60),
    ] {
        close(
            prob_at_least(f64::from(a), f64::from(b), 0.7),
            binomial_reference(a, b, 0.7),
            1e-12,
        );
    }
}

#[test]
fn beta_non_integer_parameters_match_simpson_integration() {
    for (a, b) in [(3.7, 2.2), (2.5, 4.25), (12.4, 3.1)] {
        close(
            prob_at_least(a, b, 0.7),
            simpson_reference(a, b, 0.7),
            1e-10,
        );
    }
}

#[test]
fn hand_computed_sequence_without_decay() {
    // Three correct and one incorrect on one day in four families: Beta(4, 2).
    let attempts = [
        attempt("a1", T0, true, Some("f1"), 1),
        attempt("a2", T0 + 60.0, true, Some("f2"), 2),
        attempt("a3", T0 + 120.0, false, Some("f3"), 3),
        attempt("a4", T0 + 180.0, true, Some("f4"), 4),
    ];
    let s = skill(&no_decay(), UTC, &attempts);
    assert_eq!((s.alpha, s.beta, s.effective_weight), (4.0, 2.0, 4.0));
    // 1 - I_0.7(4, 2) = 1 - (5 * 0.7^4 * 0.3 + 0.7^5) = 0.47178.
    close(s.p_mastery, 0.47178, 1e-12);
}

#[test]
fn four_successes_reach_0_832_but_stay_insufficient() {
    let attempts: Vec<_> = (0..4)
        .map(|i| {
            let family = format!("f{i}");
            attempt(
                &format!("a{i}"),
                T0 + f64::from(i) * DAY,
                true,
                Some(&family),
                i64::from(i),
            )
        })
        .collect();
    let s = skill(&no_decay(), UTC, &attempts);
    assert_eq!((s.alpha, s.beta), (5.0, 1.0));
    close(s.p_mastery, 1.0 - 0.7f64.powi(5), 1e-13);
    assert_eq!((s.distinct_dates, s.distinct_families), (4, 4));
    assert_eq!(s.status, SkillStatus::Insufficient);

    // Under the pilot half-life the decayed value still clears 0.80, and the floor still holds.
    let s = skill(&SkillPolicy::default(), UTC, &attempts);
    assert!(
        s.p_mastery >= 0.80 && s.p_mastery < 0.832,
        "{}",
        s.p_mastery
    );
    assert_eq!(s.status, SkillStatus::Insufficient);
}

#[test]
fn decay_halves_evidence_after_one_half_life() {
    let policy = SkillPolicy::default();
    let first = skill(&policy, UTC, &[attempt("a1", T0, true, None, 1)]);
    assert_eq!(
        (first.alpha, first.beta, first.effective_weight),
        (2.0, 1.0, 1.0)
    );

    // An incorrect answer exactly 30 days later: the earlier weight of 1 decays to 0.5 first.
    let attempts = [
        attempt("a1", T0, true, None, 1),
        attempt("a2", T0 + 30.0 * DAY, false, None, 1),
    ];
    let s = skill(&policy, UTC, &attempts);
    close(s.alpha, 1.5, 1e-15);
    close(s.beta, 2.0, 1e-15);
    close(s.effective_weight, 1.5, 1e-15);
    // Beta(1.5, 2) has CDF 2.5 x^1.5 - 1.5 x^2.5.
    let x: f64 = 0.7;
    close(
        s.p_mastery,
        1.0 - (2.5 * x.powf(1.5) - 1.5 * x.powf(2.5)),
        1e-12,
    );
}

fn mixed_stream() -> Vec<ClosedAttempt> {
    let mut attempts = Vec::new();
    for i in 0..48i64 {
        let at = T0 + (i as f64) * 0.37 * DAY;
        let family = ["f1", "f2", "f3", "f4"][(i % 4) as usize];
        let mut a = if i % 7 == 3 {
            ineligible(&format!("a{i}"), at, &["assisted"], Some(family), i % 9)
        } else {
            attempt(
                &format!("a{i}"),
                at,
                i % 5 != 0,
                (i % 6 != 0).then_some(family),
                i % 9,
            )
        };
        a.skill_ids = match i % 3 {
            0 => vec!["alg".to_owned()],
            1 => vec!["alg".to_owned(), "geo".to_owned()],
            _ => vec!["geo".to_owned(), "geo".to_owned()],
        };
        attempts.push(a);
    }
    attempts
}

#[test]
fn batch_split_and_resume_give_identical_state() {
    let attempts = mixed_stream();
    let mut whole = SkillFold::new(SkillPolicy::default(), New_York);
    for a in &attempts {
        whole.push(a).unwrap();
    }
    assert_eq!(
        whole.states(),
        project(&SkillPolicy::default(), New_York, &attempts).unwrap()
    );
    for k in 0..=attempts.len() {
        let mut first = SkillFold::new(SkillPolicy::default(), New_York);
        for a in &attempts[..k] {
            first.push(a).unwrap();
        }
        // Persist and resume, as the shell would between catch-ups.
        let saved = serde_json::to_string(&first).unwrap();
        let mut resumed: SkillFold = serde_json::from_str(&saved).unwrap();
        for a in &attempts[k..] {
            first.push(a).unwrap();
            resumed.push(a).unwrap();
        }
        assert_eq!(first, whole, "split at {k}");
        assert_eq!(resumed, whole, "resume at {k}");
        assert_eq!(resumed.states(), whole.states());
        assert_eq!(resumed.excluded(), whole.excluded());
    }
}

fn new_york(y: i32, m: u32, d: u32, h: u32, min: u32) -> f64 {
    New_York
        .with_ymd_and_hms(y, m, d, h, min, 0)
        .single()
        .unwrap()
        .timestamp() as f64
}

fn dependent_count(tz: Tz, times: [f64; 2]) -> u32 {
    let attempts = [
        attempt("a1", times[0], true, Some("f"), 1),
        attempt("a2", times[1], true, Some("f"), 2),
    ];
    let s = skill(&SkillPolicy::default(), tz, &attempts);
    assert_eq!(s.eligible_observations, 2);
    s.dependent_observations
}

#[test]
fn dependence_cap_uses_the_local_date_across_dst() {
    // 2026-11-01 is 25 hours long in New York: 00:30 EDT and 23:30 EST are one local date but
    // two UTC dates.
    let same_local_day = [new_york(2026, 11, 1, 0, 30), new_york(2026, 11, 1, 23, 30)];
    assert_eq!(same_local_day[1] - same_local_day[0], 24.0 * 3600.0);
    assert_eq!(dependent_count(New_York, same_local_day), 1);
    assert_eq!(dependent_count(UTC, same_local_day), 0);

    // 23:30 EST and 00:30 the next morning are two local dates but one UTC date.
    let two_local_days = [new_york(2026, 11, 1, 23, 30), new_york(2026, 11, 2, 0, 30)];
    assert_eq!(dependent_count(New_York, two_local_days), 0);
    assert_eq!(dependent_count(UTC, two_local_days), 1);

    // 2026-03-08 is 23 hours long: 01:30 EST and 23:30 EDT are one local date.
    let spring = [new_york(2026, 3, 8, 1, 30), new_york(2026, 3, 8, 23, 30)];
    assert_eq!(spring[1] - spring[0], 21.0 * 3600.0);
    assert_eq!(dependent_count(New_York, spring), 1);

    // The dependent observation is kept with weight 0: only one unit of evidence remains.
    let attempts = [
        attempt("a1", same_local_day[0], true, Some("f"), 1),
        attempt("a2", same_local_day[1], false, Some("f"), 2),
    ];
    let s = skill(&SkillPolicy::default(), New_York, &attempts);
    assert_eq!(s.beta, 1.0);
    close(s.effective_weight, 0.5f64.powf(1.0 / 30.0), 1e-12);
    assert_eq!(s.distinct_dates, 1);
    assert!(
        s.limitations.iter().any(|l| l.contains("dependent")),
        "{:?}",
        s.limitations
    );
}

#[test]
fn ineligible_attempts_never_move_the_posterior_and_are_counted_by_reason() {
    let eligible = [
        attempt("a1", T0, true, Some("f1"), 1),
        attempt("a3", T0 + 2.0 * DAY, false, Some("f2"), 2),
        attempt("a5", T0 + 5.0 * DAY, true, Some("f1"), 1),
    ];
    let mixed = [
        eligible[0].clone(),
        ineligible("a2", T0 + DAY, &["assisted"], Some("f9"), 9),
        eligible[1].clone(),
        ineligible(
            "a4",
            T0 + 3.0 * DAY,
            &["exposed_within_24h", "assisted", "assisted"],
            None,
            8,
        ),
        ineligible("a4b", T0 + 4.0 * DAY, &[], None, 7),
        eligible[2].clone(),
    ];
    let policy = SkillPolicy::default();
    let clean = skill(&policy, UTC, &eligible);
    let with_excluded = skill(&policy, UTC, &mixed);
    assert_eq!(with_excluded.alpha, clean.alpha);
    assert_eq!(with_excluded.beta, clean.beta);
    assert_eq!(with_excluded.p_mastery, clean.p_mastery);
    assert_eq!(with_excluded.eligible_observations, 3);
    assert_eq!(with_excluded.distinct_families, 2);
    // Families seen in ineligible attempts still count as available (f9, entry 8, entry 7).
    assert_eq!(with_excluded.families_available, 5);

    let mut fold = SkillFold::new(policy, UTC);
    for a in &mixed {
        fold.push(a).unwrap();
    }
    let expected: BTreeMap<String, u64> = [
        ("assisted".to_owned(), 2),
        ("exposed_within_24h".to_owned(), 1),
        ("unspecified".to_owned(), 1),
    ]
    .into();
    assert_eq!(fold.excluded(), &expected);
}

#[test]
fn exposures_include_ineligible_attempts_for_delayed_evidence() {
    let policy = SkillPolicy::default();
    let without = skill(
        &policy,
        UTC,
        &[
            attempt("a1", T0, true, None, 1),
            attempt("a2", T0 + 10.0 * DAY, true, None, 1),
        ],
    );
    assert_eq!(without.delayed_evidence.len(), 1);
    close(without.delayed_evidence[0].gap_days, 10.0, 1e-12);
    assert_eq!(without.delayed_evidence[0].y, 1);

    let with = skill(
        &policy,
        UTC,
        &[
            attempt("a1", T0, true, None, 1),
            ineligible("a2", T0 + 6.0 * DAY, &["assisted"], None, 1),
            attempt("a3", T0 + 10.0 * DAY, true, None, 1),
        ],
    );
    assert!(with.delayed_evidence.is_empty());
    assert_eq!(with.last_exposure_at, Some(T0 + 10.0 * DAY));
}

/// 15 correct observations over days 0-2 in 15 families, then one correct 8 days later.
fn mastered_stream() -> Vec<ClosedAttempt> {
    let mut attempts: Vec<_> = (0..15i64)
        .map(|i| {
            let at = T0 + ((i / 5) as f64) * DAY + ((i % 5) as f64) * 600.0;
            attempt(&format!("m{i}"), at, true, Some(&format!("f{i}")), i)
        })
        .collect();
    attempts.push(attempt("d1", T0 + 10.0 * DAY, true, Some("f0"), 0));
    attempts
}

#[test]
fn needs_review_after_mastery_then_a_delayed_miss() {
    let policy = SkillPolicy::default();
    let mut attempts = mastered_stream();
    let before_delay = skill(&policy, UTC, &attempts[..15]);
    assert_eq!(
        before_delay.status,
        SkillStatus::Developing,
        "no delayed evidence yet"
    );

    let mastered = skill(&policy, UTC, &attempts);
    assert_eq!(mastered.status, SkillStatus::ProvisionallyMastered);
    assert_eq!(mastered.delayed_evidence.len(), 1);

    attempts.push(attempt("d2", T0 + 20.0 * DAY, false, Some("f1"), 1));
    let missed = skill(&policy, UTC, &attempts);
    assert!(
        missed.p_mastery >= policy.p_required,
        "the delayed miss alone triggers review"
    );
    assert_eq!(missed.delayed_evidence.last().unwrap().y, 0);
    assert_eq!(missed.status, SkillStatus::NeedsReview);

    // Advisory and reversible: a later delayed success restores it.
    attempts.push(attempt("d3", T0 + 30.0 * DAY, true, Some("f2"), 2));
    assert_eq!(
        skill(&policy, UTC, &attempts).status,
        SkillStatus::ProvisionallyMastered
    );
}

#[test]
fn family_rule_applies_only_when_three_families_are_available() {
    let policy = SkillPolicy::default();
    // Two families on 8 dates (never the same family twice on a date), then a delayed success.
    let mut two: Vec<_> = (0..15i64)
        .map(|i| {
            let at = T0 + ((i / 2) as f64) * DAY;
            attempt(
                &format!("m{i}"),
                at,
                true,
                Some(["f0", "f1"][(i % 2) as usize]),
                i,
            )
        })
        .collect();
    two.push(attempt("d1", T0 + 17.0 * DAY, true, Some("f0"), 0));
    let s = skill(&policy, UTC, &two);
    assert_eq!((s.distinct_families, s.families_available), (2, 2));
    assert_eq!(s.dependent_observations, 0);
    assert_eq!(s.status, SkillStatus::ProvisionallyMastered);
    assert!(
        s.limitations
            .iter()
            .any(|l| l == "fewer than 3 families authored")
    );

    // A third family seen only in an ineligible attempt makes the rule apply.
    let mut three = two.clone();
    three.insert(
        4,
        ineligible("x", T0 + 1.5 * DAY, &["assisted"], Some("f2"), 99),
    );
    let s = skill(&policy, UTC, &three);
    assert_eq!((s.distinct_families, s.families_available), (2, 3));
    assert_eq!(s.status, SkillStatus::Developing);
    assert!(!s.limitations.iter().any(|l| l.contains("families")));
}

#[test]
fn dependent_observations_do_not_count_toward_the_floor() {
    // Four families on three dates, each answered twice a day: 24 eligible, 12 of them dependent.
    let mut attempts = Vec::new();
    for day in 0..3 {
        for f in 0..4i64 {
            for k in 0..2 {
                let at = T0 + f64::from(day) * DAY + ((f * 2 + k) as f64) * 60.0;
                let family = format!("f{f}");
                attempts.push(attempt(
                    &format!("a{day}-{f}-{k}"),
                    at,
                    true,
                    Some(&family),
                    f,
                ));
            }
        }
    }
    attempts.push(attempt("late", T0 + 14.0 * DAY, true, Some("f0"), 0));
    let s = skill(&SkillPolicy::default(), UTC, &attempts);
    assert_eq!(
        (s.eligible_observations, s.dependent_observations),
        (25, 12)
    );
    assert_eq!((s.distinct_dates, s.distinct_families), (4, 4));
    assert_eq!(s.delayed_evidence.len(), 1);
    assert!(s.p_mastery >= 0.80, "{}", s.p_mastery);
    // 25 eligible observations but only 13 independent ones: below the floor of 15.
    assert_eq!(s.status, SkillStatus::Insufficient);
}

#[test]
fn sensitivity_recomputes_other_half_lives() {
    let policy = SkillPolicy::default();
    let mut fold = SkillFold::new(policy.clone(), UTC);
    let mut observations = Vec::new();
    for a in mastered_stream() {
        observations.extend(fold.push(&a).unwrap());
    }
    let state = only(fold.states());
    let at_policy = sensitivity(&policy, "s", &observations, &[policy.half_life_days]);
    assert_eq!(at_policy[0].p_mastery, state.p_mastery);

    let other = sensitivity(&policy, "s", &observations, &SENSITIVITY_HALF_LIVES);
    assert_eq!(other.len(), 2);
    assert_eq!(
        (other[0].half_life_days, other[1].half_life_days),
        (14.0, 60.0)
    );
    // Only successes: a shorter memory keeps less of them.
    assert!(other[0].p_mastery < state.p_mastery && state.p_mastery < other[1].p_mastery);
    assert!(sensitivity(&policy, "other", &observations, &[14.0])[0].p_mastery < 0.31);
}

#[test]
fn invalid_time_is_an_error_and_leaves_the_fold_unchanged() {
    let mut fold = SkillFold::new(SkillPolicy::default(), UTC);
    fold.push(&attempt("a1", T0, true, None, 1)).unwrap();
    let before = fold.clone();
    for bad in [f64::NAN, f64::INFINITY, 1e300] {
        assert!(fold.push(&attempt("bad", bad, true, None, 1)).is_err());
        assert!(
            fold.push(&ineligible("bad", bad, &["assisted"], None, 1))
                .is_err()
        );
    }
    assert_eq!(fold, before);
}

/// (gap after the previous attempt, eligible, correct, skill bitmask, family, entry id).
type RawAttempt = (f64, bool, bool, u8, Option<u8>, i64);

fn arb_attempt() -> impl Strategy<Value = RawAttempt> {
    (
        0.0..20.0 * DAY,
        any::<bool>(),
        any::<bool>(),
        1u8..8,
        proptest::option::of(0u8..4),
        0i64..6,
    )
}

fn build(raw: &[RawAttempt]) -> Vec<ClosedAttempt> {
    let mut at = T0;
    raw.iter()
        .enumerate()
        .map(|(i, &(gap, eligible, correct, skills, family, entry))| {
            at += gap;
            let family = family.map(|f| format!("f{f}"));
            let mut a = attempt(&format!("a{i}"), at, correct, family.as_deref(), entry);
            a.skill_ids = ["x", "y", "z"]
                .iter()
                .enumerate()
                .filter(|(bit, _)| skills & (1 << bit) != 0)
                .map(|(_, s)| (*s).to_owned())
                .collect();
            if !eligible {
                a.eligible = false;
                a.ineligible_reasons = vec!["assisted".to_owned()];
            }
            a
        })
        .collect()
}

proptest! {
    #[test]
    fn posterior_stays_at_or_above_prior_and_p_in_unit_interval(
        raw in proptest::collection::vec(arb_attempt(), 0..60),
        split in 0usize..60,
    ) {
        let policy = SkillPolicy::default();
        let attempts = build(&raw);
        let states = project(&policy, New_York, &attempts).unwrap();
        for s in &states {
            prop_assert!(s.alpha >= policy.alpha0 && s.beta >= policy.beta0, "{s:?}");
            prop_assert!(s.effective_weight >= 0.0);
            prop_assert!((0.0..=1.0).contains(&s.p_mastery), "{s:?}");
            prop_assert!(s.dependent_observations <= s.eligible_observations);
        }
        let split = split.min(attempts.len());
        let mut fold = SkillFold::new(policy.clone(), New_York);
        for a in &attempts[..split] {
            fold.push(a).unwrap();
        }
        let mut resumed: SkillFold = serde_json::from_str(&serde_json::to_string(&fold).unwrap()).unwrap();
        for a in &attempts[split..] {
            resumed.push(a).unwrap();
        }
        prop_assert_eq!(resumed.states(), states);
    }
}
