//! Format detection and converters. See docs/phase1/SPEC.md section 8.

mod builder;
pub mod detect;
pub mod error;
pub mod ids;
pub mod jats;
pub mod mathml;
pub mod pdfscan;
pub mod quality;
mod tables;
pub mod text;
pub mod types;
mod xmlutil;

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
