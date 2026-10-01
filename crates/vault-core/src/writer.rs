//! The single writer (intake's side of the store): originals, quarantine, batches and deliveries, `publish`,
//! metadata updates and the job queue.
//!
//! **Publish order** (crash-safe and idempotent): validate; write the text once; write the sidecar; commit the
//! SQLite rows in one transaction (the commit point: a document exists for the tools exactly when its row
//! does); append the manifest line. A crash before the commit leaves only files that no tool can reach and that
//! the re-run adopts byte for byte; a crash after it leaves a complete document whose missing manifest line the
//! re-run appends. Re-running a finished publish adds nothing except new provenance.

use crate::canon::{self, BlockSpan};
use crate::config::{Layout, OriginalsMode, VaultConfig};
use crate::db::{self, DocRow, kind_str};
use crate::error::{CoreError, Result};
use crate::ids::{self, blake3_hex};
use crate::jobs::{self, Enqueued, Job, JobState};
use crate::paths::RelPath;
use crate::sidecar::{CodeInfo, Extraction, Provenance, SIDECAR_FORMAT, Sidecar, SidecarMetadata, SourceRef};
use crate::store::{self, Manifest, ManifestEntry, ORIGINALS_MANIFEST, TEXT_MANIFEST, VerifyReport, WriteOutcome};
use crate::time::{now_rfc3339, today};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use vault_api::{DocId, DocKind, Heading, PageMark};

// ------------------------------------------------------------------------------------------------- types

/// Quarantine reason codes (SPEC section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    UnsupportedFormat,
    Protected,
    CorruptFile,
    EmptyFile,
    ExtractionFailed,
    LowQualityText,
    IncompleteCopy,
    ArchiveLimits,
    IdCollision,
    PathTooLong,
    SuspectedSecret,
}

impl ReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ReasonCode::UnsupportedFormat => "unsupported_format",
            ReasonCode::Protected => "protected",
            ReasonCode::CorruptFile => "corrupt_file",
            ReasonCode::EmptyFile => "empty_file",
            ReasonCode::ExtractionFailed => "extraction_failed",
            ReasonCode::LowQualityText => "low_quality_text",
            ReasonCode::IncompleteCopy => "incomplete_copy",
            ReasonCode::ArchiveLimits => "archive_limits",
            ReasonCode::IdCollision => "id_collision",
            ReasonCode::PathTooLong => "path_too_long",
            ReasonCode::SuspectedSecret => "suspected_secret",
        }
    }
}

/// One extraction attempt, recorded in a quarantine reason file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub tool: String,
    pub error: String,
    pub at: String,
}

#[derive(Debug, Clone)]
pub struct QuarantineRequest<'a> {
    /// The delivered file. Ignored for `suspected_secret`, which is never copied.
    pub source: Option<&'a Path>,
    pub blake3: String,
    pub ext: String,
    pub reason: ReasonCode,
    pub detail: String,
    pub batch: Option<String>,
    /// Path inside the batch, `/`-separated.
    pub original_path: Option<String>,
    pub attempts: Vec<Attempt>,
    pub delivery_id: Option<i64>,
}

/// `cold/quarantine/{h2}/{blake3}.reason.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasonFile {
    pub reason: ReasonCode,
    pub detail: String,
    pub batch: Option<String>,
    pub original_path: Option<String>,
    pub blake3: String,
    pub ext: String,
    /// False for suspected secrets: the file stays where it was delivered.
    pub copied: bool,
    pub attempts: Vec<Attempt>,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineRecord {
    pub id: i64,
    pub stored_rel_path: Option<RelPath>,
    pub reason_rel_path: RelPath,
}

#[derive(Debug, Clone)]
pub struct StoreOriginal<'a> {
    pub source: &'a Path,
    /// BLAKE3 of the source, computed by intake when it hashed the delivery.
    pub blake3: String,
    pub ext: String,
    pub format: Option<String>,
    pub batch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOriginal {
    pub blake3: String,
    pub rel_path: RelPath,
    pub bytes: u64,
    /// The same bytes were already stored.
    pub duplicate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchKind {
    Batch,
    Oaforge,
    Loose,
    Code,
}

impl BatchKind {
    fn as_str(self) -> &'static str {
        match self {
            BatchKind::Batch => "batch",
            BatchKind::Oaforge => "oaforge",
            BatchKind::Loose => "loose",
            BatchKind::Code => "code",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub batch_id: String,
    pub kind: BatchKind,
    pub inbox_name: String,
    pub detected_at: String,
    pub ready_at: Option<String>,
    pub completed_at: Option<String>,
    pub state: String,
    pub files_expected: Option<u64>,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewDelivery {
    pub batch_id: String,
    pub rel_path: String,
    pub bytes: Option<u64>,
    pub blake3: Option<String>,
    pub sha256: Option<String>,
    pub received_at: String,
    pub manifest_json: Option<String>,
    pub hint_category: Option<String>,
}

/// `deliveries.outcome` (SPEC section 6): every delivered file reaches exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Published,
    Duplicate,
    NearDuplicate,
    Quarantined,
    Excluded,
    LeftInPlace,
    ListedMissing,
    WaitingGpu,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Pending => "pending",
            Outcome::Published => "published",
            Outcome::Duplicate => "duplicate",
            Outcome::NearDuplicate => "near_duplicate",
            Outcome::Quarantined => "quarantined",
            Outcome::Excluded => "excluded",
            Outcome::LeftInPlace => "left_in_place",
            Outcome::ListedMissing => "listed_missing",
            Outcome::WaitingGpu => "waiting_gpu",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| CoreError::InvalidArgument(format!("unknown delivery outcome {s:?}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryOutcome {
    pub outcome: Outcome,
    pub reason: Option<String>,
    pub rule: Option<String>,
    pub detail: Option<String>,
    pub doc_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Repo {
    pub repo_id: String,
    pub name: String,
    pub url: Option<String>,
    pub commit: Option<String>,
    pub license: Option<String>,
    pub snapshot_at: Option<String>,
    pub doc_id: Option<String>,
    pub batch_id: Option<String>,
}

/// A newer extraction replacing an older document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supersede {
    /// The old document's id or hash16.
    pub doc: String,
    pub reason: String,
}

/// Everything `publish` needs. `text` must already be canonical ([`canon::normalize_prose`] for prose,
/// [`canon::normalize_code`] for code); structure line numbers refer to it.
#[derive(Debug, Clone)]
pub struct PublishRequest {
    pub text: String,
    pub kind: DocKind,
    pub metadata: SidecarMetadata,
    pub headings: Vec<Heading>,
    pub pages: Vec<PageMark>,
    pub blocks: Vec<BlockSpan>,
    /// Required for (and only for) `DocKind::Code`.
    pub code: Option<CodeInfo>,
    pub extraction: Extraction,
    /// Originals this text came from; each must already be stored with [`Writer::store_original`].
    pub sources: Vec<SourceRef>,
    pub level: Option<String>,
    pub license: Option<String>,
    /// Indexed for `find_docs`; not stored elsewhere.
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub supersedes: Option<Supersede>,
}

impl PublishRequest {
    pub fn new(text: String, kind: DocKind) -> Self {
        PublishRequest {
            text,
            kind,
            metadata: SidecarMetadata::default(),
            headings: Vec::new(),
            pages: Vec::new(),
            blocks: Vec::new(),
            code: None,
            extraction: Extraction::default(),
            sources: Vec::new(),
            level: None,
            license: None,
            abstract_text: None,
            keywords: Vec::new(),
            supersedes: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishStatus {
    Published,
    /// The same text was already published; only new provenance (and sources) were added.
    AlreadyPublished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishOutcome {
    pub doc_id: DocId,
    pub status: PublishStatus,
    pub rel_path: RelPath,
    pub lines: u32,
}

// ------------------------------------------------------------------------------------------------- writer

pub struct Writer {
    config: VaultConfig,
    conn: Mutex<Connection>,
    publish_lock: Mutex<()>,
    text_manifest: Mutex<Manifest>,
    originals_manifest: Mutex<Manifest>,
    _lock: File,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic while holding the guard cannot leave partial state: SQLite rolls back and files are write-once.
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Writer {
    /// Takes the vault's writer lock, opens (or creates and migrates) the database, checks the vault identity
    /// and returns interrupted jobs to the queue.
    pub fn open(config: VaultConfig) -> Result<Writer> {
        let hot = &config.layout.hot;
        let lock_path = hot.prepare_write(&RelPath::new("db/writer.lock")?)?;
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(|e| CoreError::io("open writer lock", &lock_path, e))?;
        match lock_file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Err(CoreError::WriterLocked(lock_path)),
            Err(std::fs::TryLockError::Error(e)) => return Err(CoreError::io("lock", &lock_path, e)),
        }
        let db_path = hot.prepare_write(&RelPath::new("db/vault.sqlite")?)?;
        let conn = db::open_writer(&db_path)?;
        match db::get_meta(&conn, "vault_id")? {
            None => db::set_meta(&conn, "vault_id", &config.vault.vault_id)?,
            Some(id) if id != config.vault.vault_id => {
                return Err(CoreError::Config(format!(
                    "the database belongs to vault {id} but vault.toml says {}; is the config pointing at the right vault?",
                    config.vault.vault_id
                )));
            }
            Some(_) => {}
        }
        if let Some(v) = &config.vault.taxonomy.version
            && db::get_meta(&conn, "taxonomy_version")?.is_none()
        {
            db::set_meta(&conn, "taxonomy_version", v)?;
        }
        jobs::recover(&conn)?;
        let text_manifest = Manifest::open(hot, TEXT_MANIFEST)?;
        let originals_manifest = Manifest::open(hot, ORIGINALS_MANIFEST)?;
        tracing::info!(vault_id = %config.vault.vault_id, "writer opened");
        Ok(Writer {
            config,
            conn: Mutex::new(conn),
            publish_lock: Mutex::new(()),
            text_manifest: Mutex::new(text_manifest),
            originals_manifest: Mutex::new(originals_manifest),
            _lock: lock_file,
        })
    }

    pub fn layout(&self) -> &Layout {
        &self.config.layout
    }

    /// fsyncs both manifests; intake calls this at the end of each batch.
    pub fn sync(&self) -> Result<()> {
        lock(&self.text_manifest).sync()?;
        lock(&self.originals_manifest).sync()
    }

    pub fn verify(&self) -> Result<VerifyReport> {
        self.sync()?;
        store::verify(&self.config.layout)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        db::set_meta(&lock(&self.conn), key, value)
    }

    /// Row counts per table (for status pages and idempotence checks).
    pub fn table_counts(&self) -> Result<Vec<(String, i64)>> {
        db::table_counts(&lock(&self.conn))
    }

    // -------------------------------------------------------------------------------------- originals

    /// Stores an original under `cold/originals/{h2}/{blake3}.{ext}` (copy or move per config), records it, and
    /// appends its manifest line. Bytes already stored (under any extension) are a duplicate.
    pub fn store_original(&self, req: &StoreOriginal<'_>) -> Result<StoredOriginal> {
        ids::check_blake3(&req.blake3)?;
        let cold = &self.config.layout.cold;
        let existing: Option<(String, i64)> = lock(&self.conn)
            .query_row("SELECT stored_rel_path, bytes FROM originals WHERE blake3 = ?1", [&req.blake3], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        if let Some((rel, _)) = existing {
            let rel = RelPath::new(&rel)?;
            let path = cold.resolve_existing(&rel)?;
            let (actual, bytes) = ids::hash_file(&path)?;
            if actual != req.blake3 {
                return Err(CoreError::HashMismatch { path, expected: req.blake3.clone(), actual });
            }
            self.manifest_original(&rel, &req.blake3, bytes)?;
            return Ok(StoredOriginal { blake3: req.blake3.clone(), rel_path: rel, bytes, duplicate: true });
        }
        let rel = ids::original_rel_path(&req.blake3, &req.ext)?;
        let placed = match self.config.machine.intake.originals {
            OriginalsMode::Copy => store::copy_verified(cold, req.source, &rel, &req.blake3)?,
            OriginalsMode::Move => store::move_verified(cold, req.source, &rel, &req.blake3)?,
        };
        lock(&self.conn).execute(
            "INSERT INTO originals(blake3, ext, bytes, format, stored_rel_path, first_batch, stored_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(blake3) DO NOTHING",
            params![
                req.blake3,
                ids::original_ext(&req.ext),
                i64::try_from(placed.bytes).unwrap_or(i64::MAX),
                req.format,
                rel.as_str(),
                req.batch,
                now_rfc3339()
            ],
        )?;
        self.manifest_original(&rel, &req.blake3, placed.bytes)?;
        tracing::info!(blake3 = %req.blake3, rel = %rel, "original stored");
        Ok(StoredOriginal {
            blake3: req.blake3.clone(),
            rel_path: rel,
            bytes: placed.bytes,
            duplicate: placed.outcome == WriteOutcome::AlreadyPresent,
        })
    }

    fn manifest_original(&self, rel: &RelPath, blake3: &str, bytes: u64) -> Result<()> {
        lock(&self.originals_manifest).ensure(ManifestEntry {
            path: rel.to_string(),
            blake3: blake3.to_string(),
            bytes,
            at: now_rfc3339(),
        })?;
        Ok(())
    }

    // ------------------------------------------------------------------------------------------- quarantine

    /// Copies the file to `cold/quarantine` (never for suspected secrets), writes its reason file and records
    /// the quarantine row. Re-quarantining the same bytes rewrites the reason file with the new attempts.
    pub fn quarantine(&self, req: &QuarantineRequest<'_>) -> Result<QuarantineRecord> {
        let cold = &self.config.layout.cold;
        let (file_rel, reason_rel) = ids::quarantine_rel_paths(&req.blake3, &req.ext)?;
        let copy = req.reason != ReasonCode::SuspectedSecret;
        let stored = match (copy, req.source) {
            (true, Some(src)) => {
                store::copy_verified(cold, src, &file_rel, &req.blake3)?;
                Some(file_rel)
            }
            _ => None,
        };
        let at = now_rfc3339();
        let reason = ReasonFile {
            reason: req.reason,
            detail: req.detail.clone(),
            batch: req.batch.clone(),
            original_path: req.original_path.clone(),
            blake3: req.blake3.clone(),
            ext: ids::original_ext(&req.ext),
            copied: stored.is_some(),
            attempts: req.attempts.clone(),
            at: at.clone(),
        };
        let mut json = serde_json::to_vec_pretty(&reason).map_err(|e| CoreError::json("reason file", e))?;
        json.push(b'\n');
        store::replace_atomic(cold, &reason_rel, &json)?;
        let conn = lock(&self.conn);
        conn.execute(
            "INSERT INTO quarantine(delivery_id, blake3, reason, detail, stored_rel_path, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![req.delivery_id, req.blake3, req.reason.as_str(), req.detail, stored.as_ref().map(RelPath::as_str), at],
        )?;
        tracing::info!(blake3 = %req.blake3, reason = req.reason.as_str(), copied = stored.is_some(), "quarantined");
        Ok(QuarantineRecord { id: conn.last_insert_rowid(), stored_rel_path: stored, reason_rel_path: reason_rel })
    }

    // ------------------------------------------------------------------------------- batches and deliveries

    pub fn upsert_batch(&self, b: &Batch) -> Result<()> {
        lock(&self.conn).execute(
            "INSERT INTO batches(batch_id, kind, inbox_name, detected_at, ready_at, completed_at, state, files_expected, report_path) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(batch_id) DO UPDATE SET \
             ready_at = excluded.ready_at, completed_at = excluded.completed_at, state = excluded.state, \
             files_expected = excluded.files_expected, report_path = excluded.report_path",
            params![
                b.batch_id,
                b.kind.as_str(),
                b.inbox_name,
                b.detected_at,
                b.ready_at,
                b.completed_at,
                b.state,
                b.files_expected.map(|n| i64::try_from(n).unwrap_or(i64::MAX)),
                b.report_path
            ],
        )?;
        Ok(())
    }

    /// Records a delivered file once per (batch, path) and returns its id; a repeat fills in newly known fields.
    pub fn record_delivery(&self, d: &NewDelivery) -> Result<i64> {
        Ok(lock(&self.conn).query_row(
            "INSERT INTO deliveries(batch_id, rel_path, bytes, blake3, sha256, received_at, manifest_json, hint_category) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(batch_id, rel_path) DO UPDATE SET \
             bytes = coalesce(excluded.bytes, deliveries.bytes), blake3 = coalesce(excluded.blake3, deliveries.blake3), \
             sha256 = coalesce(excluded.sha256, deliveries.sha256), \
             manifest_json = coalesce(excluded.manifest_json, deliveries.manifest_json), \
             hint_category = coalesce(excluded.hint_category, deliveries.hint_category) \
             RETURNING delivery_id",
            params![
                d.batch_id,
                d.rel_path,
                d.bytes.map(|n| i64::try_from(n).unwrap_or(i64::MAX)),
                d.blake3,
                d.sha256,
                d.received_at,
                d.manifest_json,
                d.hint_category
            ],
            |r| r.get(0),
        )?)
    }

    pub fn set_delivery_outcome(&self, delivery_id: i64, o: &DeliveryOutcome) -> Result<()> {
        let n = lock(&self.conn).execute(
            "UPDATE deliveries SET outcome = ?2, reason = ?3, rule = ?4, detail = ?5, doc_id = ?6 WHERE delivery_id = ?1",
            params![delivery_id, o.outcome.as_str(), o.reason, o.rule, o.detail, o.doc_id],
        )?;
        if n == 0 {
            return Err(CoreError::InvalidArgument(format!("no delivery {delivery_id}")));
        }
        Ok(())
    }

    /// (rel_path, outcome) for every delivery of a batch, in path order.
    pub fn delivery_outcomes(&self, batch_id: &str) -> Result<Vec<(String, Outcome)>> {
        let conn = lock(&self.conn);
        let mut stmt =
            conn.prepare("SELECT rel_path, outcome FROM deliveries WHERE batch_id = ?1 ORDER BY rel_path")?;
        let rows = stmt.query_map([batch_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.map(|r| {
            let (p, o) = r?;
            Ok((p, Outcome::parse(&o)?))
        })
        .collect()
    }

    pub fn upsert_repo(&self, r: &Repo) -> Result<()> {
        lock(&self.conn).execute(
            "INSERT INTO repos(repo_id, name, url, \"commit\", license, snapshot_at, doc_id, batch_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(repo_id) DO UPDATE SET name = excluded.name, \
             url = excluded.url, \"commit\" = excluded.\"commit\", license = excluded.license, \
             snapshot_at = excluded.snapshot_at, doc_id = excluded.doc_id, batch_id = excluded.batch_id",
            params![r.repo_id, r.name, r.url, r.commit, r.license, r.snapshot_at, r.doc_id, r.batch_id],
        )?;
        Ok(())
    }

    pub fn add_near_dup(&self, group_id: &str, doc: &str, is_primary: bool, similarity: Option<f64>) -> Result<()> {
        let conn = lock(&self.conn);
        let row = resolve_doc(&conn, doc)?;
        conn.execute(
            "INSERT INTO near_dups(group_id, doc_id, is_primary, similarity) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(group_id, doc_id) DO UPDATE SET is_primary = excluded.is_primary, similarity = excluded.similarity",
            params![group_id, row.doc_id, is_primary, similarity],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------------------------------------ jobs

    pub fn enqueue_job(&self, stage: &str, key: &str, fingerprint: &str) -> Result<Enqueued> {
        jobs::enqueue(&mut lock(&self.conn), stage, key, fingerprint)
    }

    pub fn claim_jobs(&self, stage: Option<&str>, limit: u32) -> Result<Vec<Job>> {
        jobs::claim(&mut lock(&self.conn), stage, limit)
    }

    pub fn complete_job(&self, job_id: i64, fingerprint: &str) -> Result<bool> {
        jobs::complete(&lock(&self.conn), job_id, fingerprint)
    }

    pub fn fail_job(&self, job_id: i64, error: &str, max_attempts: u32) -> Result<JobState> {
        jobs::fail(&lock(&self.conn), job_id, error, max_attempts)
    }

    pub fn park_job(&self, job_id: i64, reason: &str) -> Result<()> {
        jobs::wait(&lock(&self.conn), job_id, reason)
    }

    pub fn release_waiting_jobs(&self, stage: &str) -> Result<usize> {
        jobs::release_waiting(&lock(&self.conn), stage)
    }

    pub fn retry_job(&self, stage: &str, key: &str) -> Result<bool> {
        jobs::retry(&lock(&self.conn), stage, key)
    }

    pub fn job_is_current(&self, stage: &str, key: &str, fingerprint: &str) -> Result<bool> {
        jobs::is_current(&lock(&self.conn), stage, key, fingerprint)
    }

    pub fn job_counts(&self) -> Result<Vec<(String, JobState, u64)>> {
        jobs::counts(&lock(&self.conn))
    }

    // --------------------------------------------------------------------------------------------- publish

    /// Publishes canonical text with its structure and metadata. Idempotent: publishing the same text again
    /// returns the same id and adds only provenance and sources. Different text whose hash16 is taken fails with
    /// [`CoreError::IdCollision`] (intake quarantines it as `id_collision`).
    pub fn publish(&self, req: PublishRequest) -> Result<PublishOutcome> {
        let lines = validate_text(req.kind, &req.text)?;
        validate_structure(lines, &req)?;
        let bytes = req.text.as_bytes();
        let full = blake3_hex(bytes);
        let hash16 = &full[..vault_api::address::HASH_LEN];
        let _guard = lock(&self.publish_lock);

        // Identity, supersession and sources, read before any file is written.
        let (existing, old, repo_name) = {
            let conn = lock(&self.conn);
            let existing = db::doc_by_hash(&conn, hash16)?;
            if let Some(row) = &existing
                && row.text_blake3 != full
            {
                return Err(CoreError::IdCollision { hash16: hash16.to_string() });
            }
            let old = match &req.supersedes {
                Some(s) => {
                    let row = resolve_doc(&conn, &s.doc)?;
                    (row.hash16 != hash16).then_some((row, s.reason.clone()))
                }
                None => None,
            };
            for s in &req.sources {
                let known: Option<i64> = conn
                    .query_row("SELECT 1 FROM originals WHERE blake3 = ?1", [&s.original], |r| r.get(0))
                    .optional()?;
                if known.is_none() {
                    return Err(CoreError::InvalidArgument(format!(
                        "source original {} is not stored; call store_original first",
                        s.original
                    )));
                }
            }
            let repo_name = match req.code.as_ref().and_then(|c| c.repo.as_ref()) {
                Some(repo_id) => {
                    let name: Option<String> = conn
                        .query_row("SELECT name FROM repos WHERE repo_id = ?1", [repo_id], |r| r.get(0))
                        .optional()?;
                    Some(name.ok_or_else(|| {
                        CoreError::InvalidArgument(format!("unknown repository {repo_id}; call upsert_repo first"))
                    })?)
                }
                None => None,
            };
            (existing, old, repo_name)
        };

        let doc_id: DocId = match &existing {
            Some(row) => row.doc_id.parse().map_err(|e| CoreError::InvalidArgument(format!("stored doc_id: {e}")))?,
            None => {
                let slug = match &req.code {
                    Some(c) => ids::code_slug(repo_name.as_deref().unwrap_or(""), &c.path),
                    None => ids::slug(req.metadata.title.as_deref().unwrap_or("")),
                };
                ids::doc_id(hash16, &slug)?
            }
        };
        let hot = &self.config.layout.hot;
        let text_rel = match &existing {
            Some(row) => RelPath::new(&row.rel_path)?,
            None => ids::text_rel_path(&doc_id, req.kind, req.code.as_ref().map(|c| c.path.as_str()))?,
        };
        match store::write_once(hot, &text_rel, bytes) {
            Ok(_) => {}
            Err(CoreError::WouldOverwrite { .. }) if existing.is_none() => {
                return Err(CoreError::IdCollision { hash16: hash16.to_string() });
            }
            Err(e) => return Err(e),
        }

        let sidecar_rel = ids::sidecar_rel_path(&doc_id)?;
        let on_disk = match store::read(hot, &sidecar_rel) {
            Ok(b) => Some(Sidecar::from_json(&b)?),
            Err(CoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let requested = Sidecar {
            format: SIDECAR_FORMAT,
            doc_id: doc_id.to_string(),
            kind: req.kind,
            lines,
            bytes: bytes.len() as u64,
            title: None,
            authors: Vec::new(),
            year: None,
            language: None,
            identifiers: Default::default(),
            headings: req.headings.clone(),
            pages: req.pages.clone(),
            blocks: req.blocks.clone(),
            code: req.code.clone(),
            extraction: req.extraction.clone(),
            quality: None,
            sources: req.sources.clone(),
            provenance: Vec::new(),
            categories: None,
            identity: None,
            created: today(),
            supersedes: old.as_ref().map(|(row, _)| row.doc_id.clone()),
        }
        .with_metadata(req.metadata.clone());
        let unchanged = on_disk.clone();
        let sidecar = match (&existing, on_disk) {
            // Published: the stored sidecar stands; only provenance grows.
            (Some(_), Some(current)) => {
                let mut meta = current.metadata();
                merge_provenance(&mut meta.provenance, &req.metadata.provenance);
                current.with_metadata(meta)
            }
            (Some(_), None) => requested,
            // Files from an interrupted publish: structure must match; metadata comes from this request.
            (None, Some(current)) => {
                current.check_replacement(&requested)?;
                let mut meta = req.metadata.clone();
                let mut provenance = current.provenance.clone();
                merge_provenance(&mut provenance, &meta.provenance);
                meta.provenance = provenance;
                Sidecar { created: current.created.clone(), ..requested.with_metadata(meta) }
            }
            (None, None) => requested,
        };
        if unchanged.as_ref() != Some(&sidecar) {
            store::replace_atomic(hot, &sidecar_rel, &sidecar.to_json()?)?;
        }

        {
            let mut conn = lock(&self.conn);
            let tx = conn.transaction()?;
            if existing.is_none() {
                insert_document(&tx, &doc_id, &full, &text_rel, &req, lines, &sidecar)?;
            }
            for s in &req.sources {
                tx.execute(
                    "INSERT INTO doc_originals(doc_id, blake3) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                    params![doc_id.to_string(), s.original],
                )?;
            }
            if let Some((old_row, reason)) = &old {
                tx.execute(
                    "INSERT INTO lineage(old_doc, new_doc, reason, at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
                    params![old_row.doc_id, doc_id.to_string(), reason, now_rfc3339()],
                )?;
                tx.execute(
                    "UPDATE documents SET superseded_by = ?2 WHERE doc_id = ?1 AND superseded_by IS NOT ?2",
                    params![old_row.doc_id, doc_id.to_string()],
                )?;
            }
            tx.commit()?;
        }
        lock(&self.text_manifest).ensure(ManifestEntry {
            path: text_rel.to_string(),
            blake3: full.clone(),
            bytes: bytes.len() as u64,
            at: now_rfc3339(),
        })?;
        let status = if existing.is_some() { PublishStatus::AlreadyPublished } else { PublishStatus::Published };
        tracing::info!(doc_id = %doc_id, lines, status = ?status, "publish");
        Ok(PublishOutcome { doc_id, status, rel_path: text_rel, lines })
    }

    /// Rewrites a published document's metadata: SQLite first (authoritative), then the sidecar mirror.
    pub fn update_metadata(
        &self,
        doc: &str,
        meta: SidecarMetadata,
        level: Option<String>,
        license: Option<String>,
    ) -> Result<()> {
        let _guard = lock(&self.publish_lock);
        let hot = &self.config.layout.hot;
        let row = resolve_doc(&lock(&self.conn), doc)?;
        let doc_id: DocId =
            row.doc_id.parse().map_err(|e| CoreError::InvalidArgument(format!("stored doc_id: {e}")))?;
        let sidecar_rel = ids::sidecar_rel_path(&doc_id)?;
        let current = Sidecar::from_json(&store::read(hot, &sidecar_rel)?)?;
        let next = current.with_metadata(meta);
        current.check_replacement(&next)?;
        {
            let mut conn = lock(&self.conn);
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE documents SET title = ?2, authors_json = ?3, year = ?4, language = ?5, quality = ?6, \
                 identity_confidence = ?7, primary_category = ?8, level = ?9, license = ?10 WHERE doc_id = ?1",
                params![
                    row.doc_id,
                    next.title,
                    authors_json(&next.authors)?,
                    next.year,
                    next.language,
                    next.quality.as_ref().map(|q| q.score),
                    next.identity.as_ref().and_then(|i| i.confidence),
                    next.categories.as_ref().and_then(|c| c.primary.clone()),
                    level,
                    license
                ],
            )?;
            let rowid: i64 = tx.query_row("SELECT id FROM documents WHERE doc_id = ?1", [&row.doc_id], |r| r.get(0))?;
            let (abstract_text, keywords): (Option<String>, Option<String>) = tx
                .query_row("SELECT abstract, keywords FROM doc_fts WHERE rowid = ?1", [rowid], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()?
                .unwrap_or((None, None));
            write_metadata_rows(&tx, &row.doc_id, &next)?;
            tx.execute("DELETE FROM doc_fts WHERE rowid = ?1", [rowid])?;
            insert_fts(&tx, rowid, &row.doc_id, &next, abstract_text.as_deref(), keywords.as_deref())?;
            tx.commit()?;
        }
        store::replace_atomic(hot, &sidecar_rel, &next.to_json()?)?;
        tracing::info!(doc_id = %row.doc_id, "metadata updated");
        Ok(())
    }
}

fn merge_provenance(into: &mut Vec<Provenance>, new: &[Provenance]) {
    for p in new {
        if !into.contains(p) {
            into.push(p.clone());
        }
    }
}

pub(crate) fn resolve_doc(conn: &Connection, doc: &str) -> Result<DocRow> {
    let id: DocId = doc.parse().map_err(|e| CoreError::InvalidArgument(format!("{doc:?}: {e}")))?;
    db::doc_by_hash(conn, id.hash())?.ok_or_else(|| CoreError::UnknownDocument(doc.to_string()))
}

fn authors_json(authors: &[String]) -> Result<String> {
    serde_json::to_string(authors).map_err(|e| CoreError::json("authors", e))
}

fn insert_document(
    tx: &Transaction<'_>,
    doc_id: &DocId,
    full: &str,
    text_rel: &RelPath,
    req: &PublishRequest,
    lines: u32,
    sidecar: &Sidecar,
) -> Result<()> {
    let id_str = doc_id.to_string();
    tx.execute(
        "INSERT INTO documents(doc_id, hash16, text_blake3, slug, rel_path, kind, title, authors_json, year, language, \
         lines, bytes, extraction_path, extract_fp, quality, identity_confidence, primary_category, level, license, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            id_str,
            doc_id.hash(),
            full,
            doc_id.slug().unwrap_or(""),
            text_rel.as_str(),
            kind_str(req.kind),
            sidecar.title,
            authors_json(&sidecar.authors)?,
            sidecar.year,
            sidecar.language,
            lines,
            i64::try_from(req.text.len()).unwrap_or(i64::MAX),
            sidecar.extraction.path,
            sidecar.extraction.extract_fp,
            sidecar.quality.as_ref().map(|q| q.score),
            sidecar.identity.as_ref().and_then(|i| i.confidence),
            sidecar.categories.as_ref().and_then(|c| c.primary.clone()),
            req.level,
            req.license,
            now_rfc3339()
        ],
    )?;
    let rowid = tx.last_insert_rowid();
    write_metadata_rows(tx, &id_str, sidecar)?;
    let keywords = (!req.keywords.is_empty()).then(|| req.keywords.join(", "));
    insert_fts(tx, rowid, &id_str, sidecar, req.abstract_text.as_deref(), keywords.as_deref())?;
    if let Some(code) = &req.code
        && let Some(repo_id) = &code.repo
    {
        tx.execute(
            "INSERT INTO repo_files(repo_id, path, doc_id, language, swhid) VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(repo_id, path) DO UPDATE SET doc_id = excluded.doc_id, language = excluded.language, swhid = excluded.swhid",
            params![repo_id, code.path, id_str, code.language, code.swhid],
        )?;
    }
    Ok(())
}

/// Replaces a document's identifier, category and area rows from its sidecar metadata.
fn write_metadata_rows(tx: &Transaction<'_>, doc_id: &str, s: &Sidecar) -> Result<()> {
    for table in ["doc_identifiers", "doc_categories", "doc_areas"] {
        tx.execute(&format!("DELETE FROM {table} WHERE doc_id = ?1"), [doc_id])?;
    }
    for (scheme, value) in s.identifiers.pairs() {
        tx.execute(
            "INSERT INTO doc_identifiers(doc_id, scheme, value, source) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
            params![doc_id, scheme, value, s.identity.as_ref().and_then(|i| i.source.clone())],
        )?;
    }
    if let Some(c) = &s.categories {
        let mut rows: Vec<(&str, &str)> = Vec::new();
        if let Some(p) = &c.primary {
            rows.push((p, "primary"));
        }
        rows.extend(c.secondary.iter().map(|x| (x.as_str(), "secondary")));
        if let Some(t) = &c.openalex_topic {
            rows.push((t, "topic"));
        }
        for (category, role) in rows {
            tx.execute(
                "INSERT INTO doc_categories(doc_id, category, role, source, confidence) VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT DO NOTHING",
                params![doc_id, category, role, c.source, c.confidence],
            )?;
        }
        for area in &c.areas {
            tx.execute(
                "INSERT INTO doc_areas(doc_id, area) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                params![doc_id, area],
            )?;
        }
    }
    Ok(())
}

fn insert_fts(
    tx: &Transaction<'_>,
    rowid: i64,
    doc_id: &str,
    s: &Sidecar,
    abstract_text: Option<&str>,
    keywords: Option<&str>,
) -> Result<()> {
    let headings: Vec<&str> = s.headings.iter().map(|h| h.text.as_str()).collect();
    let mut code_terms = String::new();
    if let Some(code) = &s.code {
        code_terms = code.path.clone();
        for sym in &code.symbols {
            code_terms.push(' ');
            code_terms.push_str(&sym.name);
        }
    }
    let headings =
        if code_terms.is_empty() { headings.join("\n") } else { format!("{}\n{code_terms}", headings.join("\n")) };
    tx.execute(
        "INSERT INTO doc_fts(rowid, doc_id, title, authors, abstract, headings, keywords) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![rowid, doc_id, s.title, s.authors.join("; "), abstract_text, headings, keywords],
    )?;
    Ok(())
}

/// Checks `text` is canonical for its kind and returns its line count.
fn validate_text(kind: DocKind, text: &str) -> Result<u32> {
    if text.is_empty() {
        return Err(CoreError::NotCanonical("the text is empty".into()));
    }
    if kind == DocKind::Code {
        if text.contains('\r') || text.starts_with('\u{FEFF}') {
            return Err(CoreError::NotCanonical(
                "code must have LF line endings and no BOM (canon::normalize_code)".into(),
            ));
        }
    } else if canon::normalize_prose(text).text != text {
        return Err(CoreError::NotCanonical("prose must be the output of canon::normalize_prose".into()));
    }
    u32::try_from(canon::split_lines(text).len()).map_err(|_| CoreError::NotCanonical("more than 2^32 lines".into()))
}

fn validate_structure(lines: u32, req: &PublishRequest) -> Result<()> {
    let span = |what: &str, start: u32, end: u32| -> Result<()> {
        if start == 0 || start > end || end > lines {
            return Err(CoreError::InvalidStructure(format!(
                "{what} spans lines {start}-{end} of a {lines}-line text"
            )));
        }
        Ok(())
    };
    for h in &req.headings {
        if !(1..=6).contains(&h.level) {
            return Err(CoreError::InvalidStructure(format!("heading {:?} has level {}", h.text, h.level)));
        }
        span("a heading", h.start, h.end)?;
    }
    let mut last = 0;
    for p in &req.pages {
        span("a page mark", p.line, p.line)?;
        if p.line < last {
            return Err(CoreError::InvalidStructure("page marks must be in line order".into()));
        }
        last = p.line;
    }
    for b in &req.blocks {
        span("a block", b.start, b.end)?;
    }
    match (&req.code, req.kind) {
        (Some(code), DocKind::Code) => {
            if code.path.is_empty() {
                return Err(CoreError::InvalidStructure("code needs its path".into()));
            }
            for s in &code.symbols {
                span("a symbol", s.start, s.end)?;
            }
        }
        (None, DocKind::Code) => return Err(CoreError::InvalidStructure("a code document needs code info".into())),
        (Some(_), _) => return Err(CoreError::InvalidStructure("code info belongs to code documents only".into())),
        (None, _) => {}
    }
    for s in &req.sources {
        ids::check_blake3(&s.original)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
