//! Request and response types for the nine vault tools. Every tool takes batches (several queries, addresses
//! or documents per call) so the tutor gathers evidence in few turns. Every response stays within a token
//! budget and says when, and how, it truncated. Per-item problems are reported in `errors` without failing the
//! rest of the batch.

use crate::address::{Address, DocId};
use serde::{Deserialize, Serialize};

/// Default response budget, in approximate tokens (4 characters per token). Claude Code spills MCP results over
/// 25k tokens to a file and OpenCode over 51,200 bytes; a locked-down tutor cannot read those files.
pub const DEFAULT_MAX_TOKENS: u32 = 8_000;
pub const HARD_MAX_TOKENS: u32 = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Paper,
    Book,
    Chapter,
    Thesis,
    Report,
    Notes,
    Code,
    Repository,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Truncation {
    /// Why the response stopped early, for example "token budget 8000 reached after 12 of 30 hits".
    pub reason: String,
    /// How to get the rest, for example "call read with the remaining addresses: ...".
    pub next: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ItemError {
    /// Index of the failing item in the request's array.
    pub index: u32,
    pub message: String,
}

/// Filters shared by search, find_docs, grep (category scope) and define. Every field is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Filters {
    pub category: Option<String>,
    /// Psychology study-map area (`memory`, `clinical`, ...); requires category `psych-cogsci` or none.
    pub area: Option<String>,
    pub kind: Option<DocKind>,
    pub level: Option<String>,
    pub year_from: Option<i32>,
    pub year_to: Option<i32>,
    pub language: Option<String>,
    pub openalex_topic: Option<String>,
    pub batch: Option<String>,
    /// Restrict to documents of one source-code repository.
    pub repo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocSummary {
    pub doc_id: DocId,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub kind: DocKind,
    pub primary_category: Option<String>,
    pub language: Option<String>,
    pub lines: u32,
    /// Retrieval score when the summary comes from a ranked list; absent otherwise.
    pub score: Option<f32>,
    /// Set when a newer version of this document exists.
    pub superseded_by: Option<DocId>,
    /// For code: repository name and path inside it.
    pub repo: Option<String>,
    pub path: Option<String>,
}

/// Exact text from the canonical store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Passage {
    pub address: Address,
    pub title: Option<String>,
    pub kind: DocKind,
    /// Printed page label of the first line, when the source had pages ("214", "xii").
    pub page: Option<String>,
    /// Heading path of the first line, for example "3 Memory > 3.2 Working memory".
    pub section: Option<String>,
    pub text: String,
    pub superseded_by: Option<DocId>,
}

// ---------------------------------------------------------------------------------------------------- read

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum Expand {
    /// Add lines before and after each address.
    Lines { before: u32, after: u32 },
    /// Widen each address to the innermost heading section containing it.
    Section,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReadRequest {
    pub addresses: Vec<String>,
    #[serde(default)]
    pub expand: Option<Expand>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReadResponse {
    pub passages: Vec<Passage>,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

// ------------------------------------------------------------------------------------------------- outline

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub start: u32,
    /// Last line of the section (inclusive).
    pub end: u32,
    pub page: Option<String>,
}

/// A code symbol (function, class, method, module...) with its line span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Symbol {
    pub kind: String,
    pub name: String,
    pub start: u32,
    pub end: u32,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PageMark {
    pub label: String,
    pub index: u32,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Outline {
    pub doc: DocSummary,
    pub headings: Vec<Heading>,
    /// Page starts; may be summarised (every n-th page) when the budget is tight.
    pub pages: Vec<PageMark>,
    /// For code files: their symbols. For repository documents: empty (the tree is in `headings`).
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OutlineRequest {
    pub doc_ids: Vec<String>,
    #[serde(default)]
    pub max_depth: Option<u8>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OutlineResponse {
    pub outlines: Vec<Outline>,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

// ---------------------------------------------------------------------------------------------------- grep

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Scope {
    Docs { doc_ids: Vec<String> },
    Workspace { workspace: Option<String> },
    Category { category: String, area: Option<String> },
    Vault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GrepRequest {
    /// Regular expressions (Rust `regex` syntax). Plain text is a valid pattern.
    pub patterns: Vec<String>,
    pub scope: Scope,
    #[serde(default)]
    pub context_lines: u32,
    #[serde(default)]
    pub max_hits: Option<u32>,
    #[serde(default)]
    pub case_insensitive: bool,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GrepHit {
    pub pattern: u32,
    pub address: Address,
    pub title: Option<String>,
    pub page: Option<String>,
    pub line: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GrepResponse {
    pub hits: Vec<GrepHit>,
    /// Documents (or blocks) the trigram prefilter passed to regex verification, and documents in scope.
    pub candidates: u64,
    pub in_scope: u64,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

// ----------------------------------------------------------------------------------------- find_docs, search

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FindDocsRequest {
    pub queries: Vec<String>,
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub k: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FindDocsResponse {
    /// One ranked list per query, in request order.
    pub results: Vec<Vec<DocSummary>>,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SearchRequest {
    pub queries: Vec<String>,
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub k: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SearchHit {
    pub passage: Passage,
    pub score: f32,
    pub doc: DocSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SearchResponse {
    pub results: Vec<Vec<SearchHit>>,
    /// Retrieval legs that ran (for example ["bm25", "dense", "rerank"]) and anything degraded.
    pub legs: Vec<String>,
    pub degraded: Vec<String>,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

// ------------------------------------------------------------------------------- workspace, citations, define

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceAction {
    Add,
    Remove,
    List,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WorkspaceRequest {
    pub action: WorkspaceAction,
    #[serde(default)]
    pub doc_ids: Vec<String>,
    /// Named workspace; defaults to the caller's session workspace.
    #[serde(default)]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WorkspaceResponse {
    pub workspace: String,
    pub docs: Vec<DocSummary>,
    pub errors: Vec<ItemError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// What these documents cite (papers: reference lists; code: what it imports or calls).
    Cites,
    /// What cites these documents (papers: citing documents; code: what uses it).
    CitedBy,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CitationsRequest {
    pub doc_ids: Vec<String>,
    pub direction: Direction,
    #[serde(default)]
    pub k: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CitationEdge {
    pub from: DocId,
    pub to: Option<DocSummary>,
    /// The reference as written (reference-list entry, import line), with its address in the citing document.
    pub as_written: String,
    pub at: Option<Address>,
    /// True when the cited work is not in the vault (resolved to an external id only).
    pub external: bool,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CitationsResponse {
    pub edges: Vec<CitationEdge>,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DefineRequest {
    /// Terms, or for code, symbol names.
    pub terms: Vec<String>,
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub k: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Definition {
    pub term: u32,
    /// "index_entry", "definition_passage", "glossary" or "symbol_definition".
    pub source: String,
    pub passage: Passage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DefineResponse {
    pub definitions: Vec<Definition>,
    pub errors: Vec<ItemError>,
    pub truncated: Option<Truncation>,
}

// --------------------------------------------------------------------------------------------- vault_info

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CategoryCoverage {
    pub category: String,
    pub name: String,
    pub papers: u64,
    pub books: u64,
    pub code_files: u64,
    pub target_papers: u64,
    pub target_books: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TierDisk {
    pub tier: String,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub total_bytes: u64,
    /// True at or above 85% of the disk.
    pub warning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VaultInfo {
    pub vault_id: String,
    pub format_version: u32,
    pub stage: String,
    pub documents: u64,
    pub by_kind: Vec<(DocKind, u64)>,
    pub quarantined: u64,
    pub repositories: u64,
    pub coverage: Vec<CategoryCoverage>,
    pub taxonomy_version: String,
    pub index_version: String,
    pub tools_available: Vec<String>,
    pub disk: Vec<TierDisk>,
}
