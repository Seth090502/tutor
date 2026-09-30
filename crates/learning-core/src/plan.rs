//! Study plan (`plan-policy/1`, CONTRACTS.md C13).
//!
//! One global budget across every track. Due work comes first, ranked by lowest retrievability, with
//! a floor per active track, a minimum share for procedures and writing, and per-track shares
//! proportional to due load; new items follow. Every number is a provisional local default.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

pub const PLAN_POLICY_VERSION: &str = "plan-policy/1";

/// `fatigue: tired` multiplies the budget by this.
pub const TIRED_BUDGET_FACTOR: f64 = 0.75;

const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Atom,
    LanguageCard,
    Procedure,
    Writing,
}

impl ItemKind {
    /// Used until measured medians replace them.
    pub fn default_estimated_seconds(self) -> u32 {
        match self {
            Self::Atom => 20,
            Self::LanguageCard => 15,
            Self::Procedure => 180,
            Self::Writing => 900,
        }
    }

    fn is_procedural(self) -> bool {
        matches!(self, Self::Procedure | Self::Writing)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Atom => "atom",
            Self::LanguageCard => "language card",
            Self::Procedure => "procedure",
            Self::Writing => "writing",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fatigue {
    #[default]
    None,
    Tired,
    Rested,
}

fn default_review_budget_minutes() -> f64 {
    45.0
}

fn default_study_soft_stop_minutes() -> f64 {
    90.0
}

fn default_procedure_share_min() -> f64 {
    0.3
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanRequest {
    /// Epoch seconds.
    pub now: f64,
    /// Part of the C13 request; `plan-policy/1` does not use it.
    pub timezone: Tz,
    pub available_minutes: f64,
    pub active_tracks: Vec<String>,
    #[serde(default = "default_review_budget_minutes")]
    pub review_budget_minutes: f64,
    /// Maximum number of new items.
    pub new_item_budget: u32,
    #[serde(default = "default_study_soft_stop_minutes")]
    pub study_soft_stop_minutes: f64,
    #[serde(default = "default_procedure_share_min")]
    pub procedure_share_min: f64,
    #[serde(default)]
    pub fatigue: Fatigue,
    /// Entry ids reserved for experiments (probes); never planned.
    #[serde(default)]
    pub excluded_experiment_units: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub entry_id: i64,
    pub track: String,
    pub kind: ItemKind,
    /// `None` ranks as 0 (an overdue item with no review yet).
    pub retrievability: Option<f64>,
    pub due_at: Option<f64>,
    pub review_count: u32,
    /// `None` or 0 means not measured: the kind's default applies.
    pub estimated_seconds: Option<u32>,
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorityComponents {
    pub retrievability: Option<f64>,
    pub overdue_days: f64,
    /// The track's share of the due load (estimated seconds), 0 when it has none.
    pub track_share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanItem {
    pub entry_id: i64,
    pub track: String,
    pub reason: String,
    pub estimated_seconds: u32,
    pub priority_components: PriorityComponents,
    pub due_at: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanSummary {
    pub minutes_by_track: BTreeMap<String, f64>,
    /// Due items left out of the plan, per active track.
    pub deferred_by_track: BTreeMap<String, u32>,
    pub total_minutes: f64,
    pub budget_minutes: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub items: Vec<PlanItem>,
    pub summary: PlanSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlanError {
    /// A minutes field or `now` that is negative or not finite, or a share outside 0..=1.
    InvalidRequest { field: &'static str, value: f64 },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest { field, value } => {
                write!(f, "plan request: invalid {field} ({value})")
            }
        }
    }
}

impl std::error::Error for PlanError {}

fn validate(request: &PlanRequest) -> Result<(), PlanError> {
    let invalid = |field, value| Err(PlanError::InvalidRequest { field, value });
    if !request.now.is_finite() {
        return invalid("now", request.now);
    }
    for (field, value) in [
        ("available_minutes", request.available_minutes),
        ("review_budget_minutes", request.review_budget_minutes),
        ("study_soft_stop_minutes", request.study_soft_stop_minutes),
    ] {
        if !(value.is_finite() && value >= 0.0) {
            return invalid(field, value);
        }
    }
    if !(0.0..=1.0).contains(&request.procedure_share_min) {
        return invalid("procedure_share_min", request.procedure_share_min);
    }
    Ok(())
}

fn estimated_seconds(c: &Candidate) -> u32 {
    c.estimated_seconds
        .filter(|&s| s > 0)
        .unwrap_or_else(|| c.kind.default_estimated_seconds())
}

fn minutes_to_seconds(minutes: f64) -> u64 {
    // Saturating cast of a finite, non-negative value.
    (minutes * 60.0).floor() as u64
}

fn seconds_to_minutes(seconds: u64) -> f64 {
    seconds as f64 / 60.0
}

#[derive(Clone, Copy)]
enum Pick {
    Floor,
    ProcedureShare,
    TrackShare,
    Fill,
}

struct Due<'a> {
    candidate: &'a Candidate,
    seconds: u64,
    pick: Option<Pick>,
}

/// Builds the plan. Output order: selected due items by lowest retrievability (ties by entry id),
/// then new items. The same request and candidates always give the same plan, in any input order
/// when entry ids are unique.
pub fn plan(request: &PlanRequest, candidates: &[Candidate]) -> Result<Plan, PlanError> {
    validate(request)?;
    let mut active: Vec<&str> = Vec::new();
    for track in &request.active_tracks {
        if !active.contains(&track.as_str()) {
            active.push(track);
        }
    }
    let reserved: BTreeSet<i64> = request.excluded_experiment_units.iter().copied().collect();
    let mut reserved_left_out = 0u32;
    let mut due: Vec<Due> = Vec::new();
    let mut new_by_track: BTreeMap<&str, Vec<&Candidate>> = BTreeMap::new();
    for c in candidates {
        if !active.contains(&c.track.as_str()) {
            continue;
        }
        if reserved.contains(&c.entry_id) {
            reserved_left_out += 1;
            continue;
        }
        if c.is_new {
            new_by_track.entry(&c.track).or_default().push(c);
        } else if c.due_at.is_some_and(|d| d <= request.now) {
            due.push(Due {
                candidate: c,
                seconds: u64::from(estimated_seconds(c)),
                pick: None,
            });
        }
    }
    let rank = |c: &Candidate| c.retrievability.unwrap_or(0.0);
    due.sort_by(|a, b| {
        rank(a.candidate)
            .total_cmp(&rank(b.candidate))
            .then(a.candidate.entry_id.cmp(&b.candidate.entry_id))
    });

    // New items: round-robin over active tracks in request order, entry id order within a track.
    for items in new_by_track.values_mut() {
        items.sort_by_key(|c| c.entry_id);
    }
    let mut new_order: Vec<&Candidate> = Vec::new();
    let mut round = 0;
    loop {
        let before = new_order.len();
        for track in &active {
            if let Some(c) = new_by_track.get(track).and_then(|items| items.get(round)) {
                new_order.push(c);
            }
        }
        if new_order.len() == before {
            break;
        }
        round += 1;
    }
    let new_limit = usize::try_from(request.new_item_budget).unwrap_or(usize::MAX);
    let new_seconds: u64 = new_order
        .iter()
        .take(new_limit)
        .map(|c| u64::from(estimated_seconds(c)))
        .sum();

    // One global budget.
    let base_minutes = request
        .available_minutes
        .min(request.review_budget_minutes + seconds_to_minutes(new_seconds))
        .min(request.study_soft_stop_minutes);
    let factor = if request.fatigue == Fatigue::Tired {
        TIRED_BUDGET_FACTOR
    } else {
        1.0
    };
    let budget = minutes_to_seconds(base_minutes * factor);
    let due_budget = budget.min(minutes_to_seconds(request.review_budget_minutes));

    let mut load: BTreeMap<&str, u64> = BTreeMap::new();
    let mut by_track: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, d) in due.iter().enumerate() {
        *load.entry(&d.candidate.track).or_default() += d.seconds;
        by_track.entry(&d.candidate.track).or_default().push(i);
    }
    let total_load: u64 = load.values().sum();
    let share = |track: &str| {
        if total_load == 0 {
            0.0
        } else {
            load.get(track).copied().unwrap_or(0) as f64 / total_load as f64
        }
    };
    let mut used = 0u64;
    let select = |due: &mut [Due], i: usize, pick: Pick, used: &mut u64| -> bool {
        if due[i].pick.is_some() || *used + due[i].seconds > due_budget {
            return false;
        }
        due[i].pick = Some(pick);
        *used += due[i].seconds;
        true
    };

    // 1. Floor: every active track with due work gets its best item that fits, tracks taken in the
    //    order of their best item.
    let mut floor_order: Vec<&Vec<usize>> = by_track.values().collect();
    floor_order.sort_by_key(|idx| idx[0]);
    for idx in floor_order {
        for &i in idx {
            if select(&mut due, i, Pick::Floor, &mut used) {
                break;
            }
        }
    }

    // 2. Procedures and writing: at least `procedure_share_min` of the due budget when they exist
    //    (the item that crosses the line is kept if it fits).
    let reserve = (request.procedure_share_min * due_budget as f64).ceil() as u64;
    let mut procedural_used: u64 = due
        .iter()
        .filter(|d| d.pick.is_some() && d.candidate.kind.is_procedural())
        .map(|d| d.seconds)
        .sum();
    for i in 0..due.len() {
        if procedural_used >= reserve {
            break;
        }
        if due[i].candidate.kind.is_procedural()
            && select(&mut due, i, Pick::ProcedureShare, &mut used)
        {
            procedural_used += due[i].seconds;
        }
    }

    // 3. Track shares proportional to due load.
    for (track, idx) in &by_track {
        let target =
            (u128::from(due_budget) * u128::from(load[track]) / u128::from(total_load)) as u64;
        let mut track_used: u64 = idx
            .iter()
            .filter(|&&i| due[i].pick.is_some())
            .map(|&i| due[i].seconds)
            .sum();
        for &i in idx {
            if due[i].pick.is_none()
                && track_used + due[i].seconds <= target
                && select(&mut due, i, Pick::TrackShare, &mut used)
            {
                track_used += due[i].seconds;
            }
        }
    }

    // 4. Time the shares could not use goes to the lowest retrievability that fits.
    for i in 0..due.len() {
        select(&mut due, i, Pick::Fill, &mut used);
    }

    let now = request.now;
    let components = |c: &Candidate| PriorityComponents {
        retrievability: c.retrievability,
        overdue_days: c
            .due_at
            .map_or(0.0, |d| ((now - d) / SECONDS_PER_DAY).max(0.0)),
        track_share: share(&c.track),
    };
    let item = |c: &Candidate, seconds: u64, reason: String| PlanItem {
        entry_id: c.entry_id,
        track: c.track.clone(),
        reason,
        estimated_seconds: u32::try_from(seconds).unwrap_or(u32::MAX),
        priority_components: components(c),
        due_at: c.due_at,
    };
    let percent = |x: f64| format!("{:.0}%", x * 100.0);
    let mut items = Vec::new();
    let mut seconds_by_track: BTreeMap<&str, u64> = active.iter().map(|&t| (t, 0)).collect();
    let mut deferred_by_track: BTreeMap<String, u32> =
        active.iter().map(|&t| (t.to_owned(), 0)).collect();
    for d in &due {
        let c = d.candidate;
        let Some(pick) = d.pick else {
            *deferred_by_track.entry(c.track.clone()).or_default() += 1;
            continue;
        };
        let reason = match pick {
            Pick::Floor => format!(
                "due; the best item in {} so no active track starves",
                c.track
            ),
            Pick::ProcedureShare => format!(
                "due {}; procedures and writing keep at least {} of the minutes",
                c.kind.label(),
                percent(request.procedure_share_min)
            ),
            Pick::TrackShare => format!(
                "due; lowest retrievability within {}'s share ({} of due load)",
                c.track,
                percent(share(&c.track))
            ),
            Pick::Fill => {
                "due; lowest retrievability among items that fit the time left".to_owned()
            }
        };
        *seconds_by_track.entry(&c.track).or_default() += d.seconds;
        items.push(item(c, d.seconds, reason));
    }

    // New items only after due work, within the global budget, up to `new_item_budget`.
    let mut picked_new = 0usize;
    for c in new_order {
        if picked_new == new_limit {
            break;
        }
        let seconds = u64::from(estimated_seconds(c));
        if used + seconds > budget {
            continue;
        }
        used += seconds;
        picked_new += 1;
        *seconds_by_track.entry(&c.track).or_default() += seconds;
        let reason = format!(
            "new item {picked_new} of up to {}, after due work",
            request.new_item_budget
        );
        items.push(item(c, seconds, reason));
    }

    let mut notes = Vec::new();
    if active.is_empty() {
        notes.push("No active tracks, so nothing is planned.".to_owned());
    }
    if request.fatigue == Fatigue::Tired {
        notes.push(format!(
            "Tired: the budget is {:.1} minutes, {} of {:.1}. You can undo this by changing fatigue.",
            seconds_to_minutes(budget),
            percent(TIRED_BUDGET_FACTOR),
            base_minutes
        ));
    }
    let deferred: u32 = deferred_by_track.values().sum();
    if deferred > 0 {
        notes.push(format!(
            "{deferred} due items did not fit the budget and stay due."
        ));
    }
    if reserved_left_out > 0 {
        notes.push(format!(
            "{reserved_left_out} items reserved for experiments are left out."
        ));
    }
    notes.push(format!(
        "{PLAN_POLICY_VERSION}: every number here is a provisional local default, not a learning claim."
    ));

    Ok(Plan {
        items,
        summary: PlanSummary {
            minutes_by_track: seconds_by_track
                .into_iter()
                .map(|(t, s)| (t.to_owned(), seconds_to_minutes(s)))
                .collect(),
            deferred_by_track,
            total_minutes: seconds_to_minutes(used),
            budget_minutes: seconds_to_minutes(budget),
            notes,
        },
    })
}
