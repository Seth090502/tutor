//! Reranker client, the sanity test, and reranking quality/throughput over a fixed first stage.

use crate::data::{Split, ndcg_at, recall_at};
use crate::embed::agent;
use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Deserialize)]
struct RrItem {
    index: usize,
    relevance_score: f64,
}
#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u64,
}
#[derive(Deserialize)]
struct RrResp {
    results: Vec<RrItem>,
    #[serde(default)]
    usage: Option<Usage>,
}

/// Scores in input order plus server-reported prompt tokens.
pub fn rerank_request(agent: &ureq::Agent, base: &str, query: &str, docs: &[&str]) -> Result<(Vec<f64>, u64)> {
    let mut resp = agent
        .post(&format!("{base}/v1/rerank"))
        .send_json(json!({"model": "bench", "query": query, "documents": docs}))?;
    let status = resp.status().as_u16();
    if status != 200 {
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        bail!("POST /v1/rerank -> HTTP {status}: {}", body.chars().take(500).collect::<String>());
    }
    let r: RrResp = resp.body_mut().with_config().limit(64 << 20).read_json()?;
    if r.results.len() != docs.len() {
        bail!("server returned {} scores for {} documents", r.results.len(), docs.len());
    }
    let mut scores = vec![f64::NAN; docs.len()];
    for it in r.results {
        if it.index >= scores.len() {
            bail!("rerank index {} out of range", it.index);
        }
        scores[it.index] = it.relevance_score;
    }
    if scores.iter().any(|s| !s.is_finite()) {
        bail!("non-finite rerank score returned");
    }
    Ok((scores, r.usage.map_or(0, |u| u.prompt_tokens)))
}

/// (query, relevant passage, irrelevant passages).
const SANITY: [(&str, &str, [&str; 2]); 2] = [
    (
        "What is the capital of France?",
        "Paris is the capital and most populous city of France, situated on the river Seine.",
        [
            "Mitochondria are organelles that generate most of a cell's supply of ATP.",
            "The 2008 financial crisis was triggered by the collapse of the US housing bubble.",
        ],
    ),
    (
        "Why does ice float on water?",
        "Ice is less dense than liquid water because hydrogen bonds hold the molecules in an open hexagonal lattice, so it floats.",
        [
            "The Treaty of Westphalia in 1648 ended the Thirty Years' War in Europe.",
            "Python lists are mutable sequences that support indexing and slicing.",
        ],
    ),
];

/// Fails when the top score is near zero, or when the relevant passage does not outrank both irrelevant ones.
pub fn sanity_test(base: &str, min_abs_top: f64, min_margin: f64) -> Value {
    let agent = agent();
    let mut cases = Vec::new();
    let mut pass = true;
    let mut reasons: Vec<String> = Vec::new();
    for (q, rel, irr) in SANITY {
        let docs = [rel, irr[0], irr[1]];
        match rerank_request(&agent, base, q, &docs) {
            Ok((s, _)) => {
                let top = s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let best_irr = s[1].max(s[2]);
                let outranks = s[0] > best_irr;
                let margin = s[0] - best_irr;
                let near_zero = top.abs() < min_abs_top;
                if !outranks || margin < min_margin {
                    pass = false;
                    reasons.push(format!("'{q}': relevant {:.4} does not clearly outrank irrelevant {:.4}", s[0], best_irr));
                }
                if near_zero {
                    pass = false;
                    reasons.push(format!("'{q}': top score {top:.6} is near zero (< {min_abs_top})"));
                }
                cases.push(json!({"query": q, "scores": s, "relevant_idx": 0, "margin": margin}));
            }
            Err(e) => {
                pass = false;
                reasons.push(format!("'{q}': request failed: {e}"));
            }
        }
    }
    json!({"pass": pass, "min_abs_top": min_abs_top, "min_margin": min_margin, "cases": cases, "reasons": reasons})
}

pub struct RerankEval {
    pub metrics: Value,
    pub pairs: u64,
    pub tokens: u64,
    pub seconds: f64,
}

/// Rerank the stored first-stage candidates of the chosen queries and report nDCG@10 before and after.
pub fn eval_reranker(
    base: &str,
    splits: &[Split],
    cands: &Value,
    only: &Value, // {split: [query ids]}
    concurrency: usize,
) -> Result<RerankEval> {
    let agent = agent();
    let mut per_split = serde_json::Map::new();
    let (mut pairs, mut tokens, mut seconds) = (0u64, 0u64, 0f64);
    let (mut sum_first, mut sum_rr, mut sum_rec) = (0.0, 0.0, 0.0);
    for sp in splits {
        let wanted: HashSet<&str> = only[&sp.name].as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let jobs: Vec<usize> = sp.queries.iter().enumerate()
            .filter(|(_, q)| wanted.contains(q.id.as_str())).map(|(i, _)| i).collect();
        if jobs.is_empty() {
            bail!("no rerank queries selected for split {}", sp.name);
        }
        let next = AtomicUsize::new(0);
        let out: Mutex<Vec<(usize, Vec<usize>, Vec<usize>, u64)>> = Mutex::new(Vec::new());
        let err: Mutex<Option<anyhow::Error>> = Mutex::new(None);
        let t0 = Instant::now();
        std::thread::scope(|s| {
            for _ in 0..concurrency.max(1) {
                s.spawn(|| loop {
                    let j = next.fetch_add(1, Ordering::Relaxed);
                    if j >= jobs.len() || err.lock().unwrap().is_some() {
                        return;
                    }
                    let qi = jobs[j];
                    let q = &sp.queries[qi];
                    let ids = cands["splits"][&sp.name][&q.id].as_array();
                    let Some(ids) = ids else {
                        *err.lock().unwrap() = Some(anyhow::anyhow!("no candidates for {}/{}", sp.name, q.id));
                        return;
                    };
                    let didx: Vec<usize> = ids.iter().filter_map(|v| v.as_str())
                        .filter_map(|id| sp.doc_index.get(id).copied()).collect();
                    let texts: Vec<&str> = didx.iter().map(|&i| sp.docs[i].content.as_str()).collect();
                    match rerank_request(&agent, base, &q.query, &texts) {
                        Ok((scores, tk)) => {
                            let mut order: Vec<usize> = (0..didx.len()).collect();
                            order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]).then(a.cmp(&b)));
                            let reranked: Vec<usize> = order.iter().map(|&o| didx[o]).collect();
                            out.lock().unwrap().push((qi, didx, reranked, tk));
                        }
                        Err(e) => {
                            *err.lock().unwrap() = Some(e);
                            return;
                        }
                    }
                });
            }
        });
        let dt = t0.elapsed().as_secs_f64();
        if let Some(e) = err.into_inner().unwrap() {
            return Err(e);
        }
        let rows = out.into_inner().unwrap();
        let (mut f, mut r, mut rec, mut n, mut sp_pairs, mut sp_tok) = (0.0, 0.0, 0.0, 0usize, 0u64, 0u64);
        for (qi, first, reranked, tk) in &rows {
            let gold = sp.gold_set(&sp.queries[*qi]);
            if gold.is_empty() {
                continue;
            }
            f += ndcg_at(first, &gold, 10);
            r += ndcg_at(reranked, &gold, 10);
            rec += recall_at(first, &gold, first.len());
            n += 1;
            sp_pairs += first.len() as u64;
            sp_tok += tk;
        }
        let nn = n.max(1) as f64;
        crate::report::log(format!("{}: reranked {} queries ({} pairs) in {:.1}s; ndcg@10 {:.4} -> {:.4}", sp.name,
            n, sp_pairs, dt, f / nn, r / nn));
        per_split.insert(sp.name.clone(), json!({
            "n_queries": n, "pairs": sp_pairs, "tokens": sp_tok, "seconds": dt,
            "first_stage_ndcg@10": f / nn, "reranked_ndcg@10": r / nn, "candidate_recall@100": rec / nn,
        }));
        pairs += sp_pairs;
        tokens += sp_tok;
        seconds += dt;
        sum_first += f / nn;
        sum_rr += r / nn;
        sum_rec += rec / nn;
    }
    let ns = splits.len().max(1) as f64;
    Ok(RerankEval {
        metrics: json!({"splits": per_split, "macro_mean": {
            "first_stage_ndcg@10": sum_first / ns, "reranked_ndcg@10": sum_rr / ns, "candidate_recall@100": sum_rec / ns}}),
        pairs, tokens, seconds,
    })
}
