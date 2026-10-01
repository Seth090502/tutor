//! Format detection and converters from delivered files to canonical-ready Markdown plus structure
//! (docs/phase1/SPEC.md section 8). No storage, no database: bytes in, [`Extracted`] out.
//!
//! - [`detect::detect`] finds the format from the bytes (magic numbers, then container inspection).
//! - [`extract`] detects and routes: converted document, archive members, a PDF waiting for the GPU tools, or a
//!   format waiting for an uninstalled tool.
//! - The converters are public for direct use: [`jats`], [`tei`], [`html`], [`text`] (plain text and Markdown),
//!   [`notebook`], [`epub`], [`docx`], [`odt`], and `pdf` (PDFium, behind the default `pdfium` feature).
//! - [`quality`] scores an extraction; [`pdfclass`] classifies PDFs.
//!
//! Line numbers in [`Extracted`] refer to its Markdown exactly as returned. vault-core's normaliser re-flows
//! paragraphs into sentence lines afterwards, so it maps those line numbers through its own line mapping.

pub mod archive;
mod builder;
pub mod detect;
mod dispatch;
pub mod docx;
pub mod epub;
pub mod error;
pub mod html;
pub mod ids;
pub mod jats;
pub mod mathml;
pub mod notebook;
pub mod odt;
#[cfg(feature = "pdfium")]
pub mod pdf;
pub mod pdfclass;
#[cfg(feature = "pdfium")]
mod pdflayout;
pub mod pdfscan;
pub mod quality;
mod tables;
pub mod tei;
#[cfg(all(test, feature = "pdfium"))]
mod testpdf;
pub mod text;
pub mod types;
mod xmlutil;

pub use dispatch::{Converted, Extraction, Outcome, extract};
pub use error::{ExtractError, Result};
pub use types::{BlockKind, BlockSpan, Extracted, HeadingMark, Metadata, PageMark, Reference};

/// Settings shared by the converters.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// Repair text that is systematically UTF-8 read as Latin-1 (and record a warning).
    pub repair_mojibake: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self { repair_mojibake: true }
    }
}
