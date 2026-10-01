//! Dataset subset loader (written by scripts/make_subset.py) and retrieval metrics.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Deserialize, Clone)]
pub struct Doc {
    pub id: String,
    pub content: String,
}

#[derive(Deserialize, Clone)]
pub struct Query {
    pub id: String,
    pub query: String,
    pub gold_ids: Vec<String>,
    #[serde(default)]
    pub excluded_ids: Vec<String>,
}

pub struct Split {
    pub name: String,
    pub docs: Vec<Doc>,
    pub queries: Vec<Query>,
    pub doc_index: HashMap<String, usize>,
}

impl Split {
    pub fn gold_set(&self, q: &Query) -> HashSet<usize> {
        q.gold_ids.iter().filter_map(|g| self.doc_index.get(g).copied()).collect()
    }
    pub fn excluded_set(&self, q: &Query) -> HashSet<usize> {
        q.excluded_ids.iter().filter_map(|g| self.doc_index.get(g).copied()).collect()
    }
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, l)| serde_json::from_str(l).with_context(|| format!("{}:{}", path.display(), i + 1)))
        .collect()
}

pub fn load_split(dataset: &Path, name: &str) -> Result<Split> {
    let docs: Vec<Doc> = read_jsonl(&dataset.join(name).join("documents.jsonl"))?;
    let queries: Vec<Query> = read_jsonl(&dataset.join(name).join("queries.jsonl"))?;
    let doc_index = docs.iter().enumerate().map(|(i, d)| (d.id.clone(), i)).collect();
    Ok(Split { name: name.to_string(), docs, queries, doc_index })
}

pub fn load_subset_meta(dataset: &Path) -> Result<serde_json::Value> {
    let p = dataset.join("subset.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?)?)
}

/// Binary-relevance nDCG@k (pytrec_eval ndcg_cut convention, as used by BRIGHT).
pub fn ndcg_at(ranked: &[usize], gold: &HashSet<usize>, k: usize) -> f64 {
    let dcg: f64 = ranked.iter().take(k).enumerate()
        .filter(|(_, d)| gold.contains(d))
        .map(|(i, _)| 1.0 / ((i + 2) as f64).log2())
        .sum();
    let ideal: f64 = (0..gold.len().min(k)).map(|i| 1.0 / ((i + 2) as f64).log2()).sum();
    if ideal == 0.0 { 0.0 } else { dcg / ideal }
}

pub fn recall_at(ranked: &[usize], gold: &HashSet<usize>, k: usize) -> f64 {
    if gold.is_empty() {
        return 0.0;
    }
    ranked.iter().take(k).filter(|d| gold.contains(d)).count() as f64 / gold.len() as f64
}

/// Stable 64-bit FNV-1a, used for deterministic sampling.
pub fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndcg_perfect_and_miss() {
        let gold: HashSet<usize> = [3, 7].into_iter().collect();
        assert!((ndcg_at(&[3, 7, 1], &gold, 10) - 1.0).abs() < 1e-12);
        assert_eq!(ndcg_at(&[1, 2, 4], &gold, 10), 0.0);
        // one gold at rank 2 of 2 gold: dcg = 1/log2(3), ideal = 1 + 1/log2(3)
        let v = ndcg_at(&[1, 3], &gold, 10);
        let want = (1.0 / 3f64.log2()) / (1.0 + 1.0 / 3f64.log2());
        assert!((v - want).abs() < 1e-12);
    }

    #[test]
    fn recall_counts_hits() {
        let gold: HashSet<usize> = [1, 2, 3, 4].into_iter().collect();
        assert!((recall_at(&[1, 9, 2, 8], &gold, 3) - 0.5).abs() < 1e-12);
    }
}
