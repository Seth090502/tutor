//! `POST /search`: BM25, vector, or hybrid (reciprocal rank fusion) retrieval.
//!
//! - `bm25`: tantivy top `k`; `score` is the BM25 score.
//! - `vector`: LanceDB top `k` by cosine distance; `score` is cosine similarity (1 - distance).
//! - `hybrid`: the top [`CANDIDATES`] of each, fused with RRF (`sum of 1 / (60 + rank)`);
//!   `score` is the fused score. It runs as `bm25` (and says so in `mode_used`) when no
//!   embedding provider is configured or no vectors exist for the configured model.
//!
//! `ranks` gives each hit's 1-based rank in each list that ran (`null` where it did not appear).

use crate::bm25::Filter;
use crate::embed::InputType;
use crate::engine::Engine;
use crate::error::Result;
use crate::ingest::{EMBED_MODEL_KEY, EMBED_PROVIDER_KEY};
use crate::meta::SOURCE_TYPES;
use crate::store::{self, SpendKind};
use rusqlite::params_from_iter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub const RRF_K: f64 = 60.0;
pub const CANDIDATES: usize = 50;
pub const DEFAULT_K: usize = 10;
pub const MAX_K: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Hybrid,
    Bm25,
    Vector,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub query: String,
    pub k: Option<usize>,
    pub domains: Option<Vec<String>>,
    pub source_types: Option<Vec<String>>,
    pub mode: Option<Mode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ranks {
    pub bm25: Option<usize>,
    pub vector: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub chunk_id: String,
    pub doc_id: String,
    pub path: String,
    pub title: String,
    pub domain: String,
    pub source_type: String,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
    pub heading_path: Vec<String>,
    pub text: String,
    pub score: f64,
    pub ranks: Ranks,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub mode_used: Mode,
    pub hits: Vec<Hit>,
}

/// Why a search could not run: the request (400), the service's state (409), or a failure (500/502).
#[derive(Debug, Clone, PartialEq)]
pub enum SearchError {
    BadRequest(String),
    Unavailable(String),
    Provider(String),
    Internal(String),
}

impl From<crate::error::Error> for SearchError {
    fn from(e: crate::error::Error) -> Self {
        SearchError::Internal(e.0)
    }
}

/// Reciprocal rank fusion of ranked lists (best first). Each result carries its fused score
/// and its 1-based rank in every list. Ties are broken by the best single rank, then by id.
pub fn rrf(lists: &[Vec<String>], k: f64) -> Vec<(String, f64, Vec<Option<usize>>)> {
    let mut fused: HashMap<&str, (f64, Vec<Option<usize>>)> = HashMap::new();
    for (list_index, list) in lists.iter().enumerate() {
        for (i, id) in list.iter().enumerate() {
            let entry = fused.entry(id).or_insert_with(|| (0.0, vec![None; lists.len()]));
            if entry.1[list_index].is_none() {
                entry.0 += 1.0 / (k + (i + 1) as f64);
                entry.1[list_index] = Some(i + 1);
            }
        }
    }
    let mut out: Vec<(String, f64, Vec<Option<usize>>)> =
        fused.into_iter().map(|(id, (score, ranks))| (id.to_string(), score, ranks)).collect();
    let best = |ranks: &[Option<usize>]| ranks.iter().flatten().min().copied().unwrap_or(usize::MAX);
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(best(&a.2).cmp(&best(&b.2))).then(a.0.cmp(&b.0)));
    out
}

pub fn validate(req: &SearchRequest) -> std::result::Result<(usize, Filter, Mode), SearchError> {
    if req.query.trim().is_empty() {
        return Err(SearchError::BadRequest("query must not be empty".into()));
    }
    let k = req.k.unwrap_or(DEFAULT_K);
    if !(1..=MAX_K).contains(&k) {
        return Err(SearchError::BadRequest(format!("k must be between 1 and {MAX_K}")));
    }
    let source_types = req.source_types.clone().unwrap_or_default();
    if let Some(bad) = source_types.iter().find(|s| !SOURCE_TYPES.contains(&s.as_str())) {
        return Err(SearchError::BadRequest(format!("unknown source_type {bad:?}; expected one of {SOURCE_TYPES:?}")));
    }
    let filter = Filter { domains: req.domains.clone().unwrap_or_default(), source_types };
    Ok((k, filter, req.mode.unwrap_or(Mode::Hybrid)))
}

/// Whether vector search can run: a provider is configured and the vector store holds vectors
/// from that provider and model.
pub async fn vectors_ready(engine: &Engine) -> Result<bool> {
    let (Some(embedder), Some(vectors)) = (&engine.embedder, &engine.vectors) else { return Ok(false) };
    let (provider, model) = {
        let conn = engine.conn();
        (store::meta_get(&conn, EMBED_PROVIDER_KEY)?, store::meta_get(&conn, EMBED_MODEL_KEY)?)
    };
    let same_model = provider.as_deref() == Some(embedder.provider_name()) && model.as_deref() == Some(embedder.model());
    Ok(same_model && vectors.count().await? > 0)
}

pub async fn search(engine: Arc<Engine>, req: SearchRequest) -> std::result::Result<SearchResponse, SearchError> {
    let (k, filter, mode) = validate(&req)?;
    let ready = vectors_ready(&engine).await?;
    let mode_used = match (mode, ready) {
        (Mode::Vector, false) => {
            return Err(SearchError::Unavailable(
                "vector search is unavailable: no embedding provider is configured or no vectors exist for it".into(),
            ));
        }
        (Mode::Hybrid, false) => Mode::Bm25,
        (mode, _) => mode,
    };
    let depth = if mode_used == Mode::Hybrid { CANDIDATES.max(k) } else { k };

    let bm25 = if mode_used == Mode::Vector {
        Vec::new()
    } else {
        let (engine, query, filter) = (engine.clone(), req.query.clone(), filter.clone());
        tokio::task::spawn_blocking(move || engine.bm25.search(&query, depth, &filter))
            .await
            .map_err(|e| SearchError::Internal(e.to_string()))??
    };
    let vector = if mode_used == Mode::Bm25 {
        Vec::new()
    } else {
        let embedder = engine.embedder.as_ref().expect("vectors_ready checked the embedder");
        let out = embedder
            .embed_batch(std::slice::from_ref(&req.query), InputType::Query)
            .await
            .map_err(|e| SearchError::Provider(e.0))?;
        store::record_spend(&engine.conn(), SpendKind::Embedding, embedder.provider_name(), embedder.model(), out.tokens, out.usd)?;
        let vectors = engine.vectors.as_ref().expect("vectors_ready checked the vector store");
        vectors.search(&out.vectors[0], depth, &filter).await?
    };

    // (chunk_id, score, bm25 rank, vector rank), best first.
    let ranked: Vec<(String, f64, Option<usize>, Option<usize>)> = match mode_used {
        Mode::Bm25 => bm25.into_iter().enumerate().map(|(i, (id, s))| (id, s as f64, Some(i + 1), None)).collect(),
        Mode::Vector => vector.into_iter().enumerate().map(|(i, (id, d))| (id, 1.0 - d as f64, None, Some(i + 1))).collect(),
        Mode::Hybrid => {
            let lists = [bm25.into_iter().map(|(id, _)| id).collect(), vector.into_iter().map(|(id, _)| id).collect()];
            rrf(&lists, RRF_K).into_iter().map(|(id, s, r)| (id, s, r[0], r[1])).collect()
        }
    };
    let engine2 = engine.clone();
    let hits = tokio::task::spawn_blocking(move || load_hits(&engine2, ranked, k))
        .await
        .map_err(|e| SearchError::Internal(e.to_string()))??;
    Ok(SearchResponse { mode_used, hits })
}

/// Fill in chunk and document fields for the first `k` ranked chunks that still exist.
fn load_hits(engine: &Engine, ranked: Vec<(String, f64, Option<usize>, Option<usize>)>, k: usize) -> Result<Vec<Hit>> {
    let ids: Vec<&str> = ranked.iter().map(|r| r.0.as_str()).collect();
    let mut rows: HashMap<String, Hit> = HashMap::new();
    if !ids.is_empty() {
        let conn = engine.conn();
        let placeholders = vec!["?"; ids.len()].join(", ");
        let mut stmt = conn.prepare(&format!(
            "SELECT c.chunk_id, c.doc_id, d.path, d.title, d.domain, d.source_type, c.page_start, c.page_end, \
             c.heading_path, c.text FROM chunks c JOIN documents d ON d.doc_id = c.doc_id \
             WHERE c.chunk_id IN ({placeholders}) AND d.state = 'indexed'"
        ))?;
        let found = stmt.query_map(params_from_iter(&ids), |r| {
            let heading_path: String = r.get(8)?;
            Ok(Hit {
                chunk_id: r.get(0)?,
                doc_id: r.get(1)?,
                path: r.get(2)?,
                title: r.get(3)?,
                domain: r.get(4)?,
                source_type: r.get(5)?,
                page_start: r.get(6)?,
                page_end: r.get(7)?,
                heading_path: serde_json::from_str(&heading_path).unwrap_or_default(),
                text: r.get(9)?,
                score: 0.0,
                ranks: Ranks { bm25: None, vector: None },
            })
        })?;
        for hit in found {
            let hit = hit?;
            rows.insert(hit.chunk_id.clone(), hit);
        }
    }
    Ok(ranked
        .into_iter()
        .filter_map(|(id, score, bm25, vector)| {
            rows.remove(&id).map(|mut hit| {
                hit.score = score;
                hit.ranks = Ranks { bm25, vector };
                hit
            })
        })
        .take(k)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn rrf_sums_reciprocal_ranks() {
        let fused = rrf(&[list(&["a", "b", "c"]), list(&["c", "a", "d"])], 60.0);
        let ids: Vec<&str> = fused.iter().map(|f| f.0.as_str()).collect();
        // a: 1/61 + 1/62; c: 1/63 + 1/61; b: 1/62; d: 1/63.
        assert_eq!(ids, vec!["a", "c", "b", "d"]);
        assert!((fused[0].1 - (1.0 / 61.0 + 1.0 / 62.0)).abs() < 1e-12);
        assert_eq!(fused[0].2, vec![Some(1), Some(2)]);
        assert_eq!(fused[2].2, vec![Some(2), None]);
        assert_eq!(fused[3].2, vec![None, Some(3)]);
    }

    #[test]
    fn rrf_breaks_ties_by_best_rank_then_id() {
        // x and y both score 1/61 + 1/62; z and w both 1/63.
        let fused = rrf(&[list(&["x", "y", "z"]), list(&["y", "x", "w"])], 60.0);
        let ids: Vec<&str> = fused.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(ids, vec!["x", "y", "w", "z"]);
    }

    #[test]
    fn rrf_counts_a_duplicate_only_at_its_first_rank() {
        let fused = rrf(&[list(&["a", "a", "b"])], 60.0);
        assert_eq!(fused[0].2, vec![Some(1)]);
        assert!((fused[0].1 - 1.0 / 61.0).abs() < 1e-12);
        assert!(rrf(&[vec![], vec![]], 60.0).is_empty());
    }

    fn request(json: serde_json::Value) -> SearchRequest {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn requests_are_validated() {
        let (k, filter, mode) = validate(&request(serde_json::json!({"query": "limits"}))).unwrap();
        assert_eq!((k, mode), (DEFAULT_K, Mode::Hybrid));
        assert!(filter.domains.is_empty() && filter.source_types.is_empty());
        let (k, filter, mode) = validate(&request(
            serde_json::json!({"query": "x", "k": 3, "domains": ["math"], "source_types": ["book", "note"], "mode": "bm25"}),
        ))
        .unwrap();
        assert_eq!((k, mode), (3, Mode::Bm25));
        assert_eq!((filter.domains, filter.source_types), (vec!["math".to_string()], vec!["book".to_string(), "note".to_string()]));

        for bad in [
            serde_json::json!({"query": "  "}),
            serde_json::json!({"query": "x", "k": 0}),
            serde_json::json!({"query": "x", "k": 101}),
            serde_json::json!({"query": "x", "source_types": ["books"]}),
        ] {
            assert!(matches!(validate(&request(bad)), Err(SearchError::BadRequest(_))));
        }
        assert!(serde_json::from_value::<SearchRequest>(serde_json::json!({"query": "x", "mode": "dense"})).is_err());
        assert!(serde_json::from_value::<SearchRequest>(serde_json::json!({"query": "x", "domain": ["math"]})).is_err());
    }
}
