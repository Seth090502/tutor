//! ZIP, TAR and gzip unpacking into in-memory members, with hard limits against archive bombs and path escapes.
//!
//! Nothing is written to disk: members are returned as byte vectors named by their sanitised, `/`-separated path.
//! The caller processes members one by one and may call [`unpack_with_budget`] again for a nested archive, sharing
//! one [`Budget`] so that nesting cannot multiply the limits.

use std::cell::Cell;
use std::io::{Cursor, Read};
use std::rc::Rc;

use flate2::read::GzDecoder;

use crate::detect::{Format, detect};
use crate::error::{ExtractError, Result};

/// Limits applied to one top-level archive and everything nested inside it.
#[derive(Debug, Clone)]
pub struct Limits {
    /// Total uncompressed bytes (default 500 MiB).
    pub max_total_bytes: u64,
    /// Largest permitted uncompressed-to-compressed ratio (default 100).
    pub max_ratio: u64,
    /// The ratio is only enforced above this many uncompressed bytes (default 1 MiB), so a tiny archive of
    /// repetitive text is not mistaken for a bomb.
    pub ratio_floor_bytes: u64,
    /// Entries, directories included (default 10,000).
    pub max_entries: usize,
    /// Nesting depth the caller may unpack to (default 4); [`unpack_with_budget`] refuses deeper.
    pub max_depth: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_total_bytes: 500 * 1024 * 1024, max_ratio: 100, ratio_floor_bytes: 1024 * 1024, max_entries: 10_000, max_depth: 4 }
    }
}

/// What a tree of nested archives has used so far.
#[derive(Debug, Clone, Default)]
pub struct Budget {
    pub bytes: u64,
    pub entries: usize,
    pub depth: u32,
}

/// One file taken out of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Sanitised path inside the archive: `/` separators, no `.` or `..` components, never absolute.
    pub path: String,
    pub bytes: Vec<u8>,
    /// Format of the member's content, so the caller can route it (or unpack it in turn).
    pub format: Format,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct Unpacked {
    pub members: Vec<Member>,
    /// Entries that were not unpacked: links, devices, encrypted entries.
    pub skipped: Vec<Skipped>,
}

/// Unpack `bytes`, which `detect` reported as `format` (ZIP, TAR, gzip, gzipped tar or arXiv source).
pub fn unpack(bytes: &[u8], format: Format, limits: &Limits) -> Result<Unpacked> {
    unpack_with_budget(bytes, format, limits, &mut Budget::default())
}

pub fn unpack_with_budget(bytes: &[u8], format: Format, limits: &Limits, budget: &mut Budget) -> Result<Unpacked> {
    if budget.depth >= limits.max_depth {
        return Err(ExtractError::Limits(format!("archives nested deeper than {}", limits.max_depth)));
    }
    budget.depth += 1;
    let result = match format {
        Format::Zip => unpack_zip(bytes, limits, budget),
        Format::Tar => unpack_tar(Cursor::new(bytes), bytes.len() as u64, limits, budget),
        Format::TarGz | Format::Gzip | Format::ArxivSource => unpack_gzip(bytes, limits, budget),
        other => Err(ExtractError::Unsupported(format!("{} is not an unpackable archive", other.name()))),
    };
    budget.depth -= 1;
    result
}

/// Normalise an archive path; `None` when it would escape the archive root.
pub fn sanitize_path(raw: &str) -> Option<String> {
    if raw.is_empty() || raw.contains('\0') || raw.chars().any(|c| c.is_control()) {
        return None;
    }
    let p = raw.replace('\\', "/");
    // Absolute, UNC and drive-letter paths.
    if p.starts_with('/') {
        return None;
    }
    let bytes = p.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for c in p.split('/') {
        match c {
            "" | "." => {}
            ".." => return None,
            c => parts.push(c),
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

/// The most the archive may still expand to: the lesser of the byte limit and the ratio limit.
fn allowance(archive_len: u64, limits: &Limits, budget: &Budget) -> u64 {
    let by_ratio = archive_len.saturating_mul(limits.max_ratio).max(limits.ratio_floor_bytes);
    let by_total = limits.max_total_bytes.saturating_sub(budget.bytes);
    by_ratio.min(by_total)
}

fn limit_error(limits: &Limits, archive_len: u64, budget: &Budget) -> ExtractError {
    let by_ratio = archive_len.saturating_mul(limits.max_ratio).max(limits.ratio_floor_bytes);
    if by_ratio < limits.max_total_bytes.saturating_sub(budget.bytes) {
        ExtractError::Limits(format!("expansion ratio above {}:1", limits.max_ratio))
    } else {
        ExtractError::Limits(format!("more than {} bytes uncompressed", limits.max_total_bytes))
    }
}

fn count_entry(limits: &Limits, budget: &mut Budget) -> Result<()> {
    budget.entries += 1;
    if budget.entries > limits.max_entries {
        return Err(ExtractError::Limits(format!("more than {} entries", limits.max_entries)));
    }
    Ok(())
}

/// Read one member fully, never more than `remaining` bytes.
fn read_capped<R: Read>(r: R, remaining: u64) -> std::io::Result<Option<Vec<u8>>> {
    let mut buf = Vec::new();
    let n = r.take(remaining.saturating_add(1)).read_to_end(&mut buf)?;
    Ok(if n as u64 > remaining { None } else { Some(buf) })
}

fn unpack_zip(bytes: &[u8], limits: &Limits, budget: &mut Budget) -> Result<Unpacked> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| ExtractError::corrupt("zip", e.to_string()))?;
    if zip.len() + budget.entries > limits.max_entries {
        return Err(ExtractError::Limits(format!("more than {} entries", limits.max_entries)));
    }
    let archive_len = bytes.len() as u64;
    let mut out = Unpacked::default();
    let mut used = 0u64;
    for i in 0..zip.len() {
        count_entry(limits, budget)?;
        let listed_name = zip.name_for_index(i).unwrap_or("?").to_string();
        let mut file = match zip.by_index(i) {
            Ok(f) => f,
            Err(zip::result::ZipError::InvalidPassword) => {
                tracing::warn!(path = %listed_name, "skipped an encrypted entry");
                out.skipped.push(Skipped { path: listed_name, reason: "encrypted" });
                continue;
            }
            Err(zip::result::ZipError::UnsupportedArchive(m)) if m == zip::result::ZipError::PASSWORD_REQUIRED => {
                out.skipped.push(Skipped { path: listed_name, reason: "encrypted" });
                continue;
            }
            Err(e) => return Err(ExtractError::corrupt("zip", e.to_string())),
        };
        let raw_name = file.name().to_string();
        if file.is_dir() {
            if sanitize_path(raw_name.trim_end_matches('/')).is_none() && !raw_name.trim_matches('/').is_empty() {
                return Err(ExtractError::Limits(format!("path escapes the archive: {raw_name:?}")));
            }
            continue;
        }
        let Some(path) = sanitize_path(&raw_name) else {
            return Err(ExtractError::Limits(format!("path escapes the archive: {raw_name:?}")));
        };
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            tracing::warn!(path = %path, "skipped a symlink entry");
            out.skipped.push(Skipped { path, reason: "symlink" });
            continue;
        }
        let remaining = allowance(archive_len, limits, budget).saturating_sub(used);
        let Some(content) = read_capped(&mut file, remaining).map_err(|e| ExtractError::corrupt("zip", format!("{path}: {e}")))? else {
            return Err(limit_error(limits, archive_len, budget));
        };
        used += content.len() as u64;
        let format = detect(&content, Some(&path)).format;
        out.members.push(Member { path, bytes: content, format });
    }
    budget.bytes += used;
    Ok(out)
}

fn unpack_gzip(bytes: &[u8], limits: &Limits, budget: &mut Budget) -> Result<Unpacked> {
    let archive_len = bytes.len() as u64;
    let allowed = allowance(archive_len, limits, budget);
    let mut dec = GzDecoder::new(bytes);
    // Peek at the start to see whether a tarball is inside.
    let mut head = Vec::new();
    (&mut dec).take(512).read_to_end(&mut head).map_err(|e| ExtractError::corrupt("gzip", e.to_string()))?;
    let name = dec.header().and_then(|h| h.filename()).map(|n| String::from_utf8_lossy(n).into_owned());
    if head.len() >= 512 && is_tar(&head) {
        // Bound everything the tar reader pulls through the decoder, including data of entries it skips.
        let tripped = Rc::new(Cell::new(false));
        let capped = CapReader { inner: Cursor::new(head).chain(dec), left: allowed.saturating_add(1), tripped: Rc::clone(&tripped) };
        return match unpack_tar(capped, archive_len, limits, budget) {
            Err(_) if tripped.get() => Err(limit_error(limits, archive_len, budget)),
            other => other,
        };
    }
    // A single gzipped file.
    let mut content = head;
    let room = allowed.saturating_sub(content.len() as u64);
    let more = read_capped(&mut dec, room).map_err(|e| ExtractError::corrupt("gzip", e.to_string()))?;
    let Some(more) = more else { return Err(limit_error(limits, archive_len, budget)) };
    content.extend(more);
    budget.bytes += content.len() as u64;
    budget.entries += 1;
    let path = name.and_then(|n| sanitize_path(&n)).unwrap_or_else(|| "content".to_string());
    let format = detect(&content, Some(&path)).format;
    // Gzipped LaTeX is reported by its member name so the caller sees a .tex file.
    let path = if format == Format::Latex && !path.to_ascii_lowercase().ends_with(".tex") { format!("{path}.tex") } else { path };
    Ok(Unpacked { members: vec![Member { path, bytes: content, format }], skipped: Vec::new() })
}

/// Fails with an I/O error once more than `left` bytes have been read, and remembers that it did.
struct CapReader<R> {
    inner: R,
    left: u64,
    tripped: Rc<Cell<bool>>,
}

impl<R: Read> Read for CapReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.left == 0 {
            self.tripped.set(true);
            return Err(std::io::Error::other("archive expansion limit"));
        }
        let max = buf.len().min(self.left.min(usize::MAX as u64) as usize);
        let n = self.inner.read(&mut buf[..max])?;
        self.left -= n as u64;
        Ok(n)
    }
}

fn is_tar(h: &[u8]) -> bool {
    h.len() >= 512 && (&h[257..262] == b"ustar" || detect(h, None).format == Format::Tar || tar_checksum_ok(h))
}

fn tar_checksum_ok(h: &[u8]) -> bool {
    if h[0] == 0 {
        return false;
    }
    let digits: String = h[148..156].iter().take_while(|c| (b'0'..=b'7').contains(c)).map(|&c| c as char).collect();
    let Ok(stored) = u32::from_str_radix(&digits, 8) else { return false };
    let sum: u32 = h.iter().enumerate().map(|(i, &c)| if (148..156).contains(&i) { 32u32 } else { c as u32 }).sum();
    sum == stored
}

fn unpack_tar<R: Read>(reader: R, archive_len: u64, limits: &Limits, budget: &mut Budget) -> Result<Unpacked> {
    let allowed_total = allowance(archive_len, limits, budget);
    let mut ar = tar::Archive::new(reader);
    let mut out = Unpacked::default();
    let mut used = 0u64;
    let entries = ar.entries().map_err(|e| ExtractError::corrupt("tar", e.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| ExtractError::corrupt("tar", e.to_string()))?;
        count_entry(limits, budget)?;
        let raw_path = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            // Symlinks, hard links, devices, FIFOs and extension headers are never followed or unpacked.
            if !(kind.is_pax_global_extensions() || kind.is_pax_local_extensions() || kind.is_gnu_longname() || kind.is_gnu_longlink()) {
                out.skipped.push(Skipped { path: raw_path, reason: if kind.is_symlink() || kind.is_hard_link() { "link" } else { "special" } });
            }
            continue;
        }
        let Some(path) = sanitize_path(&raw_path) else {
            return Err(ExtractError::Limits(format!("path escapes the archive: {raw_path:?}")));
        };
        let remaining = allowed_total.saturating_sub(used);
        let Some(content) = read_capped(&mut entry, remaining).map_err(|e| ExtractError::corrupt("tar", format!("{path}: {e}")))? else {
            return Err(limit_error(limits, archive_len, budget));
        };
        used += content.len() as u64;
        let format = detect(&content, Some(&path)).format;
        out.members.push(Member { path, bytes: content, format });
    }
    budget.bytes += used;
    Ok(out)
}

/// Random access to the entries of a ZIP-based document (EPUB, DOCX, ODT), under the same expansion limits as
/// [`unpack`]: the total bytes handed out across all reads are capped by the ratio and size limits.
pub(crate) struct ZipDoc<'a> {
    zip: zip::ZipArchive<Cursor<&'a [u8]>>,
    format: &'static str,
    allowance: u64,
    used: u64,
    limits: Limits,
    archive_len: u64,
}

impl<'a> ZipDoc<'a> {
    pub fn open(bytes: &'a [u8], format: &'static str) -> Result<Self> {
        let limits = Limits::default();
        let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| ExtractError::corrupt(format, e.to_string()))?;
        if zip.len() > limits.max_entries {
            return Err(ExtractError::Limits(format!("more than {} entries", limits.max_entries)));
        }
        let archive_len = bytes.len() as u64;
        let allowance = allowance(archive_len, &limits, &Budget::default());
        Ok(Self { zip, format, allowance, used: 0, limits, archive_len })
    }

    pub fn has(&self, name: &str) -> bool {
        self.zip.file_names().any(|n| n == name)
    }

    /// The bytes of an entry, `None` when it does not exist (or the name only differs in case).
    pub fn read(&mut self, name: &str) -> Result<Option<Vec<u8>>> {
        let real = if self.has(name) {
            name.to_string()
        } else if let Some(n) = self.zip.file_names().find(|n| n.eq_ignore_ascii_case(name)) {
            n.to_string()
        } else {
            return Ok(None);
        };
        let format = self.format;
        let mut f = match self.zip.by_name(&real) {
            Ok(f) => f,
            Err(zip::result::ZipError::InvalidPassword) => return Err(ExtractError::Protected(format!("{format} entry {real} is encrypted"))),
            Err(zip::result::ZipError::UnsupportedArchive(m)) if m == zip::result::ZipError::PASSWORD_REQUIRED => {
                return Err(ExtractError::Protected(format!("{format} entry {real} is encrypted")));
            }
            Err(e) => return Err(ExtractError::corrupt(format, format!("{real}: {e}"))),
        };
        let remaining = self.allowance.saturating_sub(self.used);
        let Some(content) = read_capped(&mut f, remaining).map_err(|e| ExtractError::corrupt(format, format!("{real}: {e}")))? else {
            return Err(limit_error(&self.limits, self.archive_len, &Budget::default()));
        };
        self.used += content.len() as u64;
        Ok(Some(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_with(entries: &[(&str, &[u8])], deflate: bool) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let method = if deflate { zip::CompressionMethod::Deflated } else { zip::CompressionMethod::Stored };
        let opts = zip::write::SimpleFileOptions::default().compression_method(method);
        for (n, c) in entries {
            w.start_file(*n, opts).unwrap();
            w.write_all(c).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn tar_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for (n, c) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(c.len() as u64);
            h.set_mode(0o644);
            // Write the name raw so that hostile paths survive the builder's own checks.
            let name = h.as_old_mut();
            name.name = [0; 100];
            name.name[..n.len()].copy_from_slice(n.as_bytes());
            h.set_cksum();
            b.append(&h, *c).unwrap();
        }
        b.into_inner().unwrap()
    }

    fn gz(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn small() -> Limits {
        Limits { max_total_bytes: 8 * 1024 * 1024, max_ratio: 100, ratio_floor_bytes: 64 * 1024, max_entries: 50, max_depth: 3 }
    }

    #[test]
    fn zip_members_are_returned_with_detected_formats() {
        let z = zip_with(&[("a/readme.md", b"# Title\n\nSome text with a [link](http://a.b).\n"), ("b.txt", b"plain words here\n"), ("dir/", b"")], true);
        let u = unpack(&z, Format::Zip, &Limits::default()).unwrap();
        let got: Vec<_> = u.members.iter().map(|m| (m.path.as_str(), m.format)).collect();
        assert_eq!(got, vec![("a/readme.md", Format::Markdown), ("b.txt", Format::PlainText)]);
        assert_eq!(u.members[1].bytes, b"plain words here\n");
    }

    #[test]
    fn tar_and_tar_gz_members() {
        let t = tar_with(&[("src/main.tex", b"\\documentclass{article}\n\\begin{document}x\\end{document}\n"), ("notes.txt", b"hello\n")]);
        let u = unpack(&t, Format::Tar, &Limits::default()).unwrap();
        assert_eq!(u.members.len(), 2);
        assert_eq!(u.members[0].format, Format::Latex);
        let u = unpack(&gz(&t), Format::TarGz, &Limits::default()).unwrap();
        assert_eq!(u.members.iter().map(|m| m.path.as_str()).collect::<Vec<_>>(), vec!["src/main.tex", "notes.txt"]);
        let u = unpack(&gz(&t), Format::ArxivSource, &Limits::default()).unwrap();
        assert_eq!(u.members.len(), 2);
    }

    #[test]
    fn single_gzip_file_and_gzipped_tex() {
        let u = unpack(&gz(b"# Title\n\nbody with [a link](http://x.y)\n"), Format::Gzip, &Limits::default()).unwrap();
        assert_eq!(u.members.len(), 1);
        assert_eq!(u.members[0].format, Format::Markdown);
        assert_eq!(u.members[0].path, "content");
        let u = unpack(&gz(b"\\documentclass{article}\n\\begin{document}Hi\\end{document}\n"), Format::ArxivSource, &Limits::default()).unwrap();
        assert_eq!(u.members[0].path, "content.tex");
        assert_eq!(u.members[0].format, Format::Latex);
    }

    #[test]
    fn path_escapes_refuse_the_whole_archive() {
        for bad in ["../evil.txt", "a/../../evil.txt", "/etc/passwd", "C:\\Windows\\x.dll", "C:/x", "a\\..\\..\\b"] {
            let z = zip_with(&[("ok.txt", b"fine"), (bad, b"x")], false);
            let err = unpack(&z, Format::Zip, &Limits::default()).unwrap_err();
            assert!(matches!(err, ExtractError::Limits(ref m) if m.contains("escapes")), "{bad}: {err}");
            assert_eq!(err.reason_code(), "archive_limits");
        }
        let t = tar_with(&[("../../evil", b"x")]);
        assert!(matches!(unpack(&t, Format::Tar, &Limits::default()), Err(ExtractError::Limits(_))));
    }

    #[test]
    fn sanitize_path_rules() {
        assert_eq!(sanitize_path("./a//b/./c.txt").as_deref(), Some("a/b/c.txt"));
        assert_eq!(sanitize_path("a\\b\\c").as_deref(), Some("a/b/c"));
        for bad in ["", "..", "a/..", "/abs", "\\\\server\\share\\f", "D:file", "a\0b", "a\nb"] {
            assert_eq!(sanitize_path(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn compression_ratio_bomb_is_refused() {
        // 20 MiB of zeros deflates to roughly 20 KiB: ratio about 1000:1.
        let bomb = zip_with(&[("zeros.bin", &vec![0u8; 20 * 1024 * 1024])], true);
        assert!(bomb.len() < 100 * 1024, "{}", bomb.len());
        let err = unpack(&bomb, Format::Zip, &small()).unwrap_err();
        assert!(matches!(err, ExtractError::Limits(ref m) if m.contains("ratio")), "{err}");
        let err = unpack(&gz(&vec![0u8; 20 * 1024 * 1024]), Format::Gzip, &small()).unwrap_err();
        assert!(matches!(err, ExtractError::Limits(_)), "{err}");
    }

    #[test]
    fn total_size_limit_applies_even_when_the_ratio_is_fine() {
        let limits = Limits { max_total_bytes: 1000, max_ratio: 1_000_000, ratio_floor_bytes: 0, ..small() };
        let z = zip_with(&[("a", &[7u8; 600]), ("b", &[9u8; 600])], false);
        let err = unpack(&z, Format::Zip, &limits).unwrap_err();
        assert!(matches!(err, ExtractError::Limits(ref m) if m.contains("1000 bytes")), "{err}");
    }

    #[test]
    fn small_repetitive_archives_are_not_bombs() {
        let z = zip_with(&[("log.txt", "same line\n".repeat(5000).as_bytes())], true);
        assert!(z.len() < 500);
        assert!(unpack(&z, Format::Zip, &Limits::default()).is_ok());
    }

    #[test]
    fn entry_count_limit() {
        let names: Vec<String> = (0..60).map(|i| format!("f{i}.txt")).collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
        let z = zip_with(&entries, false);
        let err = unpack(&z, Format::Zip, &small()).unwrap_err();
        assert!(matches!(err, ExtractError::Limits(ref m) if m.contains("entries")), "{err}");
        let t = tar_with(&entries);
        assert!(matches!(unpack(&t, Format::Tar, &small()), Err(ExtractError::Limits(_))));
    }

    #[test]
    fn nesting_depth_and_shared_budget() {
        let inner = zip_with(&[("deep.txt", b"deep content here\n")], false);
        let mid = zip_with(&[("inner.zip", &inner)], false);
        let outer = zip_with(&[("mid.zip", &mid)], false);
        let limits = small();
        let mut budget = Budget::default();
        let u = unpack_with_budget(&outer, Format::Zip, &limits, &mut budget).unwrap();
        assert_eq!(u.members[0].format, Format::Zip);
        let u2 = unpack_with_budget(&u.members[0].bytes, Format::Zip, &limits, &mut budget).unwrap();
        let u3 = unpack_with_budget(&u2.members[0].bytes, Format::Zip, &limits, &mut budget).unwrap();
        assert_eq!(u3.members[0].path, "deep.txt");
        // The shared budget has counted every layer.
        assert!(budget.entries >= 3 && budget.bytes > 0);
        // A fourth level is refused when the cap is three.
        let shallow = Limits { max_depth: 1, ..limits };
        let mut b = Budget { depth: 1, ..Budget::default() };
        assert!(matches!(unpack_with_budget(&inner, Format::Zip, &shallow, &mut b), Err(ExtractError::Limits(_))));
    }

    #[test]
    fn tar_links_and_devices_are_skipped_not_followed() {
        let mut b = tar::Builder::new(Vec::new());
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Symlink);
        h.set_size(0);
        h.set_cksum();
        b.append_link(&mut h, "link", "/etc/passwd").unwrap();
        let mut f = tar::Header::new_gnu();
        f.set_size(2);
        f.set_mode(0o644);
        f.set_cksum();
        b.append_data(&mut f, "real.txt", &b"hi"[..]).unwrap();
        let t = b.into_inner().unwrap();
        let u = unpack(&t, Format::Tar, &Limits::default()).unwrap();
        assert_eq!(u.members.len(), 1);
        assert_eq!(u.skipped, vec![Skipped { path: "link".into(), reason: "link" }]);
    }

    #[test]
    fn encrypted_entries_are_skipped_and_reported() {
        use zip::unstable::write::FileOptionsExt;
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let plain = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        w.start_file("open.txt", plain).unwrap();
        w.write_all(b"visible").unwrap();
        w.start_file("secret.txt", plain.with_deprecated_encryption(b"pw").unwrap()).unwrap();
        w.write_all(b"hidden").unwrap();
        let z = w.finish().unwrap().into_inner();
        assert!(detect(&z, None).encrypted);
        let u = unpack(&z, Format::Zip, &Limits::default()).unwrap();
        assert_eq!(u.members.iter().map(|m| m.path.as_str()).collect::<Vec<_>>(), vec!["open.txt"]);
        assert_eq!(u.skipped, vec![Skipped { path: "secret.txt".into(), reason: "encrypted" }]);
    }

    #[test]
    fn corrupt_and_unsupported_inputs() {
        assert!(matches!(unpack(b"PK\x03\x04garbage", Format::Zip, &Limits::default()), Err(ExtractError::Corrupt { .. })));
        assert!(matches!(unpack(b"x", Format::Pdf, &Limits::default()), Err(ExtractError::Unsupported(_))));
        assert!(matches!(unpack(&[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3], Format::Gzip, &Limits::default()), Err(ExtractError::Corrupt { .. })));
    }
}
