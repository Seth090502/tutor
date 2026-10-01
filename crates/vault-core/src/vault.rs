//! The Phase 1 tools (SPEC section 10): `read`, `outline`, `find_docs` and `vault_info`, plus
//! `NotYetAvailable` for the tools of later phases.
//!
//! Tools accept doc ids and addresses only, never paths. Every response respects its token budget (4 characters
//! per token) and says when, and how, it truncated. Text is served only after its BLAKE3 matches the hash
//! recorded at publication, so a damaged file is reported rather than quoted.

use crate::canon::split_lines;
use crate::config::VaultConfig;
use crate::db::{self, DOC_COLUMNS, DocRow, doc_from_row, kind_str, parse_kind};
use crate::error::{CoreError, Result};
use crate::ids;
use crate::paths::RelPath;
use crate::sidecar::Sidecar;
use crate::store;
use rusqlite::types::ToSql;
use rusqlite::{Connection, ErrorCode};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use vault_api::*;

/// The tools that work in this build.
pub const TOOLS_AVAILABLE: [&str; 4] = ["read", "outline", "find_docs", "vault_info"];
const DEFAULT_K: u32 = 10;
const MAX_K: u32 = 100;
const MAX_QUERY_TERMS: usize = 32;

/// Idle reader connections kept for reuse.
const POOL_IDLE_MAX: usize = 8;

pub struct Vault {
    config: VaultConfig,
    db_path: PathBuf,
    idle: Mutex<Vec<Connection>>,
}

fn internal(e: CoreError) -> ApiError {
    match &e {
        CoreError::Db(rusqlite::Error::SqliteFailure(f, _))
            if matches!(f.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) =>
        {
            ApiError::Unavailable("the vault database is busy; retry shortly".into())
        }
        CoreError::InvalidArgument(m) => ApiError::InvalidArgument(m.clone()),
        _ => {
            tracing::error!(error = %e, "tool failed");
            ApiError::Internal("the vault could not complete this request; the server log has the details".into())
        }
    }
}

/// Characters allowed by `max_tokens`.
struct Budget {
    max_tokens: u32,
    limit: usize,
    used: usize,
}

impl Budget {
    fn new(max_tokens: Option<u32>) -> ApiResult<Budget> {
        let t = max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        if t == 0 {
            return Err(ApiError::InvalidArgument("max_tokens must be at least 1".into()));
        }
        let t = t.min(HARD_MAX_TOKENS);
        Ok(Budget { max_tokens: t, limit: t as usize * 4, used: 0 })
    }

    fn fits(&self, chars: usize) -> bool {
        self.used + chars <= self.limit
    }

    fn take(&mut self, chars: usize) {
        self.used += chars;
    }
}

fn json_chars<T: serde::Serialize>(v: &T) -> usize {
    serde_json::to_string(v).map(|s| s.chars().count()).unwrap_or(usize::MAX / 4)
}

/// A document's text, line index and sidecar, loaded once per request.
struct Loaded {
    row: DocRow,
    doc_id: DocId,
    text: String,
    /// Byte range of each line.
    lines: Vec<(usize, usize)>,
    sidecar: Option<Sidecar>,
}

impl Loaded {
    fn line(&self, n: u32) -> &str {
        let (a, b) = self.lines[(n - 1) as usize];
        &self.text[a..b]
    }

    fn range_text(&self, start: u32, end: u32) -> &str {
        let a = self.lines[(start - 1) as usize].0;
        let b = self.lines[(end - 1) as usize].1;
        &self.text[a..b]
    }

    fn line_count(&self) -> u32 {
        u32::try_from(self.lines.len()).unwrap_or(u32::MAX)
    }
}

impl Vault {
    /// Opens a vault for reading. The database must exist (intake creates it) and belong to this vault.
    pub fn open(config: VaultConfig) -> Result<Vault> {
        let rel = RelPath::new("db/vault.sqlite")?;
        let path = config.layout.hot.resolve_if_exists(&rel)?.ok_or_else(|| {
            CoreError::Config(
                "hot/db/vault.sqlite does not exist yet; run intake once to create the vault database".into(),
            )
        })?;
        let conn = db::open_reader(&path)?;
        if let Some(id) = db::get_meta(&conn, "vault_id")?
            && id != config.vault.vault_id
        {
            return Err(CoreError::Config(format!(
                "the database belongs to vault {id} but vault.toml says {}",
                config.vault.vault_id
            )));
        }
        Ok(Vault { config, db_path: path, idle: Mutex::new(vec![conn]) })
    }

    pub fn config(&self) -> &VaultConfig {
        &self.config
    }

    /// Runs `f` with a query-only connection from the pool (opened on demand) and returns it to the pool, so
    /// concurrent tool calls do not wait on each other.
    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> ApiResult<T>) -> ApiResult<T> {
        let idle = self.idle.lock().unwrap_or_else(|p| p.into_inner()).pop();
        let conn = match idle {
            Some(c) => c,
            None => db::open_reader(&self.db_path).map_err(internal)?,
        };
        let out = f(&conn);
        let mut idle = self.idle.lock().unwrap_or_else(|p| p.into_inner());
        if idle.len() < POOL_IDLE_MAX {
            idle.push(conn);
        }
        out
    }

    pub fn read_sync(&self, req: &ReadRequest) -> ApiResult<ReadResponse> {
        self.with_conn(|conn| self.read_with(conn, req))
    }

    pub fn outline_sync(&self, req: &OutlineRequest) -> ApiResult<OutlineResponse> {
        self.with_conn(|conn| self.outline_with(conn, req))
    }

    pub fn find_docs_sync(&self, req: &FindDocsRequest) -> ApiResult<FindDocsResponse> {
        self.with_conn(|conn| self.find_docs_with(conn, req))
    }

    pub fn vault_info_sync(&self) -> ApiResult<VaultInfo> {
        self.with_conn(|conn| self.vault_info_with(conn))
    }

    fn load_sidecar(&self, doc_id: &DocId) -> Option<Sidecar> {
        let rel = ids::sidecar_rel_path(doc_id).ok()?;
        match store::read(&self.config.layout.hot, &rel).and_then(|b| Sidecar::from_json(&b)) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!(doc_id = %doc_id, error = %e, "sidecar unavailable; pages and sections omitted");
                None
            }
        }
    }

    /// Loads a document by any form of its id. `Ok(Err(msg))` is a per-item problem.
    fn load(&self, conn: &Connection, doc: &DocId, with_text: bool) -> Result<std::result::Result<Loaded, String>> {
        let Some(row) = db::doc_by_hash(conn, doc.hash())? else {
            return Ok(Err(format!("unknown document {}", doc.hash())));
        };
        let doc_id: DocId =
            row.doc_id.parse().map_err(|e| CoreError::InvalidArgument(format!("stored doc_id: {e}")))?;
        let (text, lines) = if with_text {
            let rel = RelPath::new(&row.rel_path)?;
            let bytes = match store::read(&self.config.layout.hot, &rel) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!(doc_id = %doc_id, error = %e, "stored text unreadable");
                    return Ok(Err(format!("the stored text of {doc_id} cannot be read; run `vault verify`")));
                }
            };
            if ids::blake3_hex(&bytes) != row.text_blake3 {
                tracing::error!(doc_id = %doc_id, "stored text failed its integrity check");
                return Ok(Err(format!("the stored text of {doc_id} failed its integrity check; run `vault verify`")));
            }
            let text =
                String::from_utf8(bytes).map_err(|_| CoreError::NotCanonical(format!("{doc_id} is not UTF-8")))?;
            let mut ranges = Vec::new();
            let mut offset = 0;
            for l in split_lines(&text) {
                ranges.push((offset, offset + l.len()));
                offset += l.len() + 1;
            }
            (text, ranges)
        } else {
            (String::new(), Vec::new())
        };
        let sidecar = self.load_sidecar(&doc_id);
        Ok(Ok(Loaded { row, doc_id, text, lines, sidecar }))
    }

    fn summary(&self, conn: &Connection, row: &DocRow, score: Option<f32>) -> Result<DocSummary> {
        let (repo, path) = if row.kind == DocKind::Code {
            match db::repo_of(conn, &row.doc_id)? {
                Some((r, p)) => (Some(r), Some(p)),
                None => (None, None),
            }
        } else {
            (None, None)
        };
        Ok(DocSummary {
            doc_id: row.doc_id.parse().map_err(|e| CoreError::InvalidArgument(format!("stored doc_id: {e}")))?,
            title: row.title.clone(),
            authors: row.authors.clone(),
            year: row.year,
            kind: row.kind,
            primary_category: row.primary_category.clone(),
            language: row.language.clone(),
            lines: row.lines,
            score,
            superseded_by: row.superseded_by.as_deref().and_then(|s| s.parse().ok()),
            repo,
            path,
        })
    }

    // ------------------------------------------------------------------------------------------------- read

    fn read_with(&self, conn: &Connection, req: &ReadRequest) -> ApiResult<ReadResponse> {
        if req.addresses.is_empty() {
            return Err(ApiError::InvalidArgument("addresses must not be empty".into()));
        }
        let mut budget = Budget::new(req.max_tokens)?;
        let mut cache: HashMap<String, std::result::Result<Rc<Loaded>, String>> = HashMap::new();
        let mut passages = Vec::new();
        let mut errors = Vec::new();
        let mut rest_expanded: Vec<String> = Vec::new();
        let mut rest_original: Vec<String> = Vec::new();
        let mut stopped = false;
        for (i, raw) in req.addresses.iter().enumerate() {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            if stopped {
                rest_original.push(raw.clone());
                continue;
            }
            let addr: Address = match raw.parse() {
                Ok(a) => a,
                Err(e) => {
                    errors.push(ItemError { index, message: e.to_string() });
                    continue;
                }
            };
            let doc = match cache.get(addr.doc.hash()) {
                Some(d) => d.clone(),
                None => {
                    let d = self.load(conn, &addr.doc, true).map_err(internal)?.map(Rc::new);
                    cache.insert(addr.doc.hash().to_string(), d.clone());
                    d
                }
            };
            let doc = match doc {
                Ok(d) => d,
                Err(message) => {
                    errors.push(ItemError { index, message });
                    continue;
                }
            };
            let n = doc.line_count();
            if addr.end > n {
                errors.push(ItemError {
                    index,
                    message: format!(
                        "{raw}: {} has lines 1-{n}; the address asks for up to line {}",
                        doc.doc_id, addr.end
                    ),
                });
                continue;
            }
            let (start, end) = expand(&doc, addr.start, addr.end, req.expand.as_ref());
            let page = page_label(doc.sidecar.as_ref(), start);
            let section = section_path(&doc, start);
            let overhead = 96
                + doc.doc_id.to_string().len() * 2
                + doc.row.title.as_deref().map_or(0, |t| t.chars().count())
                + section.as_deref().map_or(0, |s| s.chars().count())
                + page.as_deref().map_or(0, str::len);
            let mut fit_end = start - 1;
            let mut cost = overhead;
            for l in start..=end {
                let c = doc.line(l).chars().count() + 1;
                if !budget.fits(cost + c) {
                    break;
                }
                cost += c;
                fit_end = l;
            }
            if fit_end >= start {
                budget.take(cost);
                passages.push(self.passage(&doc, start, fit_end, page, section)?);
            }
            if fit_end < end {
                stopped = true;
                if fit_end < start && passages.is_empty() {
                    errors.push(ItemError {
                        index,
                        message: format!(
                            "{raw}: no passage starting at line {start} fits in max_tokens {}; ask for more tokens (up to {HARD_MAX_TOKENS})",
                            budget.max_tokens
                        ),
                    });
                } else {
                    let rest = Address::new(doc.doc_id.clone(), fit_end + 1, end)
                        .map_err(|e| ApiError::Internal(e.to_string()))?;
                    rest_expanded.push(rest.to_string());
                }
            }
        }
        let truncated = stopped.then(|| {
            let mut next = Vec::new();
            if !rest_expanded.is_empty() {
                next.push(format!("call read with addresses {rest_expanded:?} and no expand"));
            }
            if !rest_original.is_empty() {
                next.push(format!("call read with addresses {rest_original:?} and the same expand"));
            }
            Truncation {
                reason: format!("token budget {} reached after {} passage(s)", budget.max_tokens, passages.len()),
                next: if next.is_empty() {
                    "nothing more can be returned for this request".into()
                } else {
                    next.join("; then ")
                },
            }
        });
        Ok(ReadResponse { passages, errors, truncated })
    }

    fn passage(
        &self,
        doc: &Loaded,
        start: u32,
        end: u32,
        page: Option<String>,
        section: Option<String>,
    ) -> ApiResult<Passage> {
        Ok(Passage {
            address: Address::new(doc.doc_id.clone(), start, end).map_err(|e| ApiError::Internal(e.to_string()))?,
            title: doc.row.title.clone(),
            kind: doc.row.kind,
            page,
            section,
            text: doc.range_text(start, end).to_string(),
            superseded_by: doc.row.superseded_by.as_deref().and_then(|s| s.parse().ok()),
        })
    }

    // ---------------------------------------------------------------------------------------------- outline

    fn outline_with(&self, conn: &Connection, req: &OutlineRequest) -> ApiResult<OutlineResponse> {
        if req.doc_ids.is_empty() {
            return Err(ApiError::InvalidArgument("doc_ids must not be empty".into()));
        }
        let mut budget = Budget::new(req.max_tokens)?;
        let mut outlines = Vec::new();
        let mut errors = Vec::new();
        let mut notes = Vec::new();
        let mut rest = Vec::new();
        for (i, raw) in req.doc_ids.iter().enumerate() {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            if !rest.is_empty() {
                rest.push(raw.clone());
                continue;
            }
            let id: DocId = match raw.parse() {
                Ok(d) => d,
                Err(e) => {
                    errors.push(ItemError { index, message: e.to_string() });
                    continue;
                }
            };
            let doc = match self.load(conn, &id, false).map_err(internal)? {
                Ok(d) => d,
                Err(message) => {
                    errors.push(ItemError { index, message });
                    continue;
                }
            };
            let sidecar = doc.sidecar.clone();
            let mut outline = Outline {
                doc: self.summary(conn, &doc.row, None).map_err(internal)?,
                headings: sidecar.as_ref().map(|s| s.headings.clone()).unwrap_or_default(),
                pages: sidecar.as_ref().map(|s| s.pages.clone()).unwrap_or_default(),
                symbols: sidecar.as_ref().and_then(|s| s.code.as_ref()).map(|c| c.symbols.clone()).unwrap_or_default(),
            };
            if let Some(depth) = req.max_depth {
                outline.headings.retain(|h| h.level <= depth);
            }
            // The first outline is shrunk to fit; later ones that do not fit are left for another call.
            if outlines.is_empty() {
                let room = budget.limit.saturating_sub(budget.used);
                if let Some(note) = shrink_outline(&mut outline, room) {
                    notes.push(format!("{}: {note}", doc.doc_id));
                }
            }
            let cost = json_chars(&outline);
            if !budget.fits(cost) {
                rest.push(raw.clone());
                continue;
            }
            budget.take(cost);
            outlines.push(outline);
        }
        let truncated = (!notes.is_empty() || !rest.is_empty()).then(|| {
            let mut next = notes.clone();
            if !rest.is_empty() {
                next.push(format!("call outline with doc_ids {rest:?}"));
            }
            Truncation {
                reason: format!("token budget {} reached after {} outline(s)", budget.max_tokens, outlines.len()),
                next: next.join("; "),
            }
        });
        Ok(OutlineResponse { outlines, errors, truncated })
    }

    // -------------------------------------------------------------------------------------------- find_docs

    fn find_docs_with(&self, conn: &Connection, req: &FindDocsRequest) -> ApiResult<FindDocsResponse> {
        if req.queries.is_empty() {
            return Err(ApiError::InvalidArgument("queries must not be empty".into()));
        }
        let f = &req.filters;
        if f.area.is_some() && f.category.as_deref().is_some_and(|c| c != "psych-cogsci") {
            return Err(ApiError::InvalidArgument("area filters apply to category psych-cogsci only".into()));
        }
        let k = req.k.unwrap_or(DEFAULT_K).clamp(1, MAX_K);
        let mut budget = Budget::new(req.max_tokens)?;
        let mut results = Vec::new();
        let mut errors = Vec::new();
        let mut cut = Vec::new();
        for (i, q) in req.queries.iter().enumerate() {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            let Some(fts) = fts_query(q) else {
                errors.push(ItemError { index, message: format!("{q:?} has no words to search for") });
                results.push(Vec::new());
                continue;
            };
            let rows = find(conn, &fts, f, k).map_err(internal)?;
            let mut list = Vec::new();
            for (row, rank) in rows {
                let s = self.summary(conn, &row, Some(rank)).map_err(internal)?;
                let cost = json_chars(&s);
                if !budget.fits(cost) {
                    cut.push(index);
                    break;
                }
                budget.take(cost);
                list.push(s);
            }
            results.push(list);
        }
        let truncated = (!cut.is_empty()).then(|| Truncation {
            reason: format!("token budget {} reached; results cut for queries {cut:?}", budget.max_tokens),
            next: "ask again with fewer queries, a smaller k, or a larger max_tokens".into(),
        });
        Ok(FindDocsResponse { results, errors, truncated })
    }

    // ------------------------------------------------------------------------------------------- vault_info

    fn vault_info_with(&self, conn: &Connection) -> ApiResult<VaultInfo> {
        let count = |sql: &str| -> ApiResult<u64> {
            let n: i64 = conn.query_row(sql, [], |r| r.get(0)).map_err(|e| internal(e.into()))?;
            Ok(u64::try_from(n).unwrap_or(0))
        };
        let documents = count("SELECT count(*) FROM documents WHERE superseded_by IS NULL")?;
        let quarantined = count("SELECT count(DISTINCT blake3) FROM quarantine")?;
        let repositories = count("SELECT count(*) FROM repos")?;
        let mut by_kind = Vec::new();
        let mut per_category: HashMap<(String, String), u64> = HashMap::new();
        {
            let mut stmt = conn
                .prepare("SELECT kind, coalesce(primary_category, ''), count(*) FROM documents WHERE superseded_by IS NULL GROUP BY 1, 2")
                .map_err(|e| internal(e.into()))?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))
                .map_err(|e| internal(e.into()))?;
            let mut kinds: HashMap<String, u64> = HashMap::new();
            for row in rows {
                let (kind, cat, n) = row.map_err(|e| internal(e.into()))?;
                let n = u64::try_from(n).unwrap_or(0);
                *kinds.entry(kind.clone()).or_default() += n;
                *per_category.entry((cat, kind)).or_default() += n;
            }
            for k in db::ALL_KINDS {
                if let Some(n) = kinds.get(kind_str(k)) {
                    by_kind.push((k, *n));
                }
            }
            for name in kinds.keys().filter(|k| parse_kind(k).is_none()) {
                tracing::warn!(kind = %name, "documents with an unknown kind are counted in no kind");
            }
        }
        let taxonomy = self.taxonomy();
        let coverage = taxonomy
            .as_ref()
            .map(|t| {
                t.categories
                    .iter()
                    .map(|c| {
                        let n = |kind: DocKind| {
                            per_category.get(&(c.slug.clone(), kind_str(kind).to_string())).copied().unwrap_or(0)
                        };
                        CategoryCoverage {
                            category: c.slug.clone(),
                            name: c.name.clone(),
                            papers: n(DocKind::Paper),
                            books: n(DocKind::Book),
                            code_files: n(DocKind::Code),
                            target_papers: c.targets.pilot.papers,
                            target_books: c.targets.pilot.books,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let taxonomy_version = db::get_meta(conn, "taxonomy_version")
            .map_err(internal)?
            .or_else(|| self.config.vault.taxonomy.version.clone())
            .or_else(|| taxonomy.as_ref().map(|t| t.version.clone()))
            .unwrap_or_default();
        let index_version = db::get_meta(conn, "index_version").map_err(internal)?.unwrap_or_else(|| "none".into());
        let disk = self
            .config
            .layout
            .tiers()
            .iter()
            .map(|t| match fs4::statvfs(t.path()) {
                Ok(s) => {
                    let used = s.total_space().saturating_sub(s.free_space());
                    TierDisk {
                        tier: t.name().to_string(),
                        used_bytes: used,
                        free_bytes: s.available_space(),
                        total_bytes: s.total_space(),
                        warning: s.total_space() > 0 && used.saturating_mul(100) >= s.total_space().saturating_mul(85),
                    }
                }
                Err(e) => {
                    tracing::warn!(tier = t.name(), error = %e, "disk usage unavailable");
                    TierDisk {
                        tier: t.name().to_string(),
                        used_bytes: 0,
                        free_bytes: 0,
                        total_bytes: 0,
                        warning: false,
                    }
                }
            })
            .collect();
        Ok(VaultInfo {
            vault_id: self.config.vault.vault_id.clone(),
            format_version: self.config.vault.format_version,
            stage: self.config.machine.stage.as_str().to_string(),
            documents,
            by_kind,
            quarantined,
            repositories,
            coverage,
            taxonomy_version,
            index_version,
            tools_available: TOOLS_AVAILABLE.iter().map(|s| s.to_string()).collect(),
            disk,
        })
    }

    fn taxonomy(&self) -> Option<Taxonomy> {
        let rel = RelPath::new("meta/taxonomy/categories.json").ok()?;
        match store::read(&self.config.layout.hot, &rel) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::warn!(error = %e, "categories.json does not parse; coverage omitted");
                    None
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "categories.json unavailable; coverage omitted");
                None
            }
        }
    }
}

// --------------------------------------------------------------------------------------------- helpers

#[derive(Deserialize)]
struct Taxonomy {
    #[serde(default)]
    version: String,
    categories: Vec<TaxonomyCategory>,
}

#[derive(Deserialize)]
struct TaxonomyCategory {
    slug: String,
    name: String,
    #[serde(default)]
    targets: Targets,
}

#[derive(Deserialize, Default)]
struct Targets {
    #[serde(default)]
    pilot: Target,
}

#[derive(Deserialize, Default)]
struct Target {
    #[serde(default)]
    papers: u64,
    #[serde(default)]
    books: u64,
}

/// Spans that may serve as a section: headings, and for code, symbols.
fn section_spans(doc: &Loaded) -> Vec<(u32, u32, u8, String)> {
    let Some(s) = &doc.sidecar else { return Vec::new() };
    let mut spans: Vec<(u32, u32, u8, String)> =
        s.headings.iter().map(|h| (h.start, h.end, h.level, h.text.clone())).collect();
    if let Some(code) = &s.code {
        spans.extend(code.symbols.iter().map(|sym| (sym.start, sym.end, 7, sym.name.clone())));
    }
    spans
}

fn expand(doc: &Loaded, start: u32, end: u32, how: Option<&Expand>) -> (u32, u32) {
    let n = doc.line_count();
    match how {
        None => (start, end),
        Some(Expand::Lines { before, after }) => {
            (start.saturating_sub(*before).max(1), end.saturating_add(*after).min(n))
        }
        Some(Expand::Section) => section_spans(doc)
            .into_iter()
            .filter(|(s, e, _, _)| *s <= start && *e >= end && *e <= n)
            .min_by_key(|(s, e, level, _)| (e - s, std::cmp::Reverse(*level)))
            .map_or((start, end), |(s, e, _, _)| (s, e)),
    }
}

fn page_label(sidecar: Option<&Sidecar>, line: u32) -> Option<String> {
    sidecar?.pages.iter().take_while(|p| p.line <= line).last().map(|p| p.label.clone())
}

fn section_path(doc: &Loaded, line: u32) -> Option<String> {
    let mut chain: Vec<(u32, u32, u8, String)> =
        section_spans(doc).into_iter().filter(|(s, e, _, _)| *s <= line && line <= *e).collect();
    if chain.is_empty() {
        return None;
    }
    chain.sort_by_key(|(s, e, level, _)| (*s, std::cmp::Reverse(*e), *level));
    Some(chain.into_iter().map(|(_, _, _, t)| t).collect::<Vec<_>>().join(" > "))
}

/// Shrinks an outline to `room` characters: thin the page list, then drop the deepest headings, then cut the
/// lists. Returns what was cut.
fn shrink_outline(o: &mut Outline, room: usize) -> Option<String> {
    if json_chars(o) <= room {
        return None;
    }
    let mut notes = Vec::new();
    if o.pages.len() > 1 {
        let per_page = json_chars(&o.pages) / o.pages.len() + 1;
        let keep = (room / 4 / per_page).max(1);
        if keep < o.pages.len() {
            let step = o.pages.len().div_ceil(keep);
            let total = o.pages.len();
            o.pages = o.pages.iter().step_by(step).cloned().collect();
            notes.push(format!("page marks thinned to every {step}th of {total}"));
        }
    }
    let mut depth = o.headings.iter().map(|h| h.level).max().unwrap_or(0);
    let deepest = depth;
    while json_chars(o) > room && depth > 1 {
        depth -= 1;
        o.headings.retain(|h| h.level <= depth);
    }
    if depth < deepest {
        notes.push(format!("headings deeper than level {depth} omitted (outline again with max_depth and fewer doc_ids, or more max_tokens)"));
    }
    let before = o.headings.len() + o.symbols.len() + o.pages.len();
    while json_chars(o) > room && (!o.headings.is_empty() || !o.symbols.is_empty() || !o.pages.is_empty()) {
        o.symbols.truncate(o.symbols.len() / 2);
        o.headings.truncate(o.headings.len() / 2);
        o.pages.truncate(o.pages.len() / 2);
    }
    let after = o.headings.len() + o.symbols.len() + o.pages.len();
    if after < before {
        notes.push(format!("entries cut to the first {after} of {before}; ask for more max_tokens"));
    }
    (!notes.is_empty()).then(|| notes.join("; "))
}

/// Words of a free-text query as an FTS5 OR query, or `None` if it has none.
fn fts_query(q: &str) -> Option<String> {
    let words: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(MAX_QUERY_TERMS)
        .map(|w| format!("\"{w}\""))
        .collect();
    (!words.is_empty()).then(|| words.join(" OR "))
}

fn find(conn: &Connection, fts: &str, f: &Filters, k: u32) -> Result<Vec<(DocRow, f32)>> {
    let mut sql = format!(
        "SELECT {DOC_COLUMNS}, bm25(doc_fts, 0.0, 10.0, 5.0, 2.0, 1.0, 3.0) AS rank \
         FROM doc_fts JOIN documents d ON d.id = doc_fts.rowid WHERE doc_fts MATCH ?1"
    );
    let mut params: Vec<Box<dyn ToSql>> = vec![Box::new(fts.to_string())];
    let mut add = |clause: &str, value: Box<dyn ToSql>| {
        params.push(value);
        sql.push_str(&clause.replace("?N", &format!("?{}", params.len())));
    };
    if let Some(c) = &f.category {
        add(
            " AND (d.primary_category = ?N OR EXISTS (SELECT 1 FROM doc_categories c WHERE c.doc_id = d.doc_id \
             AND c.category = ?N AND c.role IN ('primary','secondary')))",
            Box::new(c.clone()),
        );
    }
    if let Some(a) = &f.area {
        add(" AND EXISTS (SELECT 1 FROM doc_areas a WHERE a.doc_id = d.doc_id AND a.area = ?N)", Box::new(a.clone()));
    }
    if let Some(kind) = f.kind {
        add(" AND d.kind = ?N", Box::new(kind_str(kind)));
    }
    if let Some(l) = &f.level {
        add(" AND d.level = ?N", Box::new(l.clone()));
    }
    if let Some(y) = f.year_from {
        add(" AND d.year >= ?N", Box::new(y));
    }
    if let Some(y) = f.year_to {
        add(" AND d.year <= ?N", Box::new(y));
    }
    if let Some(l) = &f.language {
        add(" AND d.language = ?N", Box::new(l.clone()));
    }
    if let Some(t) = &f.openalex_topic {
        add(
            " AND EXISTS (SELECT 1 FROM doc_categories c WHERE c.doc_id = d.doc_id AND c.role = 'topic' AND c.category = ?N)",
            Box::new(t.clone()),
        );
    }
    if let Some(b) = &f.batch {
        add(
            " AND EXISTS (SELECT 1 FROM deliveries x WHERE x.batch_id = ?N AND (x.doc_id = d.doc_id OR x.doc_id = d.hash16))",
            Box::new(b.clone()),
        );
    }
    if let Some(r) = &f.repo {
        add(
            " AND EXISTS (SELECT 1 FROM repo_files rf JOIN repos r ON r.repo_id = rf.repo_id WHERE rf.doc_id = d.doc_id \
             AND (r.name = ?N OR r.repo_id = ?N))",
            Box::new(r.clone()),
        );
    }
    add(" ORDER BY rank LIMIT ?N", Box::new(k));
    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), |r| {
        let rank: f64 = r.get(13)?;
        Ok((doc_from_row(r)?, -rank as f32))
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[async_trait::async_trait]
impl VaultApi for Vault {
    async fn search(&self, _req: SearchRequest) -> ApiResult<SearchResponse> {
        Err(ApiError::NotYetAvailable(
            "search arrives in Phase 3 (BM25) and Phase 4 (dense); use find_docs meanwhile".into(),
        ))
    }

    async fn find_docs(&self, req: FindDocsRequest) -> ApiResult<FindDocsResponse> {
        self.find_docs_sync(&req)
    }

    async fn workspace(&self, _req: WorkspaceRequest) -> ApiResult<WorkspaceResponse> {
        Err(ApiError::NotYetAvailable("workspace arrives in Phase 5".into()))
    }

    async fn grep(&self, _req: GrepRequest) -> ApiResult<GrepResponse> {
        Err(ApiError::NotYetAvailable(
            "grep arrives in Phase 1 wave 2 (trigram index); use outline and read meanwhile".into(),
        ))
    }

    async fn read(&self, req: ReadRequest) -> ApiResult<ReadResponse> {
        self.read_sync(&req)
    }

    async fn outline(&self, req: OutlineRequest) -> ApiResult<OutlineResponse> {
        self.outline_sync(&req)
    }

    async fn citations(&self, _req: CitationsRequest) -> ApiResult<CitationsResponse> {
        Err(ApiError::NotYetAvailable("citations arrives in Phase 5".into()))
    }

    async fn define(&self, _req: DefineRequest) -> ApiResult<DefineResponse> {
        Err(ApiError::NotYetAvailable("define arrives in Phase 5".into()))
    }

    async fn vault_info(&self) -> ApiResult<VaultInfo> {
        self.vault_info_sync()
    }
}

#[cfg(test)]
mod tests;
