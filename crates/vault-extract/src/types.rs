//! The converters' output. Line numbers are 1-based and refer to [`Extracted::markdown`] exactly as returned.
//! The vault-core normaliser re-flows paragraphs into sentence lines afterwards, so it must map these
//! structural line numbers through its own line mapping (every source line becomes one or more lines, in order).

use serde::{Deserialize, Serialize};

/// A heading, as it sits in the Markdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingMark {
    pub level: u8,
    pub text: String,
    /// Line of the heading itself.
    pub line: u32,
}

/// A page start. `label` is the printed label ("214", "xii") when the source supplies one; `index` is the
/// physical position, 1-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageMark {
    pub label: Option<String>,
    pub index: u32,
    /// First line that belongs to the page.
    pub line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Table,
    Code,
    Equation,
}

/// A span of lines that must stay verbatim (table, fenced code, display maths). Both ends are inclusive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSpan {
    pub kind: BlockKind,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub pmcid: Option<String>,
    pub isbn: Vec<String>,
    pub year: Option<i32>,
    /// BCP 47 tag or ISO 639 code as the source declared it; never guessed from the text.
    pub language: Option<String>,
    pub keywords: Vec<String>,
    pub journal: Option<String>,
}

/// One entry of a reference list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    /// The citation as printed, on one line.
    pub raw: String,
    /// The label in the source ("1", "Smith2020").
    pub label: Option<String>,
    pub doi: Option<String>,
    pub pmid: Option<String>,
    pub pmcid: Option<String>,
    pub arxiv_id: Option<String>,
    pub isbn: Option<String>,
    pub title: Option<String>,
    pub year: Option<i32>,
}

/// The result of converting one document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Extracted {
    pub markdown: String,
    pub headings: Vec<HeadingMark>,
    pub pages: Vec<PageMark>,
    pub blocks: Vec<BlockSpan>,
    pub metadata: Metadata,
    pub references: Vec<Reference>,
    pub warnings: Vec<String>,
    /// For example `vault-extract 0.1.0 jats`.
    pub tool: String,
    /// The source encoding when the converter decoded bytes (`utf-8`, `windows-1252`...).
    pub encoding: Option<String>,
}

impl Extracted {
    /// Number of lines in [`Self::markdown`].
    pub fn line_count(&self) -> u32 {
        self.markdown.lines().count() as u32
    }
}

pub(crate) fn tool_string(path: &str) -> String {
    format!("vault-extract {} {}", env!("CARGO_PKG_VERSION"), path)
}
