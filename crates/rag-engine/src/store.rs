//! SQLite state under `--data`: files, documents, chunks, the OCR queue, jobs and spend.
//!
//! There is no per-page table: per-document page counts by triage class answer `/status`, and
//! only pages that need OCR get a row (in `ocr_queue`). At 10M chunks a row per page would be
//! tens of millions of rows nothing reads.

use crate::error::Result;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DB_FILE: &str = "rag.sqlite3";

const SCHEMA_VERSION: i64 = 1;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS files (
    path     TEXT PRIMARY KEY,          -- 'corpus/...' or 'notes/...'
    bytes    INTEGER NOT NULL,
    mtime_ns INTEGER NOT NULL,
    doc_id   TEXT NOT NULL              -- BLAKE3 of the content
);
CREATE INDEX IF NOT EXISTS files_doc ON files(doc_id);

CREATE TABLE IF NOT EXISTS documents (
    doc_id       TEXT PRIMARY KEY,
    path         TEXT NOT NULL,         -- canonical copy: the first path in sorted order
    kind         TEXT NOT NULL,         -- pdf | epub | md | txt
    title        TEXT NOT NULL,
    domain       TEXT NOT NULL,
    source_type  TEXT NOT NULL,
    bytes        INTEGER NOT NULL,
    state        TEXT NOT NULL,         -- indexing | indexed | error
    error        TEXT,
    pages        INTEGER NOT NULL DEFAULT 0,
    born_digital INTEGER NOT NULL DEFAULT 0,
    scanned      INTEGER NOT NULL DEFAULT 0,
    math_heavy   INTEGER NOT NULL DEFAULT 0,
    empty        INTEGER NOT NULL DEFAULT 0,
    chunks       INTEGER NOT NULL DEFAULT 0,
    ocr_pages    INTEGER NOT NULL DEFAULT 0,  -- OCR'd pages whose text was used
    indexed_at   REAL
);
-- Library browsing (GET /documents) sorts indexed documents by these.
CREATE INDEX IF NOT EXISTS documents_title ON documents(state, title COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS documents_added ON documents(state, indexed_at);
CREATE INDEX IF NOT EXISTS documents_pages ON documents(state, pages);

CREATE TABLE IF NOT EXISTS chunks (
    chunk_id     TEXT PRIMARY KEY,      -- '<doc_id>:<ordinal>'
    doc_id       TEXT NOT NULL,
    ordinal      INTEGER NOT NULL,
    page_start   INTEGER,
    page_end     INTEGER,
    heading_path TEXT NOT NULL,         -- JSON array of strings
    text         TEXT NOT NULL,
    tokens       INTEGER NOT NULL,
    embedded     INTEGER NOT NULL DEFAULT 0  -- 0 pending, 1 in the vector store, 2 being written
);
-- A document's chunks in reading order (outline, guided reading); also serves lookups by doc_id,
-- so it replaces the earlier chunks_doc(doc_id) index.
DROP INDEX IF EXISTS chunks_doc;
CREATE INDEX IF NOT EXISTS chunks_doc_ordinal ON chunks(doc_id, ordinal);
CREATE INDEX IF NOT EXISTS chunks_unembedded ON chunks(embedded) WHERE embedded <> 1;

CREATE TABLE IF NOT EXISTS ocr_queue (
    doc_id     TEXT NOT NULL,
    page       INTEGER NOT NULL,        -- 1-based
    class      TEXT NOT NULL,           -- scanned | math_heavy
    state      TEXT NOT NULL DEFAULT 'pending',  -- pending | done | failed
    text       TEXT,
    error      TEXT,
    provider   TEXT,
    updated_at REAL,
    PRIMARY KEY (doc_id, page)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS ocr_state ON ocr_queue(state);

CREATE TABLE IF NOT EXISTS jobs (
    job_id      TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    state       TEXT NOT NULL,
    done        INTEGER NOT NULL,
    total       INTEGER NOT NULL,
    started_at  REAL,
    finished_at REAL,
    error       TEXT,
    created_at  REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS spend (
    id       INTEGER PRIMARY KEY,
    kind     TEXT NOT NULL,             -- embedding | ocr
    provider TEXT NOT NULL,
    model    TEXT NOT NULL,
    tokens   INTEGER NOT NULL,
    usd      REAL NOT NULL,
    at       REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// Open (creating if needed) the database in `data_dir`.
pub fn open(data_dir: &Path) -> Result<Connection> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("cannot create data directory {}: {e}", data_dir.display()))?;
    let path = data_dir.join(DB_FILE);
    let conn = Connection::open(&path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(30))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", -65_536)?; // 64 MiB page cache
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(format!("{} has schema version {version}; this build knows {SCHEMA_VERSION}", path.display()).into());
    }
    conn.execute_batch(SCHEMA)?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(conn)
}

/// A read-only connection to the database in `data_dir`, which must exist (for ingest's
/// extraction thread, next to the connection that writes).
pub fn open_read(data_dir: &Path) -> Result<Connection> {
    let path = data_dir.join(DB_FILE);
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(30))?;
    Ok(conn)
}

/// Seconds since the Unix epoch.
pub fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

pub fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}

pub fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute("INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [key, value])?;
    Ok(())
}

pub fn meta_delete(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM meta WHERE key = ?1", [key])?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendKind {
    Embedding,
    Ocr,
}

impl SpendKind {
    fn as_str(self) -> &'static str {
        match self {
            SpendKind::Embedding => "embedding",
            SpendKind::Ocr => "ocr",
        }
    }
}

pub fn record_spend(conn: &Connection, kind: SpendKind, provider: &str, model: &str, tokens: u64, usd: f64) -> Result<()> {
    conn.execute(
        "INSERT INTO spend(kind, provider, model, tokens, usd, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![kind.as_str(), provider, model, tokens as i64, usd, now()],
    )?;
    Ok(())
}

/// Total USD spent on `kind`.
pub fn spend_usd(conn: &Connection, kind: SpendKind) -> Result<f64> {
    Ok(conn.query_row("SELECT COALESCE(SUM(usd), 0.0) FROM spend WHERE kind = ?1", [kind.as_str()], |r| r.get(0))?)
}

#[cfg(test)]
pub fn temp_dir(name: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rag-engine-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_the_schema_and_is_idempotent() {
        let dir = temp_dir("store-open");
        drop(open(&dir).unwrap());
        let conn = open(&dir).unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(tables, ["chunks", "documents", "files", "jobs", "meta", "ocr_queue", "spend"]);
        let mode: String = conn.pragma_query_value(None, "journal_mode", |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn meta_round_trips() {
        let dir = temp_dir("store-meta");
        let conn = open(&dir).unwrap();
        assert_eq!(meta_get(&conn, "k").unwrap(), None);
        meta_set(&conn, "k", "1").unwrap();
        meta_set(&conn, "k", "2").unwrap();
        assert_eq!(meta_get(&conn, "k").unwrap().as_deref(), Some("2"));
        meta_delete(&conn, "k").unwrap();
        assert_eq!(meta_get(&conn, "k").unwrap(), None);
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn spend_sums_per_kind() {
        let dir = temp_dir("store-spend");
        let conn = open(&dir).unwrap();
        assert_eq!(spend_usd(&conn, SpendKind::Embedding).unwrap(), 0.0);
        record_spend(&conn, SpendKind::Embedding, "voyage", "m", 1000, 0.25).unwrap();
        record_spend(&conn, SpendKind::Embedding, "voyage", "m", 1000, 0.5).unwrap();
        record_spend(&conn, SpendKind::Ocr, "x", "y", 0, 1.0).unwrap();
        assert_eq!(spend_usd(&conn, SpendKind::Embedding).unwrap(), 0.75);
        assert_eq!(spend_usd(&conn, SpendKind::Ocr).unwrap(), 1.0);
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
