//! Recursive discovery. Only `.pdf`, `.epub`, `.md` and `.txt` (any case) are documents;
//! every other file is counted by extension from its metadata and never read.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    Pdf,
    Epub,
    Md,
    Txt,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FileTotals {
    pub files: u64,
    pub bytes: u64,
}

impl FileTotals {
    pub fn add(&mut self, bytes: u64) {
        self.files += 1;
        self.bytes += bytes;
    }
}

#[derive(Debug, Clone)]
pub struct DocFile {
    /// Path relative to the survey root, with `/` separators.
    pub rel_path: String,
    pub path: PathBuf,
    pub kind: DocKind,
    pub bytes: u64,
}

#[derive(Debug, Default)]
pub struct Discovery {
    /// Sorted by `rel_path`, so reports and duplicate "canonical" picks are deterministic.
    pub docs: Vec<DocFile>,
    /// Non-document files keyed by lowercase extension (`(none)` if there is none).
    pub other: BTreeMap<String, FileTotals>,
    pub walk_errors: Vec<String>,
}

/// Lowercase extension of `path`, or `(none)`.
pub fn extension_key(path: &Path) -> String {
    path.extension()
        .map_or_else(|| "(none)".to_string(), |e| e.to_string_lossy().to_lowercase())
}

pub fn doc_kind(path: &Path) -> Option<DocKind> {
    match path.extension()?.to_string_lossy().to_lowercase().as_str() {
        "pdf" => Some(DocKind::Pdf),
        "epub" => Some(DocKind::Epub),
        "md" => Some(DocKind::Md),
        "txt" => Some(DocKind::Txt),
        _ => None,
    }
}

pub fn discover(root: &Path) -> Discovery {
    let mut found = Discovery::default();
    for entry in walkdir::WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) if entry.file_type().is_file() => entry,
            Ok(_) => continue,
            Err(e) => {
                found.walk_errors.push(e.to_string());
                continue;
            }
        };
        let bytes = match entry.metadata() {
            Ok(meta) => meta.len(),
            Err(e) => {
                found.walk_errors.push(format!("{}: {e}", entry.path().display()));
                continue;
            }
        };
        let path = entry.into_path();
        match doc_kind(&path) {
            Some(kind) => {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                found.docs.push(DocFile {
                    rel_path: rel.to_string_lossy().replace('\\', "/"),
                    path,
                    kind,
                    bytes,
                });
            }
            None => found.other.entry(extension_key(&path)).or_default().add(bytes),
        }
    }
    found.docs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn document_kinds_are_case_insensitive() {
        assert_eq!(doc_kind(Path::new("a/Book.PDF")), Some(DocKind::Pdf));
        assert_eq!(doc_kind(Path::new("b.Epub")), Some(DocKind::Epub));
        assert_eq!(doc_kind(Path::new("notes.MD")), Some(DocKind::Md));
        assert_eq!(doc_kind(Path::new("x.txt")), Some(DocKind::Txt));
    }

    #[test]
    fn other_files_are_not_documents() {
        for p in ["a.djvu", "b.mobi", "c.pdf.part", "d", ".pdf", "e.markdown", "f.azw3"] {
            assert_eq!(doc_kind(Path::new(p)), None, "{p}");
        }
    }

    #[test]
    fn extension_key_is_lowercase_or_none() {
        assert_eq!(extension_key(Path::new("a/B.DjVu")), "djvu");
        assert_eq!(extension_key(Path::new("README")), "(none)");
        assert_eq!(extension_key(Path::new(".gitignore")), "(none)");
    }

    #[test]
    fn discover_walks_recursively_and_counts_others_without_reading() {
        let root = std::env::temp_dir().join(format!("rag-engine-discover-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub/deeper")).unwrap();
        fs::write(root.join("b.PDF"), b"12345").unwrap();
        fs::write(root.join("sub/a.txt"), b"hello").unwrap();
        fs::write(root.join("sub/deeper/c.epub"), b"x").unwrap();
        fs::write(root.join("sub/deeper/d.md"), b"").unwrap();
        fs::write(root.join("scan.djvu"), b"1234567").unwrap();
        fs::write(root.join("sub/other.DJVU"), b"123").unwrap();
        fs::write(root.join("LICENSE"), b"ab").unwrap();

        let found = discover(&root);
        fs::remove_dir_all(&root).unwrap();

        let docs: Vec<(&str, DocKind, u64)> =
            found.docs.iter().map(|d| (d.rel_path.as_str(), d.kind, d.bytes)).collect();
        assert_eq!(
            docs,
            vec![
                ("b.PDF", DocKind::Pdf, 5),
                ("sub/a.txt", DocKind::Txt, 5),
                ("sub/deeper/c.epub", DocKind::Epub, 1),
                ("sub/deeper/d.md", DocKind::Md, 0),
            ]
        );
        assert_eq!(found.other.get("djvu"), Some(&FileTotals { files: 2, bytes: 10 }));
        assert_eq!(found.other.get("(none)"), Some(&FileTotals { files: 1, bytes: 2 }));
        assert_eq!(found.other.len(), 2);
        assert!(found.walk_errors.is_empty());
    }

    #[test]
    fn discover_reports_a_missing_root_as_a_walk_error() {
        let found = discover(Path::new("definitely/not/a/real/dir"));
        assert!(found.docs.is_empty());
        assert_eq!(found.walk_errors.len(), 1);
    }
}
