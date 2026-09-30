mod bm25;
mod chunk;
mod dedup;
mod discover;
mod embed;
mod engine;
mod error;
mod extract;
mod headings;
mod ingest;
mod jobs;
mod library;
mod meta;
mod ocr;
mod pages;
mod report;
mod search;
mod server;
mod store;
mod survey;
mod text;
mod tokenize;
mod triage;
mod vector;

use crate::embed::Embedder;
use crate::engine::{Engine, Roots};
use crate::error::{Error, Result};
use clap::{Parser, Subcommand};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

/// Default PDFium location: the gitignored vendor folder of this crate.
const DEFAULT_PDFIUM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/pdfium/bin");

#[derive(Parser)]
#[command(name = "rag-engine", version, about = "Corpus tools and retrieval service for the DeepTutor RAG engine")]
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
    /// Run the HTTP service (survey and ingest jobs, status, search, library browsing). With
    /// RAG_ENGINE_TOKEN set, every route except GET /health needs `Authorization: Bearer <token>`.
    Serve {
        /// Library root: corpus/<domain>/books|papers/..., corpus/inbox/... (read-only).
        #[arg(long)]
        corpus: PathBuf,
        /// Notes root: notes/<domain>/... (read-only).
        #[arg(long)]
        notes: Option<PathBuf>,
        /// Directory for the database and indexes.
        #[arg(long)]
        data: PathBuf,
        /// Port; 0 picks a free one (the address is logged).
        #[arg(long, default_value_t = server::DEFAULT_PORT)]
        port: u16,
        /// Address to listen on. Anything but loopback needs RAG_ENGINE_TOKEN.
        #[arg(long, default_value = "127.0.0.1")]
        bind: IpAddr,
        /// PDFium dynamic library, or the directory that contains it.
        #[arg(long, env = "PDFIUM_DYNAMIC_LIB_PATH", default_value = DEFAULT_PDFIUM)]
        pdfium: PathBuf,
    },
    /// Ingest incrementally in the foreground and print a summary as JSON.
    Ingest {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        notes: Option<PathBuf>,
        #[arg(long)]
        data: PathBuf,
        /// Only walk this folder (inside the corpus or notes root).
        #[arg(long)]
        root: Option<PathBuf>,
        /// Clear the indexes and ingest everything again (finished OCR text is kept).
        #[arg(long)]
        rebuild: bool,
        #[arg(long, env = "PDFIUM_DYNAMIC_LIB_PATH", default_value = DEFAULT_PDFIUM)]
        pdfium: PathBuf,
    },
    /// Search the index and print the response as JSON (same shape as POST /search).
    Search {
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = search::DEFAULT_K)]
        k: usize,
        /// Restrict to a domain; repeat for several.
        #[arg(long = "domain")]
        domains: Vec<String>,
        /// Restrict to a source type (book, paper, note, unknown); repeat for several.
        #[arg(long = "source-type")]
        source_types: Vec<String>,
        /// hybrid, bm25 or vector.
        #[arg(long, default_value = "hybrid")]
        mode: String,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Survey { root, out, pdfium } => survey::run(&root, out.as_deref(), &pdfium).map_err(Error::from),
        Command::Serve { corpus, notes, data, port, bind, pdfium } => serve(Roots { corpus, notes }, &data, bind, port, pdfium),
        Command::Ingest { corpus, notes, data, root, rebuild, pdfium } => {
            run_ingest(Roots { corpus, notes }, &data, root.as_deref(), rebuild, &pdfium)
        }
        Command::Search { data, query, k, domains, source_types, mode } => {
            run_search(&data, query, k, domains, source_types, &mode)
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread().enable_all().build()?)
}

/// The engine with embeddings and OCR as configured by the environment.
fn open_engine(rt: &tokio::runtime::Runtime, data: &Path) -> Result<Engine> {
    let embedder = Embedder::from_env()?;
    let ocr = ocr::provider_from_env()?;
    let tuning = bm25::Tuning::from_env()?;
    match &embedder {
        Some(e) => eprintln!("rag-engine: embeddings: {} / {}", e.provider_name(), e.model()),
        None => eprintln!("rag-engine: embeddings: none (RAG_EMBED_PROVIDER unset); search runs BM25 only"),
    }
    rt.block_on(Engine::open(data, embedder, ocr, tuning))
}

fn serve(roots: Roots, data: &Path, bind: IpAddr, port: u16, pdfium: PathBuf) -> Result<()> {
    // Read here rather than through clap, so the value never appears in --help.
    let token = std::env::var("RAG_ENGINE_TOKEN").ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    server::check_bind(bind, token.as_deref())?;
    roots.check(data)?;
    let rt = runtime()?;
    let engine = Arc::new(open_engine(&rt, data)?);
    server::fail_interrupted_jobs(&engine)?;
    eprintln!(
        "rag-engine: corpus {}, notes {}, data {}",
        engine::display_path(&roots.corpus),
        roots.notes.as_deref().map_or_else(|| "none".to_string(), engine::display_path),
        engine::display_path(data)
    );
    let state =
        Arc::new(server::AppState { engine, roots, pdfium, jobs: jobs::Jobs::default(), rt: rt.handle().clone(), token });
    let result = rt.block_on(server::serve(state, bind, port));
    // Do not wait for a running job: the next ingest repairs a batch it left uncommitted.
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
    result
}

fn run_ingest(roots: Roots, data: &Path, root: Option<&Path>, rebuild: bool, pdfium: &Path) -> Result<()> {
    roots.check(data)?;
    let rt = runtime()?;
    let engine = open_engine(&rt, data)?;
    let report = ingest::ingest(&engine, &roots, pdfium, root, rebuild, rt.handle(), &jobs::Progress::default())?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn run_search(data: &Path, query: String, k: usize, domains: Vec<String>, source_types: Vec<String>, mode: &str) -> Result<()> {
    if !data.join(store::DB_FILE).is_file() {
        return Err(format!("{} has no index yet; run ingest first", data.display()).into());
    }
    let mode: search::Mode = serde_json::from_value(serde_json::Value::String(mode.to_string()))
        .map_err(|_| format!("--mode must be hybrid, bm25 or vector, got {mode:?}"))?;
    let rt = runtime()?;
    let engine = Arc::new(open_engine(&rt, data)?);
    let request = search::SearchRequest {
        query,
        k: Some(k),
        domains: (!domains.is_empty()).then_some(domains),
        source_types: (!source_types.is_empty()).then_some(source_types),
        mode: Some(mode),
    };
    let response = rt.block_on(search::search(engine, request)).map_err(|e| match e {
        search::SearchError::BadRequest(m)
        | search::SearchError::Unavailable(m)
        | search::SearchError::Provider(m)
        | search::SearchError::Internal(m) => Error(m),
    })?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
