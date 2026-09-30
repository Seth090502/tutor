//! Document metadata decided by where a file sits (contract C1).
//!
//! | Location (relative to the root) | domain | source_type |
//! |---|---|---|
//! | `corpus/<domain>/books/**` | `<domain>` | `book` |
//! | `corpus/<domain>/papers/**` | `<domain>` | `paper` |
//! | `corpus/inbox/**` | `unsorted` | `unknown` |
//! | `corpus/<domain>/<other>/**` or `corpus/<domain>/<file>` | `<domain>` | `unknown` |
//! | `corpus/<file>` (directly in the root) | `unsorted` | `unknown` |
//! | `notes/<domain>/**` | `<domain>` | `note` |
//! | `notes/<file>` (directly in the root) | `unsorted` | `note` |
//!
//! Folder names are matched case-insensitively and domains are lowercased, so `Math/Books`
//! and `math/books` mean the same thing on Windows.

/// Every `source_type` value the service reports or accepts as a filter.
pub const SOURCE_TYPES: [&str; 4] = ["book", "paper", "note", "unknown"];

/// Domain for files that are not sorted into a domain folder yet.
pub const UNSORTED: &str = "unsorted";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    Corpus,
    Notes,
}

impl Root {
    /// Prefix of every stored and reported path under this root, e.g. `corpus/math/books/x.pdf`.
    pub fn prefix(self) -> &'static str {
        match self {
            Root::Corpus => "corpus",
            Root::Notes => "notes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocMeta {
    pub domain: String,
    pub source_type: &'static str,
}

/// Metadata for a file at `rel` (relative to `root`, `/`-separated).
pub fn classify(root: Root, rel: &str) -> DocMeta {
    let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
    let dirs = &parts[..parts.len().saturating_sub(1)];
    let meta = |domain: &str, source_type| DocMeta { domain: domain.to_lowercase(), source_type };
    match root {
        Root::Notes => match dirs.first() {
            Some(domain) => meta(domain, "note"),
            None => meta(UNSORTED, "note"),
        },
        Root::Corpus => match dirs {
            [] => meta(UNSORTED, "unknown"),
            [first, ..] if first.eq_ignore_ascii_case("inbox") => meta(UNSORTED, "unknown"),
            [domain, kind, ..] if kind.eq_ignore_ascii_case("books") => meta(domain, "book"),
            [domain, kind, ..] if kind.eq_ignore_ascii_case("papers") => meta(domain, "paper"),
            [domain, ..] => meta(domain, "unknown"),
        },
    }
}

/// Split a stored path (`corpus/...` or `notes/...`) into its root and the rest.
pub fn split_stored(path: &str) -> Option<(Root, &str)> {
    let (prefix, rel) = path.split_once('/')?;
    match prefix {
        "corpus" => Some((Root::Corpus, rel)),
        "notes" => Some((Root::Notes, rel)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(rel: &str) -> (String, &'static str) {
        let m = classify(Root::Corpus, rel);
        (m.domain, m.source_type)
    }

    fn n(rel: &str) -> (String, &'static str) {
        let m = classify(Root::Notes, rel);
        (m.domain, m.source_type)
    }

    #[test]
    fn books_and_papers_take_their_domain_folder() {
        assert_eq!(c("math/books/calculus.pdf"), ("math".into(), "book"));
        assert_eq!(c("math/books/calculus/vol1/ch1.pdf"), ("math".into(), "book"));
        assert_eq!(c("chemistry/papers/2024/nmr.pdf"), ("chemistry".into(), "paper"));
        assert_eq!(c("english/papers/x.epub"), ("english".into(), "paper"));
    }

    #[test]
    fn inbox_is_unsorted_and_unknown() {
        assert_eq!(c("inbox/scan.pdf"), ("unsorted".into(), "unknown"));
        assert_eq!(c("inbox/math/books/x.pdf"), ("unsorted".into(), "unknown"));
        assert_eq!(c("Inbox/x.pdf"), ("unsorted".into(), "unknown"));
    }

    #[test]
    fn other_corpus_locations_are_unknown() {
        assert_eq!(c("math/lectures/week1.pdf"), ("math".into(), "unknown"));
        assert_eq!(c("math/loose.pdf"), ("math".into(), "unknown"));
        assert_eq!(c("README.md"), ("unsorted".into(), "unknown"));
    }

    #[test]
    fn folder_names_are_case_insensitive_and_domains_lowercase() {
        assert_eq!(c("Math/Books/x.pdf"), ("math".into(), "book"));
        assert_eq!(c("Science/PAPERS/x.pdf"), ("science".into(), "paper"));
    }

    #[test]
    fn notes_are_notes_in_their_domain() {
        assert_eq!(n("chemistry/redox.md"), ("chemistry".into(), "note"));
        assert_eq!(n("math/deep/er/x.txt"), ("math".into(), "note"));
        assert_eq!(n("loose.md"), ("unsorted".into(), "note"));
    }

    #[test]
    fn stored_paths_split_into_root_and_rest() {
        assert_eq!(split_stored("corpus/math/books/x.pdf"), Some((Root::Corpus, "math/books/x.pdf")));
        assert_eq!(split_stored("notes/chemistry/x.md"), Some((Root::Notes, "chemistry/x.md")));
        assert_eq!(split_stored("other/x.md"), None);
        assert_eq!(split_stored("corpus"), None);
    }
}
