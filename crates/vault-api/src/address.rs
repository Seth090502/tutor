//! Document ids and line addresses. Grammar (docs/phase1/SPEC.md section 2):
//!
//! ```text
//! doc_id  = hash16 [ "-" slug ]          hash16 = 16 lowercase hex chars (BLAKE3 prefix of the canonical text)
//! slug    = word *( "-" word ), <= 48    word   = 1*( a-z / 0-9 )
//! address = doc_id "#L" line [ "-L" line ]   line = 1-based decimal without leading zeros, start <= end
//! ```
//!
//! Addresses carry no paths, drive letters or machine names, and resolve by `hash16` alone, so the slug is
//! cosmetic and may be omitted by callers.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub const HASH_LEN: usize = 16;
pub const SLUG_MAX: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AddressError {
    #[error("document id must start with 16 lowercase hex characters: {0:?}")]
    BadHash(String),
    #[error("slug must be lowercase letters and digits separated by single hyphens, at most 48 characters: {0:?}")]
    BadSlug(String),
    #[error("address must look like <doc_id>#L<start> or <doc_id>#L<start>-L<end>: {0:?}")]
    BadAddress(String),
    #[error("line numbers start at 1 and the range must not run backwards: {0:?}")]
    BadRange(String),
}

/// A document id: the hash part identifies the document; the slug only helps humans read citations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DocId {
    hash: String,
    slug: Option<String>,
}

impl DocId {
    pub fn new(hash: &str, slug: Option<&str>) -> Result<Self, AddressError> {
        if hash.len() != HASH_LEN || !hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(AddressError::BadHash(hash.to_string()));
        }
        if let Some(s) = slug
            && !valid_slug(s)
        {
            return Err(AddressError::BadSlug(s.to_string()));
        }
        Ok(Self { hash: hash.to_string(), slug: slug.map(str::to_string) })
    }

    /// The 16-hex identity. Two `DocId`s with the same hash are the same document.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn slug(&self) -> Option<&str> {
        self.slug.as_deref()
    }

    /// The two-character shard used in store paths (`text/{h2}/...`).
    pub fn shard(&self) -> &str {
        &self.hash[..2]
    }

    pub fn same_document(&self, other: &DocId) -> bool {
        self.hash == other.hash
    }
}

fn valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= SLUG_MAX
        && s.split('-').all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
}

impl FromStr for DocId {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('-') {
            Some((hash, slug)) => DocId::new(hash, Some(slug)),
            None => DocId::new(s, None),
        }
    }
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.slug {
            Some(slug) => write!(f, "{}-{}", self.hash, slug),
            None => f.write_str(&self.hash),
        }
    }
}

impl TryFrom<String> for DocId {
    type Error = AddressError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<DocId> for String {
    fn from(d: DocId) -> String {
        d.to_string()
    }
}

/// An inclusive, 1-based line range in one document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Address {
    pub doc: DocId,
    pub start: u32,
    pub end: u32,
}

impl Address {
    pub fn new(doc: DocId, start: u32, end: u32) -> Result<Self, AddressError> {
        if start == 0 || end < start {
            return Err(AddressError::BadRange(format!("{doc}#L{start}-L{end}")));
        }
        Ok(Self { doc, start, end })
    }

    pub fn line_count(&self) -> u32 {
        self.end - self.start + 1
    }
}

fn parse_line(s: &str, whole: &str) -> Result<u32, AddressError> {
    let digits = s.strip_prefix('L').ok_or_else(|| AddressError::BadAddress(whole.to_string()))?;
    if digits.is_empty() || digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AddressError::BadAddress(whole.to_string()));
    }
    digits.parse().map_err(|_| AddressError::BadAddress(whole.to_string()))
}

impl FromStr for Address {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (doc, lines) = s.split_once('#').ok_or_else(|| AddressError::BadAddress(s.to_string()))?;
        let doc: DocId = doc.parse()?;
        let (start, end) = match lines.split_once('-') {
            Some((a, b)) => (parse_line(a, s)?, parse_line(b, s)?),
            None => {
                let n = parse_line(lines, s)?;
                (n, n)
            }
        };
        Address::new(doc, start, end)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}#L{}", self.doc, self.start)
        } else {
            write!(f, "{}#L{}-L{}", self.doc, self.start, self.end)
        }
    }
}

impl TryFrom<String> for Address {
    type Error = AddressError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Address> for String {
    fn from(a: Address) -> String {
        a.to_string()
    }
}

// Both serialise as plain strings (`#[serde(into = "String")]`), so the schema is a string with the grammar.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for DocId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DocId".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "Document id: 16 lowercase hex characters, optionally followed by -slug.",
            "pattern": "^[0-9a-f]{16}(-[a-z0-9]+(-[a-z0-9]+)*)?$"
        })
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Address {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Address".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "Line address: <doc_id>#L<start> or <doc_id>#L<start>-L<end> (1-based, inclusive).",
            "pattern": "^[0-9a-f]{16}(-[a-z0-9]+(-[a-z0-9]+)*)?#L[1-9][0-9]*(-L[1-9][0-9]*)?$"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_full_and_short_forms() {
        for s in [
            "3f9a1c07b2e4d55e-cowan-magical-number-four#L120-L148",
            "3f9a1c07b2e4d55e#L120-L148",
            "3f9a1c07b2e4d55e#L7",
            "00000000000000ff-a#L1-L2",
        ] {
            let a: Address = s.parse().unwrap();
            assert_eq!(a.to_string(), s);
        }
    }

    #[test]
    fn single_line_range_prints_short() {
        let a: Address = "3f9a1c07b2e4d55e#L9-L9".parse().unwrap();
        assert_eq!(a.to_string(), "3f9a1c07b2e4d55e#L9");
    }

    #[test]
    fn slug_does_not_change_identity() {
        let a: DocId = "3f9a1c07b2e4d55e-some-title".parse().unwrap();
        let b: DocId = "3f9a1c07b2e4d55e".parse().unwrap();
        assert!(a.same_document(&b));
        assert_eq!(a.shard(), "3f");
    }

    #[test]
    fn rejects_malformed() {
        for s in [
            "3F9A1C07B2E4D55E#L1",        // uppercase hex
            "3f9a1c07b2e4d55#L1",         // 15 hex
            "3f9a1c07b2e4d55e#L0",        // line zero
            "3f9a1c07b2e4d55e#L5-L4",     // backwards
            "3f9a1c07b2e4d55e#L05",       // leading zero
            "3f9a1c07b2e4d55e#120-148",   // missing L
            "3f9a1c07b2e4d55e-Bad_Slug#L1",
            "3f9a1c07b2e4d55e--x#L1",
            "C:\\vault\\text\\3f#L1",
        ] {
            assert!(s.parse::<Address>().is_err(), "accepted {s}");
        }
        let long = format!("3f9a1c07b2e4d55e-{}", "a".repeat(SLUG_MAX + 1));
        assert!(long.parse::<DocId>().is_err());
    }

    #[test]
    fn serde_uses_the_string_form() {
        let a: Address = "3f9a1c07b2e4d55e#L3-L4".parse().unwrap();
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "\"3f9a1c07b2e4d55e#L3-L4\"");
        assert_eq!(serde_json::from_str::<Address>(&json).unwrap(), a);
        assert!(serde_json::from_str::<Address>("\"nope\"").is_err());
    }
}
