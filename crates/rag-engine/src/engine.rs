//! Everything under `--data` that searches and jobs share, plus the read-only source roots.

use crate::bm25::Bm25;
use crate::embed::Embedder;
use crate::error::Result;
use crate::meta::{self, Root};
use crate::ocr::OcrProvider;
use crate::store;
use crate::vector::Vectors;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

pub const BM25_DIR: &str = "bm25";
pub const LANCEDB_DIR: &str = "lancedb";
pub const SURVEY_FILE: &str = "survey/latest.json";

/// The folders documents are read from. Nothing under them is ever written or deleted.
#[derive(Debug, Clone)]
pub struct Roots {
    pub corpus: PathBuf,
    pub notes: Option<PathBuf>,
}

impl Roots {
    /// Both roots exist, and `data_dir` (created if missing) is not inside either of them.
    pub fn check(&self, data_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(data_dir).map_err(|e| format!("cannot create {}: {e}", data_dir.display()))?;
        let data = data_dir.canonicalize()?;
        for (flag, dir) in [("--corpus", Some(&self.corpus)), ("--notes", self.notes.as_ref())] {
            let Some(dir) = dir else { continue };
            if !dir.is_dir() {
                return Err(format!("{flag} {} is not a directory", dir.display()).into());
            }
            if data.starts_with(dir.canonicalize()?) {
                return Err(format!("--data must not be inside {flag} {}: the corpus is read-only", dir.display()).into());
            }
        }
        Ok(())
    }

    pub fn dir(&self, root: Root) -> Option<&Path> {
        match root {
            Root::Corpus => Some(&self.corpus),
            Root::Notes => self.notes.as_deref(),
        }
    }

    /// The file behind a stored path such as `corpus/math/books/x.pdf`.
    pub fn resolve(&self, stored: &str) -> Option<PathBuf> {
        let (root, rel) = meta::split_stored(stored)?;
        Some(rel.split('/').fold(self.dir(root)?.to_path_buf(), |path, part| path.join(part)))
    }
}

/// An absolute path with `/` separators, for reports.
pub fn display_path(path: &Path) -> String {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()).display().to_string().replace('\\', "/")
}

pub struct Engine {
    pub data_dir: PathBuf,
    pub bm25: Bm25,
    /// Opened when an embedding provider is configured, or when vectors exist from earlier runs
    /// (so removed documents are also removed there).
    pub vectors: Option<Vectors>,
    pub embedder: Option<Embedder>,
    pub ocr: Option<Box<dyn OcrProvider>>,
    conn: Arc<Mutex<Connection>>,
}

impl Engine {
    pub async fn open(data_dir: &Path, embedder: Option<Embedder>, ocr: Option<Box<dyn OcrProvider>>) -> Result<Engine> {
        let conn = store::open(data_dir)?;
        let bm25 = Bm25::open(&data_dir.join(BM25_DIR))?;
        let lancedb = data_dir.join(LANCEDB_DIR);
        let vectors = if embedder.is_some() || lancedb.is_dir() { Some(Vectors::open(&lancedb).await?) } else { None };
        Ok(Engine { data_dir: data_dir.to_path_buf(), bm25, vectors, embedder, ocr, conn: Arc::new(Mutex::new(conn)) })
    }

    /// The connection used by requests (searches, status, jobs). Ingest opens its own.
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The same connection, for jobs to record their state.
    pub fn shared_conn(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::temp_dir;

    #[test]
    fn resolve_maps_stored_paths_to_files() {
        let roots = Roots { corpus: PathBuf::from("C:/lib/corpus"), notes: None };
        assert_eq!(roots.resolve("corpus/math/books/x.pdf"), Some(PathBuf::from("C:/lib/corpus/math/books/x.pdf")));
        assert_eq!(roots.resolve("notes/chem/x.md"), None);
        assert_eq!(roots.resolve("elsewhere/x.md"), None);
    }

    #[test]
    fn data_inside_the_corpus_is_refused() {
        let dir = temp_dir("roots");
        let corpus = dir.join("corpus");
        std::fs::create_dir_all(&corpus).unwrap();
        let roots = Roots { corpus: corpus.clone(), notes: None };
        assert!(roots.check(&dir.join("data")).is_ok());
        let err = roots.check(&corpus.join("data")).unwrap_err().0;
        assert!(err.contains("read-only"), "{err}");
        let missing = Roots { corpus: dir.join("missing"), notes: None };
        assert!(missing.check(&dir.join("data")).unwrap_err().0.contains("not a directory"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
