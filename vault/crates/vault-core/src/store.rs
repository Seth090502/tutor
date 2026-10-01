//! Store operations (SPEC section 5): write-once files, atomic replacement, verified copies and moves of
//! originals, append-only integrity manifests, and `verify`.
//!
//! A write-once file goes to a temporary name in its final directory, is fsynced, and is then hard-linked to its
//! final name, which fails if the name exists, so a stored file can never be replaced, even by a racing writer.
//! Volumes without hard links (FAT, exFAT) fall back to an existence check plus rename.

use crate::config::Layout;
use crate::error::{CoreError, Result};
use crate::ids::hash_file;
use crate::paths::{RelPath, TierRoot};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const TEMP_SUFFIX: &str = ".vault-tmp";
pub const TEXT_MANIFEST: &str = "manifests/text.jsonl";
pub const ORIGINALS_MANIFEST: &str = "manifests/originals.jsonl";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Created,
    /// The identical file was already there (a re-run, or a duplicate delivery).
    AlreadyPresent,
}

fn temp_path(target: &Path) -> PathBuf {
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    target.with_file_name(format!(".{name}.{}-{n}{TEMP_SUFFIX}", std::process::id()))
}

/// Removes a temporary file on drop unless it was consumed.
struct TempFile {
    path: PathBuf,
    armed: bool,
}

impl TempFile {
    fn create(target: &Path) -> Result<(Self, File)> {
        let path = temp_path(target);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| CoreError::io("create temporary file", &path, e))?;
        Ok((TempFile { path, armed: true }, file))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.armed
            && let Err(e) = fs::remove_file(&self.path)
            && e.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(path = %self.path.display(), error = %e, "could not remove temporary file");
        }
    }
}

/// Gives `tmp` the name `target` unless `target` exists. Returns false (leaving `tmp` alone) if it does.
fn link_no_clobber(tmp: &Path, target: &Path) -> Result<bool> {
    match fs::hard_link(tmp, target) {
        Ok(()) => {
            if let Err(e) = fs::remove_file(tmp) {
                tracing::warn!(path = %tmp.display(), error = %e, "could not remove temporary name after linking");
            }
            sync_parent(target);
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => {
            tracing::debug!(error = %e, "hard link unavailable; falling back to check-and-rename");
            if fs::symlink_metadata(target).is_ok() {
                return Ok(false);
            }
            fs::rename(tmp, target).map_err(|e| CoreError::io("rename into place", target, e))?;
            sync_parent(target);
            Ok(true)
        }
    }
}

#[cfg(unix)]
fn sync_parent(target: &Path) {
    if let Some(dir) = target.parent() {
        if let Err(e) = File::open(dir).and_then(|d| d.sync_all()) {
            tracing::warn!(dir = %dir.display(), error = %e, "could not fsync directory");
        }
    }
}

#[cfg(not(unix))]
fn sync_parent(_target: &Path) {}

fn write_synced(file: &mut File, path: &Path, bytes: &[u8]) -> Result<()> {
    file.write_all(bytes).map_err(|e| CoreError::io("write", path, e))?;
    file.sync_all().map_err(|e| CoreError::io("fsync", path, e))
}

/// Writes `bytes` at `rel` once. An identical existing file is accepted; a different one is refused.
pub fn write_once(tier: &TierRoot, rel: &RelPath, bytes: &[u8]) -> Result<WriteOutcome> {
    let target = tier.prepare_write(rel)?;
    if let Some(outcome) = compare_existing(&target, bytes)? {
        return Ok(outcome);
    }
    let (tmp, mut file) = TempFile::create(&target)?;
    write_synced(&mut file, &tmp.path, bytes)?;
    drop(file);
    if link_no_clobber(&tmp.path, &target)? {
        let mut tmp = tmp;
        tmp.armed = false;
        return Ok(WriteOutcome::Created);
    }
    compare_existing(&target, bytes)?.ok_or_else(|| CoreError::WouldOverwrite { path: target.clone() })
}

fn compare_existing(target: &Path, bytes: &[u8]) -> Result<Option<WriteOutcome>> {
    match fs::read(target) {
        Ok(existing) if existing == bytes => Ok(Some(WriteOutcome::AlreadyPresent)),
        Ok(_) => Err(CoreError::WouldOverwrite { path: target.to_path_buf() }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::io("read existing", target, e)),
    }
}

/// Replaces the file at `rel` atomically (temporary file, fsync, rename). For mutable files only: sidecars,
/// reason files.
pub fn replace_atomic(tier: &TierRoot, rel: &RelPath, bytes: &[u8]) -> Result<()> {
    let target = tier.prepare_write(rel)?;
    let (tmp, mut file) = TempFile::create(&target)?;
    write_synced(&mut file, &tmp.path, bytes)?;
    drop(file);
    fs::rename(&tmp.path, &target).map_err(|e| CoreError::io("replace", &target, e))?;
    let mut tmp = tmp;
    tmp.armed = false;
    sync_parent(&target);
    Ok(())
}

/// Reads a stored file through confinement.
pub fn read(tier: &TierRoot, rel: &RelPath) -> Result<Vec<u8>> {
    let path = tier.resolve_existing(rel)?;
    fs::read(&path).map_err(|e| CoreError::io("read", &path, e))
}

/// A file placed in the cold tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub rel: RelPath,
    pub bytes: u64,
    pub outcome: WriteOutcome,
}

fn verify_existing(target: &Path, rel: &RelPath, expected: &str) -> Result<Placed> {
    let (actual, bytes) = hash_file(target)?;
    if actual != expected {
        return Err(CoreError::HashMismatch { path: target.to_path_buf(), expected: expected.into(), actual });
    }
    Ok(Placed { rel: rel.clone(), bytes, outcome: WriteOutcome::AlreadyPresent })
}

/// Copies `src` to `rel`: temporary name in the same directory, fsync, BLAKE3 check of what was read and of
/// what landed on disk, then a no-clobber rename. An existing file is verified and kept (a duplicate).
pub fn copy_verified(tier: &TierRoot, src: &Path, rel: &RelPath, expected_blake3: &str) -> Result<Placed> {
    let target = tier.prepare_write(rel)?;
    if fs::symlink_metadata(&target).is_ok() {
        return verify_existing(&target, rel, expected_blake3);
    }
    let (tmp, mut out) = TempFile::create(&target)?;
    let mut input = File::open(src).map_err(|e| CoreError::io("open", src, e))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = input.read(&mut buf).map_err(|e| CoreError::io("read", src, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| CoreError::io("write", &tmp.path, e))?;
    }
    out.sync_all().map_err(|e| CoreError::io("fsync", &tmp.path, e))?;
    drop(out);
    let read_hash = hasher.finalize().to_hex().to_string();
    if read_hash != expected_blake3 {
        return Err(CoreError::HashMismatch {
            path: src.to_path_buf(),
            expected: expected_blake3.into(),
            actual: read_hash,
        });
    }
    let (landed, bytes) = hash_file(&tmp.path)?;
    if landed != expected_blake3 {
        return Err(CoreError::HashMismatch {
            path: tmp.path.clone(),
            expected: expected_blake3.into(),
            actual: landed,
        });
    }
    if link_no_clobber(&tmp.path, &target)? {
        let mut tmp = tmp;
        tmp.armed = false;
        return Ok(Placed { rel: rel.clone(), bytes, outcome: WriteOutcome::Created });
    }
    verify_existing(&target, rel, expected_blake3)
}

/// Moves `src` to `rel` by renaming within the volume, verifying BLAKE3 before and after. If anything fails
/// after the rename, the file is renamed back. An existing file is verified and `src` is left in place.
pub fn move_verified(tier: &TierRoot, src: &Path, rel: &RelPath, expected_blake3: &str) -> Result<Placed> {
    let (before, _) = hash_file(src)?;
    if before != expected_blake3 {
        return Err(CoreError::HashMismatch {
            path: src.to_path_buf(),
            expected: expected_blake3.into(),
            actual: before,
        });
    }
    let target = tier.prepare_write(rel)?;
    if fs::symlink_metadata(&target).is_ok() {
        return verify_existing(&target, rel, expected_blake3);
    }
    let staged = temp_path(&target);
    fs::rename(src, &staged).map_err(|e| {
        if e.kind() == io::ErrorKind::CrossesDevices {
            CoreError::CrossVolumeMove { src: src.to_path_buf(), dest: target.clone() }
        } else {
            CoreError::io("move", src, e)
        }
    })?;
    let put_back = |why: CoreError| -> CoreError {
        if let Err(e) = fs::rename(&staged, src) {
            tracing::error!(staged = %staged.display(), src = %src.display(), error = %e, "could not move the original back");
        }
        why
    };
    let (after, bytes) = match hash_file(&staged) {
        Ok(v) => v,
        Err(e) => return Err(put_back(e)),
    };
    if after != expected_blake3 {
        return Err(put_back(CoreError::HashMismatch {
            path: staged.clone(),
            expected: expected_blake3.into(),
            actual: after,
        }));
    }
    match link_no_clobber(&staged, &target) {
        Ok(true) => Ok(Placed { rel: rel.clone(), bytes, outcome: WriteOutcome::Created }),
        Ok(false) => {
            let _ = put_back(CoreError::WouldOverwrite { path: target.clone() });
            verify_existing(&target, rel, expected_blake3)
        }
        Err(e) => Err(put_back(e)),
    }
}

// ---------------------------------------------------------------------------------------------- manifests

/// One line of an integrity manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub blake3: String,
    pub bytes: u64,
    pub at: String,
}

#[derive(Debug, Clone, Default)]
pub struct ManifestRead {
    pub entries: Vec<ManifestEntry>,
    /// 1-based numbers of lines that do not parse (a torn final line after a crash, or damage).
    pub bad_lines: Vec<usize>,
}

pub fn read_manifest(path: &Path) -> Result<ManifestRead> {
    let text = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(ManifestRead::default()),
        Err(e) => return Err(CoreError::io("read manifest", path, e)),
    };
    let mut read = ManifestRead::default();
    for (i, line) in text.split(|&b| b == b'\n').enumerate() {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match serde_json::from_slice::<ManifestEntry>(line) {
            Ok(e) => read.entries.push(e),
            Err(_) => read.bad_lines.push(i + 1),
        }
    }
    Ok(read)
}

/// An append-only manifest open for writing, with the paths it already lists.
pub struct Manifest {
    path: PathBuf,
    file: File,
    known: HashMap<String, (String, u64)>,
}

impl Manifest {
    pub fn open(hot: &TierRoot, rel: &str) -> Result<Self> {
        let rel = RelPath::new(rel)?;
        let path = hot.prepare_write(&rel)?;
        let read = read_manifest(&path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|e| CoreError::io("open manifest", &path, e))?;
        let len = file.metadata().map_err(|e| CoreError::io("inspect manifest", &path, e))?.len();
        if len > 0 && !ends_with_newline(&path)? {
            // Isolate a torn last line so the next entry starts cleanly.
            file.write_all(b"\n").map_err(|e| CoreError::io("append", &path, e))?;
        }
        let known = read.entries.into_iter().map(|e| (e.path, (e.blake3, e.bytes))).collect();
        Ok(Manifest { path, file, known })
    }

    pub fn contains(&self, path: &str) -> bool {
        self.known.contains_key(path)
    }

    /// Appends `entry` unless the path is already listed with the same hash. A different hash for a listed path
    /// is refused: stored files never change.
    pub fn ensure(&mut self, entry: ManifestEntry) -> Result<bool> {
        if let Some((blake3, bytes)) = self.known.get(&entry.path) {
            if *blake3 == entry.blake3 && *bytes == entry.bytes {
                return Ok(false);
            }
            return Err(CoreError::HashMismatch {
                path: PathBuf::from(&entry.path),
                expected: blake3.clone(),
                actual: entry.blake3,
            });
        }
        let mut line = serde_json::to_vec(&entry).map_err(|e| CoreError::json("manifest entry", e))?;
        line.push(b'\n');
        self.file.write_all(&line).map_err(|e| CoreError::io("append", &self.path, e))?;
        self.file.flush().map_err(|e| CoreError::io("flush", &self.path, e))?;
        self.known.insert(entry.path, (entry.blake3, entry.bytes));
        Ok(true)
    }

    /// fsync; intake calls this once per batch.
    pub fn sync(&self) -> Result<()> {
        self.file.sync_all().map_err(|e| CoreError::io("fsync", &self.path, e))
    }
}

fn ends_with_newline(path: &Path) -> Result<bool> {
    let bytes = fs::read(path).map_err(|e| CoreError::io("read manifest", path, e))?;
    Ok(bytes.last() == Some(&b'\n'))
}

// --------------------------------------------------------------------------------------------------- verify

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mismatch {
    pub path: String,
    pub expected: String,
    pub actual: String,
    pub expected_bytes: u64,
    pub actual_bytes: u64,
}

/// What `vault verify` found. Paths are `<tier>/<path>`, for example `hot/text/3f/....md`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    pub checked: u64,
    pub missing: Vec<String>,
    pub extra: Vec<String>,
    pub mismatched: Vec<Mismatch>,
    /// `<manifest>:<line>` for lines that do not parse.
    pub bad_manifest_lines: Vec<String>,
    /// Paths listed twice with different hashes.
    pub conflicting: Vec<String>,
    /// Leftovers of an interrupted write; harmless, reported so they can be cleared.
    pub temp_files: Vec<String>,
}

impl VerifyReport {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty()
            && self.extra.is_empty()
            && self.mismatched.is_empty()
            && self.bad_manifest_lines.is_empty()
            && self.conflicting.is_empty()
    }
}

/// Re-hashes every file under `hot/text` and `cold/originals` against the manifests.
pub fn verify(layout: &Layout) -> Result<VerifyReport> {
    let mut report = VerifyReport::default();
    verify_set(&layout.hot, TEXT_MANIFEST, &layout.hot, "text", &mut report)?;
    verify_set(&layout.hot, ORIGINALS_MANIFEST, &layout.cold, "originals", &mut report)?;
    if report.is_clean() {
        tracing::info!(checked = report.checked, "verify passed");
    } else {
        tracing::warn!(
            checked = report.checked,
            missing = report.missing.len(),
            extra = report.extra.len(),
            mismatched = report.mismatched.len(),
            "verify found problems"
        );
    }
    Ok(report)
}

fn verify_set(
    manifest_tier: &TierRoot,
    manifest_rel: &str,
    data: &TierRoot,
    dir: &str,
    r: &mut VerifyReport,
) -> Result<()> {
    let read = match manifest_tier.resolve_if_exists(&RelPath::new(manifest_rel)?)? {
        Some(path) => read_manifest(&path)?,
        None => ManifestRead::default(),
    };
    r.bad_manifest_lines.extend(read.bad_lines.iter().map(|n| format!("{}/{manifest_rel}:{n}", manifest_tier.name())));
    let mut expected: BTreeMap<String, (String, u64)> = BTreeMap::new();
    for e in read.entries {
        match expected.get(&e.path) {
            Some((b, n)) if *b != e.blake3 || *n != e.bytes => {
                r.conflicting.push(format!("{}/{}", data.name(), e.path))
            }
            Some(_) => {}
            None => {
                expected.insert(e.path, (e.blake3, e.bytes));
            }
        }
    }
    let mut on_disk = BTreeSet::new();
    walk(data, &data.path().join(dir), &mut on_disk, &mut r.temp_files)?;
    let present: Vec<(&String, &(String, u64))> = expected.iter().filter(|(p, _)| on_disk.contains(*p)).collect();
    let hashed: Vec<Result<Option<Mismatch>>> = present
        .par_iter()
        .map(|(rel, (blake3, bytes))| {
            let rel_path = RelPath::new(rel)?;
            let path = data.resolve_existing(&rel_path)?;
            let (actual, actual_bytes) = hash_file(&path)?;
            Ok((actual != *blake3 || actual_bytes != *bytes).then(|| Mismatch {
                path: format!("{}/{rel}", data.name()),
                expected: blake3.clone(),
                actual,
                expected_bytes: *bytes,
                actual_bytes,
            }))
        })
        .collect();
    for h in hashed {
        if let Some(m) = h? {
            r.mismatched.push(m);
        }
        r.checked += 1;
    }
    r.missing.extend(expected.keys().filter(|p| !on_disk.contains(*p)).map(|p| format!("{}/{p}", data.name())));
    r.extra.extend(on_disk.iter().filter(|p| !expected.contains_key(*p)).map(|p| format!("{}/{p}", data.name())));
    Ok(())
}

/// Collects tier-relative paths of every entry under `dir` without following links (a link is reported as a
/// file so it shows up as extra).
fn walk(tier: &TierRoot, dir: &Path, out: &mut BTreeSet<String>, temps: &mut Vec<String>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(CoreError::io("list", dir, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io("list", dir, e))?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|e| CoreError::io("inspect", &path, e))?;
        if ft.is_dir() && !ft.is_symlink() {
            walk(tier, &path, out, temps)?;
            continue;
        }
        let rel = tier.relativize(&path).unwrap_or_else(|| path.display().to_string());
        if rel.ends_with(TEMP_SUFFIX) {
            temps.push(format!("{}/{rel}", tier.name()));
        } else {
            out.insert(rel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::blake3_hex;

    fn tier(dir: &Path, name: &'static str) -> TierRoot {
        let p = dir.join(name);
        fs::create_dir_all(&p).unwrap();
        TierRoot::open(name, &p).unwrap()
    }

    fn rel(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    #[test]
    fn write_once_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let hot = tier(dir.path(), "hot");
        let r = rel("text/ab/x.md");
        assert_eq!(write_once(&hot, &r, b"one\n").unwrap(), WriteOutcome::Created);
        assert_eq!(write_once(&hot, &r, b"one\n").unwrap(), WriteOutcome::AlreadyPresent);
        assert!(matches!(write_once(&hot, &r, b"two\n"), Err(CoreError::WouldOverwrite { .. })));
        assert_eq!(read(&hot, &r).unwrap(), b"one\n");
        let leftovers: Vec<_> = fs::read_dir(hot.path().join("text").join("ab")).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "no temporary files remain");
    }

    #[test]
    fn replace_atomic_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let hot = tier(dir.path(), "hot");
        let r = rel("sidecars/ab/x.json");
        replace_atomic(&hot, &r, b"{}").unwrap();
        replace_atomic(&hot, &r, b"{\"a\":1}").unwrap();
        assert_eq!(read(&hot, &r).unwrap(), b"{\"a\":1}");
    }

    #[test]
    fn copies_verify_and_deduplicate() {
        let dir = tempfile::tempdir().unwrap();
        let cold = tier(dir.path(), "cold");
        let src = dir.path().join("paper.pdf");
        let data: Vec<u8> = (0..2_000_000u32).map(|i| (i % 253) as u8).collect();
        fs::write(&src, &data).unwrap();
        let b3 = blake3_hex(&data);
        let r = rel(&format!("originals/{}/{b3}.pdf", &b3[..2]));
        let placed = copy_verified(&cold, &src, &r, &b3).unwrap();
        assert_eq!((placed.outcome, placed.bytes), (WriteOutcome::Created, data.len() as u64));
        assert!(src.exists(), "copy leaves the source");
        assert_eq!(copy_verified(&cold, &src, &r, &b3).unwrap().outcome, WriteOutcome::AlreadyPresent);
        let wrong = blake3_hex(b"other");
        let r2 = rel(&format!("originals/{}/{wrong}.pdf", &wrong[..2]));
        assert!(matches!(copy_verified(&cold, &src, &r2, &wrong), Err(CoreError::HashMismatch { .. })));
        assert!(cold.resolve_if_exists(&r2).unwrap().is_none(), "nothing stored on a mismatch");
        // A damaged stored copy is detected rather than silently treated as a duplicate.
        fs::write(cold.resolve_existing(&r).unwrap(), b"damaged").unwrap();
        assert!(matches!(copy_verified(&cold, &src, &r, &b3), Err(CoreError::HashMismatch { .. })));
    }

    #[test]
    fn moves_verify_and_keep_duplicates_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let cold = tier(dir.path(), "cold");
        let src = dir.path().join("a.epub");
        fs::write(&src, b"epub bytes").unwrap();
        let b3 = blake3_hex(b"epub bytes");
        let r = rel(&format!("originals/{}/{b3}.epub", &b3[..2]));
        assert_eq!(move_verified(&cold, &src, &r, &b3).unwrap().outcome, WriteOutcome::Created);
        assert!(!src.exists());
        assert_eq!(read(&cold, &r).unwrap(), b"epub bytes");
        fs::write(&src, b"epub bytes").unwrap();
        assert_eq!(move_verified(&cold, &src, &r, &b3).unwrap().outcome, WriteOutcome::AlreadyPresent);
        assert!(src.exists(), "a duplicate stays where it was delivered");
        assert!(matches!(move_verified(&cold, &src, &r, &blake3_hex(b"x")), Err(CoreError::HashMismatch { .. })));
        assert!(src.exists());
    }

    #[test]
    fn manifests_append_once_and_survive_a_torn_line() {
        let dir = tempfile::tempdir().unwrap();
        let hot = tier(dir.path(), "hot");
        let entry = |p: &str, h: &str| ManifestEntry { path: p.into(), blake3: h.into(), bytes: 1, at: "t".into() };
        {
            let mut m = Manifest::open(&hot, TEXT_MANIFEST).unwrap();
            assert!(m.ensure(entry("text/a", "h1")).unwrap());
            assert!(!m.ensure(entry("text/a", "h1")).unwrap());
            assert!(matches!(m.ensure(entry("text/a", "h2")), Err(CoreError::HashMismatch { .. })));
            m.sync().unwrap();
        }
        let path = hot.path().join("manifests").join("text.jsonl");
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{\"path\":\"text/b\",\"bla").unwrap();
        drop(f);
        let mut m = Manifest::open(&hot, TEXT_MANIFEST).unwrap();
        assert!(m.contains("text/a") && !m.contains("text/b"));
        assert!(m.ensure(entry("text/c", "h3")).unwrap());
        let read = read_manifest(&path).unwrap();
        assert_eq!(read.entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(), vec!["text/a", "text/c"]);
        assert_eq!(read.bad_lines, vec![2]);
    }

    #[test]
    fn verify_reports_missing_extra_and_mismatched() {
        let dir = tempfile::tempdir().unwrap();
        for t in ["hot", "cold", "inbox"] {
            fs::create_dir_all(dir.path().join(t)).unwrap();
        }
        let layout = Layout {
            root: TierRoot::open("root", dir.path()).unwrap(),
            hot: TierRoot::open("hot", &dir.path().join("hot")).unwrap(),
            cold: TierRoot::open("cold", &dir.path().join("cold")).unwrap(),
            inbox: TierRoot::open("inbox", &dir.path().join("inbox")).unwrap(),
        };
        let mut text = Manifest::open(&layout.hot, TEXT_MANIFEST).unwrap();
        for (name, body) in [("a", "alpha\n"), ("b", "beta\n"), ("c", "gamma\n")] {
            let r = rel(&format!("text/aa/{name}.md"));
            write_once(&layout.hot, &r, body.as_bytes()).unwrap();
            text.ensure(ManifestEntry {
                path: r.to_string(),
                blake3: blake3_hex(body.as_bytes()),
                bytes: body.len() as u64,
                at: "t".into(),
            })
            .unwrap();
        }
        let src = dir.path().join("o.bin");
        fs::write(&src, b"orig").unwrap();
        let b3 = blake3_hex(b"orig");
        let orel = rel(&format!("originals/{}/{b3}.bin", &b3[..2]));
        copy_verified(&layout.cold, &src, &orel, &b3).unwrap();
        let mut originals = Manifest::open(&layout.hot, ORIGINALS_MANIFEST).unwrap();
        originals
            .ensure(ManifestEntry { path: orel.to_string(), blake3: b3.clone(), bytes: 4, at: "t".into() })
            .unwrap();

        let clean = verify(&layout).unwrap();
        assert!(clean.is_clean(), "{clean:?}");
        assert_eq!(clean.checked, 4);

        fs::remove_file(layout.hot.resolve_existing(&rel("text/aa/a.md")).unwrap()).unwrap();
        write_once(&layout.hot, &rel("text/aa/z.md"), b"stray\n").unwrap();
        let p = layout.hot.resolve_existing(&rel("text/aa/b.md")).unwrap();
        let mut bytes = fs::read(&p).unwrap();
        bytes[0] ^= 1;
        fs::write(&p, &bytes).unwrap();
        fs::write(layout.hot.path().join("text").join("aa").join(".x.md.1-1.vault-tmp"), b"").unwrap();

        let r = verify(&layout).unwrap();
        assert!(!r.is_clean());
        assert_eq!(r.missing, vec!["hot/text/aa/a.md"]);
        assert_eq!(r.extra, vec!["hot/text/aa/z.md"]);
        assert_eq!(r.mismatched.iter().map(|m| m.path.as_str()).collect::<Vec<_>>(), vec!["hot/text/aa/b.md"]);
        assert_eq!(r.temp_files.len(), 1);
    }
}
