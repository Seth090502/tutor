//! `survey`: discover, hash, dedup, measure each unique document once, and report.

use crate::dedup;
use crate::discover::{self, DocFile, DocKind};
use crate::extract::{self, Measurement};
use crate::report::{self, DuplicateGroup, FileRecord, Report, Totals, TriageThresholds};
use crate::text::TextSize;
use crate::triage;
use pdfium_render::prelude::{Pdfium, PdfiumError};
use std::path::Path;

/// A progress line goes to stderr after every this many files.
const PROGRESS_EVERY: usize = 25;

pub fn run(root: &Path, out: Option<&Path>, pdfium_lib: &Path) -> Result<(), String> {
    let report = build(root, pdfium_lib, |_, _| {})?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| format!("cannot encode report: {e}"))?;
    match out {
        Some(path) => {
            std::fs::write(path, json + "\n")
                .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            eprintln!("survey: report written to {}", path.display());
        }
        None => println!("{json}"),
    }
    Ok(())
}

/// The survey report for `root`. `on_measured(done, total)` follows the measuring of unique
/// documents.
pub fn build(root: &Path, pdfium_lib: &Path, on_measured: impl Fn(usize, usize)) -> Result<Report, String> {
    if !root.is_dir() {
        return Err(format!("--root {} is not a directory", root.display()));
    }
    let found = discover::discover(root);
    let docs = &found.docs;
    eprintln!(
        "survey: {} documents, {} other files, {} walk errors under {}",
        docs.len(),
        found.other.values().map(|t| t.files).sum::<u64>(),
        found.walk_errors.len(),
        root.display()
    );

    // PDFium is bound before any work so a missing library fails fast.
    let pdfium = if docs.iter().any(|d| d.kind == DocKind::Pdf) {
        Some(bind_pdfium(pdfium_lib)?)
    } else {
        None
    };

    let paths: Vec<&Path> = docs.iter().map(|d| d.path.as_path()).collect();
    let hashes = dedup::hash_all(&paths, |done| progress("hashed", done, docs.len(), None));
    let hash_refs: Vec<Option<&str>> = hashes.iter().map(|h| h.as_deref().ok()).collect();
    let groups = dedup::duplicate_groups(&hash_refs);
    let mut canonical: Vec<Option<usize>> = vec![None; docs.len()];
    for group in &groups {
        for &copy in &group[1..] {
            canonical[copy] = Some(group[0]);
        }
    }

    // PDFium is not thread-safe, so documents are measured one at a time.
    let unique: Vec<usize> = (0..docs.len()).filter(|&i| canonical[i].is_none()).collect();
    let mut results: Vec<Option<Result<Measurement, String>>> = vec![None; docs.len()];
    on_measured(0, unique.len());
    for (n, &i) in unique.iter().enumerate() {
        results[i] = Some(match &hashes[i] {
            Err(e) => Err(e.clone()),
            Ok(_) => measure(&docs[i], pdfium.as_ref()),
        });
        progress("measured", n + 1, unique.len(), Some(&docs[i].rel_path));
        on_measured(n + 1, unique.len());
    }
    let result_of = |i: usize| -> &Result<Measurement, String> {
        results[canonical[i].unwrap_or(i)].as_ref().expect("every unique document is measured")
    };

    let mut totals = Totals {
        other_by_extension: found.other.clone(),
        duplicate_groups: groups.len() as u64,
        duplicate_files: groups.iter().map(|g| g.len() as u64 - 1).sum(),
        duplicate_bytes: dedup::duplicate_bytes(
            &groups,
            &docs.iter().map(|d| d.bytes).collect::<Vec<_>>(),
        ),
        errors: found.walk_errors.len() as u64,
        ..Totals::default()
    };
    for doc in docs {
        totals.files += 1;
        totals.bytes += doc.bytes;
        totals.by_type.entry(doc.kind).or_default().add(doc.bytes);
    }
    let mut size = TextSize::default();
    let mut measured_bytes = 0;
    for &i in &unique {
        totals.unique_files += 1;
        totals.unique_bytes += docs[i].bytes;
        match result_of(i) {
            Ok(m) => {
                totals.pages += m.pages.unwrap_or(0);
                if let Some(counts) = &m.triage {
                    totals.pages_by_class.merge(counts);
                }
                size += m.size;
                measured_bytes += docs[i].bytes;
            }
            Err(_) => {
                totals.errors += 1;
                totals.error_bytes += docs[i].bytes;
            }
        }
    }
    totals.characters = size.characters;
    totals.whitespace_tokens = size.whitespace_tokens;
    totals.tokens_est = size.tokens_est();

    let files = docs
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            let (m, error) = match result_of(i) {
                Ok(m) => (Some(m), None),
                Err(e) => (None, Some(e.clone())),
            };
            FileRecord {
                path: doc.rel_path.clone(),
                kind: doc.kind,
                bytes: doc.bytes,
                blake3: hashes[i].as_ref().ok().cloned(),
                duplicate_of: canonical[i].map(|c| docs[c].rel_path.clone()),
                pages: m.and_then(|m| m.pages),
                triage: m.and_then(|m| m.triage),
                characters: m.map(|m| m.size.characters),
                whitespace_tokens: m.map(|m| m.size.whitespace_tokens),
                tokens_est: m.map(|m| m.size.tokens_est()),
                error,
            }
        })
        .collect();

    Ok(Report {
        root: root.display().to_string(),
        triage_thresholds: TriageThresholds {
            min_text_chars_per_page: triage::MIN_TEXT_CHARS_PER_PAGE,
            math_char_ratio_min: triage::MATH_CHAR_RATIO_MIN,
            short_token_share_min: triage::SHORT_TOKEN_SHARE_MIN,
        },
        extrapolation: report::extrapolate(measured_bytes, totals.tokens_est, totals.pages),
        totals,
        duplicate_groups: groups
            .iter()
            .map(|g| DuplicateGroup {
                blake3: hash_refs[g[0]].unwrap_or_default().to_string(),
                bytes_each: docs[g[0]].bytes,
                paths: g.iter().map(|&i| docs[i].rel_path.clone()).collect(),
            })
            .collect(),
        walk_errors: found.walk_errors,
        files,
    })
}

/// PDFium from `lib` (the library file or its directory). The bindings are process-wide, so
/// once bound they are reused and `lib` is not loaded again.
pub fn bind_pdfium(lib: &Path) -> Result<Pdfium, String> {
    let file = if lib.is_dir() {
        Pdfium::pdfium_platform_library_name_at_path(lib)
    } else {
        lib.to_path_buf()
    };
    match Pdfium::bind_to_library(&file) {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(Pdfium::default()),
        Err(e) => Err(format!(
            "cannot load PDFium from {}: {e}. Put the bblanchon/pdfium-binaries build in \
             vendor/pdfium/ (see README), pass --pdfium <path>, or set PDFIUM_DYNAMIC_LIB_PATH",
            file.display()
        )),
    }
}

/// Measure one document. A panic inside a parser becomes this file's error so the run continues.
fn measure(doc: &DocFile, pdfium: Option<&Pdfium>) -> Result<Measurement, String> {
    extract::catch_panics(|| match doc.kind {
        DocKind::Pdf => extract::measure_pdf(
            pdfium.expect("PDFium is bound whenever PDFs were discovered"),
            &doc.path,
        ),
        DocKind::Epub => extract::measure_epub(&doc.path),
        DocKind::Md | DocKind::Txt => extract::measure_plain(&doc.path),
    })
}

fn progress(stage: &str, done: usize, total: usize, last: Option<&str>) {
    if done.is_multiple_of(PROGRESS_EVERY) || done == total {
        match last {
            Some(path) => eprintln!("{stage} {done}/{total} (last: {path})"),
            None => eprintln!("{stage} {done}/{total}"),
        }
    }
}
