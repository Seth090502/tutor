//! Development probe: convert a PDF with PDFium and print counts, headings and pages (no body text unless
//! `--text`), or the laid-out lines of one page with `--lines N`.
//!
//! cargo run --release --example pdf_probe -- <pdfium.dll> <file.pdf> [--text <out.md>] [--lines N]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: pdf_probe <pdfium.dll> <file.pdf> [--text out.md] [--lines N]");
        std::process::exit(2);
    }
    if let Err(e) = vault_extract::pdf::init(std::path::Path::new(&args[1])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    let bytes = std::fs::read(&args[2]).expect("read pdf");
    if let Some(i) = args.iter().position(|a| a == "--lines") {
        let n: usize = args[i + 1].parse().expect("page number");
        for l in vault_extract::pdf::layout_lines_debug(&bytes, n).expect("layout") {
            println!("{l}");
        }
        return;
    }
    let t = std::time::Instant::now();
    let r = vault_extract::pdf::convert(&bytes, &vault_extract::ConvertOptions::default());
    let secs = t.elapsed().as_secs_f64();
    match r {
        Ok(c) => {
            let ex = &c.extracted;
            let q = vault_extract::quality::assess(ex, vault_extract::quality::QualityContext { expect_paper_sections: true, source_pages: Some(c.signals.pages) });
            println!(
                "{}",
                serde_json::json!({
                    "pages": c.signals.pages, "secs": secs, "lines": ex.line_count(), "chars": q.chars, "words": q.words,
                    "headings": ex.headings.len(), "refs": ex.references.len(), "blocks": ex.blocks.len(),
                    "labels": ex.pages.iter().map(|p| p.label.clone().unwrap_or_default()).take(12).collect::<Vec<_>>(),
                    "route": c.classification.route, "kind": c.classification.kind, "math_ratio": c.classification.math_ratio,
                    "scanned_ratio": c.classification.scanned_page_ratio, "flags": c.classification.flags, "quality": q.score, "qflags": q.flags,
                    "fonts": c.signals.fonts.iter().take(25).collect::<Vec<_>>(), "title": ex.metadata.title, "arxiv": ex.metadata.arxiv_id, "doi": ex.metadata.doi,
                    "outline": ex.headings.iter().take(40).map(|h| format!("{}{}", "#".repeat(h.level as usize), h.text.chars().take(60).collect::<String>())).collect::<Vec<_>>(),
                })
            );
            if let Some(i) = args.iter().position(|a| a == "--text") {
                std::fs::write(&args[i + 1], &ex.markdown).expect("write text");
            }
        }
        Err(e) => println!("{}", serde_json::json!({"error": e.to_string(), "reason": e.reason_code()})),
    }
}
