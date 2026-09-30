//! Background jobs (`survey`, `ingest`), one at a time, persisted in SQLite so `/jobs/{id}`
//! still answers after a restart. A job still marked running when the service starts was
//! interrupted and is recorded as failed.

use crate::error::Result;
use crate::store;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobKind {
    Survey,
    Ingest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
}

fn parse<T: for<'de> Deserialize<'de>>(s: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn name<T: Serialize>(value: T) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// Work done so far out of the work known so far. `total` can grow as a job discovers work
/// (ingest adds OCR pages and chunks to embed once it knows them).
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicU64,
    total: AtomicU64,
}

impl Progress {
    pub fn add_total(&self, n: u64) {
        self.total.fetch_add(n, Ordering::Relaxed);
    }
    pub fn add_done(&self, n: u64) {
        self.done.fetch_add(n, Ordering::Relaxed);
    }
    pub fn set(&self, done: u64, total: u64) {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
    }
    pub fn get(&self) -> (u64, u64) {
        (self.done.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressView {
    pub done: u64,
    pub total: u64,
}

/// The contract's job object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobView {
    pub job_id: String,
    pub kind: JobKind,
    pub state: JobState,
    pub progress: ProgressView,
    pub started_at: Option<f64>,
    pub finished_at: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
struct Status {
    state: JobState,
    started_at: Option<f64>,
    finished_at: Option<f64>,
    error: Option<String>,
}

impl Status {
    fn new(state: JobState, started_at: Option<f64>) -> Mutex<Status> {
        Mutex::new(Status { state, started_at, finished_at: None, error: None })
    }
}

pub struct Job {
    pub id: String,
    pub kind: JobKind,
    pub progress: Progress,
    created_at: f64,
    status: Mutex<Status>,
}

impl Job {
    pub fn view(&self) -> JobView {
        let Status { state, started_at, finished_at, error } = self.status.lock().expect("job status lock").clone();
        let (done, total) = self.progress.get();
        JobView { job_id: self.id.clone(), kind: self.kind, state, progress: ProgressView { done, total }, started_at, finished_at, error }
    }

    fn set(&self, state: JobState, error: Option<String>) {
        let mut status = self.status.lock().expect("job status lock");
        status.state = state;
        match state {
            JobState::Running => status.started_at = Some(store::now()),
            JobState::Done | JobState::Failed => status.finished_at = Some(store::now()),
            JobState::Queued => {}
        }
        status.error = error;
    }
}

fn save(conn: &Connection, job: &Job) -> Result<()> {
    let v = job.view();
    conn.execute(
        "INSERT INTO jobs(job_id, kind, state, done, total, started_at, finished_at, error, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
         ON CONFLICT(job_id) DO UPDATE SET state = excluded.state, done = excluded.done, total = excluded.total, \
         started_at = excluded.started_at, finished_at = excluded.finished_at, error = excluded.error",
        params![
            v.job_id,
            name(v.kind),
            name(v.state),
            v.progress.done as i64,
            v.progress.total as i64,
            v.started_at,
            v.finished_at,
            v.error,
            job.created_at
        ],
    )?;
    Ok(())
}

pub fn load(conn: &Connection, job_id: &str) -> Result<Option<JobView>> {
    let row = conn
        .query_row(
            "SELECT job_id, kind, state, done, total, started_at, finished_at, error FROM jobs WHERE job_id = ?1",
            [job_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<f64>>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .optional()?;
    let Some((job_id, kind, state, done, total, started_at, finished_at, error)) = row else { return Ok(None) };
    Ok(Some(JobView {
        job_id,
        kind: parse(&kind).ok_or(format!("unknown job kind {kind:?}"))?,
        state: parse(&state).ok_or(format!("unknown job state {state:?}"))?,
        progress: ProgressView { done: done as u64, total: total as u64 },
        started_at,
        finished_at,
        error,
    }))
}

/// Mark jobs left queued or running by a previous process as failed.
pub fn fail_interrupted(conn: &Connection) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE jobs SET state = 'failed', finished_at = ?1, \
         error = 'interrupted: the service stopped before the job finished' WHERE state IN ('queued', 'running')",
        [store::now()],
    )?)
}

#[derive(Default)]
pub struct Jobs {
    active: Mutex<Option<Arc<Job>>>,
    counter: AtomicUsize,
}

impl Jobs {
    /// The running (or just queued) job, if any.
    pub fn active(&self) -> Option<Arc<Job>> {
        let active = self.active.lock().expect("active job lock");
        active.as_ref().filter(|j| matches!(j.view().state, JobState::Queued | JobState::Running)).cloned()
    }

    /// Start `work` on a blocking thread unless a job is already running, in which case the
    /// running job's id is returned as the error. `conn` persists the job's states.
    pub fn start<F>(&self, kind: JobKind, conn: Arc<Mutex<Connection>>, work: F) -> std::result::Result<String, String>
    where
        F: FnOnce(&Progress) -> Result<()> + Send + 'static,
    {
        let mut active = self.active.lock().expect("active job lock");
        if let Some(job) = active.as_ref().filter(|j| matches!(j.view().state, JobState::Queued | JobState::Running)) {
            return Err(job.id.clone());
        }
        let created_at = store::now();
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let job = Arc::new(Job {
            id: format!("{}-{}-{n}", name(kind), (created_at * 1000.0) as u64),
            kind,
            progress: Progress::default(),
            created_at,
            status: Status::new(JobState::Queued, None),
        });
        let persist = {
            let conn = conn.clone();
            move |job: &Job| {
                let conn = conn.lock().unwrap_or_else(|p| p.into_inner());
                if let Err(e) = save(&conn, job) {
                    eprintln!("jobs: cannot save {}: {e}", job.id);
                }
            }
        };
        *active = Some(job.clone());
        // Release the lock before touching the database: readers lock the connection first.
        drop(active);
        persist(&job);
        let id = job.id.clone();
        tokio::task::spawn_blocking(move || {
            job.set(JobState::Running, None);
            persist(&job);
            eprintln!("jobs: {} started", job.id);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&job.progress)))
                .unwrap_or_else(|_| Err("the job panicked".into()));
            match result {
                Ok(()) => {
                    job.set(JobState::Done, None);
                    eprintln!("jobs: {} done", job.id);
                }
                Err(e) => {
                    eprintln!("jobs: {} failed: {e}", job.id);
                    job.set(JobState::Failed, Some(e.0));
                }
            }
            persist(&job);
        });
        Ok(id)
    }

    /// A job by id: the active one from memory, others from the database. The job lock is
    /// released before the connection is locked, never held while waiting for it.
    pub fn get(&self, conn: &Mutex<Connection>, job_id: &str) -> Result<Option<JobView>> {
        let active = self.active.lock().expect("active job lock").as_ref().filter(|j| j.id == job_id).cloned();
        if let Some(job) = active {
            return Ok(Some(job.view()));
        }
        load(&conn.lock().unwrap_or_else(|p| p.into_inner()), job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::temp_dir;
    use std::time::Duration;

    async fn wait_for(jobs: &Jobs, conn: &Arc<Mutex<Connection>>, id: &str, state: JobState) -> JobView {
        for _ in 0..500 {
            let view = jobs.get(conn, id).unwrap().unwrap();
            if view.state == state {
                return view;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job {id} never reached {state:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lifecycle_conflict_failure_and_persistence() {
        let dir = temp_dir("jobs");
        let conn = Arc::new(Mutex::new(store::open(&dir).unwrap()));
        let jobs = Jobs::default();
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let id = jobs
            .start(JobKind::Ingest, conn.clone(), move |p| {
                p.add_total(4);
                p.add_done(1);
                gate.recv().unwrap();
                p.add_done(3);
                Ok(())
            })
            .unwrap();
        let running = wait_for(&jobs, &conn, &id, JobState::Running).await;
        assert_eq!(running.kind, JobKind::Ingest);
        assert!(running.started_at.is_some() && running.finished_at.is_none());
        assert_eq!(jobs.active().unwrap().id, id);

        // A second job is refused while the first runs.
        assert_eq!(jobs.start(JobKind::Survey, conn.clone(), |_| Ok(())), Err(id.clone()));

        release.send(()).unwrap();
        let done = wait_for(&jobs, &conn, &id, JobState::Done).await;
        assert_eq!(done.progress, ProgressView { done: 4, total: 4 });
        assert!(done.finished_at.is_some() && done.error.is_none());
        for _ in 0..100 {
            if jobs.active().is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(jobs.active().is_none());

        let failing = jobs.start(JobKind::Survey, conn.clone(), |_| Err("no such folder".into())).unwrap();
        let failed = wait_for(&jobs, &conn, &failing, JobState::Failed).await;
        assert_eq!(failed.error.as_deref(), Some("no such folder"));
        assert_eq!(failed.kind, JobKind::Survey);

        // Finished jobs are readable from the database alone.
        assert_eq!(load(&conn.lock().unwrap(), &id).unwrap().unwrap(), done);
        assert_eq!(jobs.get(&conn, "nope").unwrap(), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reading_jobs_while_the_database_is_busy_does_not_deadlock() {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let dir = temp_dir("jobs-lock-order");
        let conn = Arc::new(Mutex::new(store::open(&dir).unwrap()));
        let jobs = Arc::new(Jobs::default());
        // A request (such as /status) holds the connection while a job is being started.
        let busy = conn.lock().unwrap();
        let (j, c, handle) = (jobs.clone(), conn.clone(), rt.handle().clone());
        let starter = std::thread::spawn(move || {
            let _runtime = handle.enter();
            j.start(JobKind::Survey, c, |_| Ok(()))
        });
        std::thread::sleep(Duration::from_millis(200));
        // Reading the active job must not wait for the connection.
        let (tx, rx) = std::sync::mpsc::channel();
        let j = jobs.clone();
        std::thread::spawn(move || tx.send(j.active().map(|job| job.id.clone())).unwrap());
        let seen = rx.recv_timeout(Duration::from_secs(5)).expect("jobs.active() blocked behind the database");
        drop(busy);
        let id = starter.join().unwrap().unwrap();
        assert_eq!(seen, Some(id));
        drop(rt);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn interrupted_jobs_become_failed() {
        let dir = temp_dir("jobs-interrupted");
        let conn = store::open(&dir).unwrap();
        let job = Job {
            id: "ingest-1-0".into(),
            kind: JobKind::Ingest,
            progress: Progress::default(),
            created_at: 1.0,
            status: Status::new(JobState::Running, Some(1.0)),
        };
        save(&conn, &job).unwrap();
        assert_eq!(fail_interrupted(&conn).unwrap(), 1);
        let view = load(&conn, "ingest-1-0").unwrap().unwrap();
        assert_eq!(view.state, JobState::Failed);
        assert!(view.error.unwrap().contains("interrupted"));
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
