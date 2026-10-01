//! The crate's error type. Every variant names what failed and where, so intake can record it as a reason and
//! the owner can act on it without a debugger.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("config: {0}")]
    Config(String),
    #[error("path {path:?} rejected: {reason}")]
    Path { path: String, reason: String },
    #[error("{path} resolves outside its tier root {root}")]
    Escape { path: PathBuf, root: PathBuf },
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("database schema version {found} is {relation} this build's version {expected}: {hint}")]
    Schema { found: i64, expected: i64, relation: &'static str, hint: &'static str },
    #[error("json {what}: {source}")]
    Json {
        what: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("refusing to overwrite {path}: a stored file is written once and never changed")]
    WouldOverwrite { path: PathBuf },
    #[error("{path} should hash to {expected} but hashes to {actual}")]
    HashMismatch { path: PathBuf, expected: String, actual: String },
    #[error("id collision: hash16 {hash16} already belongs to different text")]
    IdCollision { hash16: String },
    #[error("text is not canonical: {0}")]
    NotCanonical(String),
    #[error("invalid structure: {0}")]
    InvalidStructure(String),
    #[error("sidecar {doc_id}: structural fields may not change ({fields})")]
    SidecarImmutable { doc_id: String, fields: String },
    #[error("unknown document {0}")]
    UnknownDocument(String),
    #[error("another writer holds {0}; only one intake process may write to a vault")]
    WriterLocked(PathBuf),
    #[error("moving {src} to {dest} crosses volumes; move mode needs both on one volume")]
    CrossVolumeMove { src: PathBuf, dest: PathBuf },
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

impl CoreError {
    pub(crate) fn io(action: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io { action, path: path.into(), source }
    }

    pub(crate) fn json(what: impl Into<String>, source: serde_json::Error) -> Self {
        CoreError::Json { what: what.into(), source }
    }
}
