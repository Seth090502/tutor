//! Phase 1 extraction spike: measure xberg 1.3.0 (pure-Rust PDF backend) and pdfium-render 0.9.4 (vendored PDFium)
//! on real PDFs. Prints one JSON object per (file, tool) with counts only; extracted text is written to the
//! output directory for manual inspection and must never be committed.
//!
//! Usage: extract-spike <pdfium|xberg|both> <out_dir> <pdfium.dll> <file.pdf>...

use std::path::{Path, PathBuf};
use std::time::Instant;

use pdfium_render::prelude::*;
use serde_json::json;
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, PageConfig, extract};

fn norm_alnum(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn is_math_char(c: char) -> bool {
    matches!(c as u32,
        0x0370..=0x03FF | 0x2190..=0x22FF | 0x27C0..=0x27EF | 0x2980..=0x2AFF | 0x1D400..=0x1D7FF | 0x2070..=0x209F | 0x00B1 | 0x00D7 | 0x00F7 | 0x00B2 | 0x00B3 | 0x00B9 | 0x2032..=0x2034)
}

#[derive(Default, Clone, Copy)]
struct Counts {
    chars: usize,
    words: usize,
    math: usize,
    replacement: usize,
    pua: usize,
    cid: usize,
}

fn counts(text: &str) -> Counts {
    let mut c = Counts::default();
    for ch in text.chars().filter(|c| !c.is_whitespace()) {
        c.chars += 1;
        if is_math_char(ch) {
            c.math += 1;
        }
        match ch {
            '\u{FFFD}' => c.replacement += 1,
            '\u{E000}'..='\u{F8FF}' => c.pua += 1,
            _ => {}
        }
    }
    c.words = text.split_whitespace().count();
    c.cid = text.matches("(cid:").count();
    c
}

#[derive(Clone)]
struct Seg {
    l: f32,
    r: f32,
    t: f32,
    b: f32,
    text: String,
}

/// Reference reading order from segment geometry: full-width segments flush the column buffers; two-column pages
/// read the left column, then the right column, between full-width segments.
fn reference_lines(segs: &[Seg]) -> (Vec<String>, bool) {
    let segs: Vec<&Seg> = segs.iter().filter(|s| !s.text.trim().is_empty()).collect();
    if segs.is_empty() {
        return (Vec::new(), false);
    }
    let xmin = segs.iter().map(|s| s.l).fold(f32::MAX, f32::min);
    let xmax = segs.iter().map(|s| s.r).fold(f32::MIN, f32::max);
    let w = (xmax - xmin).max(1.0);
    let mid = (xmin + xmax) / 2.0;
    let class = |s: &Seg| -> u8 {
        if s.r <= mid + 0.03 * w {
            0
        } else if s.l >= mid - 0.03 * w {
            1
        } else {
            2
        }
    };
    let total: usize = segs.iter().map(|s| s.text.chars().count()).sum();
    let left: usize = segs.iter().filter(|s| class(s) == 0).map(|s| s.text.chars().count()).sum();
    let right: usize = segs.iter().filter(|s| class(s) == 1).map(|s| s.text.chars().count()).sum();
    let two_col = (left + right) as f32 >= 0.6 * total as f32 && left as f32 >= 0.2 * total as f32 && right as f32 >= 0.2 * total as f32;

    let mut sorted: Vec<&Seg> = segs.clone();
    // Top to bottom (PDF y grows upward), then left to right inside a line.
    sorted.sort_by(|a, b| {
        let ya = ((a.t + a.b) / 2.0 / 4.0).round();
        let yb = ((b.t + b.b) / 2.0 / 4.0).round();
        yb.partial_cmp(&ya).unwrap_or(std::cmp::Ordering::Equal).then(a.l.partial_cmp(&b.l).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut out: Vec<String> = Vec::new();
    let mut lcol: Vec<&Seg> = Vec::new();
    let mut rcol: Vec<&Seg> = Vec::new();
    let flush = |l: &mut Vec<&Seg>, r: &mut Vec<&Seg>, out: &mut Vec<String>| {
        for col in [&*l, &*r] {
            let mut line = String::new();
            let mut last_y = f32::MAX;
            for s in col.iter() {
                let y = ((s.t + s.b) / 2.0 / 4.0).round();
                if (y - last_y).abs() > 0.5 && !line.is_empty() {
                    out.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(s.text.trim());
                last_y = y;
            }
            if !line.is_empty() {
                out.push(line);
            }
        }
        l.clear();
        r.clear();
    };
    for s in sorted {
        let c = if two_col { class(s) } else { 2 };
        match c {
            0 => lcol.push(s),
            1 => rcol.push(s),
            _ => {
                flush(&mut lcol, &mut rcol, &mut out);
                lcol.push(s);
                flush(&mut lcol, &mut rcol, &mut out);
            }
        }
    }
    flush(&mut lcol, &mut rcol, &mut out);
    (out, two_col)
}

/// Anchors: the first five words of a reference line, alphanumerics only, at least 14 characters.
fn anchors(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| {
            let a = norm_alnum(&l.split_whitespace().take(5).collect::<Vec<_>>().join(" "));
            (a.chars().count() >= 14).then_some(a)
        })
        .collect()
}

/// (anchors, found uniquely, adjacent pairs, pairs in order)
fn order_score(page_text: &str, anchors: &[String]) -> (usize, usize, usize, usize) {
    let norm = norm_alnum(page_text);
    let mut positions: Vec<usize> = Vec::new();
    for a in anchors {
        let mut it = norm.match_indices(a.as_str());
        if let (Some((p, _)), None) = (it.next(), it.next()) {
            positions.push(p);
        }
    }
    let pairs = positions.len().saturating_sub(1);
    let ok = positions.windows(2).filter(|w| w[1] > w[0]).count();
    (anchors.len(), positions.len(), pairs, ok)
}

struct PageOut {
    label: Option<String>,
    text: String,
}

struct PdfiumRun {
    pages: Vec<PageOut>,
    anchors: Vec<Vec<String>>,
    two_col: Vec<bool>,
    text_secs: f64,
}

fn run_pdfium(pdfium: &Pdfium, bytes: &[u8]) -> Result<PdfiumRun, String> {
    let doc = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(|e| format!("open: {e:?}"))?;
    let mut pages = Vec::new();
    let mut anchor_sets = Vec::new();
    let mut two_col = Vec::new();
    let mut text_secs = 0.0;
    for page in doc.pages().iter() {
        let label = page.label().map(str::to_string);
        let t = Instant::now();
        let text = page.text().map_err(|e| format!("text: {e:?}"))?;
        let all = text.all();
        text_secs += t.elapsed().as_secs_f64();
        let segs: Vec<Seg> = text
            .segments()
            .iter()
            .map(|s| {
                let b = s.bounds();
                Seg { l: b.left().value, r: b.right().value, t: b.top().value, b: b.bottom().value, text: s.text() }
            })
            .collect();
        let (lines, tc) = reference_lines(&segs);
        two_col.push(tc);
        anchor_sets.push(anchors(&lines));
        pages.push(PageOut { label, text: all });
    }
    Ok(PdfiumRun { pages, anchors: anchor_sets, two_col, text_secs })
}

fn report(tool: &str, file: &Path, size: usize, pages: &[PageOut], secs: f64, ref_anchors: Option<&Vec<Vec<String>>>, two_col: Option<&Vec<bool>>, out_dir: &Path) {
    let all: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let c = counts(&all);
    let per_page: Vec<usize> = pages.iter().map(|p| counts(&p.text).chars).collect();
    let empty = per_page.iter().filter(|&&n| n < 20).count();
    let (mut a, mut f, mut pr, mut ok) = (0, 0, 0, 0);
    let scored = ref_anchors.filter(|r| r.len() == pages.len());
    let (mut tc_pages, mut tc_pairs, mut tc_ok) = (0, 0, 0);
    if let Some(refs) = scored {
        for (i, (p, anc)) in pages.iter().zip(refs).enumerate() {
            let (x, y, z, w) = order_score(&p.text, anc);
            a += x;
            f += y;
            pr += z;
            ok += w;
            if two_col.is_some_and(|t| t.get(i).copied().unwrap_or(false)) {
                tc_pages += 1;
                tc_pairs += z;
                tc_ok += w;
            }
        }
    }
    let labels: Vec<String> = pages.iter().map(|p| p.label.clone().unwrap_or_default()).collect();
    let non_numeric = labels.iter().enumerate().filter(|(i, l)| !l.is_empty() && **l != (i + 1).to_string()).count();
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("x");
    let _ = std::fs::create_dir_all(out_dir);
    let mut text_out = String::new();
    for (i, p) in pages.iter().enumerate() {
        text_out.push_str(&format!("\n\n===== page {} (label {:?}) =====\n\n", i + 1, p.label));
        text_out.push_str(&p.text);
    }
    let _ = std::fs::write(out_dir.join(format!("{stem}.{tool}.txt")), text_out);
    println!(
        "{}",
        json!({
            "tool": tool, "file": stem, "bytes": size, "ok": true, "pages": pages.len(), "secs": secs,
            "pages_per_s": if secs > 0.0 { pages.len() as f64 / secs } else { 0.0 },
            "chars": c.chars, "words": c.words, "chars_per_page": if pages.is_empty() { 0.0 } else { c.chars as f64 / pages.len() as f64 },
            "min_page_chars": per_page.iter().min().copied().unwrap_or(0), "empty_pages": empty,
            "math_chars": c.math, "replacement": c.replacement, "pua": c.pua, "cid": c.cid,
            "label_nonnumeric": non_numeric, "labels_first": labels.iter().take(14).collect::<Vec<_>>(),
            "scored": scored.is_some(), "anchors": a, "anchors_found": f, "order_pairs": pr, "order_ok": ok,
            "two_col_pages": tc_pages, "two_col_pairs": tc_pairs, "two_col_ok": tc_ok,
        })
    );
}

fn fail(tool: &str, file: &Path, size: usize, err: &str) {
    println!("{}", json!({"tool": tool, "file": file.file_stem().and_then(|s| s.to_str()), "bytes": size, "ok": false, "error": err}));
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: extract-spike <pdfium|xberg|both> <out_dir> <pdfium.dll> <file.pdf>...");
        std::process::exit(2);
    }
    let mode = args[1].as_str();
    let out_dir = PathBuf::from(&args[2]);
    let dll = &args[3];
    let files: Vec<PathBuf> = args[4..].iter().map(PathBuf::from).collect();

    let pdfium = if mode != "xberg" {
        match Pdfium::bind_to_library(dll) {
            Ok(b) => Some(Pdfium::new(b)),
            Err(e) => {
                eprintln!("pdfium bind failed: {e:?}");
                None
            }
        }
    } else {
        None
    };

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig { extract_pages: true, insert_page_markers: false, ..Default::default() }),
        use_cache: false,
        disable_ocr: true,
        ..Default::default()
    };

    for file in &files {
        let bytes = match std::fs::read(file) {
            Ok(b) => b,
            Err(e) => {
                fail("io", file, 0, &e.to_string());
                continue;
            }
        };
        let mut reference: Option<(Vec<Vec<String>>, Vec<bool>)> = None;
        if let (true, Some(pdfium)) = (mode == "pdfium" || mode == "both", pdfium.as_ref()) {
            let t = Instant::now();
            match run_pdfium(pdfium, &bytes) {
                Ok(run) => {
                    eprintln!("pdfium wall incl. geometry: {:.2}s", t.elapsed().as_secs_f64());
                    report("pdfium", file, bytes.len(), &run.pages, run.text_secs, Some(&run.anchors), Some(&run.two_col), &out_dir);
                    reference = Some((run.anchors, run.two_col));
                }
                Err(e) => fail("pdfium", file, bytes.len(), &e),
            }
        }
        if mode == "xberg" || mode == "both" {
            let name = file.file_name().and_then(|n| n.to_str()).map(str::to_string);
            let input = ExtractInput::from_bytes(bytes.clone(), "application/pdf", name);
            let t = Instant::now();
            let res = extract(input, &config).await;
            let secs = t.elapsed().as_secs_f64();
            match res {
                Ok(out) => match out.results.first() {
                    Some(doc) => {
                        let pages: Vec<PageOut> = match &doc.pages {
                            Some(ps) => ps.iter().map(|p| PageOut { label: Some(p.page_number.to_string()), text: p.content.clone() }).collect(),
                            None => vec![PageOut { label: None, text: doc.content.clone() }],
                        };
                        report("xberg", file, bytes.len(), &pages, secs, reference.as_ref().map(|r| &r.0), reference.as_ref().map(|r| &r.1), &out_dir);
                        eprintln!("xberg: counts.pages={} errors={} page_labels={:?}", doc.counts.pages, out.errors.len(), doc.metadata.additional.get("page_labels").map(|v| v.to_string().chars().take(120).collect::<String>()));
                    }
                    None => fail("xberg", file, bytes.len(), &format!("no results; errors: {:?}", out.errors.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>())),
                },
                Err(e) => fail("xberg", file, bytes.len(), &format!("{e}")),
            }
        }
    }
}
