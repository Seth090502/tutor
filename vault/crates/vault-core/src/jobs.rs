//! The SQLite job queue. A job is identified by (stage, key) and carries the fingerprint of its inputs; a new
//! fingerprint makes it stale and queues it again, and a worker's result only counts if the fingerprint it
//! worked on is still current. Running jobs return to the queue when the writer starts (crash recovery).

use crate::error::{CoreError, Result};
use crate::time::now_rfc3339;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
    /// Parked until something outside the queue changes (for example `waiting_gpu`).
    Waiting,
}

impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Done => "done",
            JobState::Failed => "failed",
            JobState::Waiting => "waiting",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "queued" => JobState::Queued,
            "running" => JobState::Running,
            "done" => JobState::Done,
            "failed" => JobState::Failed,
            "waiting" => JobState::Waiting,
            other => return Err(CoreError::InvalidArgument(format!("unknown job state {other:?}"))),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub job_id: i64,
    pub stage: String,
    pub key: String,
    pub state: JobState,
    pub attempts: u32,
    pub fingerprint: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enqueued {
    New,
    /// Already known with this fingerprint; left as it was (done stays done, failed stays failed).
    Unchanged,
    /// Known with another fingerprint: its inputs changed, so it was queued again.
    Requeued,
}

const JOB_COLUMNS: &str = "job_id, stage, key, state, attempts, fingerprint, last_error";

/// A `jobs` row as read, before its state is parsed.
type JobRow = (i64, String, String, String, u32, String, Option<String>);

fn job_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<JobRow> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
}

fn to_job(t: JobRow) -> Result<Job> {
    Ok(Job {
        job_id: t.0,
        stage: t.1,
        key: t.2,
        state: JobState::parse(&t.3)?,
        attempts: t.4,
        fingerprint: t.5,
        last_error: t.6,
    })
}

pub fn enqueue(conn: &mut Connection, stage: &str, key: &str, fingerprint: &str) -> Result<Enqueued> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let existing: Option<String> = tx
        .query_row("SELECT fingerprint FROM jobs WHERE stage = ?1 AND key = ?2", params![stage, key], |r| r.get(0))
        .optional()?;
    let now = now_rfc3339();
    let outcome = match existing {
        None => {
            tx.execute(
                "INSERT INTO jobs(stage, key, state, attempts, fingerprint, updated_at) VALUES (?1, ?2, 'queued', 0, ?3, ?4)",
                params![stage, key, fingerprint, now],
            )?;
            Enqueued::New
        }
        Some(fp) if fp == fingerprint => Enqueued::Unchanged,
        Some(_) => {
            tx.execute(
                "UPDATE jobs SET state = 'queued', attempts = 0, fingerprint = ?3, last_error = NULL, updated_at = ?4 \
                 WHERE stage = ?1 AND key = ?2",
                params![stage, key, fingerprint, now],
            )?;
            Enqueued::Requeued
        }
    };
    tx.commit()?;
    Ok(outcome)
}

/// Marks up to `limit` queued jobs (of one stage, or any) as running, oldest first, and returns them.
pub fn claim(conn: &mut Connection, stage: Option<&str>, limit: u32) -> Result<Vec<Job>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let rows = {
        let mut stmt = tx.prepare(&format!(
            "SELECT {JOB_COLUMNS} FROM jobs WHERE state = 'queued' AND (?1 IS NULL OR stage = ?1) ORDER BY job_id LIMIT ?2"
        ))?;
        stmt.query_map(params![stage, limit], job_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let now = now_rfc3339();
    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let mut job = to_job(row)?;
        tx.execute(
            "UPDATE jobs SET state = 'running', attempts = attempts + 1, updated_at = ?2 WHERE job_id = ?1",
            params![job.job_id, now],
        )?;
        job.state = JobState::Running;
        job.attempts += 1;
        jobs.push(job);
    }
    tx.commit()?;
    Ok(jobs)
}

/// Marks a running job done if `fingerprint` is still its fingerprint. False means the inputs changed while it
/// ran; the job is already queued again.
pub fn complete(conn: &Connection, job_id: i64, fingerprint: &str) -> Result<bool> {
    let n = conn.execute(
        "UPDATE jobs SET state = 'done', last_error = NULL, updated_at = ?3 \
         WHERE job_id = ?1 AND state = 'running' AND fingerprint = ?2",
        params![job_id, fingerprint, now_rfc3339()],
    )?;
    Ok(n == 1)
}

/// Records a failure: back to the queue while attempts < `max_attempts`, otherwise failed. Returns the state
/// the job is in afterwards.
pub fn fail(conn: &Connection, job_id: i64, error: &str, max_attempts: u32) -> Result<JobState> {
    conn.execute(
        "UPDATE jobs SET state = CASE WHEN attempts < ?3 THEN 'queued' ELSE 'failed' END, last_error = ?2, updated_at = ?4 \
         WHERE job_id = ?1 AND state = 'running'",
        params![job_id, error, max_attempts, now_rfc3339()],
    )?;
    state_of(conn, job_id)
}

/// Parks a running or queued job with a reason (for example `waiting_gpu`).
pub fn wait(conn: &Connection, job_id: i64, reason: &str) -> Result<()> {
    conn.execute(
        "UPDATE jobs SET state = 'waiting', last_error = ?2, updated_at = ?3 WHERE job_id = ?1 AND state IN ('running','queued')",
        params![job_id, reason, now_rfc3339()],
    )?;
    Ok(())
}

/// Returns every waiting job of `stage` to the queue.
pub fn release_waiting(conn: &Connection, stage: &str) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE jobs SET state = 'queued', updated_at = ?2 WHERE stage = ?1 AND state = 'waiting'",
        params![stage, now_rfc3339()],
    )?)
}

/// Gives a failed job a fresh set of attempts.
pub fn retry(conn: &Connection, stage: &str, key: &str) -> Result<bool> {
    let n = conn.execute(
        "UPDATE jobs SET state = 'queued', attempts = 0, updated_at = ?3 WHERE stage = ?1 AND key = ?2 AND state = 'failed'",
        params![stage, key, now_rfc3339()],
    )?;
    Ok(n == 1)
}

/// Crash recovery: every running job goes back to the queue. Called when the writer opens.
pub fn recover(conn: &Connection) -> Result<usize> {
    let n =
        conn.execute("UPDATE jobs SET state = 'queued', updated_at = ?1 WHERE state = 'running'", [now_rfc3339()])?;
    if n > 0 {
        tracing::warn!(jobs = n, "running jobs from an interrupted run returned to the queue");
    }
    Ok(n)
}

/// True when (stage, key) is done with exactly this fingerprint, so the work need not run again.
pub fn is_current(conn: &Connection, stage: &str, key: &str, fingerprint: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM jobs WHERE stage = ?1 AND key = ?2 AND fingerprint = ?3 AND state = 'done'",
            params![stage, key, fingerprint],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn get(conn: &Connection, stage: &str, key: &str) -> Result<Option<Job>> {
    conn.query_row(
        &format!("SELECT {JOB_COLUMNS} FROM jobs WHERE stage = ?1 AND key = ?2"),
        params![stage, key],
        job_from_row,
    )
    .optional()?
    .map(to_job)
    .transpose()
}

fn state_of(conn: &Connection, job_id: i64) -> Result<JobState> {
    let s: String = conn.query_row("SELECT state FROM jobs WHERE job_id = ?1", [job_id], |r| r.get(0))?;
    JobState::parse(&s)
}

/// (stage, state, count) for every non-empty combination.
pub fn counts(conn: &Connection) -> Result<Vec<(String, JobState, u64)>> {
    let mut stmt =
        conn.prepare("SELECT stage, state, count(*) FROM jobs GROUP BY stage, state ORDER BY stage, state")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(stage, state, n)| Ok((stage, JobState::parse(&state)?, u64::try_from(n).unwrap_or(0))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_writer;

    fn conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let c = open_writer(&dir.path().join("q.sqlite")).unwrap();
        (dir, c)
    }

    #[test]
    fn enqueue_is_idempotent_and_fingerprints_make_jobs_stale() {
        let (_d, mut c) = conn();
        assert_eq!(enqueue(&mut c, "extract", "doc1", "fp1").unwrap(), Enqueued::New);
        assert_eq!(enqueue(&mut c, "extract", "doc1", "fp1").unwrap(), Enqueued::Unchanged);
        let job = claim(&mut c, Some("extract"), 10).unwrap().pop().unwrap();
        assert!(complete(&c, job.job_id, "fp1").unwrap());
        assert!(is_current(&c, "extract", "doc1", "fp1").unwrap());
        assert_eq!(enqueue(&mut c, "extract", "doc1", "fp1").unwrap(), Enqueued::Unchanged);
        assert_eq!(get(&c, "extract", "doc1").unwrap().unwrap().state, JobState::Done);
        assert_eq!(enqueue(&mut c, "extract", "doc1", "fp2").unwrap(), Enqueued::Requeued);
        assert!(!is_current(&c, "extract", "doc1", "fp1").unwrap());
        let j = get(&c, "extract", "doc1").unwrap().unwrap();
        assert_eq!((j.state, j.attempts, j.fingerprint.as_str()), (JobState::Queued, 0, "fp2"));
    }

    #[test]
    fn a_result_for_a_stale_fingerprint_does_not_count() {
        let (_d, mut c) = conn();
        enqueue(&mut c, "s", "k", "old").unwrap();
        let job = claim(&mut c, None, 1).unwrap().pop().unwrap();
        enqueue(&mut c, "s", "k", "new").unwrap();
        assert!(!complete(&c, job.job_id, "old").unwrap());
        assert_eq!(get(&c, "s", "k").unwrap().unwrap().state, JobState::Queued);
    }

    #[test]
    fn claim_orders_filters_and_counts_attempts() {
        let (_d, mut c) = conn();
        for k in ["a", "b", "c"] {
            enqueue(&mut c, "one", k, "f").unwrap();
        }
        enqueue(&mut c, "two", "x", "f").unwrap();
        let got = claim(&mut c, Some("one"), 2).unwrap();
        assert_eq!(got.iter().map(|j| j.key.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
        assert!(got.iter().all(|j| j.state == JobState::Running && j.attempts == 1));
        assert_eq!(claim(&mut c, None, 10).unwrap().len(), 2, "c and x remain");
        assert!(claim(&mut c, None, 10).unwrap().is_empty());
    }

    #[test]
    fn failures_retry_until_the_limit() {
        let (_d, mut c) = conn();
        enqueue(&mut c, "s", "k", "f").unwrap();
        let j = claim(&mut c, None, 1).unwrap().pop().unwrap();
        assert_eq!(fail(&c, j.job_id, "boom 1", 2).unwrap(), JobState::Queued);
        let j = claim(&mut c, None, 1).unwrap().pop().unwrap();
        assert_eq!(j.attempts, 2);
        assert_eq!(fail(&c, j.job_id, "boom 2", 2).unwrap(), JobState::Failed);
        assert_eq!(get(&c, "s", "k").unwrap().unwrap().last_error.as_deref(), Some("boom 2"));
        assert!(claim(&mut c, None, 1).unwrap().is_empty());
        assert!(retry(&c, "s", "k").unwrap());
        assert_eq!(claim(&mut c, None, 1).unwrap()[0].attempts, 1);
    }

    #[test]
    fn waiting_jobs_park_and_release() {
        let (_d, mut c) = conn();
        enqueue(&mut c, "gpu", "scan.pdf", "f").unwrap();
        let j = claim(&mut c, None, 1).unwrap().pop().unwrap();
        wait(&c, j.job_id, "waiting_gpu").unwrap();
        assert!(claim(&mut c, None, 1).unwrap().is_empty());
        assert_eq!(release_waiting(&c, "gpu").unwrap(), 1);
        assert_eq!(claim(&mut c, None, 1).unwrap().len(), 1);
    }

    #[test]
    fn running_jobs_return_to_the_queue_on_recovery() {
        let (_d, mut c) = conn();
        enqueue(&mut c, "s", "a", "f").unwrap();
        enqueue(&mut c, "s", "b", "f").unwrap();
        claim(&mut c, None, 2).unwrap();
        assert_eq!(recover(&c).unwrap(), 2);
        assert_eq!(counts(&c).unwrap(), vec![("s".to_string(), JobState::Queued, 2)]);
        assert_eq!(claim(&mut c, None, 5).unwrap().len(), 2);
    }
}
