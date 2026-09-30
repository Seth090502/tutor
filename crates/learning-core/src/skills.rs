//! Skill projection (`skill-policy/1`, CONTRACTS.md C13).
//!
//! A fold over closed attempts in feed (`event_seq`) order. Eligible attempts add one observation per
//! skill; ineligible ones only count as exposures and in `excluded`. Every number is a provisional
//! local policy default, not evidence of learning.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::LN_2;
use std::fmt;

use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

pub const SKILL_POLICY_VERSION: &str = "skill-policy/1";

/// Half-lives (days) the report shows next to the policy's own (C13).
pub const SENSITIVITY_HALF_LIVES: [f64; 2] = [14.0, 60.0];

const SECONDS_PER_DAY: f64 = 86_400.0;

/// The `excluded` key for an ineligible attempt that carried no reason.
pub const UNSPECIFIED_REASON: &str = "unspecified";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillPolicy {
    pub version: String,
    pub alpha0: f64,
    pub beta0: f64,
    pub half_life_days: f64,
    /// `p_mastery = P(p >= p_threshold)`.
    pub p_threshold: f64,
    /// `p_mastery` needed for `provisionally_mastered`.
    pub p_required: f64,
    pub min_observations: u32,
    pub min_dates: u32,
    pub min_families: u32,
    pub delayed_gap_days: f64,
}

impl Default for SkillPolicy {
    fn default() -> Self {
        Self {
            version: SKILL_POLICY_VERSION.to_owned(),
            alpha0: 1.0,
            beta0: 1.0,
            half_life_days: 30.0,
            p_threshold: 0.70,
            p_required: 0.80,
            min_observations: 15,
            min_dates: 3,
            min_families: 3,
            delayed_gap_days: 7.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstOutcome {
    Correct,
    Incorrect,
    Other,
}

/// One `attempt_closed` event joined with its `item_opened` event (C12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedAttempt {
    pub attempt_id: String,
    /// The event's `recorded_at`, epoch seconds.
    pub closed_at: f64,
    /// `measurement_eligibility.eligible`.
    pub eligible: bool,
    /// `measurement_eligibility` reasons; read only when `eligible` is false.
    pub ineligible_reasons: Vec<String>,
    pub first_outcome: FirstOutcome,
    pub skill_ids: Vec<String>,
    /// `None` means the item is its own family.
    pub family_id: Option<String>,
    pub entry_id: i64,
}

/// One eligible observation of one skill, as the fold applied it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub skill_id: String,
    pub attempt_id: String,
    pub entry_id: i64,
    pub family_id: Option<String>,
    pub at: f64,
    pub local_date: NaiveDate,
    /// 1 when the first outcome was correct, else 0.
    pub y: u8,
    /// A later observation of the same (skill, family, local date): kept with weight `c = 0`.
    pub dependent: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelayedEvidence {
    pub attempt_id: String,
    pub at: f64,
    /// Days since the previous exposure to the skill (any attempt, eligible or not).
    pub gap_days: f64,
    pub y: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Insufficient,
    Developing,
    ProvisionallyMastered,
    NeedsReview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillState {
    pub skill_id: String,
    pub policy_version: String,
    pub alpha: f64,
    pub beta: f64,
    pub effective_weight: f64,
    /// Every eligible observation, dependent ones included.
    pub eligible_observations: u32,
    pub dependent_observations: u32,
    pub distinct_families: u32,
    /// Distinct families seen for the skill in any attempt, eligible or not.
    pub families_available: u32,
    pub distinct_dates: u32,
    pub last_exposure_at: Option<f64>,
    pub delayed_evidence: Vec<DelayedEvidence>,
    pub p_mastery: f64,
    pub status: SkillStatus,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputError {
    /// `closed_at` is not finite or outside the dates chrono can represent.
    InvalidTime { attempt_id: String, closed_at: f64 },
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTime {
                attempt_id,
                closed_at,
            } => write!(
                f,
                "attempt {attempt_id}: closed_at {closed_at} is not a valid epoch time"
            ),
        }
    }
}

impl std::error::Error for InputError {}

/// `P(p >= threshold)` for `p ~ Beta(alpha, beta)`, i.e. `1 - I_threshold(alpha, beta)`.
pub fn prob_at_least(alpha: f64, beta: f64, threshold: f64) -> f64 {
    (1.0 - statrs::function::beta::beta_reg(alpha, beta, threshold)).clamp(0.0, 1.0)
}

/// Beta posterior with exponential decay toward the prior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Posterior {
    alpha: f64,
    beta: f64,
    last_at: Option<f64>,
}

impl Posterior {
    fn new(policy: &SkillPolicy) -> Self {
        Self {
            alpha: policy.alpha0,
            beta: policy.beta0,
            last_at: None,
        }
    }

    /// Decays toward the prior for the time since the previous observation, then adds `c` (1, or 0
    /// when dependent) to alpha or beta. A time earlier than the previous one decays by nothing.
    fn observe(&mut self, policy: &SkillPolicy, at: f64, y: u8, dependent: bool) {
        if let Some(prev) = self.last_at {
            let days = (at - prev).max(0.0) / SECONDS_PER_DAY;
            let w = (-LN_2 * days / policy.half_life_days).exp();
            self.alpha = policy.alpha0 + w * (self.alpha - policy.alpha0);
            self.beta = policy.beta0 + w * (self.beta - policy.beta0);
        }
        let c = if dependent { 0.0 } else { 1.0 };
        let y = f64::from(y);
        self.alpha += y * c;
        self.beta += (1.0 - y) * c;
        self.last_at = Some(self.last_at.map_or(at, |prev| prev.max(at)));
    }

    fn p_mastery(&self, policy: &SkillPolicy) -> f64 {
        prob_at_least(self.alpha, self.beta, policy.p_threshold)
    }
}

/// A family is the authored `family_id`, else the item itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum FamilyKey {
    Family(String),
    Entry(i64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SkillAcc {
    posterior: Posterior,
    eligible_observations: u32,
    dependent_observations: u32,
    /// (family, local date) pairs that already had their one weighted observation.
    capped: BTreeSet<(FamilyKey, NaiveDate)>,
    families: BTreeSet<FamilyKey>,
    families_available: BTreeSet<FamilyKey>,
    dates: BTreeSet<NaiveDate>,
    last_exposure_at: Option<f64>,
    delayed_evidence: Vec<DelayedEvidence>,
    was_mastered: bool,
}

struct Criteria {
    enough_evidence: bool,
    probability: bool,
    families: bool,
    delayed: bool,
}

impl SkillAcc {
    fn new(policy: &SkillPolicy) -> Self {
        Self {
            posterior: Posterior::new(policy),
            eligible_observations: 0,
            dependent_observations: 0,
            capped: BTreeSet::new(),
            families: BTreeSet::new(),
            families_available: BTreeSet::new(),
            dates: BTreeSet::new(),
            last_exposure_at: None,
            delayed_evidence: Vec::new(),
            was_mastered: false,
        }
    }

    fn criteria(&self, policy: &SkillPolicy) -> Criteria {
        // Dependent observations carry no weight, so they do not count toward the floor.
        let independent = self.eligible_observations - self.dependent_observations;
        Criteria {
            enough_evidence: independent >= policy.min_observations
                && count(self.dates.len()) >= policy.min_dates,
            probability: self.posterior.p_mastery(policy) >= policy.p_required,
            families: count(self.families_available.len()) < policy.min_families
                || count(self.families.len()) >= policy.min_families,
            // A later delayed miss withdraws an earlier delayed success.
            delayed: self.delayed_evidence.last().is_some_and(|d| d.y == 1),
        }
    }

    fn status(&self, policy: &SkillPolicy) -> SkillStatus {
        let c = self.criteria(policy);
        if !c.enough_evidence {
            SkillStatus::Insufficient
        } else if c.probability && c.families && c.delayed {
            SkillStatus::ProvisionallyMastered
        } else if self.was_mastered && !(c.probability && c.delayed) {
            SkillStatus::NeedsReview
        } else {
            SkillStatus::Developing
        }
    }

    fn state(&self, skill_id: &str, policy: &SkillPolicy) -> SkillState {
        let families_available = count(self.families_available.len());
        let mut limitations = Vec::new();
        if families_available < policy.min_families {
            limitations.push(format!(
                "fewer than {} families authored",
                policy.min_families
            ));
        }
        if self.dependent_observations > 0 {
            limitations.push(format!(
                "{} dependent observations (same family, same local date) kept with no weight",
                self.dependent_observations
            ));
        }
        limitations.push(format!(
            "{} is a provisional local policy; p_mastery is not a learning claim",
            policy.version
        ));
        let p = &self.posterior;
        SkillState {
            skill_id: skill_id.to_owned(),
            policy_version: policy.version.clone(),
            alpha: p.alpha,
            beta: p.beta,
            effective_weight: (p.alpha - policy.alpha0) + (p.beta - policy.beta0),
            eligible_observations: self.eligible_observations,
            dependent_observations: self.dependent_observations,
            distinct_families: count(self.families.len()),
            families_available,
            distinct_dates: count(self.dates.len()),
            last_exposure_at: self.last_exposure_at,
            delayed_evidence: self.delayed_evidence.clone(),
            p_mastery: p.p_mastery(policy),
            status: self.status(policy),
            limitations,
        }
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn local_date(at: f64, timezone: Tz) -> Option<NaiveDate> {
    if !at.is_finite() {
        return None;
    }
    // Saturating cast; out-of-range seconds make `from_timestamp` return None.
    DateTime::from_timestamp(at.floor() as i64, 0).map(|t| t.with_timezone(&timezone).date_naive())
}

/// Resumable projection state. Pushing attempts one batch at a time gives exactly the state of
/// pushing them all at once, so the shell can catch up from the feed and persist this value
/// (serde; with JSON, enable serde_json's `float_roundtrip` so floats come back bit-exact).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillFold {
    policy: SkillPolicy,
    timezone: Tz,
    skills: BTreeMap<String, SkillAcc>,
    excluded: BTreeMap<String, u64>,
}

impl SkillFold {
    pub fn new(policy: SkillPolicy, timezone: Tz) -> Self {
        Self {
            policy,
            timezone,
            skills: BTreeMap::new(),
            excluded: BTreeMap::new(),
        }
    }

    pub fn policy(&self) -> &SkillPolicy {
        &self.policy
    }

    pub fn timezone(&self) -> Tz {
        self.timezone
    }

    /// Applies one closed attempt (feed order) and returns the observations it added, one per
    /// distinct skill id when eligible. On error the fold is unchanged.
    pub fn push(&mut self, attempt: &ClosedAttempt) -> Result<Vec<Observation>, InputError> {
        let local_date = local_date(attempt.closed_at, self.timezone).ok_or_else(|| {
            InputError::InvalidTime {
                attempt_id: attempt.attempt_id.clone(),
                closed_at: attempt.closed_at,
            }
        })?;
        if !attempt.eligible {
            let reasons: BTreeSet<&str> = attempt
                .ineligible_reasons
                .iter()
                .map(String::as_str)
                .collect();
            if reasons.is_empty() {
                *self
                    .excluded
                    .entry(UNSPECIFIED_REASON.to_owned())
                    .or_default() += 1;
            }
            for reason in reasons {
                *self.excluded.entry(reason.to_owned()).or_default() += 1;
            }
        }
        let family = match &attempt.family_id {
            Some(id) => FamilyKey::Family(id.clone()),
            None => FamilyKey::Entry(attempt.entry_id),
        };
        let at = attempt.closed_at;
        let y = u8::from(attempt.first_outcome == FirstOutcome::Correct);
        let skill_ids: BTreeSet<&str> = attempt.skill_ids.iter().map(String::as_str).collect();
        let mut observations = Vec::new();
        for skill_id in skill_ids {
            let acc = self
                .skills
                .entry(skill_id.to_owned())
                .or_insert_with(|| SkillAcc::new(&self.policy));
            acc.families_available.insert(family.clone());
            let previous_exposure = acc.last_exposure_at;
            acc.last_exposure_at = Some(previous_exposure.map_or(at, |prev| prev.max(at)));
            if !attempt.eligible {
                continue;
            }
            let dependent = !acc.capped.insert((family.clone(), local_date));
            acc.posterior.observe(&self.policy, at, y, dependent);
            acc.eligible_observations += 1;
            if dependent {
                acc.dependent_observations += 1;
            }
            acc.families.insert(family.clone());
            acc.dates.insert(local_date);
            if let Some(prev) = previous_exposure {
                let gap_days = (at - prev) / SECONDS_PER_DAY;
                if gap_days >= self.policy.delayed_gap_days {
                    acc.delayed_evidence.push(DelayedEvidence {
                        attempt_id: attempt.attempt_id.clone(),
                        at,
                        gap_days,
                        y,
                    });
                }
            }
            if acc.status(&self.policy) == SkillStatus::ProvisionallyMastered {
                acc.was_mastered = true;
            }
            observations.push(Observation {
                skill_id: skill_id.to_owned(),
                attempt_id: attempt.attempt_id.clone(),
                entry_id: attempt.entry_id,
                family_id: attempt.family_id.clone(),
                at,
                local_date,
                y,
                dependent,
            });
        }
        Ok(observations)
    }

    /// One state per skill seen in any attempt, ordered by skill id.
    pub fn states(&self) -> Vec<SkillState> {
        self.skills
            .iter()
            .map(|(id, acc)| acc.state(id, &self.policy))
            .collect()
    }

    /// Ineligible attempts counted once per distinct reason (`unspecified` when none was given).
    pub fn excluded(&self) -> &BTreeMap<String, u64> {
        &self.excluded
    }
}

/// Folds `attempts` (feed order) from scratch.
pub fn project(
    policy: &SkillPolicy,
    timezone: Tz,
    attempts: &[ClosedAttempt],
) -> Result<Vec<SkillState>, InputError> {
    let mut fold = SkillFold::new(policy.clone(), timezone);
    for attempt in attempts {
        fold.push(attempt)?;
    }
    Ok(fold.states())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalfLifeP {
    pub half_life_days: f64,
    pub p_mastery: f64,
}

/// Recomputes one skill's `p_mastery` at other half-lives (use [`SENSITIVITY_HALF_LIVES`]) from the
/// observations [`SkillFold::push`] returned, in the order it returned them. Observations of other
/// skills are ignored; dependence flags are kept as they were.
pub fn sensitivity(
    policy: &SkillPolicy,
    skill_id: &str,
    observations: &[Observation],
    half_lives: &[f64],
) -> Vec<HalfLifeP> {
    half_lives
        .iter()
        .map(|&half_life_days| {
            let policy = SkillPolicy {
                half_life_days,
                ..policy.clone()
            };
            let mut posterior = Posterior::new(&policy);
            for o in observations.iter().filter(|o| o.skill_id == skill_id) {
                posterior.observe(&policy, o.at, o.y, o.dependent);
            }
            HalfLifeP {
                half_life_days,
                p_mastery: posterior.p_mastery(&policy),
            }
        })
        .collect()
}
