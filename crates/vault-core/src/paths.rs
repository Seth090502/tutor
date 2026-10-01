//! Relative-path validation and tier-root confinement (SPEC section 4).
//!
//! Every stored path is a [`RelPath`]: relative to a tier root, `/`-separated, and free of anything Windows
//! would treat specially. [`TierRoot`] joins one to its canonical root component by component, then checks the
//! canonical result is still under the root, so a junction or symlink inside the vault cannot redirect a read
//! or a write outside it. On Windows the canonical root carries the `\\?\` prefix, so every derived path is a
//! long path.

use crate::error::{CoreError, Result};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

const MAX_REL_LEN: usize = 1024;
const MAX_COMPONENT_LEN: usize = 255;
const FORBIDDEN: &[char] = &['<', '>', ':', '"', '|', '?', '*', '\\'];

/// A validated path relative to a tier root, for example `text/3f/3f9a1c07b2e4d55e-title.md`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelPath(String);

impl RelPath {
    pub fn new(s: &str) -> Result<Self> {
        let reject = |reason: &str| Err(CoreError::Path { path: s.to_string(), reason: reason.to_string() });
        if s.is_empty() {
            return reject("empty");
        }
        if s.len() > MAX_REL_LEN {
            return reject("longer than 1024 bytes");
        }
        if s.starts_with('/') {
            return reject("absolute");
        }
        for component in s.split('/') {
            if let Err(reason) = check_component(component) {
                return reject(&reason);
            }
        }
        Ok(RelPath(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// The path with one more validated component appended.
    pub fn join(&self, component: &str) -> Result<RelPath> {
        RelPath::new(&format!("{}/{component}", self.0))
    }

    fn split_last(&self) -> (Option<&str>, &str) {
        match self.0.rsplit_once('/') {
            Some((parent, name)) => (Some(parent), name),
            None => (None, &self.0),
        }
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn check_component(c: &str) -> std::result::Result<(), String> {
    if c.is_empty() {
        return Err("empty component (leading, trailing or doubled '/')".into());
    }
    if c == "." || c == ".." {
        return Err(format!("component {c:?} is not allowed"));
    }
    if c.chars().count() > MAX_COMPONENT_LEN {
        return Err("component longer than 255 characters".into());
    }
    if let Some(bad) = c.chars().find(|ch| FORBIDDEN.contains(ch) || ch.is_control()) {
        return Err(format!("character {bad:?} is not allowed"));
    }
    if c.ends_with('.') || c.ends_with(' ') {
        return Err("component ends with a dot or a space".into());
    }
    if is_reserved_windows_name(c) {
        return Err(format!("{c:?} is a reserved Windows device name"));
    }
    Ok(())
}

/// `CON`, `NUL`, `COM1`, `LPT¹`... with or without an extension, in any case.
fn is_reserved_windows_name(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component).trim_end_matches(' ').to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$") {
        return true;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(rest) = stem.strip_prefix(prefix) {
            let mut chars = rest.chars();
            if let (Some(d), None) = (chars.next(), chars.next())
                && (d.is_ascii_digit() || matches!(d, '\u{b9}' | '\u{b2}' | '\u{b3}'))
            {
                return true;
            }
        }
    }
    false
}

/// A tier root (hot, cold, inbox, or the vault root) in canonical form.
#[derive(Debug, Clone)]
pub struct TierRoot {
    name: &'static str,
    root: PathBuf,
}

impl TierRoot {
    /// Canonicalises `path`, which must be an existing directory.
    pub fn open(name: &'static str, path: &Path) -> Result<Self> {
        let root = std::fs::canonicalize(path).map_err(|e| CoreError::io("open tier root", path, e))?;
        if !root.is_dir() {
            return Err(CoreError::Config(format!("{name} root {} is not a directory", path.display())));
        }
        Ok(TierRoot { name, root })
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Lexical join, component by component (a `\\?\` path does not treat `/` as a separator).
    fn join(&self, rel: &str) -> PathBuf {
        let mut p = self.root.clone();
        for c in rel.split('/') {
            p.push(c);
        }
        p
    }

    fn confine(&self, canonical: PathBuf) -> Result<PathBuf> {
        if canonical.starts_with(&self.root) {
            Ok(canonical)
        } else {
            Err(CoreError::Escape { path: canonical, root: self.root.clone() })
        }
    }

    /// The canonical path of an existing file or directory, refused if it resolves outside the root.
    pub fn resolve_existing(&self, rel: &RelPath) -> Result<PathBuf> {
        let joined = self.join(rel.as_str());
        let canonical = std::fs::canonicalize(&joined).map_err(|e| CoreError::io("resolve", &joined, e))?;
        self.confine(canonical)
    }

    /// Like [`resolve_existing`](Self::resolve_existing), but `Ok(None)` when nothing exists at `rel`.
    pub fn resolve_if_exists(&self, rel: &RelPath) -> Result<Option<PathBuf>> {
        match self.resolve_existing(rel) {
            Ok(p) => Ok(Some(p)),
            Err(CoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Creates the parent directories of `rel` one at a time, checking confinement before each step so nothing
    /// is ever created through a link that leaves the root, and returns the path to write. The file itself may
    /// or may not exist; if it is a link, it is refused.
    pub fn prepare_write(&self, rel: &RelPath) -> Result<PathBuf> {
        let (parent, name) = rel.split_last();
        let mut dir = self.root.clone();
        if let Some(parent) = parent {
            for c in parent.split('/') {
                let next = dir.join(c);
                match std::fs::symlink_metadata(&next) {
                    Ok(meta) if meta.file_type().is_symlink() => {
                        let canonical = std::fs::canonicalize(&next).map_err(|e| CoreError::io("resolve", &next, e))?;
                        dir = self.confine(canonical)?;
                    }
                    Ok(_) => dir = next,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        match std::fs::create_dir(&next) {
                            Ok(()) => {}
                            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                            Err(e) => return Err(CoreError::io("create directory", &next, e)),
                        }
                        dir = next;
                    }
                    Err(e) => return Err(CoreError::io("inspect", &next, e)),
                }
            }
        }
        let canonical = std::fs::canonicalize(&dir).map_err(|e| CoreError::io("resolve", &dir, e))?;
        let dir = self.confine(canonical)?;
        let target = dir.join(name);
        if let Ok(meta) = std::fs::symlink_metadata(&target)
            && meta.file_type().is_symlink()
        {
            return Err(CoreError::Path { path: rel.to_string(), reason: "the target is a link".into() });
        }
        Ok(target)
    }

    /// The path of `rel` relative to this root, for a path already inside it (used by directory walks).
    pub fn relativize(&self, path: &Path) -> Option<String> {
        let rest = path.strip_prefix(&self.root).ok()?;
        let parts: Option<Vec<&str>> = rest.components().map(|c| c.as_os_str().to_str()).collect();
        Some(parts?.join("/"))
    }
}

/// True for `\\server\share\...`, including the verbatim `\\?\UNC\` form that canonicalising a mapped network
/// drive produces. SQLite must never run over a network share (SPEC section 6).
pub fn is_network_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return rest.len() >= 4 && rest[..4].eq_ignore_ascii_case(r"UNC\");
    }
    s.starts_with(r"\\") && !s.starts_with(r"\\.\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn accepts_store_paths() {
        for ok in [
            "text/3f/3f9a1c07b2e4d55e-cowan.md",
            "originals/ab/abcdef.pdf",
            "a",
            "manifests/text.jsonl",
            "x/.hidden",
            "con-tents/console.txt",
            "COM10",
        ] {
            assert!(RelPath::new(ok).is_ok(), "rejected {ok}");
        }
    }

    #[test]
    fn rejects_escapes_and_windows_specials() {
        for bad in [
            "",
            "/abs",
            "../x",
            "a/../b",
            "a/./b",
            "a//b",
            "a/",
            "C:/x",
            "C:",
            "a\\b",
            "a/b:stream",
            "NUL",
            "nul.txt",
            "a/CON",
            "a/com1.log",
            "LPT9",
            "COM\u{b9}",
            "CONIN$",
            "trailing.",
            "trailing ",
            "a/b?",
            "a/b*",
            "a\u{0}b",
            "tab\there",
        ] {
            assert!(RelPath::new(bad).is_err(), "accepted {bad:?}");
        }
        assert!(RelPath::new(&"a".repeat(256)).is_err());
        assert!(RelPath::new(&"a/".repeat(600)).is_err());
    }

    #[test]
    fn network_paths_are_detected() {
        assert!(is_network_path(Path::new(r"\\server\share\vault")));
        assert!(is_network_path(Path::new(r"\\?\UNC\server\share\vault")));
        assert!(!is_network_path(Path::new(r"\\?\C:\vault")));
        assert!(!is_network_path(Path::new(r"C:\vault")));
        assert!(!is_network_path(Path::new("/home/v")));
    }

    #[test]
    fn resolves_and_creates_inside_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = TierRoot::open("hot", dir.path()).unwrap();
        let rel = RelPath::new("text/ab/file.md").unwrap();
        let target = root.prepare_write(&rel).unwrap();
        assert!(target.starts_with(root.path()));
        std::fs::write(&target, b"x").unwrap();
        let resolved = root.resolve_existing(&rel).unwrap();
        assert_eq!(resolved, target);
        assert_eq!(root.relativize(&resolved).as_deref(), Some("text/ab/file.md"));
        assert!(root.resolve_if_exists(&RelPath::new("text/ab/none.md").unwrap()).unwrap().is_none());
    }

    #[test]
    fn long_paths_work() {
        let dir = tempfile::tempdir().unwrap();
        let root = TierRoot::open("hot", dir.path()).unwrap();
        let deep: Vec<String> = (0..12).map(|i| format!("{i:02}{}", "d".repeat(28))).collect();
        let rel = RelPath::new(&format!("{}/f.txt", deep.join("/"))).unwrap();
        let target = root.prepare_write(&rel).unwrap();
        assert!(target.as_os_str().len() > 300, "test path is not long: {}", target.display());
        std::fs::write(&target, b"long").unwrap();
        assert_eq!(std::fs::read(root.resolve_existing(&rel).unwrap()).unwrap(), b"long");
    }

    /// Creates a directory junction (Windows) or symlink (elsewhere) at `link` pointing to `target`.
    fn make_dir_link(link: &Path, target: &Path) -> bool {
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .stdout(std::process::Stdio::null())
                .status();
            matches!(status, Ok(s) if s.success())
        }
        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
    }

    #[test]
    fn links_cannot_escape() {
        let dir = tempfile::tempdir().unwrap();
        let root_dir = dir.path().join("root");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root_dir).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), b"outside").unwrap();
        assert!(make_dir_link(&root_dir.join("text"), &outside), "could not create a junction for the test");
        let root = TierRoot::open("hot", &root_dir).unwrap();

        let read = root.resolve_existing(&RelPath::new("text/secret.txt").unwrap());
        assert!(matches!(read, Err(CoreError::Escape { .. })), "{read:?}");

        let write = root.prepare_write(&RelPath::new("text/ab/new.md").unwrap());
        assert!(matches!(write, Err(CoreError::Escape { .. })), "{write:?}");
        assert!(!outside.join("ab").exists(), "a directory was created through the junction");

        let link_target = root.prepare_write(&RelPath::new("text").unwrap());
        assert!(link_target.is_err(), "writing onto a link must be refused");
    }

    fn arb_component() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-z0-9]{1,12}",
            Just("..".to_string()),
            Just(".".to_string()),
            Just("CON".to_string()),
            Just("c:".to_string()),
            Just("a ".to_string()),
            Just("b.".to_string()),
            Just("".to_string()),
            "[a-zA-Z0-9 ._:\\\\-]{1,6}",
        ]
    }

    proptest! {
        #[test]
        fn accepted_paths_stay_lexically_inside(parts in prop::collection::vec(arb_component(), 1..6)) {
            let s = parts.join("/");
            if let Ok(rel) = RelPath::new(&s) {
                let base = Path::new("base");
                let mut p = base.to_path_buf();
                for c in rel.components() {
                    prop_assert!(c != ".." && c != "." && !c.contains(':') && !c.contains('\\'));
                    p.push(c);
                }
                prop_assert!(p.components().all(|c| matches!(c, std::path::Component::Normal(_))));
                prop_assert!(p.starts_with(base));
            }
        }

        #[test]
        fn dot_dot_is_always_rejected(
            before in prop::collection::vec("[a-z]{1,5}", 0..3),
            after in prop::collection::vec("[a-z]{1,5}", 0..3),
        ) {
            let mut parts = before.clone();
            parts.push("..".to_string());
            parts.extend(after);
            prop_assert!(RelPath::new(&parts.join("/")).is_err());
        }

        #[test]
        fn plain_paths_are_accepted(parts in prop::collection::vec("[a-z0-9][a-z0-9_-]{0,10}", 1..5)) {
            let s = parts.join("/");
            prop_assert!(RelPath::new(&s).is_ok() || parts.iter().any(|p| is_reserved_windows_name(p)));
        }
    }
}
