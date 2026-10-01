//! Splitting batched requests into one-item requests, and merging the per-item responses back.
//!
//! The streaming RPCs use this on the server to run the items of a large batch concurrently and send each
//! result as soon as it is ready, and on the client to reassemble a stream into the single response a unary call
//! would have returned.

use vault_api as api;

/// A batched request whose items are independent.
pub trait Chunked: Sized {
    type Response;

    /// One request per item, in order. A request with no items yields itself, so the tool still reports the
    /// empty batch the way it does for a unary call.
    fn split(&self) -> Vec<Self>;

    /// Rewrites item indexes in the response to a one-item request so they point into the original request;
    /// `offset` is the position of that item.
    fn rebase(response: &mut Self::Response, offset: u32);

    /// Concatenates responses to consecutive chunks (already rebased) into one response.
    fn merge(parts: Vec<Self::Response>) -> Self::Response;
}

fn split_by<T: Clone, R>(items: &[T], request: &R, build: impl Fn(&R, T) -> R) -> Vec<R>
where
    R: Clone,
{
    if items.is_empty() {
        return vec![request.clone()];
    }
    items.iter().cloned().map(|item| build(request, item)).collect()
}

fn rebase_errors(errors: &mut [api::ItemError], offset: u32) {
    for e in errors {
        e.index = e.index.saturating_add(offset);
    }
}

/// The first truncation notice stays as is; later ones are appended so no reason is lost.
fn merge_truncations(parts: impl Iterator<Item = Option<api::Truncation>>) -> Option<api::Truncation> {
    let mut merged: Option<api::Truncation> = None;
    for t in parts.flatten() {
        match merged.as_mut() {
            None => merged = Some(t),
            Some(m) => {
                m.reason = format!("{}; {}", m.reason, t.reason);
                m.next = format!("{}; {}", m.next, t.next);
            }
        }
    }
    merged
}

fn union(parts: impl Iterator<Item = Vec<String>>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in parts.flatten() {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

impl Chunked for api::ReadRequest {
    type Response = api::ReadResponse;

    fn split(&self) -> Vec<Self> {
        split_by(&self.addresses, self, |r, a| Self { addresses: vec![a], ..r.clone() })
    }

    fn rebase(response: &mut Self::Response, offset: u32) {
        rebase_errors(&mut response.errors, offset);
    }

    fn merge(parts: Vec<Self::Response>) -> Self::Response {
        let truncated = merge_truncations(parts.iter().map(|p| p.truncated.clone()));
        let mut out = api::ReadResponse { passages: vec![], errors: vec![], truncated };
        for p in parts {
            out.passages.extend(p.passages);
            out.errors.extend(p.errors);
        }
        out
    }
}

impl Chunked for api::OutlineRequest {
    type Response = api::OutlineResponse;

    fn split(&self) -> Vec<Self> {
        split_by(&self.doc_ids, self, |r, d| Self { doc_ids: vec![d], ..r.clone() })
    }

    fn rebase(response: &mut Self::Response, offset: u32) {
        rebase_errors(&mut response.errors, offset);
    }

    fn merge(parts: Vec<Self::Response>) -> Self::Response {
        let truncated = merge_truncations(parts.iter().map(|p| p.truncated.clone()));
        let mut out = api::OutlineResponse { outlines: vec![], errors: vec![], truncated };
        for p in parts {
            out.outlines.extend(p.outlines);
            out.errors.extend(p.errors);
        }
        out
    }
}

impl Chunked for api::GrepRequest {
    type Response = api::GrepResponse;

    fn split(&self) -> Vec<Self> {
        split_by(&self.patterns, self, |r, p| Self { patterns: vec![p], ..r.clone() })
    }

    fn rebase(response: &mut Self::Response, offset: u32) {
        rebase_errors(&mut response.errors, offset);
        for hit in &mut response.hits {
            hit.pattern = hit.pattern.saturating_add(offset);
        }
    }

    fn merge(parts: Vec<Self::Response>) -> Self::Response {
        let truncated = merge_truncations(parts.iter().map(|p| p.truncated.clone()));
        let mut out = api::GrepResponse { hits: vec![], candidates: 0, in_scope: 0, errors: vec![], truncated };
        for p in parts {
            out.hits.extend(p.hits);
            out.candidates = out.candidates.saturating_add(p.candidates);
            // Every chunk searches the same scope, so the scope size is the same in each.
            out.in_scope = out.in_scope.max(p.in_scope);
            out.errors.extend(p.errors);
        }
        out
    }
}

impl Chunked for api::FindDocsRequest {
    type Response = api::FindDocsResponse;

    fn split(&self) -> Vec<Self> {
        split_by(&self.queries, self, |r, q| Self { queries: vec![q], ..r.clone() })
    }

    fn rebase(response: &mut Self::Response, offset: u32) {
        rebase_errors(&mut response.errors, offset);
    }

    fn merge(parts: Vec<Self::Response>) -> Self::Response {
        let truncated = merge_truncations(parts.iter().map(|p| p.truncated.clone()));
        let mut out = api::FindDocsResponse { results: vec![], errors: vec![], truncated };
        for p in parts {
            out.results.extend(p.results);
            out.errors.extend(p.errors);
        }
        out
    }
}

impl Chunked for api::SearchRequest {
    type Response = api::SearchResponse;

    fn split(&self) -> Vec<Self> {
        split_by(&self.queries, self, |r, q| Self { queries: vec![q], ..r.clone() })
    }

    fn rebase(response: &mut Self::Response, offset: u32) {
        rebase_errors(&mut response.errors, offset);
    }

    fn merge(parts: Vec<Self::Response>) -> Self::Response {
        let truncated = merge_truncations(parts.iter().map(|p| p.truncated.clone()));
        let legs = union(parts.iter().map(|p| p.legs.clone()));
        let degraded = union(parts.iter().map(|p| p.degraded.clone()));
        let mut out = api::SearchResponse { results: vec![], legs, degraded, errors: vec![], truncated };
        for p in parts {
            out.results.extend(p.results);
            out.errors.extend(p.errors);
        }
        out
    }
}

impl Chunked for api::DefineRequest {
    type Response = api::DefineResponse;

    fn split(&self) -> Vec<Self> {
        split_by(&self.terms, self, |r, t| Self { terms: vec![t], ..r.clone() })
    }

    fn rebase(response: &mut Self::Response, offset: u32) {
        rebase_errors(&mut response.errors, offset);
        for d in &mut response.definitions {
            d.term = d.term.saturating_add(offset);
        }
    }

    fn merge(parts: Vec<Self::Response>) -> Self::Response {
        let truncated = merge_truncations(parts.iter().map(|p| p.truncated.clone()));
        let mut out = api::DefineResponse { definitions: vec![], errors: vec![], truncated };
        for p in parts {
            out.definitions.extend(p.definitions);
            out.errors.extend(p.errors);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(index: u32) -> api::ItemError {
        api::ItemError { index, message: format!("bad {index}") }
    }

    #[test]
    fn split_makes_one_request_per_item_and_keeps_the_rest() {
        let req = api::ReadRequest {
            addresses: vec!["a".into(), "b".into(), "c".into()],
            expand: Some(api::Expand::Section),
            max_tokens: Some(500),
        };
        let parts = req.split();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].addresses, vec!["b".to_string()]);
        assert!(parts.iter().all(|p| p.expand == Some(api::Expand::Section) && p.max_tokens == Some(500)));
    }

    #[test]
    fn an_empty_batch_stays_one_request() {
        let req = api::GrepRequest {
            patterns: vec![],
            scope: api::Scope::Vault,
            context_lines: 0,
            max_hits: None,
            case_insensitive: false,
            max_tokens: None,
        };
        assert_eq!(req.split(), vec![req]);
    }

    #[test]
    fn rebase_and_merge_restore_original_indexes() {
        let req = api::DefineRequest {
            terms: vec!["x".into(), "y".into()],
            filters: Default::default(),
            k: None,
            max_tokens: None,
        };
        let passage = api::Passage {
            address: "00000000000000ff#L1".parse().unwrap(),
            title: None,
            kind: api::DocKind::Book,
            page: None,
            section: None,
            text: "t".into(),
            superseded_by: None,
        };
        let mut parts = Vec::new();
        for (i, _) in req.split().iter().enumerate() {
            let mut r = api::DefineResponse {
                definitions: vec![api::Definition { term: 0, source: "glossary".into(), passage: passage.clone() }],
                errors: vec![err(0)],
                truncated: None,
            };
            api::DefineRequest::rebase(&mut r, i as u32);
            parts.push(r);
        }
        let merged = api::DefineRequest::merge(parts);
        assert_eq!(merged.definitions.iter().map(|d| d.term).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(merged.errors.iter().map(|e| e.index).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn merged_truncations_keep_every_reason() {
        let t = |n: &str| Some(api::Truncation { reason: format!("r{n}"), next: format!("n{n}") });
        let merged = merge_truncations([t("1"), None, t("2")].into_iter()).unwrap();
        assert_eq!(merged.reason, "r1; r2");
        assert_eq!(merged.next, "n1; n2");
        assert!(merge_truncations([None, None].into_iter()).is_none());
    }

    #[test]
    fn search_merge_unions_legs_in_order() {
        let part = |legs: &[&str]| api::SearchResponse {
            results: vec![vec![]],
            legs: legs.iter().map(|s| s.to_string()).collect(),
            degraded: vec![],
            errors: vec![],
            truncated: None,
        };
        let merged = api::SearchRequest::merge(vec![part(&["bm25", "dense"]), part(&["dense", "rerank"])]);
        assert_eq!(merged.legs, vec!["bm25", "dense", "rerank"]);
        assert_eq!(merged.results.len(), 2);
    }
}
