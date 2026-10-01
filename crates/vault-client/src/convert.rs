//! Conversions between the `vault-api` types and the protobuf messages in `proto/vault.proto`.
//!
//! `vault-api` to protobuf cannot fail (`From`). Protobuf to `vault-api` validates what the wire format cannot
//! express (ids and addresses, the zero value of enums, missing required messages, out-of-range integers) and
//! fails with [`ConvertError`] (`TryFrom`).

use crate::proto as pb;
use vault_api as api;

/// A protobuf message that does not describe a valid `vault-api` value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {field}: {message}")]
pub struct ConvertError {
    pub field: &'static str,
    pub message: String,
}

impl ConvertError {
    fn new(field: &'static str, message: impl ToString) -> Self {
        Self { field, message: message.to_string() }
    }
}

type Res<T> = Result<T, ConvertError>;

fn doc_id(field: &'static str, s: String) -> Res<api::DocId> {
    s.parse().map_err(|e: api::AddressError| ConvertError::new(field, e))
}

fn opt_doc_id(field: &'static str, s: Option<String>) -> Res<Option<api::DocId>> {
    s.map(|s| doc_id(field, s)).transpose()
}

fn address(field: &'static str, s: String) -> Res<api::Address> {
    s.parse().map_err(|e: api::AddressError| ConvertError::new(field, e))
}

fn required<T>(field: &'static str, value: Option<T>) -> Res<T> {
    value.ok_or_else(|| ConvertError::new(field, "required message is missing"))
}

fn list<T, U>(items: Vec<T>, f: impl Fn(T) -> Res<U>) -> Res<Vec<U>> {
    items.into_iter().map(f).collect()
}

fn doc_kind(field: &'static str, value: i32) -> Res<api::DocKind> {
    use pb::DocKind as P;
    match P::try_from(value) {
        Ok(P::Paper) => Ok(api::DocKind::Paper),
        Ok(P::Book) => Ok(api::DocKind::Book),
        Ok(P::Chapter) => Ok(api::DocKind::Chapter),
        Ok(P::Thesis) => Ok(api::DocKind::Thesis),
        Ok(P::Report) => Ok(api::DocKind::Report),
        Ok(P::Notes) => Ok(api::DocKind::Notes),
        Ok(P::Code) => Ok(api::DocKind::Code),
        Ok(P::Repository) => Ok(api::DocKind::Repository),
        Ok(P::Other) => Ok(api::DocKind::Other),
        Ok(P::Unspecified) | Err(_) => Err(ConvertError::new(field, format!("unknown document kind {value}"))),
    }
}

impl From<api::DocKind> for pb::DocKind {
    fn from(kind: api::DocKind) -> Self {
        use api::DocKind as A;
        match kind {
            A::Paper => Self::Paper,
            A::Book => Self::Book,
            A::Chapter => Self::Chapter,
            A::Thesis => Self::Thesis,
            A::Report => Self::Report,
            A::Notes => Self::Notes,
            A::Code => Self::Code,
            A::Repository => Self::Repository,
            A::Other => Self::Other,
        }
    }
}

fn kind_i32(kind: api::DocKind) -> i32 {
    pb::DocKind::from(kind) as i32
}

// ------------------------------------------------------------------------------------------------ shared

impl From<api::Truncation> for pb::Truncation {
    fn from(t: api::Truncation) -> Self {
        Self { reason: t.reason, next: t.next }
    }
}

impl From<pb::Truncation> for api::Truncation {
    fn from(t: pb::Truncation) -> Self {
        Self { reason: t.reason, next: t.next }
    }
}

impl From<api::ItemError> for pb::ItemError {
    fn from(e: api::ItemError) -> Self {
        Self { index: e.index, message: e.message }
    }
}

impl From<pb::ItemError> for api::ItemError {
    fn from(e: pb::ItemError) -> Self {
        Self { index: e.index, message: e.message }
    }
}

fn errors_to_pb(errors: Vec<api::ItemError>) -> Vec<pb::ItemError> {
    errors.into_iter().map(Into::into).collect()
}

fn errors_from_pb(errors: Vec<pb::ItemError>) -> Vec<api::ItemError> {
    errors.into_iter().map(Into::into).collect()
}

impl From<api::Filters> for pb::Filters {
    fn from(f: api::Filters) -> Self {
        Self {
            category: f.category,
            area: f.area,
            kind: f.kind.map(kind_i32),
            level: f.level,
            year_from: f.year_from,
            year_to: f.year_to,
            language: f.language,
            openalex_topic: f.openalex_topic,
            batch: f.batch,
            repo: f.repo,
        }
    }
}

impl TryFrom<pb::Filters> for api::Filters {
    type Error = ConvertError;
    fn try_from(f: pb::Filters) -> Res<Self> {
        Ok(Self {
            category: f.category,
            area: f.area,
            kind: f.kind.map(|k| doc_kind("filters.kind", k)).transpose()?,
            level: f.level,
            year_from: f.year_from,
            year_to: f.year_to,
            language: f.language,
            openalex_topic: f.openalex_topic,
            batch: f.batch,
            repo: f.repo,
        })
    }
}

impl From<api::DocSummary> for pb::DocSummary {
    fn from(d: api::DocSummary) -> Self {
        Self {
            doc_id: d.doc_id.to_string(),
            title: d.title,
            authors: d.authors,
            year: d.year,
            kind: kind_i32(d.kind),
            primary_category: d.primary_category,
            language: d.language,
            lines: d.lines,
            score: d.score,
            superseded_by: d.superseded_by.map(|d| d.to_string()),
            repo: d.repo,
            path: d.path,
        }
    }
}

impl TryFrom<pb::DocSummary> for api::DocSummary {
    type Error = ConvertError;
    fn try_from(d: pb::DocSummary) -> Res<Self> {
        Ok(Self {
            doc_id: doc_id("doc.doc_id", d.doc_id)?,
            title: d.title,
            authors: d.authors,
            year: d.year,
            kind: doc_kind("doc.kind", d.kind)?,
            primary_category: d.primary_category,
            language: d.language,
            lines: d.lines,
            score: d.score,
            superseded_by: opt_doc_id("doc.superseded_by", d.superseded_by)?,
            repo: d.repo,
            path: d.path,
        })
    }
}

impl From<api::Passage> for pb::Passage {
    fn from(p: api::Passage) -> Self {
        Self {
            address: p.address.to_string(),
            title: p.title,
            kind: kind_i32(p.kind),
            page: p.page,
            section: p.section,
            text: p.text,
            superseded_by: p.superseded_by.map(|d| d.to_string()),
        }
    }
}

impl TryFrom<pb::Passage> for api::Passage {
    type Error = ConvertError;
    fn try_from(p: pb::Passage) -> Res<Self> {
        Ok(Self {
            address: address("passage.address", p.address)?,
            title: p.title,
            kind: doc_kind("passage.kind", p.kind)?,
            page: p.page,
            section: p.section,
            text: p.text,
            superseded_by: opt_doc_id("passage.superseded_by", p.superseded_by)?,
        })
    }
}

// ------------------------------------------------------------------------------------------------- read

impl From<api::Expand> for pb::Expand {
    fn from(e: api::Expand) -> Self {
        let mode = match e {
            api::Expand::Lines { before, after } => pb::expand::Mode::Lines(pb::LinesExpand { before, after }),
            api::Expand::Section => pb::expand::Mode::Section(pb::SectionExpand {}),
        };
        Self { mode: Some(mode) }
    }
}

impl TryFrom<pb::Expand> for api::Expand {
    type Error = ConvertError;
    fn try_from(e: pb::Expand) -> Res<Self> {
        match required("expand.mode", e.mode)? {
            pb::expand::Mode::Lines(l) => Ok(Self::Lines { before: l.before, after: l.after }),
            pb::expand::Mode::Section(_) => Ok(Self::Section),
        }
    }
}

impl From<api::ReadRequest> for pb::ReadRequest {
    fn from(r: api::ReadRequest) -> Self {
        Self { addresses: r.addresses, expand: r.expand.map(Into::into), max_tokens: r.max_tokens }
    }
}

impl TryFrom<pb::ReadRequest> for api::ReadRequest {
    type Error = ConvertError;
    fn try_from(r: pb::ReadRequest) -> Res<Self> {
        Ok(Self {
            addresses: r.addresses,
            expand: r.expand.map(TryInto::try_into).transpose()?,
            max_tokens: r.max_tokens,
        })
    }
}

impl From<api::ReadResponse> for pb::ReadResponse {
    fn from(r: api::ReadResponse) -> Self {
        Self {
            passages: r.passages.into_iter().map(Into::into).collect(),
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::ReadResponse> for api::ReadResponse {
    type Error = ConvertError;
    fn try_from(r: pb::ReadResponse) -> Res<Self> {
        Ok(Self {
            passages: list(r.passages, TryInto::try_into)?,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

// ---------------------------------------------------------------------------------------------- outline

fn level(field: &'static str, value: u32) -> Res<u8> {
    u8::try_from(value).map_err(|_| ConvertError::new(field, format!("{value} does not fit in 8 bits")))
}

impl From<api::Heading> for pb::Heading {
    fn from(h: api::Heading) -> Self {
        Self { level: h.level.into(), text: h.text, start: h.start, end: h.end, page: h.page }
    }
}

impl TryFrom<pb::Heading> for api::Heading {
    type Error = ConvertError;
    fn try_from(h: pb::Heading) -> Res<Self> {
        Ok(Self { level: level("heading.level", h.level)?, text: h.text, start: h.start, end: h.end, page: h.page })
    }
}

impl From<api::Symbol> for pb::Symbol {
    fn from(s: api::Symbol) -> Self {
        Self { kind: s.kind, name: s.name, start: s.start, end: s.end, parent: s.parent }
    }
}

impl From<pb::Symbol> for api::Symbol {
    fn from(s: pb::Symbol) -> Self {
        Self { kind: s.kind, name: s.name, start: s.start, end: s.end, parent: s.parent }
    }
}

impl From<api::PageMark> for pb::PageMark {
    fn from(p: api::PageMark) -> Self {
        Self { label: p.label, index: p.index, line: p.line }
    }
}

impl From<pb::PageMark> for api::PageMark {
    fn from(p: pb::PageMark) -> Self {
        Self { label: p.label, index: p.index, line: p.line }
    }
}

impl From<api::Outline> for pb::Outline {
    fn from(o: api::Outline) -> Self {
        Self {
            doc: Some(o.doc.into()),
            headings: o.headings.into_iter().map(Into::into).collect(),
            pages: o.pages.into_iter().map(Into::into).collect(),
            symbols: o.symbols.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<pb::Outline> for api::Outline {
    type Error = ConvertError;
    fn try_from(o: pb::Outline) -> Res<Self> {
        Ok(Self {
            doc: required("outline.doc", o.doc)?.try_into()?,
            headings: list(o.headings, TryInto::try_into)?,
            pages: o.pages.into_iter().map(Into::into).collect(),
            symbols: o.symbols.into_iter().map(Into::into).collect(),
        })
    }
}

impl From<api::OutlineRequest> for pb::OutlineRequest {
    fn from(r: api::OutlineRequest) -> Self {
        Self { doc_ids: r.doc_ids, max_depth: r.max_depth.map(Into::into), max_tokens: r.max_tokens }
    }
}

impl TryFrom<pb::OutlineRequest> for api::OutlineRequest {
    type Error = ConvertError;
    fn try_from(r: pb::OutlineRequest) -> Res<Self> {
        Ok(Self {
            doc_ids: r.doc_ids,
            max_depth: r.max_depth.map(|d| level("max_depth", d)).transpose()?,
            max_tokens: r.max_tokens,
        })
    }
}

impl From<api::OutlineResponse> for pb::OutlineResponse {
    fn from(r: api::OutlineResponse) -> Self {
        Self {
            outlines: r.outlines.into_iter().map(Into::into).collect(),
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::OutlineResponse> for api::OutlineResponse {
    type Error = ConvertError;
    fn try_from(r: pb::OutlineResponse) -> Res<Self> {
        Ok(Self {
            outlines: list(r.outlines, TryInto::try_into)?,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

// ------------------------------------------------------------------------------------------------- grep

impl From<api::Scope> for pb::Scope {
    fn from(s: api::Scope) -> Self {
        use pb::scope::Kind;
        let kind = match s {
            api::Scope::Docs { doc_ids } => Kind::Docs(pb::DocsScope { doc_ids }),
            api::Scope::Workspace { workspace } => Kind::Workspace(pb::WorkspaceScope { workspace }),
            api::Scope::Category { category, area } => Kind::Category(pb::CategoryScope { category, area }),
            api::Scope::Vault => Kind::Vault(pb::VaultScope {}),
        };
        Self { kind: Some(kind) }
    }
}

impl TryFrom<pb::Scope> for api::Scope {
    type Error = ConvertError;
    fn try_from(s: pb::Scope) -> Res<Self> {
        use pb::scope::Kind;
        Ok(match required("scope.kind", s.kind)? {
            Kind::Docs(d) => Self::Docs { doc_ids: d.doc_ids },
            Kind::Workspace(w) => Self::Workspace { workspace: w.workspace },
            Kind::Category(c) => Self::Category { category: c.category, area: c.area },
            Kind::Vault(_) => Self::Vault,
        })
    }
}

impl From<api::GrepRequest> for pb::GrepRequest {
    fn from(r: api::GrepRequest) -> Self {
        Self {
            patterns: r.patterns,
            scope: Some(r.scope.into()),
            context_lines: r.context_lines,
            max_hits: r.max_hits,
            case_insensitive: r.case_insensitive,
            max_tokens: r.max_tokens,
        }
    }
}

impl TryFrom<pb::GrepRequest> for api::GrepRequest {
    type Error = ConvertError;
    fn try_from(r: pb::GrepRequest) -> Res<Self> {
        Ok(Self {
            patterns: r.patterns,
            scope: required("scope", r.scope)?.try_into()?,
            context_lines: r.context_lines,
            max_hits: r.max_hits,
            case_insensitive: r.case_insensitive,
            max_tokens: r.max_tokens,
        })
    }
}

impl From<api::GrepHit> for pb::GrepHit {
    fn from(h: api::GrepHit) -> Self {
        Self {
            pattern: h.pattern,
            address: h.address.to_string(),
            title: h.title,
            page: h.page,
            line: h.line,
            before: h.before,
            after: h.after,
        }
    }
}

impl TryFrom<pb::GrepHit> for api::GrepHit {
    type Error = ConvertError;
    fn try_from(h: pb::GrepHit) -> Res<Self> {
        Ok(Self {
            pattern: h.pattern,
            address: address("hit.address", h.address)?,
            title: h.title,
            page: h.page,
            line: h.line,
            before: h.before,
            after: h.after,
        })
    }
}

impl From<api::GrepResponse> for pb::GrepResponse {
    fn from(r: api::GrepResponse) -> Self {
        Self {
            hits: r.hits.into_iter().map(Into::into).collect(),
            candidates: r.candidates,
            in_scope: r.in_scope,
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::GrepResponse> for api::GrepResponse {
    type Error = ConvertError;
    fn try_from(r: pb::GrepResponse) -> Res<Self> {
        Ok(Self {
            hits: list(r.hits, TryInto::try_into)?,
            candidates: r.candidates,
            in_scope: r.in_scope,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

// ------------------------------------------------------------------------------------ find_docs, search

impl From<api::FindDocsRequest> for pb::FindDocsRequest {
    fn from(r: api::FindDocsRequest) -> Self {
        Self { queries: r.queries, filters: Some(r.filters.into()), k: r.k, max_tokens: r.max_tokens }
    }
}

impl TryFrom<pb::FindDocsRequest> for api::FindDocsRequest {
    type Error = ConvertError;
    fn try_from(r: pb::FindDocsRequest) -> Res<Self> {
        Ok(Self {
            queries: r.queries,
            filters: r.filters.map(TryInto::try_into).transpose()?.unwrap_or_default(),
            k: r.k,
            max_tokens: r.max_tokens,
        })
    }
}

impl From<api::FindDocsResponse> for pb::FindDocsResponse {
    fn from(r: api::FindDocsResponse) -> Self {
        Self {
            results: r
                .results
                .into_iter()
                .map(|docs| pb::DocList { docs: docs.into_iter().map(Into::into).collect() })
                .collect(),
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::FindDocsResponse> for api::FindDocsResponse {
    type Error = ConvertError;
    fn try_from(r: pb::FindDocsResponse) -> Res<Self> {
        Ok(Self {
            results: list(r.results, |l| list(l.docs, TryInto::try_into))?,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

impl From<api::SearchRequest> for pb::SearchRequest {
    fn from(r: api::SearchRequest) -> Self {
        Self { queries: r.queries, filters: Some(r.filters.into()), k: r.k, max_tokens: r.max_tokens }
    }
}

impl TryFrom<pb::SearchRequest> for api::SearchRequest {
    type Error = ConvertError;
    fn try_from(r: pb::SearchRequest) -> Res<Self> {
        Ok(Self {
            queries: r.queries,
            filters: r.filters.map(TryInto::try_into).transpose()?.unwrap_or_default(),
            k: r.k,
            max_tokens: r.max_tokens,
        })
    }
}

impl From<api::SearchHit> for pb::SearchHit {
    fn from(h: api::SearchHit) -> Self {
        Self { passage: Some(h.passage.into()), score: h.score, doc: Some(h.doc.into()) }
    }
}

impl TryFrom<pb::SearchHit> for api::SearchHit {
    type Error = ConvertError;
    fn try_from(h: pb::SearchHit) -> Res<Self> {
        Ok(Self {
            passage: required("hit.passage", h.passage)?.try_into()?,
            score: h.score,
            doc: required("hit.doc", h.doc)?.try_into()?,
        })
    }
}

impl From<api::SearchResponse> for pb::SearchResponse {
    fn from(r: api::SearchResponse) -> Self {
        Self {
            results: r
                .results
                .into_iter()
                .map(|hits| pb::SearchHitList { hits: hits.into_iter().map(Into::into).collect() })
                .collect(),
            legs: r.legs,
            degraded: r.degraded,
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::SearchResponse> for api::SearchResponse {
    type Error = ConvertError;
    fn try_from(r: pb::SearchResponse) -> Res<Self> {
        Ok(Self {
            results: list(r.results, |l| list(l.hits, TryInto::try_into))?,
            legs: r.legs,
            degraded: r.degraded,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

// ------------------------------------------------------------------------- workspace, citations, define

fn workspace_action(value: i32) -> Res<api::WorkspaceAction> {
    use pb::WorkspaceAction as P;
    match P::try_from(value) {
        Ok(P::Add) => Ok(api::WorkspaceAction::Add),
        Ok(P::Remove) => Ok(api::WorkspaceAction::Remove),
        Ok(P::List) => Ok(api::WorkspaceAction::List),
        Ok(P::Clear) => Ok(api::WorkspaceAction::Clear),
        Ok(P::Unspecified) | Err(_) => Err(ConvertError::new("action", format!("unknown workspace action {value}"))),
    }
}

impl From<api::WorkspaceAction> for pb::WorkspaceAction {
    fn from(a: api::WorkspaceAction) -> Self {
        match a {
            api::WorkspaceAction::Add => Self::Add,
            api::WorkspaceAction::Remove => Self::Remove,
            api::WorkspaceAction::List => Self::List,
            api::WorkspaceAction::Clear => Self::Clear,
        }
    }
}

impl From<api::WorkspaceRequest> for pb::WorkspaceRequest {
    fn from(r: api::WorkspaceRequest) -> Self {
        Self { action: pb::WorkspaceAction::from(r.action) as i32, doc_ids: r.doc_ids, workspace: r.workspace }
    }
}

impl TryFrom<pb::WorkspaceRequest> for api::WorkspaceRequest {
    type Error = ConvertError;
    fn try_from(r: pb::WorkspaceRequest) -> Res<Self> {
        Ok(Self { action: workspace_action(r.action)?, doc_ids: r.doc_ids, workspace: r.workspace })
    }
}

impl From<api::WorkspaceResponse> for pb::WorkspaceResponse {
    fn from(r: api::WorkspaceResponse) -> Self {
        Self {
            workspace: r.workspace,
            docs: r.docs.into_iter().map(Into::into).collect(),
            errors: errors_to_pb(r.errors),
        }
    }
}

impl TryFrom<pb::WorkspaceResponse> for api::WorkspaceResponse {
    type Error = ConvertError;
    fn try_from(r: pb::WorkspaceResponse) -> Res<Self> {
        Ok(Self { workspace: r.workspace, docs: list(r.docs, TryInto::try_into)?, errors: errors_from_pb(r.errors) })
    }
}

fn direction(value: i32) -> Res<api::Direction> {
    use pb::Direction as P;
    match P::try_from(value) {
        Ok(P::Cites) => Ok(api::Direction::Cites),
        Ok(P::CitedBy) => Ok(api::Direction::CitedBy),
        Ok(P::Both) => Ok(api::Direction::Both),
        Ok(P::Unspecified) | Err(_) => Err(ConvertError::new("direction", format!("unknown direction {value}"))),
    }
}

impl From<api::Direction> for pb::Direction {
    fn from(d: api::Direction) -> Self {
        match d {
            api::Direction::Cites => Self::Cites,
            api::Direction::CitedBy => Self::CitedBy,
            api::Direction::Both => Self::Both,
        }
    }
}

impl From<api::CitationsRequest> for pb::CitationsRequest {
    fn from(r: api::CitationsRequest) -> Self {
        Self {
            doc_ids: r.doc_ids,
            direction: pb::Direction::from(r.direction) as i32,
            k: r.k,
            max_tokens: r.max_tokens,
        }
    }
}

impl TryFrom<pb::CitationsRequest> for api::CitationsRequest {
    type Error = ConvertError;
    fn try_from(r: pb::CitationsRequest) -> Res<Self> {
        Ok(Self { doc_ids: r.doc_ids, direction: direction(r.direction)?, k: r.k, max_tokens: r.max_tokens })
    }
}

impl From<api::CitationEdge> for pb::CitationEdge {
    fn from(e: api::CitationEdge) -> Self {
        Self {
            from: e.from.to_string(),
            to: e.to.map(Into::into),
            as_written: e.as_written,
            at: e.at.map(|a| a.to_string()),
            external: e.external,
            external_id: e.external_id,
        }
    }
}

impl TryFrom<pb::CitationEdge> for api::CitationEdge {
    type Error = ConvertError;
    fn try_from(e: pb::CitationEdge) -> Res<Self> {
        Ok(Self {
            from: doc_id("edge.from", e.from)?,
            to: e.to.map(TryInto::try_into).transpose()?,
            as_written: e.as_written,
            at: e.at.map(|a| address("edge.at", a)).transpose()?,
            external: e.external,
            external_id: e.external_id,
        })
    }
}

impl From<api::CitationsResponse> for pb::CitationsResponse {
    fn from(r: api::CitationsResponse) -> Self {
        Self {
            edges: r.edges.into_iter().map(Into::into).collect(),
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::CitationsResponse> for api::CitationsResponse {
    type Error = ConvertError;
    fn try_from(r: pb::CitationsResponse) -> Res<Self> {
        Ok(Self {
            edges: list(r.edges, TryInto::try_into)?,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

impl From<api::DefineRequest> for pb::DefineRequest {
    fn from(r: api::DefineRequest) -> Self {
        Self { terms: r.terms, filters: Some(r.filters.into()), k: r.k, max_tokens: r.max_tokens }
    }
}

impl TryFrom<pb::DefineRequest> for api::DefineRequest {
    type Error = ConvertError;
    fn try_from(r: pb::DefineRequest) -> Res<Self> {
        Ok(Self {
            terms: r.terms,
            filters: r.filters.map(TryInto::try_into).transpose()?.unwrap_or_default(),
            k: r.k,
            max_tokens: r.max_tokens,
        })
    }
}

impl From<api::Definition> for pb::Definition {
    fn from(d: api::Definition) -> Self {
        Self { term: d.term, source: d.source, passage: Some(d.passage.into()) }
    }
}

impl TryFrom<pb::Definition> for api::Definition {
    type Error = ConvertError;
    fn try_from(d: pb::Definition) -> Res<Self> {
        Ok(Self { term: d.term, source: d.source, passage: required("definition.passage", d.passage)?.try_into()? })
    }
}

impl From<api::DefineResponse> for pb::DefineResponse {
    fn from(r: api::DefineResponse) -> Self {
        Self {
            definitions: r.definitions.into_iter().map(Into::into).collect(),
            errors: errors_to_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        }
    }
}

impl TryFrom<pb::DefineResponse> for api::DefineResponse {
    type Error = ConvertError;
    fn try_from(r: pb::DefineResponse) -> Res<Self> {
        Ok(Self {
            definitions: list(r.definitions, TryInto::try_into)?,
            errors: errors_from_pb(r.errors),
            truncated: r.truncated.map(Into::into),
        })
    }
}

// -------------------------------------------------------------------------------------------- vault_info

impl From<api::CategoryCoverage> for pb::CategoryCoverage {
    fn from(c: api::CategoryCoverage) -> Self {
        Self {
            category: c.category,
            name: c.name,
            papers: c.papers,
            books: c.books,
            code_files: c.code_files,
            target_papers: c.target_papers,
            target_books: c.target_books,
        }
    }
}

impl From<pb::CategoryCoverage> for api::CategoryCoverage {
    fn from(c: pb::CategoryCoverage) -> Self {
        Self {
            category: c.category,
            name: c.name,
            papers: c.papers,
            books: c.books,
            code_files: c.code_files,
            target_papers: c.target_papers,
            target_books: c.target_books,
        }
    }
}

impl From<api::TierDisk> for pb::TierDisk {
    fn from(t: api::TierDisk) -> Self {
        Self {
            tier: t.tier,
            used_bytes: t.used_bytes,
            free_bytes: t.free_bytes,
            total_bytes: t.total_bytes,
            warning: t.warning,
        }
    }
}

impl From<pb::TierDisk> for api::TierDisk {
    fn from(t: pb::TierDisk) -> Self {
        Self {
            tier: t.tier,
            used_bytes: t.used_bytes,
            free_bytes: t.free_bytes,
            total_bytes: t.total_bytes,
            warning: t.warning,
        }
    }
}

impl From<api::VaultInfo> for pb::VaultInfo {
    fn from(v: api::VaultInfo) -> Self {
        Self {
            vault_id: v.vault_id,
            format_version: v.format_version,
            stage: v.stage,
            documents: v.documents,
            by_kind: v.by_kind.into_iter().map(|(kind, count)| pb::KindCount { kind: kind_i32(kind), count }).collect(),
            quarantined: v.quarantined,
            repositories: v.repositories,
            coverage: v.coverage.into_iter().map(Into::into).collect(),
            taxonomy_version: v.taxonomy_version,
            index_version: v.index_version,
            tools_available: v.tools_available,
            disk: v.disk.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<pb::VaultInfo> for api::VaultInfo {
    type Error = ConvertError;
    fn try_from(v: pb::VaultInfo) -> Res<Self> {
        Ok(Self {
            vault_id: v.vault_id,
            format_version: v.format_version,
            stage: v.stage,
            documents: v.documents,
            by_kind: list(v.by_kind, |k| Ok((doc_kind("by_kind.kind", k.kind)?, k.count)))?,
            quarantined: v.quarantined,
            repositories: v.repositories,
            coverage: v.coverage.into_iter().map(Into::into).collect(),
            taxonomy_version: v.taxonomy_version,
            index_version: v.index_version,
            tools_available: v.tools_available,
            disk: v.disk.into_iter().map(Into::into).collect(),
        })
    }
}

// ----------------------------------------------------------------------------------------------- errors

/// Maps a tool error to a gRPC status. The code carries the kind and the message carries the detail, so
/// [`api_error_from_status`] restores the same [`api::ApiError`] on the client.
pub fn status_from_api_error(error: &api::ApiError) -> tonic::Status {
    use api::ApiError as E;
    match error {
        E::InvalidArgument(m) => tonic::Status::invalid_argument(m),
        E::NotFound(m) => tonic::Status::not_found(m),
        E::NotYetAvailable(m) => tonic::Status::unimplemented(m),
        E::Unavailable(m) => tonic::Status::unavailable(m),
        E::Internal(m) => tonic::Status::internal(m),
    }
}

/// The inverse of [`status_from_api_error`]. Codes the server never uses for tool errors (deadline, cancelled,
/// resource exhausted, transport failures) become [`api::ApiError::Unavailable`] when retrying could help and
/// [`api::ApiError::Internal`] otherwise.
pub fn api_error_from_status(status: &tonic::Status) -> api::ApiError {
    use api::ApiError as E;
    use tonic::Code;
    let message = status.message().to_string();
    match status.code() {
        Code::InvalidArgument | Code::OutOfRange => E::InvalidArgument(message),
        Code::NotFound => E::NotFound(message),
        Code::Unimplemented => E::NotYetAvailable(message),
        Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted | Code::Aborted | Code::Cancelled => {
            E::Unavailable(message)
        }
        Code::Unauthenticated | Code::PermissionDenied => E::Unavailable(format!("not authorised: {message}")),
        _ => E::Internal(message),
    }
}

/// A request that failed validation on the server side.
impl From<ConvertError> for tonic::Status {
    fn from(e: ConvertError) -> Self {
        tonic::Status::invalid_argument(e.to_string())
    }
}
