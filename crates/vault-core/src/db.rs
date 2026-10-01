//! SQLite (SPEC section 6): schema, pragmas, forward-only migrations in `PRAGMA user_version`, and the typed
//! queries the writer and the tools share.

use crate::error::{CoreError, Result};
use crate::paths::is_network_path;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::path::Path;
use std::time::Duration;
use vault_api::DocKind;

pub const SCHEMA_VERSION: i64 = 1;

/// Forward-only migrations; entry `i` takes `user_version` from `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;

CREATE TABLE batches (
  batch_id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('batch','oaforge','loose','code')),
  inbox_name TEXT NOT NULL,
  detected_at TEXT NOT NULL,
  ready_at TEXT,
  completed_at TEXT,
  state TEXT NOT NULL,
  files_expected INTEGER,
  report_path TEXT
) STRICT;

CREATE TABLE deliveries (
  delivery_id INTEGER PRIMARY KEY,
  batch_id TEXT NOT NULL REFERENCES batches(batch_id),
  rel_path TEXT NOT NULL,
  bytes INTEGER,
  blake3 TEXT,
  sha256 TEXT,
  received_at TEXT NOT NULL,
  manifest_json TEXT,
  hint_category TEXT,
  outcome TEXT NOT NULL DEFAULT 'pending' CHECK (outcome IN ('pending','published','duplicate','near_duplicate',
    'quarantined','excluded','left_in_place','listed_missing','waiting_gpu')),
  reason TEXT,
  rule TEXT,
  detail TEXT,
  doc_id TEXT,
  UNIQUE (batch_id, rel_path)
) STRICT;
CREATE INDEX deliveries_blake3 ON deliveries(blake3);
CREATE INDEX deliveries_doc ON deliveries(doc_id);

CREATE TABLE originals (
  blake3 TEXT PRIMARY KEY,
  ext TEXT NOT NULL,
  bytes INTEGER NOT NULL,
  format TEXT,
  stored_rel_path TEXT NOT NULL,
  first_batch TEXT,
  stored_at TEXT NOT NULL
) STRICT;

CREATE TABLE documents (
  id INTEGER PRIMARY KEY,
  doc_id TEXT NOT NULL UNIQUE,
  hash16 TEXT NOT NULL UNIQUE,
  text_blake3 TEXT NOT NULL,
  slug TEXT NOT NULL,
  rel_path TEXT NOT NULL,
  kind TEXT NOT NULL,
  title TEXT,
  authors_json TEXT NOT NULL DEFAULT '[]',
  year INTEGER,
  language TEXT,
  lines INTEGER NOT NULL,
  bytes INTEGER NOT NULL,
  extraction_path TEXT,
  extract_fp TEXT,
  quality REAL,
  identity_confidence REAL,
  primary_category TEXT,
  level TEXT,
  license TEXT,
  created_at TEXT NOT NULL,
  superseded_by TEXT REFERENCES documents(doc_id)
) STRICT;
CREATE INDEX documents_category ON documents(primary_category, kind);
CREATE INDEX documents_kind ON documents(kind);

CREATE TABLE doc_originals (
  doc_id TEXT NOT NULL REFERENCES documents(doc_id),
  blake3 TEXT NOT NULL REFERENCES originals(blake3),
  PRIMARY KEY (doc_id, blake3)
) STRICT, WITHOUT ROWID;
CREATE INDEX doc_originals_blake3 ON doc_originals(blake3);

CREATE TABLE doc_identifiers (
  doc_id TEXT NOT NULL REFERENCES documents(doc_id),
  scheme TEXT NOT NULL,
  value TEXT NOT NULL,
  source TEXT,
  PRIMARY KEY (doc_id, scheme, value)
) STRICT, WITHOUT ROWID;
CREATE INDEX doc_identifiers_value ON doc_identifiers(scheme, value);

CREATE TABLE doc_categories (
  doc_id TEXT NOT NULL REFERENCES documents(doc_id),
  category TEXT NOT NULL,
  role TEXT NOT NULL,
  source TEXT,
  confidence REAL,
  PRIMARY KEY (doc_id, category, role)
) STRICT, WITHOUT ROWID;
CREATE INDEX doc_categories_category ON doc_categories(category, role);

CREATE TABLE doc_areas (
  doc_id TEXT NOT NULL REFERENCES documents(doc_id),
  area TEXT NOT NULL,
  PRIMARY KEY (doc_id, area)
) STRICT, WITHOUT ROWID;
CREATE INDEX doc_areas_area ON doc_areas(area);

CREATE TABLE lineage (
  old_doc TEXT NOT NULL REFERENCES documents(doc_id),
  new_doc TEXT NOT NULL REFERENCES documents(doc_id),
  reason TEXT NOT NULL,
  at TEXT NOT NULL,
  PRIMARY KEY (old_doc, new_doc)
) STRICT, WITHOUT ROWID;

CREATE TABLE near_dups (
  group_id TEXT NOT NULL,
  doc_id TEXT NOT NULL REFERENCES documents(doc_id),
  is_primary INTEGER NOT NULL,
  similarity REAL,
  PRIMARY KEY (group_id, doc_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE repos (
  repo_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT,
  "commit" TEXT,
  license TEXT,
  snapshot_at TEXT,
  doc_id TEXT,
  batch_id TEXT
) STRICT;

CREATE TABLE repo_files (
  repo_id TEXT NOT NULL REFERENCES repos(repo_id),
  path TEXT NOT NULL,
  doc_id TEXT NOT NULL REFERENCES documents(doc_id),
  language TEXT,
  swhid TEXT,
  PRIMARY KEY (repo_id, path)
) STRICT, WITHOUT ROWID;
CREATE INDEX repo_files_doc ON repo_files(doc_id);

CREATE TABLE quarantine (
  id INTEGER PRIMARY KEY,
  delivery_id INTEGER REFERENCES deliveries(delivery_id),
  blake3 TEXT NOT NULL,
  reason TEXT NOT NULL,
  detail TEXT,
  stored_rel_path TEXT,
  at TEXT NOT NULL
) STRICT;
CREATE INDEX quarantine_blake3 ON quarantine(blake3);

CREATE TABLE jobs (
  job_id INTEGER PRIMARY KEY,
  stage TEXT NOT NULL,
  key TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('queued','running','done','failed','waiting')),
  attempts INTEGER NOT NULL DEFAULT 0,
  fingerprint TEXT NOT NULL,
  last_error TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE (stage, key)
) STRICT;
CREATE INDEX jobs_state ON jobs(state, stage, job_id);

CREATE VIRTUAL TABLE doc_fts USING fts5(
  doc_id UNINDEXED, title, authors, abstract, headings, keywords,
  tokenize = 'unicode61 remove_diacritics 2'
);
"#];

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    Ok(())
}

fn refuse_network(path: &Path) -> Result<()> {
    if is_network_path(path) {
        return Err(CoreError::Config(format!("{} is on a network share; SQLite must stay local", path.display())));
    }
    Ok(())
}

/// Opens (creating if needed) the writer connection and brings the schema up to date.
pub fn open_writer(path: &Path) -> Result<Connection> {
    refuse_network(path)?;
    let mut conn =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(CoreError::Config(format!("{} could not switch to WAL (got {mode})", path.display())));
    }
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Opens an existing database for reading only; the schema must be current.
pub fn open_reader(path: &Path) -> Result<Connection> {
    refuse_network(path)?;
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| {
        CoreError::Config(format!(
            "{} cannot be opened ({e}); run intake once to create the vault database",
            path.display()
        ))
    })?;
    configure(&conn)?;
    conn.pragma_update(None, "query_only", "ON")?;
    let found = user_version(&conn)?;
    if found != SCHEMA_VERSION {
        let (relation, hint) = if found < SCHEMA_VERSION {
            ("older than", "run intake (the writer) once to migrate it")
        } else {
            ("newer than", "upgrade this program")
        };
        return Err(CoreError::Schema { found, expected: SCHEMA_VERSION, relation, hint });
    }
    Ok(conn)
}

fn user_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

pub(crate) fn migrate(conn: &mut Connection) -> Result<()> {
    let found = user_version(conn)?;
    if found > SCHEMA_VERSION {
        return Err(CoreError::Schema {
            found,
            expected: SCHEMA_VERSION,
            relation: "newer than",
            hint: "upgrade this program",
        });
    }
    let start = usize::try_from(found).unwrap_or(0);
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(start) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i64::try_from(i + 1).unwrap_or(i64::MAX))?;
        tx.commit()?;
        tracing::info!(version = i + 1, "database migrated");
    }
    Ok(())
}

// --------------------------------------------------------------------------------------------------- kinds

pub fn kind_str(k: DocKind) -> &'static str {
    match k {
        DocKind::Paper => "paper",
        DocKind::Book => "book",
        DocKind::Chapter => "chapter",
        DocKind::Thesis => "thesis",
        DocKind::Report => "report",
        DocKind::Notes => "notes",
        DocKind::Code => "code",
        DocKind::Repository => "repository",
        DocKind::Other => "other",
    }
}

pub fn parse_kind(s: &str) -> Option<DocKind> {
    Some(match s {
        "paper" => DocKind::Paper,
        "book" => DocKind::Book,
        "chapter" => DocKind::Chapter,
        "thesis" => DocKind::Thesis,
        "report" => DocKind::Report,
        "notes" => DocKind::Notes,
        "code" => DocKind::Code,
        "repository" => DocKind::Repository,
        "other" => DocKind::Other,
        _ => return None,
    })
}

pub const ALL_KINDS: [DocKind; 9] = [
    DocKind::Paper,
    DocKind::Book,
    DocKind::Chapter,
    DocKind::Thesis,
    DocKind::Report,
    DocKind::Notes,
    DocKind::Code,
    DocKind::Repository,
    DocKind::Other,
];

// ------------------------------------------------------------------------------------------------ documents

/// The `documents` columns the tools need.
#[derive(Debug, Clone, PartialEq)]
pub struct DocRow {
    pub doc_id: String,
    pub hash16: String,
    pub text_blake3: String,
    pub rel_path: String,
    pub kind: DocKind,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub language: Option<String>,
    pub lines: u32,
    pub bytes: u64,
    pub primary_category: Option<String>,
    pub superseded_by: Option<String>,
}

pub(crate) const DOC_COLUMNS: &str = "d.doc_id, d.hash16, d.text_blake3, d.rel_path, d.kind, d.title, d.authors_json, \
     d.year, d.language, d.lines, d.bytes, d.primary_category, d.superseded_by";

pub(crate) fn doc_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DocRow> {
    let kind: String = r.get(4)?;
    let authors_json: String = r.get(6)?;
    Ok(DocRow {
        doc_id: r.get(0)?,
        hash16: r.get(1)?,
        text_blake3: r.get(2)?,
        rel_path: r.get(3)?,
        kind: parse_kind(&kind).unwrap_or(DocKind::Other),
        title: r.get(5)?,
        authors: serde_json::from_str(&authors_json).unwrap_or_default(),
        year: r.get(7)?,
        language: r.get(8)?,
        lines: r.get(9)?,
        bytes: r.get::<_, i64>(10)?.try_into().unwrap_or(0),
        primary_category: r.get(11)?,
        superseded_by: r.get(12)?,
    })
}

pub fn doc_by_hash(conn: &Connection, hash16: &str) -> Result<Option<DocRow>> {
    let sql = format!("SELECT {DOC_COLUMNS} FROM documents d WHERE d.hash16 = ?1");
    Ok(conn.query_row(&sql, [hash16], doc_from_row).optional()?)
}

/// Repository name and path of a code document, when it belongs to a repository.
pub fn repo_of(conn: &Connection, doc_id: &str) -> Result<Option<(String, String)>> {
    Ok(conn
        .query_row(
            "SELECT r.name, f.path FROM repo_files f JOIN repos r ON r.repo_id = f.repo_id WHERE f.doc_id = ?1 \
             ORDER BY r.snapshot_at DESC, f.path LIMIT 1",
            [doc_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Row counts of every table, for idempotence checks.
pub fn table_counts(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut out = Vec::new();
    for t in [
        "meta",
        "batches",
        "deliveries",
        "originals",
        "documents",
        "doc_originals",
        "doc_identifiers",
        "doc_categories",
        "doc_areas",
        "lineage",
        "near_dups",
        "repos",
        "repo_files",
        "quarantine",
        "jobs",
        "doc_fts",
    ] {
        let n: i64 = conn.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0))?;
        out.push((t.to_string(), n));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_once_and_sets_pragmas() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.sqlite");
        let conn = open_writer(&path).unwrap();
        assert_eq!(user_version(&conn).unwrap(), SCHEMA_VERSION);
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
        let sync: i64 = conn.query_row("PRAGMA synchronous", [], |r| r.get(0)).unwrap();
        assert_eq!(sync, 1, "NORMAL");
        assert_eq!(table_counts(&conn).unwrap().len(), 16);
        drop(conn);
        let conn = open_writer(&path).unwrap();
        assert_eq!(user_version(&conn).unwrap(), SCHEMA_VERSION);

        let reader = open_reader(&path).unwrap();
        assert!(reader.execute("INSERT INTO meta VALUES ('a','b')", []).is_err(), "readers are query-only");
        assert!(get_meta(&reader, "a").unwrap().is_none());
        set_meta(&conn, "a", "1").unwrap();
        set_meta(&conn, "a", "2").unwrap();
        assert_eq!(get_meta(&reader, "a").unwrap().as_deref(), Some("2"));
    }

    #[test]
    fn refuses_newer_schemas_and_missing_databases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.sqlite");
        assert!(open_reader(&path).is_err(), "a reader never creates the database");
        assert!(!path.exists());
        let conn = open_writer(&path).unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1).unwrap();
        drop(conn);
        assert!(matches!(open_writer(&path), Err(CoreError::Schema { .. })));
        assert!(matches!(open_reader(&path), Err(CoreError::Schema { .. })));
    }

    #[test]
    fn constraints_hold() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_writer(&dir.path().join("v.sqlite")).unwrap();
        assert!(conn.execute("INSERT INTO doc_areas VALUES ('nope', 'memory')", []).is_err(), "foreign keys are on");
        assert!(
            conn.execute(
                "INSERT INTO jobs(stage, key, state, fingerprint, updated_at) VALUES ('s','k','bogus','f','t')",
                []
            )
            .is_err()
        );
        assert!(conn.execute("INSERT INTO batches VALUES ('b','other','n','t',NULL,NULL,'s',NULL,NULL)", []).is_err());
        let fts: i64 =
            conn.query_row("SELECT count(*) FROM doc_fts WHERE doc_fts MATCH 'x'", [], |r| r.get(0)).unwrap();
        assert_eq!(fts, 0);
    }

    #[test]
    fn kinds_round_trip() {
        for k in ALL_KINDS {
            assert_eq!(parse_kind(kind_str(k)), Some(k));
            assert_eq!(serde_json::to_value(k).unwrap(), kind_str(k));
        }
        assert_eq!(parse_kind("nope"), None);
    }
}
