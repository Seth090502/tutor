//! Learner projection host (contract C13, docs/tutor/CONTRACTS.md): catches a `learning_core` skill
//! fold up from the backend's study feed, keeps it in `<app data dir>/learning.sqlite`, and plans
//! study from the backend's candidates.
//!
//! The backend (C12) stays the only writer of study events. The file here is a cache: one row per
//! (store, policy version, time zone), resumable from its `last_event_seq`, and always rebuildable
//! from event 0.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono_tz::Tz;
use learning_core::plan::{self, Candidate, ItemKind, Plan, PlanRequest};
use learning_core::skills::{ClosedAttempt, FirstOutcome, SkillFold, SkillPolicy, SkillState};
use rusqlite::{params, Connection, OptionalExtension, Params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use crate::services::ServiceName;
use crate::AppState;

pub const DB_FILE: &str = "learning.sqlite";
/// `PRAGMA user_version` of the file; 0 is a missing or older file.
const SCHEMA_VERSION: i64 = 1;
/// The feed and candidates `schema_version` this build reads.
const FEED_SCHEMA_VERSION: u32 = 1;
/// Events per feed request (the backend allows 1 to 1000).
const PAGE_LIMIT: u32 = 500;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS projections (
    store_id TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    timezone TEXT NOT NULL,
    last_event_seq INTEGER NOT NULL,
    -- serde_json SkillFold (float_roundtrip, so a resume is bit-exact)
    fold TEXT NOT NULL,
    -- {attempt_id: {entry_id, skill_ids, family_id}}: opened, not closed yet
    pending TEXT NOT NULL,
    skipped_unknown INTEGER NOT NULL,
    updated_at REAL NOT NULL,
    PRIMARY KEY (store_id, policy_version, timezone)
);
";

/// `learning_skills` answer.
#[derive(Debug, Serialize)]
pub struct Skills {
    store_id: String,
    policy_version: String,
    last_event_seq: i64,
    skills: Vec<SkillState>,
    excluded: BTreeMap<String, u64>,
    skipped_unknown: u32,
    limitations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FeedPage {
    schema_version: u32,
    store_id: String,
    events: Vec<FeedEvent>,
}

#[derive(Debug, Deserialize)]
struct FeedEvent {
    event_seq: i64,
    kind: String,
    attempt_id: Option<String>,
    recorded_at: f64,
    payload: Value,
}

/// An `item_opened` payload (the fields projections need), kept until its attempt closes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Opened {
    entry_id: i64,
    skill_ids: Vec<String>,
    family_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AttemptClosed {
    scheduler_disposition: Disposition,
    measurement_eligibility: Eligibility,
}

#[derive(Debug, Deserialize)]
struct Disposition {
    /// Absent when no outcome was reached (abandoned before a check, invalidated).
    #[serde(default)]
    first_outcome: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Eligibility {
    eligible: bool,
    reasons: Vec<String>,
}

fn malformed(event: &FeedEvent, detail: impl Display) -> String {
    format!(
        "The study feed is malformed: event {} ({}) {detail}.",
        event.event_seq, event.kind
    )
}

/// The fold for one (store, policy, time zone) and where it stopped in the feed.
#[derive(Debug, Clone, PartialEq)]
struct Projection {
    store_id: String,
    last_event_seq: i64,
    fold: SkillFold,
    /// Attempts with an `item_opened` event and no `attempt_closed` yet.
    pending: BTreeMap<String, Opened>,
    /// `attempt_closed` events whose `item_opened` never appeared in the feed.
    skipped_unknown: u32,
}

impl Projection {
    fn new(store_id: String, policy: SkillPolicy, timezone: Tz) -> Self {
        Self {
            store_id,
            last_event_seq: 0,
            fold: SkillFold::new(policy, timezone),
            pending: BTreeMap::new(),
            skipped_unknown: 0,
        }
    }

    /// Records an `item_opened`, or joins an `attempt_closed` to it. Other kinds are ignored, and
    /// so is (counted in `skipped_unknown`) a closure whose opening is unknown.
    fn join(&mut self, event: &FeedEvent) -> Result<Option<ClosedAttempt>, String> {
        let kind = event.kind.as_str();
        if kind != "item_opened" && kind != "attempt_closed" {
            return Ok(None);
        }
        let attempt_id = event
            .attempt_id
            .clone()
            .ok_or_else(|| malformed(event, "has no attempt_id"))?;
        if kind == "item_opened" {
            let opened = Opened::deserialize(&event.payload)
                .map_err(|e| malformed(event, format!("has an unreadable payload: {e}")))?;
            self.pending.insert(attempt_id, opened);
            return Ok(None);
        }
        let closed = AttemptClosed::deserialize(&event.payload)
            .map_err(|e| malformed(event, format!("has an unreadable payload: {e}")))?;
        let Some(opened) = self.pending.remove(&attempt_id) else {
            self.skipped_unknown += 1;
            return Ok(None);
        };
        let first_outcome = match closed.scheduler_disposition.first_outcome.as_deref() {
            Some("correct") => FirstOutcome::Correct,
            Some("incorrect") => FirstOutcome::Incorrect,
            _ => FirstOutcome::Other,
        };
        Ok(Some(ClosedAttempt {
            attempt_id,
            closed_at: event.recorded_at,
            eligible: closed.measurement_eligibility.eligible,
            ineligible_reasons: closed.measurement_eligibility.reasons,
            first_outcome,
            skill_ids: opened.skill_ids,
            family_id: opened.family_id,
            entry_id: opened.entry_id,
        }))
    }

    /// Applies one feed page: events after `last_event_seq`, in `event_seq` order.
    fn apply(&mut self, events: &[FeedEvent]) -> Result<(), String> {
        for event in events {
            if event.event_seq <= self.last_event_seq {
                return Err(malformed(
                    event,
                    format!("is out of order (after event {})", self.last_event_seq),
                ));
            }
            if let Some(attempt) = self.join(event)? {
                self.fold.push(&attempt).map_err(|e| malformed(event, e))?;
            }
            self.last_event_seq = event.event_seq;
        }
        Ok(())
    }

    fn report(self) -> Skills {
        let policy_version = self.fold.policy().version.clone();
        let mut limitations = Vec::new();
        if self.skipped_unknown > 0 {
            limitations.push(format!(
                "Closed attempts not counted because their item_opened event is missing from the feed: {}.",
                self.skipped_unknown
            ));
        }
        limitations.push(format!(
            "{policy_version} is a provisional local policy: statuses are advisory, never a lesson gate, and not a learning claim."
        ));
        Skills {
            store_id: self.store_id,
            policy_version,
            last_event_seq: self.last_event_seq,
            skills: self.fold.states(),
            excluded: self.fold.excluded().clone(),
            skipped_unknown: self.skipped_unknown,
            limitations,
        }
    }
}

// ---- storage ----------------------------------------------------------------------------------

fn sql_err(e: rusqlite::Error) -> String {
    format!("learning database: {e}")
}

/// Opens (creating or upgrading if needed) the database in `dir`.
fn open(dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let conn = Connection::open(dir.join(DB_FILE)).map_err(sql_err)?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(sql_err)?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(sql_err)?;
    match version {
        0 => conn
            .execute_batch(&format!(
                "BEGIN; {SCHEMA} PRAGMA user_version = {SCHEMA_VERSION}; COMMIT;"
            ))
            .map_err(sql_err)?,
        SCHEMA_VERSION => {}
        newer => {
            return Err(format!(
                "{DB_FILE} has schema version {newer}, newer than this app reads ({SCHEMA_VERSION}). Update the app, or delete the file: it is rebuilt from the study feed."
            ))
        }
    }
    Ok(conn)
}

/// The row `sql_where` selects, if it is readable and was folded with exactly `policy` and
/// `timezone`. Anything else is rebuilt from event 0, which gives the same state a resume would.
fn find(
    conn: &Connection,
    policy: &SkillPolicy,
    timezone: Tz,
    sql_where: &str,
    params: impl Params,
) -> Result<Option<Projection>, String> {
    type Row = (String, i64, String, String, u32);
    let row: Option<Row> = conn
        .query_row(
            &format!(
                "SELECT store_id, last_event_seq, fold, pending, skipped_unknown FROM projections {sql_where}"
            ),
            params,
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((store_id, last_event_seq, fold, pending, skipped_unknown)) = row else {
        return Ok(None);
    };
    let decoded = serde_json::from_str::<SkillFold>(&fold)
        .and_then(|fold| Ok((fold, serde_json::from_str(&pending)?)));
    match decoded {
        Ok((fold, pending)) if fold.policy() == policy && fold.timezone() == timezone => {
            Ok(Some(Projection {
                store_id,
                last_event_seq,
                fold,
                pending,
                skipped_unknown,
            }))
        }
        Ok(_) => {
            eprintln!("[tutor-shell] {DB_FILE}: the saved fold for store {store_id} has other policy settings; rebuilding it from event 0");
            Ok(None)
        }
        Err(e) => {
            eprintln!("[tutor-shell] {DB_FILE}: the saved fold for store {store_id} is unreadable ({e}); rebuilding it from event 0");
            Ok(None)
        }
    }
}

/// The most recently saved row for this policy and time zone, whatever its store.
fn latest(
    conn: &Connection,
    policy: &SkillPolicy,
    timezone: Tz,
) -> Result<Option<Projection>, String> {
    find(
        conn,
        policy,
        timezone,
        "WHERE policy_version = ?1 AND timezone = ?2 ORDER BY updated_at DESC, rowid DESC LIMIT 1",
        params![policy.version, timezone.name()],
    )
}

fn load(
    conn: &Connection,
    store_id: &str,
    policy: &SkillPolicy,
    timezone: Tz,
) -> Result<Option<Projection>, String> {
    find(
        conn,
        policy,
        timezone,
        "WHERE store_id = ?1 AND policy_version = ?2 AND timezone = ?3",
        params![store_id, policy.version, timezone.name()],
    )
}

/// Saves the projection in one transaction. A row already further along (a concurrent catch-up
/// that saw more events) is kept.
fn save(conn: &mut Connection, p: &Projection) -> Result<(), String> {
    let json_err = |e: serde_json::Error| format!("learning database: could not encode: {e}");
    let fold = serde_json::to_string(&p.fold).map_err(json_err)?;
    let pending = serde_json::to_string(&p.pending).map_err(json_err)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64());
    let tx = conn.transaction().map_err(sql_err)?;
    tx.execute(
        "INSERT INTO projections (store_id, policy_version, timezone, last_event_seq, fold, pending,
             skipped_unknown, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (store_id, policy_version, timezone) DO UPDATE SET
             last_event_seq = excluded.last_event_seq, fold = excluded.fold,
             pending = excluded.pending, skipped_unknown = excluded.skipped_unknown,
             updated_at = excluded.updated_at
         WHERE excluded.last_event_seq >= projections.last_event_seq",
        params![
            p.store_id,
            p.fold.policy().version,
            p.fold.timezone().name(),
            p.last_event_seq,
            fold,
            pending,
            p.skipped_unknown,
            now,
        ],
    )
    .map_err(sql_err)?;
    tx.commit().map_err(sql_err)
}

// ---- backend ----------------------------------------------------------------------------------

async fn get_json<T: DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
    what: &str,
) -> Result<T, String> {
    let response = http
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Could not reach the study backend for the {what} ({e}). Check that the backend is running in Settings."
            )
        })?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("The {what} could not be read ({e})."))?;
    if !status.is_success() {
        return Err(format!(
            "The study backend answered {status} for the {what}."
        ));
    }
    serde_json::from_str(&body).map_err(|e| format!("The {what} is malformed: {e}."))
}

fn check_schema(version: u32, what: &str) -> Result<(), String> {
    if version == FEED_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(format!(
            "The {what} has schema version {version}; this app reads version {FEED_SCHEMA_VERSION}. Update the app."
        ))
    }
}

async fn feed_page(http: &reqwest::Client, base: &str, after: i64) -> Result<FeedPage, String> {
    let url = format!("{base}/api/study/feed?after_event_seq={after}&limit={PAGE_LIMIT}");
    let page: FeedPage = get_json(http, &url, "study feed").await?;
    check_schema(page.schema_version, "study feed")?;
    Ok(page)
}

/// Resumes the saved fold for the feed's store (or starts one at event 0), reads the feed until a
/// page has no events, and saves the result.
async fn catch_up(
    http: &reqwest::Client,
    base: &str,
    dir: &Path,
    policy: SkillPolicy,
    timezone: Tz,
) -> Result<Projection, String> {
    let resume = latest(&open(dir)?, &policy, timezone)?;
    let requested = resume.as_ref().map_or(0, |p| p.last_event_seq);
    let mut page = feed_page(http, base, requested).await?;
    let mut projection = match resume {
        Some(p) if p.store_id == page.store_id => p,
        _ => {
            // First run, or another workspace: that store's own row, else a new one.
            let p = load(&open(dir)?, &page.store_id, &policy, timezone)?.unwrap_or_else(|| {
                Projection::new(page.store_id.clone(), policy.clone(), timezone)
            });
            if p.last_event_seq != requested {
                page = feed_page(http, base, p.last_event_seq).await?;
            }
            p
        }
    };
    loop {
        if page.store_id != projection.store_id {
            return Err(
                "The study store changed while the skills were catching up (another workspace was opened). Try again."
                    .into(),
            );
        }
        if page.events.is_empty() {
            break;
        }
        projection.apply(&page.events)?;
        page = feed_page(http, base, projection.last_event_seq).await?;
    }
    save(&mut open(dir)?, &projection)?;
    Ok(projection)
}

#[derive(Debug, Deserialize)]
struct Candidates {
    schema_version: u32,
    items: Vec<CandidateItem>,
}

#[derive(Debug, Deserialize)]
struct CandidateItem {
    entry_id: i64,
    track: String,
    question_type: String,
    due_at: Option<f64>,
    retrievability: Option<f64>,
    review_count: u32,
    estimated_seconds: Option<u32>,
    is_new: bool,
}

/// The plan's item kind. The backend sends none, so the shell derives it:
/// - a `lang:<code>` track is a language card (recognition, production and listening alike);
/// - question types answered in extended prose (`essay`, `writing`, `free_response`) are writing;
/// - question types that need a worked, multi-step solution (`proof`, `derivation`,
///   `calculation`, `problem`, `multi_step`) are procedures;
/// - everything else (multiple choice, short answer, cloze, ...) is an atom.
///
/// The kind sets the default time estimate and the procedures-and-writing share of the plan.
fn candidate_kind(track: &str, question_type: &str) -> ItemKind {
    if track.starts_with("lang:") {
        return ItemKind::LanguageCard;
    }
    match question_type {
        "essay" | "writing" | "free_response" => ItemKind::Writing,
        "proof" | "derivation" | "calculation" | "problem" | "multi_step" => ItemKind::Procedure,
        _ => ItemKind::Atom,
    }
}

impl CandidateItem {
    fn into_candidate(self) -> Candidate {
        Candidate {
            kind: candidate_kind(&self.track, &self.question_type),
            entry_id: self.entry_id,
            track: self.track,
            retrievability: self.retrievability,
            due_at: self.due_at,
            review_count: self.review_count,
            estimated_seconds: self.estimated_seconds,
            is_new: self.is_new,
        }
    }
}

async fn plan_from_backend(
    http: &reqwest::Client,
    base: &str,
    request: &PlanRequest,
) -> Result<Plan, String> {
    let url = format!("{base}/api/study/candidates");
    let body: Candidates = get_json(http, &url, "study candidates").await?;
    check_schema(body.schema_version, "study candidates")?;
    let candidates: Vec<Candidate> = body
        .items
        .into_iter()
        .map(CandidateItem::into_candidate)
        .collect();
    plan::plan(request, &candidates).map_err(|e| format!("The study plan could not be made: {e}."))
}

fn parse_timezone(name: &str) -> Result<Tz, String> {
    name.trim().parse().map_err(|_| {
        format!(
            "{name:?} is not a time zone this app knows; use an IANA name such as Europe/London."
        )
    })
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn learning_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    timezone: String,
) -> Result<Skills, String> {
    let timezone = parse_timezone(&timezone)?;
    let projection = catch_up(
        &state.http,
        ServiceName::Backend.url(),
        &data_dir(&app)?,
        SkillPolicy::default(),
        timezone,
    )
    .await?;
    Ok(projection.report())
}

#[tauri::command]
pub(crate) async fn learning_plan(
    state: State<'_, AppState>,
    request: PlanRequest,
) -> Result<Plan, String> {
    plan_from_backend(&state.http, ServiceName::Backend.url(), &request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::New_York;
    use chrono_tz::UTC;
    use serde_json::json;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    const T0: f64 = 1_790_000_000.0;
    const DAY: f64 = 86_400.0;
    const STORE_A: &str = "0f6f7a0e-a000-4000-8000-00000000000a";
    const STORE_B: &str = "0f6f7a0e-b000-4000-8000-00000000000b";
    /// The fake backend serves at most this many events per page, whatever the limit asked.
    const FAKE_PAGE: usize = 2;

    fn event(seq: i64, kind: &str, attempt: Option<&str>, at: f64, payload: Value) -> Value {
        json!({"event_seq": seq, "session_id": "s1", "seq": seq, "attempt_id": attempt,
               "kind": kind, "recorded_at": at, "source": "ui", "payload": payload})
    }

    fn opened(
        seq: i64,
        attempt: &str,
        at: f64,
        entry: i64,
        skills: &[&str],
        family: Option<&str>,
    ) -> Value {
        let payload = json!({"entry_id": entry, "item_version": "v1", "skill_ids": skills,
                             "family_id": family, "exposure": []});
        event(seq, "item_opened", Some(attempt), at, payload)
    }

    fn closed(
        seq: i64,
        attempt: &str,
        at: f64,
        reason: &str,
        disposition: Value,
        eligible: bool,
        reasons: &[&str],
    ) -> Value {
        let payload = json!({"reason": reason, "scheduler_disposition": disposition,
                             "measurement_eligibility": {"eligible": eligible, "reasons": reasons}});
        event(seq, "attempt_closed", Some(attempt), at, payload)
    }

    fn review(first_outcome: &str, provenance: &str) -> Value {
        json!({"first_outcome": first_outcome, "assistance_before_check": provenance == "assisted",
               "eligible_for_fsrs": true, "rating": "again", "provenance": provenance,
               "applied": {"request_id": "ev:x", "due_at": T0 + DAY}})
    }

    /// A C12 feed: a verified success, an abandoned attempt with no first outcome, an assisted
    /// (ineligible) success, a closure whose opening is unknown, an attempt left open, and a
    /// verified miss 8 days later.
    fn fixture() -> Vec<Value> {
        vec![
            event(
                1,
                "session_started",
                None,
                T0 - 10.0,
                json!({"mode": "practice"}),
            ),
            opened(2, "a1", T0, 11, &["frac.add"], Some("f1")),
            event(
                3,
                "response_committed",
                Some("a1"),
                T0 + 30.0,
                json!({"input_mode": "typed"}),
            ),
            closed(
                4,
                "a1",
                T0 + 60.0,
                "correct",
                review("correct", "verified"),
                true,
                &[],
            ),
            opened(5, "a2", T0 + 90.0, 12, &["frac.add", "frac.sub"], None),
            closed(
                6,
                "a2",
                T0 + 120.0,
                "abandoned",
                json!({"eligible_for_fsrs": false, "rating": null}),
                false,
                &["no_checked_response"],
            ),
            opened(7, "a3", T0 + 130.0, 13, &["frac.sub"], Some("f2")),
            event(
                8,
                "assistance_given",
                Some("a3"),
                T0 + 140.0,
                json!({"kind": "hint", "rung": 1}),
            ),
            closed(
                9,
                "a3",
                T0 + 180.0,
                "correct",
                review("correct", "assisted"),
                false,
                &["assisted"],
            ),
            closed(
                10,
                "a9",
                T0 + 200.0,
                "correct",
                review("correct", "verified"),
                true,
                &[],
            ),
            opened(11, "a4", T0 + 300.0, 14, &["frac.add"], Some("f1")),
            opened(12, "a5", T0 + 8.0 * DAY, 15, &["frac.add"], Some("f3")),
            closed(
                13,
                "a5",
                T0 + 8.0 * DAY + 30.0,
                "remediated",
                review("incorrect", "verified"),
                true,
                &[],
            ),
        ]
    }

    fn expected_attempts() -> Vec<ClosedAttempt> {
        let attempt = |id: &str,
                       at: f64,
                       eligible: bool,
                       reasons: &[&str],
                       first_outcome,
                       skills: &[&str],
                       family: Option<&str>,
                       entry_id| ClosedAttempt {
            attempt_id: id.into(),
            closed_at: at,
            eligible,
            ineligible_reasons: reasons.iter().map(|r| r.to_string()).collect(),
            first_outcome,
            skill_ids: skills.iter().map(|s| s.to_string()).collect(),
            family_id: family.map(str::to_owned),
            entry_id,
        };
        vec![
            attempt(
                "a1",
                T0 + 60.0,
                true,
                &[],
                FirstOutcome::Correct,
                &["frac.add"],
                Some("f1"),
                11,
            ),
            attempt(
                "a2",
                T0 + 120.0,
                false,
                &["no_checked_response"],
                FirstOutcome::Other,
                &["frac.add", "frac.sub"],
                None,
                12,
            ),
            attempt(
                "a3",
                T0 + 180.0,
                false,
                &["assisted"],
                FirstOutcome::Correct,
                &["frac.sub"],
                Some("f2"),
                13,
            ),
            attempt(
                "a5",
                T0 + 8.0 * DAY + 30.0,
                true,
                &[],
                FirstOutcome::Incorrect,
                &["frac.add"],
                Some("f3"),
                15,
            ),
        ]
    }

    fn events(values: &[Value]) -> Vec<FeedEvent> {
        values
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .collect()
    }

    fn fresh(store_id: &str, policy: SkillPolicy, timezone: Tz) -> Projection {
        Projection::new(store_id.into(), policy, timezone)
    }

    fn fold_of(attempts: &[ClosedAttempt], policy: SkillPolicy, timezone: Tz) -> SkillFold {
        let mut fold = SkillFold::new(policy, timezone);
        for a in attempts {
            fold.push(a).unwrap();
        }
        fold
    }

    #[test]
    fn feed_events_map_to_closed_attempts() {
        let mut p = fresh(STORE_A, SkillPolicy::default(), UTC);
        let joined: Vec<ClosedAttempt> = events(&fixture())
            .iter()
            .filter_map(|e| p.join(e).unwrap())
            .collect();
        assert_eq!(joined, expected_attempts());
        assert_eq!(p.skipped_unknown, 1);
        assert_eq!(p.pending.keys().collect::<Vec<_>>(), ["a4"]);
        assert_eq!(
            p.pending["a4"],
            Opened {
                entry_id: 14,
                skill_ids: vec!["frac.add".into()],
                family_id: Some("f1".into())
            }
        );
    }

    #[test]
    fn pages_split_anywhere_fold_like_the_attempts_in_one_pass() {
        let all = events(&fixture());
        let expected = fold_of(&expected_attempts(), SkillPolicy::default(), UTC);
        for split in 0..=all.len() {
            let mut p = fresh(STORE_A, SkillPolicy::default(), UTC);
            p.apply(&all[..split]).unwrap();
            p.apply(&all[split..]).unwrap();
            assert_eq!(p.fold, expected, "split at {split}");
            assert_eq!((p.last_event_seq, p.skipped_unknown), (13, 1));
        }
        let p = {
            let mut p = fresh(STORE_A, SkillPolicy::default(), UTC);
            p.apply(&all).unwrap();
            p
        };
        assert_eq!(
            p.fold.excluded(),
            &BTreeMap::from([("assisted".into(), 1), ("no_checked_response".into(), 1)])
        );
        let states = p.fold.states();
        let skills: Vec<&str> = states.iter().map(|s| s.skill_id.as_str()).collect();
        assert_eq!(skills, ["frac.add", "frac.sub"]);
        let add = &states[0];
        assert_eq!(
            (add.eligible_observations, add.delayed_evidence.len()),
            (2, 1)
        );
        assert_eq!(add.delayed_evidence[0].y, 0);
    }

    #[test]
    fn out_of_order_and_unreadable_events_are_readable_errors() {
        let mut p = fresh(STORE_A, SkillPolicy::default(), UTC);
        p.apply(&events(&fixture()[..4])).unwrap();
        let err = p.apply(&events(&fixture()[2..3])).unwrap_err();
        assert_eq!(
            err,
            "The study feed is malformed: event 3 (response_committed) is out of order (after event 4)."
        );

        let mut bad = fixture()[5].clone();
        bad["payload"]
            .as_object_mut()
            .unwrap()
            .remove("measurement_eligibility");
        let mut p = fresh(STORE_A, SkillPolicy::default(), UTC);
        let err = p.apply(&events(&[fixture()[4].clone(), bad])).unwrap_err();
        assert!(
            err.starts_with("The study feed is malformed: event 6 (attempt_closed) has an unreadable payload: missing field `measurement_eligibility`"),
            "{err}"
        );

        let mut no_attempt = fixture()[1].clone();
        no_attempt["attempt_id"] = Value::Null;
        let err = fresh(STORE_A, SkillPolicy::default(), UTC)
            .apply(&events(&[no_attempt]))
            .unwrap_err();
        assert_eq!(
            err,
            "The study feed is malformed: event 2 (item_opened) has no attempt_id."
        );
    }

    // ---- a local backend for the feed and candidates routes ---------------------------------

    #[derive(Default)]
    struct Backend {
        store_id: String,
        events: Vec<Value>,
        candidates: Vec<Value>,
        /// Answers every request with this status and body instead.
        fixed: Option<(&'static str, String)>,
        /// Request targets (path and query), in order.
        requests: Vec<String>,
    }

    impl Backend {
        fn answer(&mut self, target: &str) -> (&'static str, String) {
            self.requests.push(target.to_owned());
            if let Some(fixed) = &self.fixed {
                return fixed.clone();
            }
            let (path, query) = target.split_once('?').unwrap_or((target, ""));
            let param = |name: &str| {
                query
                    .split('&')
                    .find_map(|kv| kv.strip_prefix(name)?.strip_prefix('='))
                    .map(|v| v.parse::<i64>().unwrap())
            };
            match path {
                "/api/study/feed" => {
                    let after = param("after_event_seq").unwrap();
                    let limit = usize::try_from(param("limit").unwrap()).unwrap();
                    let page: Vec<&Value> = self
                        .events
                        .iter()
                        .filter(|e| e["event_seq"].as_i64().unwrap() > after)
                        .take(limit.min(FAKE_PAGE))
                        .collect();
                    let next_after = page
                        .last()
                        .map_or(after, |e| e["event_seq"].as_i64().unwrap());
                    let body = json!({"schema_version": 1, "store_id": self.store_id,
                                      "events": page, "next_after": next_after});
                    ("200 OK", body.to_string())
                }
                "/api/study/candidates" => {
                    let body = json!({"schema_version": 1, "store_id": self.store_id, "now": T0,
                                      "items": self.candidates});
                    ("200 OK", body.to_string())
                }
                _ => ("404 Not Found", r#"{"detail": "Not Found"}"#.into()),
            }
        }

        fn feed_afters(&self) -> Vec<i64> {
            self.requests
                .iter()
                .filter_map(|t| t.strip_prefix("/api/study/feed?after_event_seq="))
                .map(|rest| rest.split('&').next().unwrap().parse().unwrap())
                .collect()
        }
    }

    /// Serves `backend` on 127.0.0.1, one request per connection, until the test process ends.
    fn serve(backend: Backend) -> (String, Arc<Mutex<Backend>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let shared = Arc::new(Mutex::new(backend));
        let state = shared.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                        break;
                    }
                }
                let target = request_line.split_whitespace().nth(1).unwrap_or("");
                let (status, body) = state.lock().unwrap().answer(target);
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = (&stream).write_all(response.as_bytes());
            }
        });
        (base, shared)
    }

    fn backend(store_id: &str, events: &[Value]) -> Backend {
        Backend {
            store_id: store_id.into(),
            events: events.to_vec(),
            ..Backend::default()
        }
    }

    fn run(
        base: &str,
        dir: &Path,
        policy: SkillPolicy,
        timezone: Tz,
    ) -> Result<Projection, String> {
        tauri::async_runtime::block_on(catch_up(&crate::http_client(), base, dir, policy, timezone))
    }

    fn take_afters(state: &Arc<Mutex<Backend>>) -> Vec<i64> {
        let mut backend = state.lock().unwrap();
        let afters = backend.feed_afters();
        backend.requests.clear();
        afters
    }

    fn rows(dir: &Path) -> Vec<(String, String, String, i64)> {
        let conn = open(dir).unwrap();
        let mut stmt = conn
            .prepare("SELECT store_id, policy_version, timezone, last_event_seq FROM projections ORDER BY rowid")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn catch_up_in_two_steps_equals_catch_up_in_one() {
        let policy = SkillPolicy::default();
        let (base, one_state) = serve(backend(STORE_A, &fixture()));
        let one_dir = tempfile::tempdir().unwrap();
        let one = run(&base, one_dir.path(), policy.clone(), UTC).unwrap();
        assert_eq!(take_afters(&one_state), [0, 2, 4, 6, 8, 10, 12, 13]);

        // The split leaves attempt a3 open across the saved state.
        let (base, two_state) = serve(backend(STORE_A, &fixture()[..7]));
        let two_dir = tempfile::tempdir().unwrap();
        let first = run(&base, two_dir.path(), policy.clone(), UTC).unwrap();
        assert_eq!(first.last_event_seq, 7);
        assert!(first.pending.contains_key("a3"));
        two_state.lock().unwrap().events = fixture();
        take_afters(&two_state);
        let two = run(&base, two_dir.path(), policy.clone(), UTC).unwrap();
        assert_eq!(take_afters(&two_state), [7, 9, 11, 13]);

        assert_eq!(
            serde_json::to_string(&two.fold).unwrap(),
            serde_json::to_string(&one.fold).unwrap()
        );
        assert_eq!(two, one);
        assert_eq!(two.fold, fold_of(&expected_attempts(), policy.clone(), UTC));

        // Nothing new: one request, and the saved row is what the next call resumes.
        let again = run(&base, two_dir.path(), policy.clone(), UTC).unwrap();
        assert_eq!(take_afters(&two_state), [13]);
        assert_eq!(again, one);
        assert_eq!(
            rows(two_dir.path()),
            [(STORE_A.into(), "skill-policy/1".into(), "UTC".into(), 13)]
        );

        let report = serde_json::to_value(again.report()).unwrap();
        let mut keys: Vec<&str> = report
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "excluded",
                "last_event_seq",
                "limitations",
                "policy_version",
                "skills",
                "skipped_unknown",
                "store_id"
            ]
        );
        assert_eq!(report["store_id"], STORE_A);
        assert_eq!(report["policy_version"], "skill-policy/1");
        assert_eq!(report["last_event_seq"], 13);
        assert_eq!(report["skipped_unknown"], 1);
        assert_eq!(
            report["excluded"],
            json!({"assisted": 1, "no_checked_response": 1})
        );
        assert_eq!(report["skills"][0]["skill_id"], "frac.add");
        assert_eq!(report["skills"][0]["status"], "insufficient");
        assert_eq!(
            report["limitations"][0],
            "Closed attempts not counted because their item_opened event is missing from the feed: 1."
        );
    }

    #[test]
    fn a_different_store_id_starts_a_new_row() {
        let policy = SkillPolicy::default();
        let dir = tempfile::tempdir().unwrap();
        let (base, state) = serve(backend(STORE_A, &fixture()));
        let a = run(&base, dir.path(), policy.clone(), UTC).unwrap();
        take_afters(&state);

        // Another workspace: the feed's store differs, so its events start at 0 in a new row.
        {
            let mut b = state.lock().unwrap();
            b.store_id = STORE_B.into();
            b.events = fixture()[..4].to_vec();
        }
        let b = run(&base, dir.path(), policy.clone(), UTC).unwrap();
        assert_eq!(take_afters(&state), [13, 0, 2, 4]);
        assert_eq!(b.store_id, STORE_B);
        assert_eq!(b.last_event_seq, 4);
        assert_eq!(
            b.fold,
            fold_of(&expected_attempts()[..1], policy.clone(), UTC)
        );
        assert_eq!(
            rows(dir.path()),
            [
                (STORE_A.into(), "skill-policy/1".into(), "UTC".into(), 13),
                (STORE_B.into(), "skill-policy/1".into(), "UTC".into(), 4),
            ]
        );

        // Back to the first workspace: its own row resumes where it stopped.
        {
            let mut back = state.lock().unwrap();
            back.store_id = STORE_A.into();
            back.events = fixture();
        }
        let resumed = run(&base, dir.path(), policy, UTC).unwrap();
        assert_eq!(take_afters(&state), [4, 13]);
        assert_eq!(resumed, a);
    }

    #[test]
    fn a_policy_or_timezone_change_rebuilds_from_event_zero() {
        let dir = tempfile::tempdir().unwrap();
        let (base, state) = serve(backend(STORE_A, &fixture()));
        run(&base, dir.path(), SkillPolicy::default(), UTC).unwrap();
        take_afters(&state);

        let ny = run(&base, dir.path(), SkillPolicy::default(), New_York).unwrap();
        assert_eq!(take_afters(&state)[0], 0);
        assert_eq!(
            ny.fold,
            fold_of(&expected_attempts(), SkillPolicy::default(), New_York)
        );

        let other = SkillPolicy {
            version: "skill-policy/test".into(),
            half_life_days: 14.0,
            ..SkillPolicy::default()
        };
        let rebuilt = run(&base, dir.path(), other.clone(), New_York).unwrap();
        assert_eq!(take_afters(&state)[0], 0);
        assert_eq!(rebuilt.fold, fold_of(&expected_attempts(), other, New_York));
        let keys: Vec<(String, String)> =
            rows(dir.path()).into_iter().map(|r| (r.1, r.2)).collect();
        assert_eq!(
            keys,
            [
                ("skill-policy/1".into(), "UTC".into()),
                ("skill-policy/1".into(), "America/New_York".into()),
                ("skill-policy/test".into(), "America/New_York".into()),
            ]
        );
    }

    #[test]
    fn an_unreadable_or_mismatched_saved_fold_is_rebuilt_from_event_zero() {
        let dir = tempfile::tempdir().unwrap();
        let (base, state) = serve(backend(STORE_A, &fixture()));
        let good = run(&base, dir.path(), SkillPolicy::default(), UTC).unwrap();
        take_afters(&state);

        open(dir.path())
            .unwrap()
            .execute("UPDATE projections SET fold = '{\"not\": \"a fold\"}'", [])
            .unwrap();
        let rebuilt = run(&base, dir.path(), SkillPolicy::default(), UTC).unwrap();
        assert_eq!(take_afters(&state)[0], 0);
        assert_eq!(rebuilt, good);
        assert_eq!(rows(dir.path())[0].3, 13);

        // Same version, other numbers: the saved fold is not resumed either.
        let tweaked = SkillPolicy {
            half_life_days: 60.0,
            ..SkillPolicy::default()
        };
        let p = run(&base, dir.path(), tweaked.clone(), UTC).unwrap();
        assert_eq!(take_afters(&state)[0], 0);
        assert_eq!(p.fold, fold_of(&expected_attempts(), tweaked, UTC));
    }

    #[test]
    fn store_opens_missing_and_older_files_and_refuses_newer_ones() {
        let root = tempfile::tempdir().unwrap();
        let version = |conn: &Connection| -> i64 {
            conn.query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap()
        };
        let has_table = |conn: &Connection| -> bool {
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'projections'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
                == 1
        };

        // Missing directory and file.
        let missing = root.path().join("new").join("dir");
        let conn = open(&missing).unwrap();
        assert!(missing.join(DB_FILE).is_file());
        assert_eq!(version(&conn), SCHEMA_VERSION);
        assert!(has_table(&conn));
        drop(conn);
        drop(open(&missing).unwrap()); // idempotent

        // An empty file and an older (version 0) file with other content.
        let empty = root.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::write(empty.join(DB_FILE), b"").unwrap();
        assert!(has_table(&open(&empty).unwrap()));

        let older = root.path().join("older");
        std::fs::create_dir_all(&older).unwrap();
        Connection::open(older.join(DB_FILE))
            .unwrap()
            .execute_batch("CREATE TABLE notes (x TEXT); INSERT INTO notes VALUES ('kept');")
            .unwrap();
        let conn = open(&older).unwrap();
        assert_eq!(version(&conn), SCHEMA_VERSION);
        assert!(has_table(&conn));
        let kept: String = conn
            .query_row("SELECT x FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kept, "kept");
        drop(conn);

        // A newer file is refused, not overwritten.
        let newer = root.path().join("newer");
        std::fs::create_dir_all(&newer).unwrap();
        Connection::open(newer.join(DB_FILE))
            .unwrap()
            .execute_batch("PRAGMA user_version = 2;")
            .unwrap();
        let err = open(&newer).unwrap_err();
        assert!(
            err.starts_with("learning.sqlite has schema version 2, newer than this app reads (1)."),
            "{err}"
        );
    }

    #[test]
    fn backend_failures_are_readable_errors() {
        let dir = tempfile::tempdir().unwrap();
        let policy = SkillPolicy::default;
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = run(
            &format!("http://127.0.0.1:{port}"),
            dir.path(),
            policy(),
            UTC,
        )
        .unwrap_err();
        assert!(
            err.starts_with("Could not reach the study backend for the study feed ("),
            "{err}"
        );
        assert!(
            err.ends_with("Check that the backend is running in Settings."),
            "{err}"
        );

        let fixed = |status: &'static str, body: &str| {
            serve(Backend {
                fixed: Some((status, body.into())),
                ..Backend::default()
            })
            .0
        };
        let base = fixed("500 Internal Server Error", r#"{"detail": "boom"}"#);
        assert_eq!(
            run(&base, dir.path(), policy(), UTC).unwrap_err(),
            "The study backend answered 500 Internal Server Error for the study feed."
        );
        let base = fixed("200 OK", "<html>not json</html>");
        let err = run(&base, dir.path(), policy(), UTC).unwrap_err();
        assert!(
            err.starts_with("The study feed is malformed: expected value"),
            "{err}"
        );
        let base = fixed(
            "200 OK",
            r#"{"schema_version": 2, "store_id": "s", "events": [], "next_after": 0}"#,
        );
        assert_eq!(
            run(&base, dir.path(), policy(), UTC).unwrap_err(),
            "The study feed has schema version 2; this app reads version 1. Update the app."
        );

        // A malformed event saves nothing.
        let mut events = fixture()[..4].to_vec();
        events[3]["payload"]["measurement_eligibility"] = json!({"reasons": []});
        let (base, _state) = serve(backend(STORE_A, &events));
        let err = run(&base, dir.path(), policy(), UTC).unwrap_err();
        assert!(
            err.starts_with("The study feed is malformed: event 4 (attempt_closed) has an unreadable payload: missing field `eligible`"),
            "{err}"
        );
        assert!(rows(dir.path()).is_empty());

        assert_eq!(
            parse_timezone(" Europe/London ").unwrap(),
            chrono_tz::Europe::London
        );
        assert_eq!(
            parse_timezone("Mars/Olympus").unwrap_err(),
            "\"Mars/Olympus\" is not a time zone this app knows; use an IANA name such as Europe/London."
        );
    }

    #[test]
    fn candidate_kinds_follow_the_documented_mapping() {
        use ItemKind::*;
        for (track, question_type, kind) in [
            ("lang:es", "translation", LanguageCard),
            ("lang:ja", "essay", LanguageCard),
            ("lang:fr", "proof", LanguageCard),
            ("english", "essay", Writing),
            ("english", "writing", Writing),
            ("general", "free_response", Writing),
            ("math", "proof", Procedure),
            ("math", "derivation", Procedure),
            ("chemistry", "calculation", Procedure),
            ("science", "problem", Procedure),
            ("math", "multi_step", Procedure),
            ("math", "multiple_choice", Atom),
            ("math", "short_answer", Atom),
            ("general", "", Atom),
            ("language", "translation", Atom),
            ("math", "Proof", Atom),
        ] {
            assert_eq!(
                candidate_kind(track, question_type),
                kind,
                "{track} {question_type}"
            );
        }
    }

    #[test]
    fn learning_plan_plans_the_backend_candidates() {
        let item = |entry_id: i64,
                    track: &str,
                    question_type: &str,
                    retrievability: Option<f64>,
                    is_new: bool| {
            json!({"entry_id": entry_id, "track": track, "question_type": question_type,
                   "lang": track.strip_prefix("lang:"), "skill_ids": [], "family_id": null, "tags": [],
                   "due_at": T0 - DAY, "retrievability": retrievability, "stability": 3.0,
                   "difficulty": 5.0, "review_count": if is_new { 0 } else { 2 },
                   "last_review_at": null, "fsrs_state": 2, "estimated_seconds": null,
                   "is_new": is_new})
        };
        let (base, state) = serve(Backend {
            store_id: STORE_A.into(),
            candidates: vec![
                item(1, "lang:es", "translation", Some(0.4), false),
                item(2, "math", "proof", Some(0.9), false),
                item(3, "math", "multiple_choice", None, true),
                item(4, "chemistry", "calculation", Some(0.1), false),
            ],
            ..Backend::default()
        });
        let request: PlanRequest = serde_json::from_value(json!({
            "now": T0, "timezone": "Europe/London", "available_minutes": 30,
            "active_tracks": ["lang:es", "math"], "new_item_budget": 1
        }))
        .unwrap();
        let plan = tauri::async_runtime::block_on(plan_from_backend(
            &crate::http_client(),
            &base,
            &request,
        ))
        .unwrap();
        assert_eq!(state.lock().unwrap().requests, ["/api/study/candidates"]);

        let picked: Vec<(i64, u32)> = plan
            .items
            .iter()
            .map(|i| (i.entry_id, i.estimated_seconds))
            .collect();
        // Language card 15 s, procedure 180 s, atom 20 s; chemistry is not an active track.
        assert_eq!(picked, [(1, 15), (2, 180), (3, 20)]);
        let candidate = |entry_id, track: &str, kind, retrievability, is_new: bool| Candidate {
            entry_id,
            track: track.into(),
            kind,
            retrievability,
            due_at: Some(T0 - DAY),
            review_count: if is_new { 0 } else { 2 },
            estimated_seconds: None,
            is_new,
        };
        let by_hand = plan::plan(
            &request,
            &[
                candidate(1, "lang:es", ItemKind::LanguageCard, Some(0.4), false),
                candidate(2, "math", ItemKind::Procedure, Some(0.9), false),
                candidate(3, "math", ItemKind::Atom, None, true),
                candidate(4, "chemistry", ItemKind::Procedure, Some(0.1), false),
            ],
        )
        .unwrap();
        assert_eq!(plan, by_hand);

        let bad = PlanRequest {
            available_minutes: -1.0,
            ..request
        };
        assert_eq!(
            tauri::async_runtime::block_on(plan_from_backend(&crate::http_client(), &base, &bad))
                .unwrap_err(),
            "The study plan could not be made: plan request: invalid available_minutes (-1)."
        );
    }
}
