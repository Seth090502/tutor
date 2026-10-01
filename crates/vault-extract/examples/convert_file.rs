//! Detect and convert one file; print the detected format, the outcome and the structure, and the Markdown.
//!
//! cargo run --release --example convert_file -- <file> [--pdfium <pdfium.dll>] [--md]

use vault_extract::{ConvertOptions, Outcome, archive::Limits};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: convert_file <file> [--pdfium <dll>] [--md]");
        std::process::exit(2);
    };
    #[cfg(feature = "pdfium")]
    if let Some(i) = args.iter().position(|a| a == "--pdfium")
        && let Err(e) = vault_extract::pdf::init(std::path::Path::new(&args[i + 1])) {
            eprintln!("{e}");
        }
    let bytes = std::fs::read(path).expect("read file");
    let name = std::path::Path::new(path).file_name().and_then(|n| n.to_str());
    let started = std::time::Instant::now();
    match vault_extract::extract(&bytes, name, &ConvertOptions::default(), &Limits::default()) {
        Ok(e) => {
            println!("format: {} ({})", e.detected.format.name(), e.detected.format.mime());
            match e.outcome {
                Outcome::Converted(c) => {
                    let x = &c.extracted;
                    println!("path: {}  tool: {}", c.path, x.tool);
                    println!("lines: {}  headings: {}  pages: {}  blocks: {}  refs: {}  warnings: {:?}", x.line_count(), x.headings.len(), x.pages.len(), x.blocks.len(), x.references.len(), x.warnings);
                    println!("quality: {:.2} {:?}", c.quality.score, c.quality.flags);
                    println!("metadata: {}", serde_json::to_string(&x.metadata).unwrap_or_default());
                    if args.iter().any(|a| a == "--md") {
                        println!("--- markdown ---\n{}", x.markdown);
                    }
                }
                Outcome::Members(u) => {
                    println!("members: {}", u.members.len());
                    for m in u.members.iter().take(20) {
                        println!("  {} ({}, {} bytes)", m.path, m.format.name(), m.bytes.len());
                    }
                }
                Outcome::WaitingGpu { reason, classification, .. } => println!("waiting_gpu: {reason:?} ({classification:?})"),
                Outcome::WaitingTool { tool, format } => println!("waiting_tool: {tool} for {}", format.name()),
            }
        }
        Err(e) => println!("error [{}]: {e}", e.reason_code()),
    }
    eprintln!("{:.1} ms", started.elapsed().as_secs_f64() * 1000.0);
}
