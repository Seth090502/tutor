//! model-bench: Stage 1 embedder/reranker comparison through llama-server.
//!
//! Subcommands: `embed` (quality + throughput), `rerank` (sanity test + quality + throughput),
//! `parity` (the same chunks through two servers, e.g. CUDA vs Vulkan). Every run appends one JSON
//! line to --results carrying the full configuration. Exit codes: 0 ok, 2 server/infrastructure
//! failure, 3 reranker sanity test failed (a result line is still written).

mod data;
mod embed;
mod report;
mod rerank;
mod server;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};
use server::{Role, Server, ServerSpec};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Embed(EmbedArgs),
    Rerank(RerankArgs),
    Parity(ParityArgs),
}

#[derive(Args, Clone)]
struct Common {
    #[arg(long)]
    server_bin: PathBuf,
    #[arg(long)]
    model: PathBuf,
    #[arg(long)]
    model_name: String,
    /// SHA-256 of the model file; default: looked up in <models dir>/../manifest.json (verified downloads).
    #[arg(long)]
    model_sha256: Option<String>,
    #[arg(long)]
    quant: String,
    /// llama.cpp build tag, e.g. b11310.
    #[arg(long)]
    build: String,
    /// Label for the backend of --server-bin: cuda, vulkan or cpu.
    #[arg(long)]
    backend: String,
    #[arg(long)]
    config_id: String,
    #[arg(long)]
    results: PathBuf,
    /// Directory for server logs (and VRAM samples).
    #[arg(long)]
    log_dir: PathBuf,
    /// Subset directory written by scripts/make_subset.py.
    #[arg(long)]
    dataset: PathBuf,
    #[arg(long, value_delimiter = ',', default_value = "psychology,biology,economics")]
    splits: Vec<String>,
    #[arg(long)]
    pooling: String,
    #[arg(long, default_value_t = 8)]
    np: u32,
    #[arg(long, default_value_t = 2048)]
    slot_ctx: u32,
    /// Used for both -b and -ub.
    #[arg(long, default_value_t = 8192)]
    ubatch: u32,
    #[arg(long, default_value_t = 99)]
    ngl: i32,
    #[arg(long, default_value_t = 4)]
    threads: u32,
    #[arg(long)]
    device: Option<String>,
    /// Run one long-lived `nvidia-smi -lms 250` child for this run (GPU runs only).
    #[arg(long)]
    monitor_vram: bool,
    /// Written with the PIDs of the children this process started; removed on a clean stop.
    #[arg(long)]
    pid_file: Option<PathBuf>,
    #[arg(long, default_value_t = 900)]
    ready_timeout_s: u64,
    /// Extra llama-server argument (repeatable, e.g. --server-arg=-fa --server-arg=on).
    #[arg(long = "server-arg", allow_hyphen_values = true)]
    server_args: Vec<String>,
}

#[derive(Args)]
struct EmbedArgs {
    #[command(flatten)]
    c: Common,
    /// qwen3 = "Instruct: <task>\nQuery:<q>" for queries; harrier = the same with "Query: <q>"; none = raw query (BGE-M3).
    #[arg(long, default_value = "qwen3", value_parser = ["qwen3", "harrier", "none"])]
    query_format: String,
    #[arg(long, default_value_t = 32)]
    embed_batch: usize,
    #[arg(long, default_value_t = 8)]
    concurrency: usize,
    /// Pick batch size on a small sample before the real run.
    #[arg(long)]
    autotune: bool,
    /// Write the top-100 candidates per query (first stage for the rerankers).
    #[arg(long)]
    save_candidates: Option<PathBuf>,
}

#[derive(Args)]
struct RerankArgs {
    #[command(flatten)]
    c: Common,
    #[arg(long)]
    candidates: PathBuf,
    #[arg(long, default_value_t = 2)]
    concurrency: usize,
    #[arg(long, default_value_t = 0.05)]
    sanity_min_top: f64,
    #[arg(long, default_value_t = 0.05)]
    sanity_min_margin: f64,
}

#[derive(Args)]
struct ParityArgs {
    #[command(flatten)]
    c: Common,
    /// Second server binary (backend B); `--server-bin` is backend A.
    #[arg(long)]
    server_bin_b: PathBuf,
    #[arg(long)]
    backend_b: String,
    #[arg(long)]
    build_b: String,
    #[arg(long)]
    device_b: Option<String>,
    #[arg(long, default_value_t = 500)]
    n: usize,
    #[arg(long, default_value_t = 0.999)]
    threshold: f32,
    #[arg(long, default_value_t = 32)]
    embed_batch: usize,
    #[arg(long, default_value_t = 8)]
    concurrency: usize,
}

fn spec(c: &Common, role: Role, bin: &Path, device: &Option<String>) -> ServerSpec {
    ServerSpec {
        bin: bin.to_path_buf(),
        model: c.model.clone(),
        role,
        pooling: c.pooling.clone(),
        ngl: c.ngl,
        np: c.np,
        slot_ctx: c.slot_ctx,
        ubatch: c.ubatch,
        threads: c.threads,
        device: device.clone(),
        extra: c.server_args.clone(),
    }
}

fn load_splits(c: &Common) -> Result<Vec<data::Split>> {
    c.splits.iter().map(|s| data::load_split(&c.dataset, s)).collect()
}

/// The model's SHA-256: from the flag, else from the download manifest (which records the verified hash).
fn resolve_sha(c: &Common) -> Result<String> {
    if let Some(s) = &c.model_sha256 {
        return Ok(s.clone());
    }
    let mpath = c.model.parent().and_then(Path::parent).map(|d| d.join("manifest.json"))
        .context("model path has no manifest directory")?;
    let m: Value = serde_json::from_slice(&std::fs::read(&mpath).with_context(|| format!("read {}", mpath.display()))?)?;
    let name = c.model.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_lowercase();
    let size = std::fs::metadata(&c.model)?.len();
    for e in m["files"].as_object().into_iter().flat_map(|f| f.values()) {
        let p = e["path"].as_str().unwrap_or_default().to_lowercase();
        if p.ends_with(&name) && e["verified"] == json!(true) && e["size"].as_u64() == Some(size) {
            return Ok(e["actual"].as_str().unwrap_or_default().to_string());
        }
    }
    bail!("{} is not a verified entry in {}; pass --model-sha256", c.model.display(), mpath.display())
}

fn config_json(c: &Common, sha: &str, role: &str, flags: &[String], batch: Value, subset: &Value) -> Value {
    json!({
        "role": role, "model_name": c.model_name, "model_file": c.model.display().to_string(),
        "model_sha256": sha, "quantization": c.quant, "llama_cpp_build": c.build,
        "backend": c.backend, "device": c.device, "server_bin": c.server_bin.display().to_string(),
        "server_flags": flags, "pooling": c.pooling,
        "server": {"np": c.np, "slot_ctx": c.slot_ctx, "ubatch": c.ubatch, "ngl": c.ngl, "threads": c.threads},
        "batch": batch, "dataset_subset": {"dir": c.dataset.display().to_string(), "splits": c.splits, "meta": subset},
    })
}

fn write_pids(c: &Common, srv: &Server, mon: &Option<report::VramMonitor>) {
    if let Some(p) = &c.pid_file {
        let mut s = format!("llama-server {}\n", srv.pid());
        if let Some(m) = mon {
            s += &format!("nvidia-smi {}\n", m.pid());
        }
        let _ = std::fs::write(p, s);
    }
}

/// Start the optional VRAM monitor and the server; return once the server is healthy.
fn start_stack(c: &Common, sp: &ServerSpec, tag: &str) -> Result<(Server, Option<report::VramMonitor>, f64)> {
    let mon = if c.monitor_vram {
        let m = report::VramMonitor::start(c.log_dir.join(format!("{}-{tag}-vram.csv", c.config_id)))?;
        std::thread::sleep(Duration::from_millis(1200)); // baseline samples before the model loads
        Some(m)
    } else {
        None
    };
    let t0 = Instant::now();
    let mut srv = Server::start(sp, c.log_dir.join(format!("{}-{tag}-server.log", c.config_id)))?;
    write_pids(c, &srv, &mon);
    report::log(format!("started {} pid {}: {}", sp.bin.display(), srv.pid(), srv.args.join(" ")));
    srv.wait_ready(Duration::from_secs(c.ready_timeout_s))?;
    let load = t0.elapsed().as_secs_f64();
    report::log(format!("server ready after {load:.1}s"));
    Ok((srv, mon, load))
}

fn finish(c: &Common) {
    if let Some(p) = &c.pid_file {
        let _ = std::fs::remove_file(p);
    }
}

fn run_embed(a: &EmbedArgs) -> Result<ExitCode> {
    let c = &a.c;
    let sha = resolve_sha(c)?;
    let splits = load_splits(c)?;
    let subset = data::load_subset_meta(&c.dataset)?;
    let sp = spec(c, Role::Embed, &c.server_bin, &c.device);
    let t_all = Instant::now();
    let (mut srv, mon, load_s) = start_stack(c, &sp, "embed")?;
    let np = c.np as usize;
    let (batch, conc, tune) = if a.autotune {
        let sample = embed::sample_docs(&splits, 256, "autotune");
        let (b, k, trials) = embed::autotune(&srv.base, &sample, np)?;
        report::log(format!("autotune picked batch {b} concurrency {k}"));
        (b, k, trials)
    } else {
        (a.embed_batch, a.concurrency, Value::Null)
    };
    let ev = embed::eval_embedder(&srv.base, &splits, &a.query_format, batch, conc)?;
    let flags = srv.args.clone();
    srv.stop();
    let vram = mon.map(|m| m.stop());
    finish(c);
    let tps = ev.tokens as f64 / ev.seconds;
    let line = json!({
        "kind": "embed", "status": "ok", "config_id": c.config_id, "timestamp": report::iso_now(),
        "config": config_json(c, &sha, "embedder", &flags,
            json!({"embed_batch": batch, "concurrency": conc, "autotune_trials": tune, "query_format": a.query_format}),
            &subset),
        "metrics": ev.metrics,
        "throughput": {"doc_tokens": ev.tokens, "docs": ev.docs, "seconds": ev.seconds, "tokens_per_s": tps,
                       "docs_per_s": ev.docs as f64 / ev.seconds,
                       "projected_hours_for_2.5B_tokens": 2.5e9 / tps / 3600.0},
        "server_load_seconds": load_s, "vram": vram, "wall_seconds": t_all.elapsed().as_secs_f64(),
    });
    report::append_result(&c.results, &line)?;
    if let Some(p) = &a.save_candidates {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut cj = ev.candidates;
        cj["config_id"] = json!(c.config_id);
        std::fs::write(p, serde_json::to_vec(&cj)?)?;
        report::log(format!("wrote candidates {}", p.display()));
    }
    Ok(ExitCode::SUCCESS)
}

fn run_rerank(a: &RerankArgs) -> Result<ExitCode> {
    let c = &a.c;
    let sha = resolve_sha(c)?;
    let splits = load_splits(c)?;
    let subset = data::load_subset_meta(&c.dataset)?;
    let cands: Value = serde_json::from_slice(&std::fs::read(&a.candidates)
        .with_context(|| format!("read candidates {}", a.candidates.display()))?)?;
    let only = subset["rerank_queries"].clone();
    if !only.is_object() {
        bail!("subset.json has no rerank_queries");
    }
    let sp = spec(c, Role::Rerank, &c.server_bin, &c.device);
    let t_all = Instant::now();
    let (mut srv, mon, load_s) = start_stack(c, &sp, "rerank")?;
    let flags = srv.args.clone();
    let cfg = config_json(c, &sha, "reranker", &flags,
        json!({"concurrency": a.concurrency, "candidates_file": a.candidates.display().to_string(),
               "first_stage": cands["config_id"], "top_k": cands["k"]}), &subset);

    let sanity = rerank::sanity_test(&srv.base, a.sanity_min_top, a.sanity_min_margin);
    report::log(format!("sanity: {sanity}"));
    if sanity["pass"] != json!(true) {
        srv.stop();
        let vram = mon.map(|m| m.stop());
        finish(c);
        let line = json!({"kind": "rerank", "status": "sanity_failed", "config_id": c.config_id,
            "timestamp": report::iso_now(), "config": cfg, "sanity": sanity, "vram": vram});
        report::append_result(&c.results, &line)?;
        eprintln!("SANITY FAILED for {}: {}", c.config_id, sanity["reasons"]);
        return Ok(ExitCode::from(3));
    }

    let ev = rerank::eval_reranker(&srv.base, &splits, &cands, &only, a.concurrency)?;
    srv.stop();
    let vram = mon.map(|m| m.stop());
    finish(c);
    let line = json!({
        "kind": "rerank", "status": "ok", "config_id": c.config_id, "timestamp": report::iso_now(),
        "config": cfg, "sanity": sanity, "metrics": ev.metrics,
        "throughput": {"pairs": ev.pairs, "tokens": ev.tokens, "seconds": ev.seconds,
                       "pairs_per_s": ev.pairs as f64 / ev.seconds, "tokens_per_s": ev.tokens as f64 / ev.seconds},
        "server_load_seconds": load_s, "vram": vram, "wall_seconds": t_all.elapsed().as_secs_f64(),
    });
    report::append_result(&c.results, &line)?;
    Ok(ExitCode::SUCCESS)
}

fn run_parity(a: &ParityArgs) -> Result<ExitCode> {
    let c = &a.c;
    let sha = resolve_sha(c)?;
    let splits = load_splits(c)?;
    let subset = data::load_subset_meta(&c.dataset)?;
    let texts = embed::sample_docs(&splits, a.n, "parity");
    if texts.len() < a.n {
        bail!("only {} documents available for parity (wanted {})", texts.len(), a.n);
    }
    let t_all = Instant::now();
    let mut runs: Vec<(embed::Embedded, Vec<String>)> = Vec::new();
    for (bin, dev, tag) in [(&c.server_bin, &c.device, "a"), (&a.server_bin_b, &a.device_b, "b")] {
        let sp = spec(c, Role::Embed, bin, dev);
        let (mut srv, mon, _) = start_stack(c, &sp, &format!("parity-{tag}"))?;
        let e = embed::embed_texts(&srv.base, &texts, a.embed_batch, a.concurrency)?;
        report::log(format!("parity {tag}: {} tokens in {:.1}s", e.tokens, e.seconds));
        let flags = srv.args.clone();
        srv.stop();
        let _ = mon.map(|m| m.stop());
        finish(c);
        runs.push((e, flags));
    }
    let (ea, fa) = (&runs[0].0, &runs[0].1);
    let (eb, fb) = (&runs[1].0, &runs[1].1);
    if ea.dim != eb.dim {
        bail!("dimension mismatch between backends: {} vs {}", ea.dim, eb.dim);
    }
    let cmp = embed::parity_compare(&ea.flat, &eb.flat, ea.dim, a.threshold);
    let mut cfg = config_json(c, &sha, "embedder-parity", fa, json!({"embed_batch": a.embed_batch, "concurrency": a.concurrency,
        "chunks": a.n, "sample_rule": "first n docs by FNV-1a hash of 'parity:<doc id>' across the chosen splits"}), &subset);
    cfg["backend_b"] = json!({"backend": a.backend_b, "build": a.build_b, "device": a.device_b,
        "server_bin": a.server_bin_b.display().to_string(), "server_flags": fb});
    let pass = cmp["pass"] == json!(true);
    let line = json!({
        "kind": "parity", "status": if pass { "ok" } else { "parity_failed" }, "config_id": c.config_id,
        "timestamp": report::iso_now(), "config": cfg, "metrics": cmp,
        "throughput": {"a_tokens_per_s": ea.tokens as f64 / ea.seconds, "b_tokens_per_s": eb.tokens as f64 / eb.seconds},
        "wall_seconds": t_all.elapsed().as_secs_f64(),
    });
    report::append_result(&c.results, &line)?;
    if !pass {
        eprintln!("PARITY FAILED for {}: {cmp}", c.config_id);
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let r = match &cli.cmd {
        Cmd::Embed(a) => run_embed(a),
        Cmd::Rerank(a) => run_rerank(a),
        Cmd::Parity(a) => run_parity(a),
    };
    match r {
        Ok(code) => code,
        Err(e) => {
            // Servers and the VRAM monitor are stopped by their Drop impls as `e` propagates.
            eprintln!("SERVER/INFRASTRUCTURE FAILURE: {e:#}");
            ExitCode::from(2)
        }
    }
}
