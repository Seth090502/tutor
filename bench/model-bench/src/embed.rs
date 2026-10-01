//! Embedding client, exact cosine search, Matryoshka truncation, quality metrics, parity.

use crate::data::{Split, fnv1a, ndcg_at, recall_at};
use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

pub const TOP_K: usize = 100;

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(1800)))
        .build()
        .into()
}

#[derive(Deserialize)]
struct EmbItem {
    index: usize,
    embedding: Vec<f32>,
}
#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u64,
}
#[derive(Deserialize)]
struct EmbResp {
    data: Vec<EmbItem>,
    #[serde(default)]
    usage: Option<Usage>,
}

/// Embeddings in row-major layout, one row per input text, in input order.
pub struct Embedded {
    pub flat: Vec<f32>,
    pub dim: usize,
    pub tokens: u64,
    pub seconds: f64,
}

fn post_embeddings(agent: &ureq::Agent, base: &str, texts: &[&str]) -> Result<(Vec<Vec<f32>>, u64)> {
    let mut resp = agent
        .post(&format!("{base}/v1/embeddings"))
        .send_json(json!({"model": "bench", "input": texts, "encoding_format": "float"}))?;
    let status = resp.status().as_u16();
    if status != 200 {
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        bail!("POST /v1/embeddings -> HTTP {status}: {}", body.chars().take(500).collect::<String>());
    }
    let r: EmbResp = resp.body_mut().with_config().limit(512 << 20).read_json()?;
    if r.data.len() != texts.len() {
        bail!("server returned {} embeddings for {} inputs", r.data.len(), texts.len());
    }
    let mut rows = vec![Vec::new(); texts.len()];
    for it in r.data {
        if it.index >= rows.len() {
            bail!("embedding index {} out of range", it.index);
        }
        rows[it.index] = it.embedding;
    }
    Ok((rows, r.usage.map_or(0, |u| u.prompt_tokens)))
}

/// Embed `texts` with `batch` inputs per request and `concurrency` requests in flight.
/// Longest texts go first so the tail of the run is cheap.
pub fn embed_texts(base: &str, texts: &[&str], batch: usize, concurrency: usize) -> Result<Embedded> {
    let agent = agent();
    let mut order: Vec<usize> = (0..texts.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(texts[i].len()));
    let chunks: Vec<&[usize]> = order.chunks(batch.max(1)).collect();
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<(usize, Vec<f32>)>> = Mutex::new(Vec::with_capacity(texts.len()));
    let tokens = AtomicUsize::new(0);
    let err: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..concurrency.max(1) {
            s.spawn(|| {
                loop {
                    let c = next.fetch_add(1, Ordering::Relaxed);
                    if c >= chunks.len() || err.lock().unwrap().is_some() {
                        return;
                    }
                    let idx = chunks[c];
                    let inputs: Vec<&str> = idx.iter().map(|&i| texts[i]).collect();
                    match post_embeddings(&agent, base, &inputs) {
                        Ok((rows, tk)) => {
                            tokens.fetch_add(tk as usize, Ordering::Relaxed);
                            let mut o = out.lock().unwrap();
                            for (k, row) in rows.into_iter().enumerate() {
                                o.push((idx[k], row));
                            }
                        }
                        Err(e) => {
                            *err.lock().unwrap() = Some(e);
                            return;
                        }
                    }
                }
            });
        }
    });
    let seconds = t0.elapsed().as_secs_f64();
    if let Some(e) = err.into_inner().unwrap() {
        return Err(e);
    }
    let mut rows = out.into_inner().unwrap();
    rows.sort_by_key(|r| r.0);
    let dim = rows.first().map_or(0, |r| r.1.len());
    let mut flat = Vec::with_capacity(rows.len() * dim);
    for (_, r) in &rows {
        if r.len() != dim || r.iter().any(|x| !x.is_finite()) {
            bail!("inconsistent or non-finite embedding (dim {} vs {dim})", r.len());
        }
        flat.extend_from_slice(r);
    }
    Ok(Embedded { flat, dim, tokens: tokens.load(Ordering::Relaxed) as u64, seconds })
}

/// First `d` dims of each row, renormalised to unit length (Matryoshka truncation).
pub fn truncate_normalise(flat: &[f32], dim: usize, d: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(flat.len() / dim * d);
    for row in flat.chunks_exact(dim) {
        let v = &row[..d];
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
        out.extend(v.iter().map(|x| x / n));
    }
    out
}

/// Exact cosine top-k (inputs are unit rows) for every query; excluded docs are skipped.
pub fn search(docs: &[f32], queries: &[f32], d: usize, split: &Split, k: usize) -> Vec<Vec<usize>> {
    let n_docs = docs.len() / d;
    let nq = queries.len() / d;
    let results: Mutex<Vec<(usize, Vec<usize>)>> = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..8 {
            s.spawn(|| loop {
                let qi = next.fetch_add(1, Ordering::Relaxed);
                if qi >= nq {
                    return;
                }
                let q = &queries[qi * d..(qi + 1) * d];
                let excl = split.excluded_set(&split.queries[qi]);
                let mut scored: Vec<(usize, f32)> = (0..n_docs)
                    .filter(|di| !excl.contains(di))
                    .map(|di| (di, docs[di * d..(di + 1) * d].iter().zip(q).map(|(a, b)| a * b).sum()))
                    .collect();
                let kk = k.min(scored.len());
                if kk < scored.len() {
                    scored.select_nth_unstable_by(kk - 1, |a, b| b.1.total_cmp(&a.1));
                    scored.truncate(kk);
                }
                scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                results.lock().unwrap().push((qi, scored.into_iter().map(|x| x.0).collect()));
            });
        }
    });
    let mut r = results.into_inner().unwrap();
    r.sort_by_key(|x| x.0);
    r.into_iter().map(|x| x.1).collect()
}

pub fn mean_metrics(split: &Split, ranked: &[Vec<usize>]) -> (f64, f64) {
    let (mut n, mut r, mut c) = (0.0, 0.0, 0usize);
    for (q, rk) in split.queries.iter().zip(ranked) {
        let gold = split.gold_set(q);
        if gold.is_empty() {
            continue;
        }
        n += ndcg_at(rk, &gold, 10);
        r += recall_at(rk, &gold, TOP_K);
        c += 1;
    }
    (n / c.max(1) as f64, r / c.max(1) as f64)
}

pub fn instruct_query(task: &str, q: &str) -> String {
    format!("Instruct: {task}\nQuery:{q}")
}

/// Task description for Qwen3-Embedding queries (BRIGHT's per-domain instruction).
pub fn task_for(split: &str) -> String {
    let title = match split {
        "biology" => "Biology",
        "economics" => "Economics",
        "psychology" => "Psychology",
        other => other,
    };
    format!("Given a {title} post, retrieve relevant passages that help answer the post")
}

pub struct EmbedEval {
    pub metrics: Value,
    pub candidates: Value,
    pub tokens: u64,
    pub seconds: f64,
    pub docs: usize,
}

/// Embed docs and queries of every split, compute nDCG@10 / recall@100 at full, 512 and 256 dims.
pub fn eval_embedder(base: &str, splits: &[Split], qwen_instruct: bool, batch: usize, conc: usize) -> Result<EmbedEval> {
    let mut per_split = serde_json::Map::new();
    let mut cands = serde_json::Map::new();
    let (mut tokens, mut seconds, mut ndocs) = (0u64, 0f64, 0usize);
    let mut sums: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
    for sp in splits {
        let dtexts: Vec<&str> = sp.docs.iter().map(|d| d.content.as_str()).collect();
        let de = embed_texts(base, &dtexts, batch, conc)?;
        crate::report::log(format!("{}: embedded {} docs, {} tokens in {:.1}s ({:.0} tok/s)", sp.name, dtexts.len(),
            de.tokens, de.seconds, de.tokens as f64 / de.seconds));
        tokens += de.tokens;
        seconds += de.seconds;
        ndocs += dtexts.len();
        let task = task_for(&sp.name);
        let qtexts: Vec<String> = sp.queries.iter()
            .map(|q| if qwen_instruct { instruct_query(&task, &q.query) } else { q.query.clone() })
            .collect();
        let qrefs: Vec<&str> = qtexts.iter().map(String::as_str).collect();
        let qe = embed_texts(base, &qrefs, batch.min(16), conc)?;
        let dim = de.dim;
        let mut entry = serde_json::Map::new();
        entry.insert("n_docs".into(), json!(sp.docs.len()));
        entry.insert("n_queries".into(), json!(sp.queries.len()));
        entry.insert("doc_tokens".into(), json!(de.tokens));
        entry.insert("doc_embed_seconds".into(), json!(de.seconds));
        entry.insert("dim".into(), json!(dim));
        for d in [dim, 512, 256] {
            if d > dim {
                continue;
            }
            let dn = truncate_normalise(&de.flat, dim, d);
            let qn = truncate_normalise(&qe.flat, dim, d);
            let ranked = search(&dn, &qn, d, sp, TOP_K);
            let (ndcg, rec) = mean_metrics(sp, &ranked);
            let key = if d == dim { "full".to_string() } else { d.to_string() };
            entry.insert(key.clone(), json!({"dims": d, "ndcg@10": ndcg, "recall@100": rec}));
            let s = sums.entry(key).or_default();
            s.0 += ndcg;
            s.1 += rec;
            if d == dim {
                let m: serde_json::Map<String, Value> = sp.queries.iter().zip(&ranked)
                    .map(|(q, r)| (q.id.clone(), json!(r.iter().map(|&i| sp.docs[i].id.clone()).collect::<Vec<_>>())))
                    .collect();
                cands.insert(sp.name.clone(), Value::Object(m));
            }
        }
        per_split.insert(sp.name.clone(), Value::Object(entry));
    }
    let n = splits.len().max(1) as f64;
    let macro_mean: serde_json::Map<String, Value> = sums.into_iter()
        .map(|(k, (a, b))| (k, json!({"ndcg@10": a / n, "recall@100": b / n}))).collect();
    Ok(EmbedEval {
        metrics: json!({"splits": per_split, "macro_mean": macro_mean}),
        candidates: json!({"k": TOP_K, "splits": cands}),
        tokens, seconds, docs: ndocs,
    })
}

/// Deterministic sample of `n` doc indices by FNV hash of "seed:id" (stable across runs and machines).
pub fn sample_docs<'a>(splits: &'a [Split], n: usize, seed: &str) -> Vec<&'a str> {
    let mut all: Vec<(u64, &str)> = splits.iter().flat_map(|s| s.docs.iter())
        .map(|d| (fnv1a(&format!("{seed}:{}", d.id)), d.content.as_str())).collect();
    all.sort_by_key(|x| x.0);
    all.into_iter().take(n).map(|x| x.1).collect()
}

/// Pick the best (batch, concurrency) among a few candidates by measuring tokens/s on a small sample.
pub fn autotune(base: &str, sample: &[&str], np: usize) -> Result<(usize, usize, Value)> {
    let cands = [(8usize, np), (32, np), (96, np)];
    let mut best = (cands[1], 0f64);
    let mut trials = Vec::new();
    // First trial doubles as warm-up and is discarded.
    embed_texts(base, &sample[..sample.len().min(16)], 8, 2)?;
    for &(b, c) in &cands {
        let e = embed_texts(base, sample, b, c)?;
        let tps = e.tokens as f64 / e.seconds;
        trials.push(json!({"batch": b, "concurrency": c, "tokens_per_s": tps}));
        if tps > best.1 {
            best = ((b, c), tps);
        }
    }
    Ok((best.0.0, best.0.1, json!(trials)))
}

/// Cosine similarity of two rows (both already unit length after `truncate_normalise`).
pub fn parity_compare(a: &[f32], b: &[f32], dim: usize, threshold: f32) -> Value {
    let an = truncate_normalise(a, dim, dim);
    let bn = truncate_normalise(b, dim, dim);
    let cos: Vec<f32> = an.chunks_exact(dim).zip(bn.chunks_exact(dim))
        .map(|(x, y)| x.iter().zip(y).map(|(p, q)| p * q).sum()).collect();
    let min = cos.iter().copied().fold(f32::INFINITY, f32::min);
    let mean = cos.iter().sum::<f32>() / cos.len().max(1) as f32;
    let failures = cos.iter().filter(|&&c| c < threshold).count();
    json!({"pairs": cos.len(), "threshold": threshold, "min_cosine": min, "mean_cosine": mean,
           "failures": failures, "pass": failures == 0})
}
