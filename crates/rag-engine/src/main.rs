mod dedup;
mod discover;
mod extract;
mod report;
mod survey;
mod text;
mod triage;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// Default PDFium location: the gitignored vendor folder of this crate.
const DEFAULT_PDFIUM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/pdfium/bin");

#[derive(Parser)]
#[command(name = "rag-engine", version, about = "Corpus tools for the DeepTutor RAG engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Measure a document corpus: discovery, exact dedup, PDF text-layer triage and sizing.
    Survey {
        /// Directory to scan recursively.
        #[arg(long)]
        root: PathBuf,
        /// Write the JSON report to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// PDFium dynamic library, or the directory that contains it.
        #[arg(long, env = "PDFIUM_DYNAMIC_LIB_PATH", default_value = DEFAULT_PDFIUM)]
        pdfium: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Survey { root, out, pdfium } => match survey::run(&root, out.as_deref(), &pdfium) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}
