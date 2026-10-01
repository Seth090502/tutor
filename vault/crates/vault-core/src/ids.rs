//! Document identity (SPEC section 2): `hash16` from the canonical bytes, slugs, and store file names.

use crate::error::{CoreError, Result};
use crate::paths::RelPath;
use std::path::Path;
use vault_api::address::SLUG_MAX;
use vault_api::{DocId, DocKind};

/// Full BLAKE3 of `bytes`, lowercase hex (64 characters).
pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// The first 16 hex characters of BLAKE3 over the canonical text bytes.
pub fn hash16(canonical: &[u8]) -> String {
    let mut h = blake3_hex(canonical);
    h.truncate(vault_api::address::HASH_LEN);
    h
}

/// BLAKE3 and length of a file, memory-mapped and multithreaded for large files.
pub fn hash_file(path: &Path) -> Result<(String, u64)> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_mmap_rayon(path).map_err(|e| CoreError::io("hash", path, e))?;
    Ok((hasher.finalize().to_hex().to_string(), hasher.count()))
}

/// Slug from a title: ASCII-folded, lowercased, non-alphanumeric runs to `-`, at most 48 characters cut at a
/// hyphen, `untitled` when nothing is left.
pub fn slug(title: &str) -> String {
    let folded = slugify(title);
    if folded.is_empty() { "untitled".to_string() } else { cut_at_hyphen(&folded, SLUG_MAX) }
}

/// Slug for a code file: `<repo>-<path tail>`, keeping as much of the end of the path as fits.
pub fn code_slug(repo: &str, path: &str) -> String {
    let repo = cut_at_hyphen(&slugify(repo), SLUG_MAX / 2);
    let path = slugify(path);
    let joined = match (repo.is_empty(), path.is_empty()) {
        (true, true) => return "untitled".to_string(),
        (true, false) => return tail_at_hyphen(&path, SLUG_MAX),
        (false, true) => return repo,
        (false, false) => format!("{repo}-{path}"),
    };
    if joined.len() <= SLUG_MAX {
        return joined;
    }
    let room = SLUG_MAX - repo.len() - 1;
    let tail = tail_at_hyphen(&path, room);
    if tail.is_empty() { repo } else { format!("{repo}-{tail}") }
}

fn slugify(s: &str) -> String {
    let ascii = deunicode::deunicode(s).to_ascii_lowercase();
    let mut out = String::with_capacity(ascii.len());
    for ch in ascii.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Longest prefix of a slug (ASCII, single hyphens) of at most `max` bytes that ends at a word boundary; a
/// single word longer than `max` is cut hard.
fn cut_at_hyphen(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if s.as_bytes()[max] == b'-' {
        return s[..max].to_string();
    }
    match s[..max].rfind('-') {
        Some(i) if i > 0 => s[..i].to_string(),
        _ => s[..max].to_string(),
    }
}

/// Longest suffix of at most `max` bytes that starts at a word boundary; a single word longer than `max` is
/// cut hard from the end.
fn tail_at_hyphen(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let start = s.len() - max;
    if start > 0 && s.as_bytes()[start - 1] == b'-' {
        return s[start..].to_string();
    }
    match s[start..].find('-') {
        Some(i) if start + i + 1 < s.len() => s[start + i + 1..].to_string(),
        _ => s[start..].trim_start_matches('-').to_string(),
    }
}

/// The full document id: `hash16-slug`.
pub fn doc_id(hash16: &str, slug: &str) -> Result<DocId> {
    DocId::new(hash16, Some(slug)).map_err(|e| CoreError::InvalidArgument(e.to_string()))
}

/// Extension for a stored code file: the original's, lowercased, when it is 1-8 ASCII letters and digits;
/// otherwise `txt`.
pub fn code_ext(original_path: &str) -> String {
    let name = original_path.rsplit(['/', '\\']).next().unwrap_or(original_path);
    match name.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty() && (1..=8).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric()) =>
        {
            ext.to_ascii_lowercase()
        }
        _ => "txt".to_string(),
    }
}

/// Extension for a stored original or quarantined file: the same allowlist as code, else `bin`.
pub fn original_ext(ext: &str) -> String {
    let e = ext.trim_start_matches('.');
    if (1..=8).contains(&e.len()) && e.bytes().all(|b| b.is_ascii_alphanumeric()) {
        e.to_ascii_lowercase()
    } else {
        "bin".to_string()
    }
}

/// `text/{h2}/{doc_id}.md` for prose, `text/{h2}/{doc_id}.{ext}` for code (relative to the hot root).
pub fn text_rel_path(doc: &DocId, kind: DocKind, code_path: Option<&str>) -> Result<RelPath> {
    let ext = match (kind, code_path) {
        (DocKind::Code, Some(p)) => code_ext(p),
        (DocKind::Code, None) => "txt".to_string(),
        _ => "md".to_string(),
    };
    RelPath::new(&format!("text/{}/{doc}.{ext}", doc.shard()))
}

/// `sidecars/{h2}/{doc_id}.json` (relative to the hot root).
pub fn sidecar_rel_path(doc: &DocId) -> Result<RelPath> {
    RelPath::new(&format!("sidecars/{}/{doc}.json", doc.shard()))
}

/// `originals/{h2}/{blake3}.{ext}` (relative to the cold root).
pub fn original_rel_path(blake3: &str, ext: &str) -> Result<RelPath> {
    check_blake3(blake3)?;
    RelPath::new(&format!("originals/{}/{blake3}.{}", &blake3[..2], original_ext(ext)))
}

/// `quarantine/{h2}/{blake3}.{ext}` and its `{blake3}.reason.json` (relative to the cold root).
pub fn quarantine_rel_paths(blake3: &str, ext: &str) -> Result<(RelPath, RelPath)> {
    check_blake3(blake3)?;
    let dir = format!("quarantine/{}", &blake3[..2]);
    Ok((
        RelPath::new(&format!("{dir}/{blake3}.{}", original_ext(ext)))?,
        RelPath::new(&format!("{dir}/{blake3}.reason.json"))?,
    ))
}

pub(crate) fn check_blake3(h: &str) -> Result<()> {
    if h.len() == 64 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        Ok(())
    } else {
        Err(CoreError::InvalidArgument(format!("not a lowercase BLAKE3 hex digest: {h:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn hash16_is_the_blake3_prefix() {
        assert_eq!(blake3_hex(b""), "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");
        assert_eq!(hash16(b""), "af1349b9f5f9a1a6");
        assert_eq!(hash16(b"abc\n").len(), 16);
    }

    #[test]
    fn hash_file_matches_in_memory_hash() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.bin");
        let data: Vec<u8> = (0..300_000u32).map(|i| (i * 7 % 251) as u8).collect();
        std::fs::write(&p, &data).unwrap();
        assert_eq!(hash_file(&p).unwrap(), (blake3_hex(&data), data.len() as u64));
    }

    #[test]
    fn slugs() {
        assert_eq!(
            slug("The Magical Number Four in Short-Term Memory"),
            "the-magical-number-four-in-short-term-memory"
        );
        assert_eq!(slug("Ærøskøbing: Ünïcödé — façade"), "aeroskobing-unicode-facade");
        assert_eq!(slug("  --  "), "untitled");
        assert_eq!(slug(""), "untitled");
        assert_eq!(slug("認知心理学"), "ren-zhi-xin-li-xue");
        let long = slug("A very long title that keeps going well beyond the forty eight character limit");
        assert!(long.len() <= 48 && !long.ends_with('-'), "{long}");
        assert_eq!(long, "a-very-long-title-that-keeps-going-well-beyond");
        assert_eq!(slug(&"x".repeat(60)), "x".repeat(48));
    }

    #[test]
    fn code_slugs_keep_the_path_tail() {
        assert_eq!(code_slug("ripgrep", "crates/core/main.rs"), "ripgrep-crates-core-main-rs");
        let s = code_slug("bevy", "crates/bevy_render/src/render_resource/pipeline_cache/specializer.rs");
        assert!(s.starts_with("bevy-") && s.ends_with("specializer-rs") && s.len() <= 48, "{s}");
        assert_eq!(code_slug("", "a.py"), "a-py");
        assert_eq!(code_slug("repo", ""), "repo");
        assert!(DocId::new("0123456789abcdef", Some(&code_slug(&"r".repeat(40), &"p".repeat(80)))).is_ok());
    }

    #[test]
    fn file_names() {
        let id = doc_id("3f9a1c07b2e4d55e", "cowan").unwrap();
        assert_eq!(text_rel_path(&id, DocKind::Paper, None).unwrap().as_str(), "text/3f/3f9a1c07b2e4d55e-cowan.md");
        assert_eq!(
            text_rel_path(&id, DocKind::Code, Some("src/Main.RS")).unwrap().as_str(),
            "text/3f/3f9a1c07b2e4d55e-cowan.rs"
        );
        assert_eq!(
            text_rel_path(&id, DocKind::Code, Some("Makefile")).unwrap().as_str(),
            "text/3f/3f9a1c07b2e4d55e-cowan.txt"
        );
        assert_eq!(
            text_rel_path(&id, DocKind::Code, Some("x.tar_gz")).unwrap().as_str(),
            "text/3f/3f9a1c07b2e4d55e-cowan.txt"
        );
        assert_eq!(
            text_rel_path(&id, DocKind::Code, Some(".bashrc")).unwrap().as_str(),
            "text/3f/3f9a1c07b2e4d55e-cowan.txt"
        );
        assert_eq!(code_ext("a.verylongext"), "txt");
        assert_eq!(sidecar_rel_path(&id).unwrap().as_str(), "sidecars/3f/3f9a1c07b2e4d55e-cowan.json");
        let b3 = blake3_hex(b"x");
        assert_eq!(original_rel_path(&b3, "PDF").unwrap().as_str(), format!("originals/{}/{b3}.pdf", &b3[..2]));
        assert_eq!(original_rel_path(&b3, "a/b").unwrap().as_str(), format!("originals/{}/{b3}.bin", &b3[..2]));
        assert!(original_rel_path("nothex", "pdf").is_err());
        let (q, r) = quarantine_rel_paths(&b3, "exe").unwrap();
        assert!(q.as_str().ends_with(".exe") && r.as_str().ends_with(".reason.json"));
    }

    proptest! {
        #[test]
        fn every_slug_fits_the_address_grammar(title in "\\PC{0,120}", repo in "\\PC{0,40}", path in "\\PC{0,120}") {
            let s = slug(&title);
            prop_assert!(DocId::new("0123456789abcdef", Some(&s)).is_ok(), "bad slug {:?}", s);
            let c = code_slug(&repo, &path);
            prop_assert!(DocId::new("0123456789abcdef", Some(&c)).is_ok(), "bad code slug {:?}", c);
        }
    }
}
