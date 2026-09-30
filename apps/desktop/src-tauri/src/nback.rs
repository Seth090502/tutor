//! Dual N-Back session storage (contract C6, docs/tutor/CONTRACTS.md): SQLite at
//! `<app data dir>/nback.sqlite` with per-trial rows, and CSV export to `<app data dir>/exports/`.

use std::path::{Path, PathBuf};

use rusqlite::types::Value;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const DB_FILE: &str = "nback.sqlite";
const DEFAULT_LIMIT: u32 = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    pub hits: u32,
    pub misses: u32,
    pub false_alarms: u32,
    pub correct_rejections: u32,
    pub d_prime: f64,
    pub c: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trial {
    pub i: u32,
    pub position: Option<u32>,
    pub letter: Option<String>,
    pub pos_match: bool,
    pub let_match: bool,
    pub lure: bool,
    pub pos_response: bool,
    pub let_response: bool,
    pub pos_rt_ms: Option<f64>,
    pub let_rt_ms: Option<f64>,
    pub intended_onset: f64,
    pub actual_onset: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockMetrics {
    pub position: Option<Metrics>,
    pub letter: Option<Metrics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub index: u32,
    pub n: u32,
    pub void: bool,
    pub void_reason: Option<String>,
    pub trials: Vec<Trial>,
    pub metrics: BlockMetrics,
    pub next_n: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Dual,
    Position,
    Letter,
    Transfer,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Dual => "dual",
            Mode::Position => "position",
            Mode::Letter => "letter",
            Mode::Transfer => "transfer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keys {
    pub position: String,
    pub letter: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub trial_ms: u32,
    pub stimulus_ms: u32,
    pub blocks: u32,
    pub up_max_errors: u32,
    pub down_min_errors: u32,
    pub lures: bool,
    pub in_trial_feedback: bool,
    pub keys: Keys,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    pub output_latency: Option<f64>,
    pub base_latency: Option<f64>,
    pub calibrated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkin {
    pub expectation: Option<f64>,
    pub sleep_hours: Option<f64>,
    pub exercise: Option<bool>,
    pub fatigue: Option<f64>,
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: f64,
    pub ended_at: f64,
    pub mode: Mode,
    pub seed: u32,
    pub settings: Settings,
    pub blocks: Vec<Block>,
    pub comparable: bool,
    pub timing: Timing,
    pub checkin: Checkin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DPrime {
    pub position: Option<f64>,
    pub letter: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub started_at: f64,
    pub mode: String,
    pub blocks: u32,
    pub void_blocks: u32,
    pub start_n: u32,
    pub max_n: u32,
    pub mean_n: f64,
    pub d_prime: DPrime,
    pub comparable: bool,
}

#[derive(Debug, Serialize)]
pub struct Saved {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct Sessions {
    pub sessions: Vec<Summary>,
}

#[derive(Debug, Serialize)]
pub struct Exported {
    pub path: String,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    started_at REAL NOT NULL,
    ended_at REAL NOT NULL,
    mode TEXT NOT NULL,
    seed INTEGER NOT NULL,
    settings TEXT NOT NULL,
    comparable INTEGER NOT NULL,
    timing TEXT NOT NULL,
    checkin TEXT NOT NULL,
    -- summary columns, computed on save
    blocks INTEGER NOT NULL,
    void_blocks INTEGER NOT NULL,
    start_n INTEGER NOT NULL,
    max_n INTEGER NOT NULL,
    mean_n REAL NOT NULL,
    d_prime_position REAL,
    d_prime_letter REAL
);
CREATE INDEX IF NOT EXISTS sessions_started_at ON sessions (started_at);
CREATE TABLE IF NOT EXISTS blocks (
    session_id TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    idx INTEGER NOT NULL,
    n INTEGER NOT NULL,
    void INTEGER NOT NULL,
    void_reason TEXT,
    metrics TEXT NOT NULL,
    next_n INTEGER NOT NULL,
    PRIMARY KEY (session_id, idx)
);
CREATE TABLE IF NOT EXISTS trials (
    session_id TEXT NOT NULL,
    block_idx INTEGER NOT NULL,
    i INTEGER NOT NULL,
    position INTEGER,
    letter TEXT,
    pos_match INTEGER NOT NULL,
    let_match INTEGER NOT NULL,
    lure INTEGER NOT NULL,
    pos_response INTEGER NOT NULL,
    let_response INTEGER NOT NULL,
    pos_rt_ms REAL,
    let_rt_ms REAL,
    intended_onset REAL NOT NULL,
    actual_onset REAL NOT NULL,
    PRIMARY KEY (session_id, block_idx, i),
    FOREIGN KEY (session_id, block_idx) REFERENCES blocks (session_id, idx) ON DELETE CASCADE
);
";

fn sql_err(e: rusqlite::Error) -> String {
    format!("n-back database: {e}")
}

fn json_err(e: serde_json::Error) -> String {
    format!("n-back database: bad stored JSON: {e}")
}

/// Open (creating if needed) the database in `dir`; the schema is idempotent.
fn open(dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let conn = Connection::open(dir.join(DB_FILE)).map_err(sql_err)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(sql_err)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(sql_err)?;
    conn.execute_batch(SCHEMA).map_err(sql_err)?;
    Ok(conn)
}

fn summarize(s: &Session) -> Summary {
    let scored: Vec<&Block> = s.blocks.iter().filter(|b| !b.void).collect();
    let start_n = s.blocks.first().map_or(0, |b| b.n);
    let (max_n, mean_n) = if scored.is_empty() {
        (start_n, f64::from(start_n))
    } else {
        let sum: u32 = scored.iter().map(|b| b.n).sum();
        (
            scored.iter().map(|b| b.n).max().unwrap_or(start_n),
            f64::from(sum) / scored.len() as f64,
        )
    };
    let mean_d = |pick: fn(&BlockMetrics) -> &Option<Metrics>| {
        let ds: Vec<f64> = scored
            .iter()
            .filter_map(|b| pick(&b.metrics).as_ref().map(|m| m.d_prime))
            .collect();
        (!ds.is_empty()).then(|| ds.iter().sum::<f64>() / ds.len() as f64)
    };
    Summary {
        id: s.id.clone(),
        started_at: s.started_at,
        mode: s.mode.as_str().to_string(),
        blocks: s.blocks.len() as u32,
        void_blocks: (s.blocks.len() - scored.len()) as u32,
        start_n,
        max_n,
        mean_n,
        d_prime: DPrime {
            position: mean_d(|m| &m.position),
            letter: mean_d(|m| &m.letter),
        },
        comparable: s.comparable,
    }
}

/// Save a session. Saving the same `id` again replaces it, so a retry or a later check-in never
/// duplicates a session.
pub fn save(dir: &Path, s: &Session) -> Result<Saved, String> {
    if s.id.trim().is_empty() {
        return Err("session id is empty".into());
    }
    let summary = summarize(s);
    let mut conn = open(dir)?;
    let tx = conn.transaction().map_err(sql_err)?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", [&s.id])
        .map_err(sql_err)?;
    tx.execute(
        "INSERT INTO sessions (id, started_at, ended_at, mode, seed, settings, comparable, timing,
             checkin, blocks, void_blocks, start_n, max_n, mean_n, d_prime_position, d_prime_letter)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            s.id,
            s.started_at,
            s.ended_at,
            s.mode.as_str(),
            s.seed,
            serde_json::to_string(&s.settings).map_err(json_err)?,
            s.comparable,
            serde_json::to_string(&s.timing).map_err(json_err)?,
            serde_json::to_string(&s.checkin).map_err(json_err)?,
            summary.blocks,
            summary.void_blocks,
            summary.start_n,
            summary.max_n,
            summary.mean_n,
            summary.d_prime.position,
            summary.d_prime.letter,
        ],
    )
    .map_err(sql_err)?;
    {
        let mut block_stmt = tx
            .prepare(
                "INSERT INTO blocks (session_id, idx, n, void, void_reason, metrics, next_n)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(sql_err)?;
        let mut trial_stmt = tx
            .prepare(
                "INSERT INTO trials (session_id, block_idx, i, position, letter, pos_match,
                     let_match, lure, pos_response, let_response, pos_rt_ms, let_rt_ms,
                     intended_onset, actual_onset)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )
            .map_err(sql_err)?;
        for b in &s.blocks {
            block_stmt
                .execute(params![
                    s.id,
                    b.index,
                    b.n,
                    b.void,
                    b.void_reason,
                    serde_json::to_string(&b.metrics).map_err(json_err)?,
                    b.next_n,
                ])
                .map_err(sql_err)?;
            for t in &b.trials {
                trial_stmt
                    .execute(params![
                        s.id,
                        b.index,
                        t.i,
                        t.position,
                        t.letter,
                        t.pos_match,
                        t.let_match,
                        t.lure,
                        t.pos_response,
                        t.let_response,
                        t.pos_rt_ms,
                        t.let_rt_ms,
                        t.intended_onset,
                        t.actual_onset,
                    ])
                    .map_err(sql_err)?;
            }
        }
    }
    tx.commit().map_err(sql_err)?;
    Ok(Saved { id: s.id.clone() })
}

/// Session summaries, newest first.
pub fn list(dir: &Path, limit: Option<u32>) -> Result<Sessions, String> {
    let conn = open(dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, started_at, mode, blocks, void_blocks, start_n, max_n, mean_n,
                    d_prime_position, d_prime_letter, comparable
             FROM sessions ORDER BY started_at DESC, id LIMIT ?1",
        )
        .map_err(sql_err)?;
    let sessions = stmt
        .query_map([limit.unwrap_or(DEFAULT_LIMIT)], |r| {
            Ok(Summary {
                id: r.get(0)?,
                started_at: r.get(1)?,
                mode: r.get(2)?,
                blocks: r.get(3)?,
                void_blocks: r.get(4)?,
                start_n: r.get(5)?,
                max_n: r.get(6)?,
                mean_n: r.get(7)?,
                d_prime: DPrime {
                    position: r.get(8)?,
                    letter: r.get(9)?,
                },
                comparable: r.get(10)?,
            })
        })
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    Ok(Sessions { sessions })
}

/// One full session, rebuilt from its rows.
pub fn get(dir: &Path, id: &str) -> Result<Session, String> {
    let conn = open(dir)?;
    type Head = (f64, f64, String, u32, String, bool, String, String);
    let head: Option<Head> = conn
        .query_row(
            "SELECT started_at, ended_at, mode, seed, settings, comparable, timing, checkin
             FROM sessions WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )
        .optional()
        .map_err(sql_err)?;
    let (started_at, ended_at, mode, seed, settings, comparable, timing, checkin) =
        head.ok_or_else(|| format!("no n-back session with id {id:?}"))?;

    let mut trial_stmt = conn
        .prepare(
            "SELECT i, position, letter, pos_match, let_match, lure, pos_response, let_response,
                    pos_rt_ms, let_rt_ms, intended_onset, actual_onset
             FROM trials WHERE session_id = ?1 AND block_idx = ?2 ORDER BY i",
        )
        .map_err(sql_err)?;
    let mut block_stmt = conn
        .prepare(
            "SELECT idx, n, void, void_reason, metrics, next_n
             FROM blocks WHERE session_id = ?1 ORDER BY idx",
        )
        .map_err(sql_err)?;
    type BlockRow = (u32, u32, bool, Option<String>, String, u32);
    let rows: Vec<BlockRow> = block_stmt
        .query_map([id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    let mut blocks = Vec::with_capacity(rows.len());
    for (index, n, void, void_reason, metrics, next_n) in rows {
        let trials = trial_stmt
            .query_map(params![id, index], |r| {
                Ok(Trial {
                    i: r.get(0)?,
                    position: r.get(1)?,
                    letter: r.get(2)?,
                    pos_match: r.get(3)?,
                    let_match: r.get(4)?,
                    lure: r.get(5)?,
                    pos_response: r.get(6)?,
                    let_response: r.get(7)?,
                    pos_rt_ms: r.get(8)?,
                    let_rt_ms: r.get(9)?,
                    intended_onset: r.get(10)?,
                    actual_onset: r.get(11)?,
                })
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        blocks.push(Block {
            index,
            n,
            void,
            void_reason,
            trials,
            metrics: serde_json::from_str(&metrics).map_err(json_err)?,
            next_n,
        });
    }
    Ok(Session {
        id: id.to_string(),
        started_at,
        ended_at,
        mode: serde_json::from_value(serde_json::Value::String(mode)).map_err(json_err)?,
        seed,
        settings: serde_json::from_str(&settings).map_err(json_err)?,
        blocks,
        comparable,
        timing: serde_json::from_str(&timing).map_err(json_err)?,
        checkin: serde_json::from_str(&checkin).map_err(json_err)?,
    })
}

const CSV_HEADER: &str = "session_id,started_at,mode,seed,comparable,calibrated,block,n,void,\
void_reason,i,position,letter,pos_match,let_match,lure,pos_response,let_response,pos_rt_ms,\
let_rt_ms,intended_onset,actual_onset";

/// Quote a CSV field when it holds a comma, quote or line break.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Every trial of every session, oldest session first, to `<dir>/exports/nback-trials-<unix>.csv`.
pub fn export_csv(dir: &Path) -> Result<Exported, String> {
    let conn = open(dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.started_at, s.mode, s.seed, s.comparable,
                    json_extract(s.timing, '$.calibrated'), b.idx, b.n, b.void,
                    b.void_reason, t.i, t.position, t.letter, t.pos_match, t.let_match, t.lure,
                    t.pos_response, t.let_response, t.pos_rt_ms, t.let_rt_ms, t.intended_onset,
                    t.actual_onset
             FROM trials t
             JOIN blocks b ON b.session_id = t.session_id AND b.idx = t.block_idx
             JOIN sessions s ON s.id = t.session_id
             ORDER BY s.started_at, s.id, b.idx, t.i",
        )
        .map_err(sql_err)?;
    let columns = stmt.column_count();
    let mut out = String::from(CSV_HEADER);
    out.push('\n');
    let mut rows = stmt.query([]).map_err(sql_err)?;
    while let Some(r) = rows.next().map_err(sql_err)? {
        let mut fields = Vec::with_capacity(columns);
        for i in 0..columns {
            fields.push(match r.get::<_, Value>(i).map_err(sql_err)? {
                Value::Null | Value::Blob(_) => String::new(),
                Value::Integer(v) => v.to_string(),
                Value::Real(v) => v.to_string(),
                Value::Text(v) => csv_field(&v),
            });
        }
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    let exports = dir.join("exports");
    std::fs::create_dir_all(&exports).map_err(|e| format!("{}: {e}", exports.display()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path: PathBuf = exports.join(format!("nback-trials-{stamp}.csv"));
    std::fs::write(&path, out).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Exported {
        path: path.display().to_string(),
    })
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn nback_save_session(app: AppHandle, session: Session) -> Result<Saved, String> {
    save(&data_dir(&app)?, &session)
}

#[tauri::command(async)]
pub fn nback_sessions(app: AppHandle, limit: Option<u32>) -> Result<Sessions, String> {
    list(&data_dir(&app)?, limit)
}

#[tauri::command(async)]
pub fn nback_session(app: AppHandle, id: String) -> Result<Session, String> {
    get(&data_dir(&app)?, &id)
}

#[tauri::command(async)]
pub fn nback_export_csv(app: AppHandle) -> Result<Exported, String> {
    export_csv(&data_dir(&app)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metrics(d_prime: f64) -> Metrics {
        Metrics {
            hits: 5,
            misses: 1,
            false_alarms: 1,
            correct_rejections: 13,
            d_prime,
            c: 0.25,
        }
    }

    fn trial(i: u32) -> Trial {
        Trial {
            i,
            position: Some(i % 8),
            letter: Some("K".into()),
            pos_match: i.is_multiple_of(4),
            let_match: false,
            lure: i == 3,
            pos_response: i.is_multiple_of(4),
            let_response: false,
            pos_rt_ms: i.is_multiple_of(4).then_some(512.5),
            let_rt_ms: None,
            intended_onset: f64::from(i) * 3000.0,
            actual_onset: f64::from(i) * 3000.0 + 4.25,
        }
    }

    fn block(index: u32, n: u32, void: bool, next_n: u32) -> Block {
        Block {
            index,
            n,
            void,
            void_reason: void.then(|| "focus lost, window hidden".to_string()),
            trials: (0..20 + n).map(trial).collect(),
            metrics: if void {
                BlockMetrics {
                    position: None,
                    letter: None,
                }
            } else {
                BlockMetrics {
                    position: Some(metrics(2.0 + f64::from(index))),
                    letter: Some(metrics(1.0)),
                }
            },
            next_n,
        }
    }

    fn session(id: &str, started_at: f64) -> Session {
        Session {
            id: id.into(),
            started_at,
            ended_at: started_at + 900.0,
            mode: Mode::Dual,
            seed: 4_000_000_000,
            settings: Settings {
                trial_ms: 3000,
                stimulus_ms: 500,
                blocks: 15,
                up_max_errors: 2,
                down_min_errors: 6,
                lures: false,
                in_trial_feedback: false,
                keys: Keys {
                    position: "KeyA".into(),
                    letter: "KeyL".into(),
                },
            },
            blocks: vec![
                block(0, 2, false, 3),
                block(1, 3, true, 3),
                block(2, 3, false, 4),
            ],
            comparable: true,
            timing: Timing {
                output_latency: Some(0.021),
                base_latency: Some(0.01),
                calibrated: true,
            },
            checkin: Checkin {
                expectation: None,
                sleep_hours: None,
                exercise: None,
                fatigue: None,
                strategy: None,
            },
        }
    }

    #[test]
    fn saves_and_reads_back_the_same_session() {
        let dir = tempfile::tempdir().unwrap();
        let s = session("a", 1_790_000_000.5);
        assert_eq!(save(dir.path(), &s).unwrap().id, "a");
        assert_eq!(get(dir.path(), "a").unwrap(), s);
        assert!(dir.path().join(DB_FILE).exists());
    }

    #[test]
    fn saving_the_same_id_again_replaces_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session("a", 100.0);
        save(dir.path(), &s).unwrap();
        s.checkin.strategy = Some("rehearsed the last two letters".into());
        s.checkin.sleep_hours = Some(7.5);
        s.blocks.pop();
        save(dir.path(), &s).unwrap();
        save(dir.path(), &s).unwrap();

        assert_eq!(list(dir.path(), None).unwrap().sessions.len(), 1);
        assert_eq!(get(dir.path(), "a").unwrap(), s);
        let conn = open(dir.path()).unwrap();
        let trials: u32 = conn
            .query_row("SELECT COUNT(*) FROM trials", [], |r| r.get(0))
            .unwrap();
        assert_eq!(trials, 22 + 23);
    }

    #[test]
    fn lists_summaries_newest_first_with_a_limit() {
        let dir = tempfile::tempdir().unwrap();
        for (id, t) in [("old", 100.0), ("new", 300.0), ("mid", 200.0)] {
            save(dir.path(), &session(id, t)).unwrap();
        }
        let ids = |limit| -> Vec<String> {
            list(dir.path(), limit)
                .unwrap()
                .sessions
                .into_iter()
                .map(|s| s.id)
                .collect()
        };
        assert_eq!(ids(None), ["new", "mid", "old"]);
        assert_eq!(ids(Some(2)), ["new", "mid"]);
    }

    #[test]
    fn summary_skips_void_blocks_and_matches_the_contract_shape() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &session("a", 100.0)).unwrap();
        let summary = &list(dir.path(), None).unwrap().sessions[0];
        assert_eq!(
            serde_json::to_value(summary).unwrap(),
            json!({"id": "a", "started_at": 100.0, "mode": "dual", "blocks": 3, "void_blocks": 1,
                   "start_n": 2, "max_n": 3, "mean_n": 2.5,
                   "d_prime": {"position": 3.0, "letter": 1.0}, "comparable": true})
        );
    }

    #[test]
    fn a_session_with_every_block_void_has_no_d_prime() {
        let mut s = session("v", 1.0);
        s.blocks = vec![block(0, 2, true, 2)];
        let summary = summarize(&s);
        assert_eq!(
            (summary.start_n, summary.max_n, summary.mean_n),
            (2, 2, 2.0)
        );
        assert_eq!(
            summary.d_prime,
            DPrime {
                position: None,
                letter: None
            }
        );
    }

    #[test]
    fn accepts_the_contract_json_and_rejects_bad_input() {
        let body = json!({
            "id": "0b6c", "started_at": 1.5, "ended_at": 2.5, "mode": "transfer", "seed": 7,
            "settings": {"trial_ms": 3000, "stimulus_ms": 500, "blocks": 3, "up_max_errors": 2,
                         "down_min_errors": 6, "lures": false, "in_trial_feedback": false,
                         "keys": {"position": "KeyA", "letter": "KeyL"}},
            "blocks": [{"index": 0, "n": 1, "void": false, "void_reason": null, "next_n": 1,
                        "metrics": {"position": null, "letter": null},
                        "trials": [{"i": 0, "position": 3, "letter": "262Hz", "pos_match": false,
                                    "let_match": false, "lure": false, "pos_response": false,
                                    "let_response": true, "pos_rt_ms": null, "let_rt_ms": 640.5,
                                    "intended_onset": 0.0, "actual_onset": 3.2}]}],
            "comparable": true,
            "timing": {"output_latency": null, "base_latency": null, "calibrated": false},
            "checkin": {"expectation": 3.0, "sleep_hours": 7.5, "exercise": true, "fatigue": 2.0,
                        "strategy": "chunking"}
        });
        let s: Session = serde_json::from_value(body.clone()).unwrap();
        assert_eq!(serde_json::to_value(&s).unwrap(), body);

        let mut bad = body.clone();
        bad["mode"] = json!("triple");
        assert!(serde_json::from_value::<Session>(bad).is_err());

        let dir = tempfile::tempdir().unwrap();
        let mut s = s;
        s.id = "  ".into();
        assert!(save(dir.path(), &s).unwrap_err().contains("empty"));
    }

    #[test]
    fn unknown_id_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get(dir.path(), "nope")
            .unwrap_err()
            .contains("no n-back session"));
    }

    #[test]
    fn schema_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &session("a", 1.0)).unwrap();
        drop(open(dir.path()).unwrap());
        drop(open(dir.path()).unwrap());
        assert_eq!(list(dir.path(), None).unwrap().sessions.len(), 1);
    }

    #[test]
    fn exports_one_csv_row_per_trial() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &session("b", 200.0)).unwrap();
        save(dir.path(), &session("a", 100.0)).unwrap();
        let path = PathBuf::from(export_csv(dir.path()).unwrap().path);
        assert_eq!(path.parent().unwrap(), dir.path().join("exports"));
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(lines.len(), 1 + 2 * (22 + 23 + 23));
        assert_eq!(
            lines[1],
            "a,100,dual,4000000000,1,1,0,2,0,,0,0,K,1,0,0,1,0,512.5,,0,4.25"
        );
        // Void reasons with commas are quoted; every other row has the header's column count.
        assert!(text.contains(",1,\"focus lost, window hidden\","));
        let columns = CSV_HEADER.split(',').count();
        assert!(lines
            .iter()
            .filter(|l| !l.contains('"'))
            .all(|l| l.split(',').count() == columns));
    }
}
