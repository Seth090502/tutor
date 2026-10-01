//! Smoke test over real documents that are not in the repository. Ignored by default; run with
//!
//! `VAULT_PDFIUM_DLL=<pdfium.dll> VAULT_SAMPLE_PDFS=<dir> cargo test -p vault-extract --test real_samples -- --ignored --nocapture`
//!
//! Every PDF under the directory must convert without error, produce text, keep a quality score of at least 0.8,
//! and have structure marks that point at the lines they claim. Nothing is written and no text is printed.

#![cfg(feature = "pdfium")]

use std::path::{Path, PathBuf};

use vault_extract::archive::Limits;
use vault_extract::{ConvertOptions, Outcome, extract};

fn pdfs_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            pdfs_under(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")) {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "needs PDFium (VAULT_PDFIUM_DLL) and a directory of sample PDFs (VAULT_SAMPLE_PDFS)"]
fn sample_pdfs_convert_cleanly() {
    let Some(dll) = std::env::var_os("VAULT_PDFIUM_DLL") else {
        eprintln!("SKIPPED: VAULT_PDFIUM_DLL is not set");
        return;
    };
    vault_extract::pdf::init(Path::new(&dll)).expect("load PDFium");
    let dir = std::env::var_os("VAULT_SAMPLE_PDFS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("D:/Corpus-pilot2"));
    let mut files = Vec::new();
    pdfs_under(&dir, &mut files);
    assert!(!files.is_empty(), "no PDFs under {}", dir.display());

    let mut pages = 0usize;
    for f in &files {
        let bytes = std::fs::read(f).expect("read sample");
        let name = f.file_name().and_then(|n| n.to_str());
        let e = extract(&bytes, name, &ConvertOptions::default(), &Limits::default()).unwrap_or_else(|err| panic!("{}: {err}", f.display()));
        let converted = match &e.outcome {
            Outcome::Converted(c) => c.as_ref(),
            Outcome::WaitingGpu { native: Some(c), .. } => c.as_ref(),
            other => panic!("{}: unexpected outcome {other:?}", f.display()),
        };
        let x = &converted.extracted;
        assert!(x.markdown.trim().len() > 200, "{}: almost no text", f.display());
        assert!(converted.quality.score >= 0.8, "{}: quality {:.2} {:?}", f.display(), converted.quality.score, converted.quality.flags);
        let lines: Vec<&str> = x.markdown.lines().collect();
        for h in &x.headings {
            assert!(lines[h.line as usize - 1].trim_start_matches('#').trim() == h.text, "{}: heading line {}", f.display(), h.line);
        }
        for b in &x.blocks {
            assert!(b.start >= 1 && b.end as usize <= lines.len() && b.start <= b.end, "{}: block {b:?}", f.display());
        }
        for w in x.pages.windows(2) {
            assert!(w[0].index < w[1].index && w[0].line <= w[1].line, "{}: page marks out of order", f.display());
        }
        pages += x.pages.len();
    }
    eprintln!("{} PDFs, {} pages converted", files.len(), pages);
}
