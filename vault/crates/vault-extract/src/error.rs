//! Errors returned by detection and the converters.

use std::io;

/// Why a delivered file could not be converted. The variants map onto the intake reason codes of SPEC section 7:
/// [`ExtractError::Limits`] is `archive_limits`, [`ExtractError::Protected`] is `protected`,
/// [`ExtractError::Corrupt`] is `corrupt_file`, [`ExtractError::Empty`] is `empty_file`,
/// [`ExtractError::Unsupported`] is `unsupported_format` and the rest are `extraction_failed`.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("empty file")]
    Empty,
    #[error("unsupported format: {0}")]
    Unsupported(String),
    #[error("password-protected or encrypted: {0}")]
    Protected(String),
    #[error("corrupt {format}: {detail}")]
    Corrupt { format: &'static str, detail: String },
    #[error("archive limit exceeded: {0}")]
    Limits(String),
    #[error("{format} conversion failed: {detail}")]
    Convert { format: &'static str, detail: String },
    #[error("a required tool is unavailable: {0}")]
    ToolUnavailable(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl ExtractError {
    pub(crate) fn corrupt(format: &'static str, detail: impl Into<String>) -> Self {
        Self::Corrupt { format, detail: detail.into() }
    }

    pub(crate) fn convert(format: &'static str, detail: impl Into<String>) -> Self {
        Self::Convert { format, detail: detail.into() }
    }

    /// The SPEC section 7 reason code this error maps to.
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::Empty => "empty_file",
            Self::Unsupported(_) => "unsupported_format",
            Self::Protected(_) => "protected",
            Self::Corrupt { .. } => "corrupt_file",
            Self::Limits(_) => "archive_limits",
            Self::Convert { .. } | Self::ToolUnavailable(_) | Self::Io(_) => "extraction_failed",
        }
    }
}

pub type Result<T, E = ExtractError> = std::result::Result<T, E>;
