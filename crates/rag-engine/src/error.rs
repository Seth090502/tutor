//! One error type for the engine: a message.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

/// The library errors that `?` converts, each kept as its message.
macro_rules! from_errors {
    ($($t:ty),* $(,)?) => {
        $(impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Error(e.to_string())
            }
        })*
    };
}

from_errors!(
    std::io::Error,
    rusqlite::Error,
    serde_json::Error,
    reqwest::Error,
    tantivy::TantivyError,
    tantivy::directory::error::OpenDirectoryError,
    lancedb::Error,
    lancedb::arrow::arrow_schema::ArrowError,
);

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Error(message.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
