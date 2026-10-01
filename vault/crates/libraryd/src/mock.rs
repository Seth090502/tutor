//! `MockVault`: a [`VaultApi`] over a few fixture documents, so the servers, the client and their tests work
//! before vault-core lands. It follows the contract (batches, per-item errors, token budgets, truncation
//! notices) closely enough to exercise every interface path, and does nothing clever: search is word counting,
//! grep is a plain regex over fixture lines, and nothing is stored on disk.

use regex::RegexBuilder;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use vault_api::{
    Address, ApiError, ApiResult, CategoryCoverage, CitationEdge, CitationsRequest, CitationsResponse,
    DEFAULT_MAX_TOKENS, DefineRequest, DefineResponse, Definition, Direction, DocId, DocKind, DocSummary, Expand,
    Filters, FindDocsRequest, FindDocsResponse, GrepHit, GrepRequest, GrepResponse, HARD_MAX_TOKENS, Heading,
    ItemError, Outline, OutlineRequest, OutlineResponse, PageMark, Passage, ReadRequest, ReadResponse, Scope,
    SearchHit, SearchRequest, SearchResponse, Symbol, TierDisk, Truncation, VaultApi, VaultInfo, WorkspaceAction,
    WorkspaceRequest, WorkspaceResponse,
};

pub const MOCK_VAULT_ID: &str = "mock-vault-0001";
pub const COWAN: &str = "3f9a1c07b2e4d55e-cowan-magical-number-four";
pub const TEXTBOOK: &str = "9c1d2e3f4a5b6c7d-memory-textbook-chapter";
pub const CODE: &str = "00000000000000c0-vault-src-main-rs";
pub const OLD_NOTES: &str = "d00d00d00d00d00d-old-notes";

const DEFAULT_K: usize = 10;
const DEFAULT_MAX_HITS: usize = 50;

struct Doc {
    summary: DocSummary,
    lines: Vec<String>,
    headings: Vec<Heading>,
    pages: Vec<PageMark>,
    symbols: Vec<Symbol>,
}

pub struct MockVault {
    docs: Vec<Doc>,
    workspaces: Mutex<BTreeMap<String, BTreeSet<String>>>,
}

impl Default for MockVault {
    fn default() -> Self {
        Self::new()
    }
}

fn id(s: &str) -> DocId {
    s.parse().expect("fixture ids are valid")
}

fn heading(level: u8, text: &str, start: u32, end: u32, page: &str) -> Heading {
    Heading { level, text: text.into(), start, end, page: Some(page.into()) }
}

fn lines(n: u32, label: &str, specials: &[(u32, &str)]) -> Vec<String> {
    (1..=n)
        .map(|i| match specials.iter().find(|(line, _)| *line == i) {
            Some((_, text)) => (*text).to_string(),
            None => format!("{label} line {i}: a filler sentence about short-term storage."),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn summary(doc: &str, title: &str, authors: &[&str], year: Option<i32>, kind: DocKind, lines: usize) -> DocSummary {
    DocSummary {
        doc_id: id(doc),
        title: Some(title.into()),
        authors: authors.iter().map(|a| (*a).to_string()).collect(),
        year,
        kind,
        primary_category: Some("psych-cogsci".into()),
        language: Some("en".into()),
        lines: u32::try_from(lines).unwrap_or(u32::MAX),
        score: None,
        superseded_by: None,
        repo: None,
        path: None,
    }
}

impl MockVault {
    pub fn new() -> Self {
        let cowan_lines = lines(
            60,
            "Cowan",
            &[
                (12, "The capacity limit of working memory is about four chunks."),
                (30, "Chunking lets people hold more than the magical number seven."),
                (31, "The embedded-processes model assigns attention to a focus."),
            ],
        );
        let book_lines = lines(
            80,
            "Textbook",
            &[
                (5, "Working memory: a limited-capacity system for holding and using information."),
                (6, "Chunking: recoding items into larger meaningful units."),
            ],
        );
        let code_lines =
            lines(30, "Code", &[(3, "fn main() {"), (21, "fn helper(x: u32) -> u32 {"), (22, "    x + 1")]);
        let old_lines = lines(10, "Notes", &[(2, "Superseded draft about the magical number seven.")]);

        let mut old = summary(
            OLD_NOTES,
            "Old notes on memory limits",
            &["A. Student"],
            Some(2019),
            DocKind::Notes,
            old_lines.len(),
        );
        old.superseded_by = Some(id(COWAN));
        let mut code = summary(CODE, "src/main.rs", &[], None, DocKind::Code, code_lines.len());
        code.primary_category = Some("software".into());
        code.language = Some("rust".into());
        code.repo = Some("vault".into());
        code.path = Some("src/main.rs".into());

        let docs = vec![
            Doc {
                summary: summary(
                    COWAN,
                    "The magical number 4 in short-term memory",
                    &["Nelson Cowan"],
                    Some(2001),
                    DocKind::Paper,
                    cowan_lines.len(),
                ),
                lines: cowan_lines,
                headings: vec![
                    heading(1, "1 Introduction", 1, 20, "87"),
                    heading(1, "2 Capacity limits", 21, 45, "88"),
                    heading(2, "2.1 Evidence", 21, 33, "88"),
                    heading(2, "2.2 Chunking", 34, 45, "88"),
                    heading(1, "3 Conclusion", 46, 60, "89"),
                ],
                pages: vec![
                    PageMark { label: "87".into(), index: 90, line: 1 },
                    PageMark { label: "88".into(), index: 91, line: 21 },
                    PageMark { label: "89".into(), index: 92, line: 46 },
                ],
                symbols: vec![],
            },
            Doc {
                summary: summary(
                    TEXTBOOK,
                    "Cognitive Psychology: Memory",
                    &["R. Author", "S. Writer"],
                    Some(2019),
                    DocKind::Chapter,
                    book_lines.len(),
                ),
                lines: book_lines,
                headings: vec![heading(1, "3 Memory", 1, 80, "iii")],
                pages: vec![PageMark { label: "iii".into(), index: 5, line: 1 }],
                symbols: vec![],
            },
            Doc {
                summary: code,
                lines: code_lines,
                headings: vec![],
                pages: vec![],
                symbols: vec![
                    Symbol { kind: "function".into(), name: "main".into(), start: 3, end: 20, parent: None },
                    Symbol {
                        kind: "function".into(),
                        name: "helper".into(),
                        start: 21,
                        end: 23,
                        parent: Some("main".into()),
                    },
                ],
            },
            Doc { summary: old, lines: old_lines, headings: vec![], pages: vec![], symbols: vec![] },
        ];
        Self { docs, workspaces: Mutex::new(BTreeMap::new()) }
    }

    fn find(&self, doc_id: &DocId) -> Option<&Doc> {
        self.docs.iter().find(|d| d.summary.doc_id.same_document(doc_id))
    }

    fn passage(&self, doc: &Doc, start: u32, end: u32) -> Passage {
        let text = doc.lines[(start as usize - 1)..(end as usize)].join("\n");
        let page = doc.pages.iter().rev().find(|p| p.line <= start).map(|p| p.label.clone());
        let mut path: Vec<&Heading> = doc.headings.iter().filter(|h| h.start <= start && start <= h.end).collect();
        path.sort_by_key(|h| h.level);
        let section = (!path.is_empty()).then(|| path.iter().map(|h| h.text.as_str()).collect::<Vec<_>>().join(" > "));
        Passage {
            address: Address::new(doc.summary.doc_id.clone(), start, end).expect("valid range"),
            title: doc.summary.title.clone(),
            kind: doc.summary.kind,
            page,
            section,
            text,
            superseded_by: doc.summary.superseded_by.clone(),
        }
    }

    fn summaries(&self, filters: &Filters) -> impl Iterator<Item = &Doc> {
        let filters = filters.clone();
        self.docs.iter().filter(move |d| matches(&filters, &d.summary))
    }
}

/// The filters the mock understands: category, kind, year range, language and repo.
fn matches(f: &Filters, d: &DocSummary) -> bool {
    f.category.as_ref().is_none_or(|c| d.primary_category.as_ref() == Some(c))
        && f.kind.is_none_or(|k| d.kind == k)
        && f.year_from.is_none_or(|y| d.year.is_some_and(|dy| dy >= y))
        && f.year_to.is_none_or(|y| d.year.is_some_and(|dy| dy <= y))
        && f.language.as_ref().is_none_or(|l| d.language.as_ref() == Some(l))
        && f.repo.as_ref().is_none_or(|r| d.repo.as_ref() == Some(r))
}

fn budget_chars(max_tokens: Option<u32>) -> usize {
    max_tokens.unwrap_or(DEFAULT_MAX_TOKENS).clamp(1, HARD_MAX_TOKENS) as usize * 4
}

fn non_empty<T>(items: &[T], what: &str) -> ApiResult<()> {
    if items.is_empty() { Err(ApiError::InvalidArgument(format!("{what} must not be empty"))) } else { Ok(()) }
}

fn item_error(index: usize, message: impl Into<String>) -> ItemError {
    ItemError { index: u32::try_from(index).unwrap_or(u32::MAX), message: message.into() }
}

fn words(query: &str) -> Vec<String> {
    query.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 2).map(str::to_lowercase).collect()
}

#[async_trait::async_trait]
impl VaultApi for MockVault {
    async fn search(&self, req: SearchRequest) -> ApiResult<SearchResponse> {
        non_empty(&req.queries, "queries")?;
        let k = req.k.map_or(DEFAULT_K, |k| k as usize);
        let mut results = Vec::new();
        for query in &req.queries {
            let ws = words(query);
            let mut scored: Vec<(usize, &Doc, u32)> = Vec::new();
            for doc in self.summaries(&req.filters) {
                for (i, line) in doc.lines.iter().enumerate() {
                    let lower = line.to_lowercase();
                    let score = ws.iter().filter(|w| lower.contains(w.as_str())).count();
                    if score > 0 {
                        scored.push((score, doc, i as u32 + 1));
                    }
                }
            }
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.summary.doc_id.cmp(&b.1.summary.doc_id)).then(a.2.cmp(&b.2)));
            results.push(
                scored
                    .into_iter()
                    .take(k)
                    .map(|(score, doc, line)| {
                        let mut d = doc.summary.clone();
                        d.score = Some(score as f32);
                        SearchHit { passage: self.passage(doc, line, line), score: score as f32, doc: d }
                    })
                    .collect(),
            );
        }
        Ok(SearchResponse { results, legs: vec!["bm25".into()], degraded: vec![], errors: vec![], truncated: None })
    }

    async fn find_docs(&self, req: FindDocsRequest) -> ApiResult<FindDocsResponse> {
        non_empty(&req.queries, "queries")?;
        let k = req.k.map_or(DEFAULT_K, |k| k as usize);
        let results = req
            .queries
            .iter()
            .map(|q| {
                let ws = words(q);
                let mut scored: Vec<(usize, &Doc)> = self
                    .summaries(&req.filters)
                    .filter_map(|d| {
                        let hay = format!(
                            "{} {} {}",
                            d.summary.title.as_deref().unwrap_or(""),
                            d.summary.authors.join(" "),
                            d.headings.iter().map(|h| h.text.as_str()).collect::<Vec<_>>().join(" ")
                        )
                        .to_lowercase();
                        let score = ws.iter().filter(|w| hay.contains(w.as_str())).count();
                        (score > 0).then_some((score, d))
                    })
                    .collect();
                scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.summary.doc_id.cmp(&b.1.summary.doc_id)));
                scored
                    .into_iter()
                    .take(k)
                    .map(|(score, d)| DocSummary { score: Some(score as f32), ..d.summary.clone() })
                    .collect()
            })
            .collect();
        Ok(FindDocsResponse { results, errors: vec![], truncated: None })
    }

    async fn workspace(&self, req: WorkspaceRequest) -> ApiResult<WorkspaceResponse> {
        let name = req.workspace.clone().unwrap_or_else(|| "session".into());
        let mut errors = Vec::new();
        let mut spaces = self.workspaces.lock().map_err(|_| ApiError::Internal("workspace lock poisoned".into()))?;
        let space = spaces.entry(name.clone()).or_default();
        match req.action {
            WorkspaceAction::Add | WorkspaceAction::Remove => {
                non_empty(&req.doc_ids, "doc_ids")?;
                for (i, raw) in req.doc_ids.iter().enumerate() {
                    match raw
                        .parse::<DocId>()
                        .ok()
                        .and_then(|d| self.find(&d).map(|doc| doc.summary.doc_id.hash().to_string()))
                    {
                        Some(hash) if req.action == WorkspaceAction::Add => {
                            space.insert(hash);
                        }
                        Some(hash) => {
                            space.remove(&hash);
                        }
                        None => errors.push(item_error(i, format!("unknown document {raw:?}"))),
                    }
                }
            }
            WorkspaceAction::Clear => space.clear(),
            WorkspaceAction::List => {}
        }
        let docs =
            self.docs.iter().filter(|d| space.contains(d.summary.doc_id.hash())).map(|d| d.summary.clone()).collect();
        Ok(WorkspaceResponse { workspace: name, docs, errors })
    }

    async fn grep(&self, req: GrepRequest) -> ApiResult<GrepResponse> {
        non_empty(&req.patterns, "patterns")?;
        let max_hits = req.max_hits.map_or(DEFAULT_MAX_HITS, |m| m as usize);
        let budget = budget_chars(req.max_tokens);
        let in_scope: Vec<&Doc> = match &req.scope {
            Scope::Vault => self.docs.iter().collect(),
            Scope::Category { category, .. } => {
                self.docs.iter().filter(|d| d.summary.primary_category.as_ref() == Some(category)).collect()
            }
            Scope::Docs { doc_ids } => {
                doc_ids.iter().filter_map(|d| d.parse::<DocId>().ok()).filter_map(|d| self.find(&d)).collect()
            }
            Scope::Workspace { workspace } => {
                let spaces =
                    self.workspaces.lock().map_err(|_| ApiError::Internal("workspace lock poisoned".into()))?;
                let name = workspace.clone().unwrap_or_else(|| "session".into());
                let members = spaces.get(&name).cloned().unwrap_or_default();
                self.docs.iter().filter(|d| members.contains(d.summary.doc_id.hash())).collect()
            }
        };
        let (mut hits, mut errors, mut used, mut candidates) = (Vec::new(), Vec::new(), 0usize, BTreeSet::new());
        let mut truncated = None;
        'patterns: for (p, pattern) in req.patterns.iter().enumerate() {
            let regex = match RegexBuilder::new(pattern).case_insensitive(req.case_insensitive).build() {
                Ok(r) => r,
                Err(e) => {
                    errors.push(item_error(p, format!("invalid pattern: {e}")));
                    continue;
                }
            };
            for doc in &in_scope {
                for (i, line) in doc.lines.iter().enumerate() {
                    if !regex.is_match(line) {
                        continue;
                    }
                    candidates.insert(doc.summary.doc_id.hash());
                    let n = i + 1;
                    let c = req.context_lines as usize;
                    let before: Vec<String> = doc.lines[n.saturating_sub(1 + c)..n - 1].to_vec();
                    let after: Vec<String> = doc.lines[n..(n + c).min(doc.lines.len())].to_vec();
                    let cost = line.len() + before.iter().chain(&after).map(String::len).sum::<usize>();
                    if hits.len() >= max_hits || (used + cost > budget && !hits.is_empty()) {
                        truncated = Some(Truncation {
                            reason: format!(
                                "stopped after {} hits (max_hits {max_hits}, token budget {} tokens)",
                                hits.len(),
                                budget / 4
                            ),
                            next: "narrow the scope or the pattern, or raise max_hits and max_tokens".into(),
                        });
                        break 'patterns;
                    }
                    used += cost;
                    let passage = self.passage(doc, n as u32, n as u32);
                    hits.push(GrepHit {
                        pattern: p as u32,
                        address: passage.address,
                        title: passage.title,
                        page: passage.page,
                        line: line.clone(),
                        before,
                        after,
                    });
                }
            }
        }
        Ok(GrepResponse {
            hits,
            candidates: candidates.len() as u64,
            in_scope: in_scope.len() as u64,
            errors,
            truncated,
        })
    }

    async fn read(&self, req: ReadRequest) -> ApiResult<ReadResponse> {
        non_empty(&req.addresses, "addresses")?;
        let budget = budget_chars(req.max_tokens);
        let (mut passages, mut errors, mut used) = (Vec::new(), Vec::new(), 0usize);
        let mut truncated = None;
        for (i, raw) in req.addresses.iter().enumerate() {
            let address: Address = match raw.parse() {
                Ok(a) => a,
                Err(e) => {
                    errors.push(item_error(i, e.to_string()));
                    continue;
                }
            };
            let Some(doc) = self.find(&address.doc) else {
                errors.push(item_error(i, format!("no document {}", address.doc)));
                continue;
            };
            let total = doc.lines.len() as u32;
            if address.end > total {
                errors.push(item_error(
                    i,
                    format!(
                        "lines {}-{} are outside the document, which has {total} lines",
                        address.start, address.end
                    ),
                ));
                continue;
            }
            let (start, end) = match &req.expand {
                None => (address.start, address.end),
                Some(Expand::Lines { before, after }) => {
                    (address.start.saturating_sub(*before).max(1), address.end.saturating_add(*after).min(total))
                }
                Some(Expand::Section) => doc
                    .headings
                    .iter()
                    .filter(|h| h.start <= address.start && address.end <= h.end)
                    .max_by_key(|h| h.level)
                    .map_or((address.start, address.end), |h| (h.start, h.end)),
            };
            let mut passage = self.passage(doc, start, end);
            if used + passage.text.len() > budget {
                if passages.is_empty() {
                    passage.text = passage.text.chars().take(budget).collect();
                } else {
                    let rest: Vec<&str> = req.addresses[i..].iter().map(String::as_str).collect();
                    truncated = Some(Truncation {
                        reason: format!(
                            "token budget {} reached after {} of {} addresses",
                            budget / 4,
                            passages.len(),
                            req.addresses.len()
                        ),
                        next: format!("call read with the remaining addresses: {}", rest.join(", ")),
                    });
                    break;
                }
            }
            used += passage.text.len();
            passages.push(passage);
        }
        Ok(ReadResponse { passages, errors, truncated })
    }

    async fn outline(&self, req: OutlineRequest) -> ApiResult<OutlineResponse> {
        non_empty(&req.doc_ids, "doc_ids")?;
        let (mut outlines, mut errors) = (Vec::new(), Vec::new());
        for (i, raw) in req.doc_ids.iter().enumerate() {
            match raw.parse::<DocId>().ok().and_then(|d| self.find(&d)) {
                Some(doc) => outlines.push(Outline {
                    doc: doc.summary.clone(),
                    headings: doc
                        .headings
                        .iter()
                        .filter(|h| req.max_depth.is_none_or(|d| h.level <= d))
                        .cloned()
                        .collect(),
                    pages: doc.pages.clone(),
                    symbols: doc.symbols.clone(),
                }),
                None => errors.push(item_error(i, format!("unknown document {raw:?}"))),
            }
        }
        Ok(OutlineResponse { outlines, errors, truncated: None })
    }

    async fn citations(&self, req: CitationsRequest) -> ApiResult<CitationsResponse> {
        non_empty(&req.doc_ids, "doc_ids")?;
        let k = req.k.map_or(usize::MAX, |k| k as usize);
        let book =
            self.find(&id(TEXTBOOK)).ok_or_else(|| ApiError::Internal("the fixture textbook is missing".into()))?;
        let edges = [
            CitationEdge {
                from: id(COWAN),
                to: Some(book.summary.clone()),
                as_written: "R. Author and S. Writer (2019). Cognitive Psychology: Memory.".into(),
                at: Address::new(id(COWAN), 40, 40).ok(),
                external: false,
                external_id: None,
            },
            CitationEdge {
                from: id(COWAN),
                to: None,
                as_written: "Miller, G. A. (1956). The magical number seven, plus or minus two.".into(),
                at: Address::new(id(COWAN), 41, 41).ok(),
                external: true,
                external_id: Some("doi:10.1037/h0043158".into()),
            },
        ];
        let (mut out, mut errors) = (Vec::new(), Vec::new());
        for (i, raw) in req.doc_ids.iter().enumerate() {
            let Some(doc) = raw.parse::<DocId>().ok().and_then(|d| self.find(&d)) else {
                errors.push(item_error(i, format!("unknown document {raw:?}")));
                continue;
            };
            for e in &edges {
                let cites = e.from.same_document(&doc.summary.doc_id);
                let cited_by = e.to.as_ref().is_some_and(|t| t.doc_id.same_document(&doc.summary.doc_id));
                let wanted = match req.direction {
                    Direction::Cites => cites,
                    Direction::CitedBy => cited_by,
                    Direction::Both => cites || cited_by,
                };
                if wanted && !out.contains(e) {
                    out.push(e.clone());
                }
            }
        }
        out.truncate(k);
        Ok(CitationsResponse { edges: out, errors, truncated: None })
    }

    async fn define(&self, req: DefineRequest) -> ApiResult<DefineResponse> {
        non_empty(&req.terms, "terms")?;
        let (mut definitions, mut errors) = (Vec::new(), Vec::new());
        for (t, term) in req.terms.iter().enumerate() {
            let prefix = format!("{}:", term.to_lowercase());
            let found = self.summaries(&req.filters).find_map(|d| {
                d.lines.iter().position(|l| l.to_lowercase().starts_with(&prefix)).map(|i| (d, i as u32 + 1))
            });
            match found {
                Some((doc, line)) => definitions.push(Definition {
                    term: t as u32,
                    source: "definition_passage".into(),
                    passage: self.passage(doc, line, line),
                }),
                None => errors.push(item_error(t, format!("no definition found for {term:?}"))),
            }
        }
        Ok(DefineResponse { definitions, errors, truncated: None })
    }

    async fn vault_info(&self) -> ApiResult<VaultInfo> {
        Ok(VaultInfo {
            vault_id: MOCK_VAULT_ID.into(),
            format_version: 1,
            stage: "local".into(),
            documents: self.docs.len() as u64,
            by_kind: vec![(DocKind::Paper, 1), (DocKind::Chapter, 1), (DocKind::Code, 1), (DocKind::Notes, 1)],
            quarantined: 0,
            repositories: 1,
            coverage: vec![CategoryCoverage {
                category: "psych-cogsci".into(),
                name: "Psychology and cognitive science".into(),
                papers: 1,
                books: 1,
                code_files: 0,
                target_papers: 500,
                target_books: 50,
            }],
            taxonomy_version: "mock-1".into(),
            index_version: "mock-1".into(),
            tools_available: vault_api::TOOL_NAMES.iter().map(|s| (*s).to_string()).collect(),
            disk: vec![TierDisk {
                tier: "hot".into(),
                used_bytes: 1_000,
                free_bytes: 9_000,
                total_bytes: 10_000,
                warning: false,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(addresses: &[&str], expand: Option<Expand>) -> ReadRequest {
        ReadRequest { addresses: addresses.iter().map(|s| (*s).to_string()).collect(), expand, max_tokens: None }
    }

    #[tokio::test]
    async fn read_returns_exact_lines_with_page_and_section() {
        let r = MockVault::new().read(read(&["3f9a1c07b2e4d55e#L12"], None)).await.unwrap();
        let p = &r.passages[0];
        assert_eq!(p.text, "The capacity limit of working memory is about four chunks.");
        assert_eq!(p.page.as_deref(), Some("87"));
        assert_eq!(p.section.as_deref(), Some("1 Introduction"));
        assert_eq!(p.address.to_string(), "3f9a1c07b2e4d55e-cowan-magical-number-four#L12");
    }

    #[tokio::test]
    async fn read_reports_bad_items_without_failing_the_batch() {
        let r = MockVault::new()
            .read(read(&["nope", "3f9a1c07b2e4d55e#L999", "ffffffffffffffff#L1", "3f9a1c07b2e4d55e#L1"], None))
            .await
            .unwrap();
        assert_eq!(r.passages.len(), 1);
        assert_eq!(r.errors.iter().map(|e| e.index).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn read_expands_by_lines_and_to_the_section() {
        let v = MockVault::new();
        let by_lines =
            v.read(read(&["3f9a1c07b2e4d55e#L30"], Some(Expand::Lines { before: 1, after: 2 }))).await.unwrap();
        assert_eq!(by_lines.passages[0].address.to_string(), "3f9a1c07b2e4d55e-cowan-magical-number-four#L29-L32");
        let section = v.read(read(&["3f9a1c07b2e4d55e#L30"], Some(Expand::Section))).await.unwrap();
        assert_eq!(section.passages[0].address.to_string(), "3f9a1c07b2e4d55e-cowan-magical-number-four#L21-L33");
    }

    #[tokio::test]
    async fn read_stops_at_the_token_budget_and_says_how_to_continue() {
        let r = MockVault::new()
            .read(ReadRequest {
                addresses: vec![
                    "3f9a1c07b2e4d55e#L1-L60".into(),
                    "9c1d2e3f4a5b6c7d#L1-L80".into(),
                    "3f9a1c07b2e4d55e#L2".into(),
                ],
                expand: None,
                max_tokens: Some(700),
            })
            .await
            .unwrap();
        assert_eq!(r.passages.len(), 1);
        let t = r.truncated.unwrap();
        assert!(t.reason.contains("after 1 of 3"));
        assert!(t.next.contains("9c1d2e3f4a5b6c7d#L1-L80"));
    }

    #[tokio::test]
    async fn empty_batches_are_invalid() {
        let v = MockVault::new();
        assert!(matches!(v.read(read(&[], None)).await, Err(ApiError::InvalidArgument(_))));
        assert!(matches!(
            v.find_docs(FindDocsRequest { queries: vec![], filters: Filters::default(), k: None, max_tokens: None })
                .await,
            Err(ApiError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn grep_finds_lines_with_context_and_reports_bad_patterns() {
        let v = MockVault::new();
        let r = v
            .grep(GrepRequest {
                patterns: vec!["magical number (seven|four)".into(), "(".into()],
                scope: Scope::Vault,
                context_lines: 1,
                max_hits: None,
                case_insensitive: true,
                max_tokens: None,
            })
            .await
            .unwrap();
        assert!(r.hits.iter().any(|h| h.address.start == 30 && h.before.len() == 1 && h.after.len() == 1));
        assert!(r.hits.iter().all(|h| h.pattern == 0));
        assert_eq!(r.errors.len(), 1);
        assert_eq!(r.errors[0].index, 1);
        assert_eq!(r.in_scope, 4);
    }

    #[tokio::test]
    async fn find_docs_ranks_and_filters() {
        let v = MockVault::new();
        let q = |filters: Filters| FindDocsRequest {
            queries: vec!["cowan magical number".into(), "textbook".into()],
            filters,
            k: None,
            max_tokens: None,
        };
        let r = v.find_docs(q(Filters::default())).await.unwrap();
        assert_eq!(r.results.len(), 2);
        assert_eq!(r.results[0][0].doc_id.hash(), "3f9a1c07b2e4d55e");
        let none = v.find_docs(q(Filters { year_from: Some(2030), ..Filters::default() })).await.unwrap();
        assert!(none.results.iter().all(Vec::is_empty));
    }

    #[tokio::test]
    async fn workspace_adds_lists_removes_and_clears() {
        let v = MockVault::new();
        let call = |action, ids: &[&str]| WorkspaceRequest {
            action,
            doc_ids: ids.iter().map(|s| (*s).to_string()).collect(),
            workspace: None,
        };
        let added = v.workspace(call(WorkspaceAction::Add, &["3f9a1c07b2e4d55e", "zzz"])).await.unwrap();
        assert_eq!(added.docs.len(), 1);
        assert_eq!(added.errors.len(), 1);
        let scoped = v
            .grep(GrepRequest {
                patterns: vec!["four".into()],
                scope: Scope::Workspace { workspace: None },
                context_lines: 0,
                max_hits: None,
                case_insensitive: false,
                max_tokens: None,
            })
            .await
            .unwrap();
        assert_eq!(scoped.in_scope, 1);
        assert_eq!(v.workspace(call(WorkspaceAction::Remove, &["3f9a1c07b2e4d55e"])).await.unwrap().docs.len(), 0);
        v.workspace(call(WorkspaceAction::Add, &["3f9a1c07b2e4d55e"])).await.unwrap();
        assert_eq!(v.workspace(call(WorkspaceAction::Clear, &[])).await.unwrap().docs.len(), 0);
    }

    #[tokio::test]
    async fn outline_citations_define_and_info_answer() {
        let v = MockVault::new();
        let o = v
            .outline(OutlineRequest {
                doc_ids: vec![COWAN.into(), CODE.into(), "bad".into()],
                max_depth: Some(1),
                max_tokens: None,
            })
            .await
            .unwrap();
        assert_eq!(o.outlines.len(), 2);
        assert_eq!(o.outlines[0].headings.len(), 3);
        assert_eq!(o.outlines[1].symbols.len(), 2);
        assert_eq!(o.errors.len(), 1);
        let c = v
            .citations(CitationsRequest {
                doc_ids: vec![COWAN.into()],
                direction: Direction::Cites,
                k: None,
                max_tokens: None,
            })
            .await
            .unwrap();
        assert_eq!(c.edges.len(), 2);
        let cb = v
            .citations(CitationsRequest {
                doc_ids: vec![TEXTBOOK.into()],
                direction: Direction::CitedBy,
                k: None,
                max_tokens: None,
            })
            .await
            .unwrap();
        assert_eq!(cb.edges.len(), 1);
        let d = v
            .define(DefineRequest {
                terms: vec!["working memory".into(), "unobtainium".into()],
                filters: Filters::default(),
                k: None,
                max_tokens: None,
            })
            .await
            .unwrap();
        assert_eq!(d.definitions.len(), 1);
        assert_eq!(d.errors[0].index, 1);
        let info = v.vault_info().await.unwrap();
        assert_eq!(info.vault_id, MOCK_VAULT_ID);
        assert_eq!(info.tools_available.len(), 9);
    }

    #[tokio::test]
    async fn search_scores_by_matching_words() {
        let v = MockVault::new();
        let r = v
            .search(SearchRequest {
                queries: vec!["capacity of working memory four chunks".into()],
                filters: Filters::default(),
                k: Some(5),
                max_tokens: None,
            })
            .await
            .unwrap();
        assert_eq!(r.results[0].len(), 2, "only the Cowan line and the textbook definition share words");
        assert_eq!(r.results[0][0].passage.address.start, 12);
        assert!(r.results[0].windows(2).all(|w| w[0].score >= w[1].score));
    }
}
