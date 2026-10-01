//! The typed sidecar (SPEC section 3a): structure that never changes, plus metadata that intake may rewrite
//! (SQLite is authoritative for metadata; the sidecar mirrors it).

use crate::canon::BlockSpan;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use vault_api::{DocKind, Heading, PageMark, Symbol};

pub const SIDECAR_FORMAT: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identifiers {
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub isbn: Vec<String>,
    pub pmcid: Option<String>,
    pub openalex: Option<String>,
    /// Any other scheme (`pmid`, `issn`, ...), kept rather than dropped.
    #[serde(flatten)]
    pub other: BTreeMap<String, String>,
}

impl Identifiers {
    /// Every identifier as (scheme, value), for the `doc_identifiers` table.
    pub fn pairs(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (scheme, value) in
            [("doi", &self.doi), ("arxiv", &self.arxiv), ("pmcid", &self.pmcid), ("openalex", &self.openalex)]
        {
            if let Some(v) = value {
                out.push((scheme.to_string(), v.clone()));
            }
        }
        out.extend(self.isbn.iter().map(|v| ("isbn".to_string(), v.clone())));
        out.extend(self.other.iter().map(|(k, v)| (k.clone(), v.clone())));
        out
    }
}

/// For code files: where the file lives and what it defines.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeInfo {
    /// `repos.repo_id` of the snapshot this file belongs to, when it came from a repository.
    pub repo: Option<String>,
    /// Path inside the repository (or the delivered path), `/`-separated.
    pub path: String,
    pub language: Option<String>,
    pub swhid: Option<String>,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Extraction {
    /// Route, for example `jats`, `markdown`, `pdfium`, `code`.
    pub path: String,
    pub tool: String,
    pub extract_fp: String,
    pub encoding: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Quality {
    pub score: f64,
    pub garbled_ratio: f64,
    pub empty_pages: u32,
    pub ocr_confidence: Option<f64>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// BLAKE3 of the original's bytes.
    pub original: String,
    pub ext: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Provenance {
    pub batch: String,
    pub received: String,
    /// Path inside the batch, `/`-separated.
    pub path: String,
    /// The manifest row as delivered, verbatim.
    pub manifest: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Categories {
    pub primary: Option<String>,
    pub secondary: Vec<String>,
    pub openalex_topic: Option<String>,
    pub areas: Vec<String>,
    pub source: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identity {
    pub confidence: Option<f64>,
    pub source: Option<String>,
    pub flags: Vec<String>,
}

/// The fields intake may rewrite after publication.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SidecarMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub language: Option<String>,
    pub identifiers: Identifiers,
    pub categories: Option<Categories>,
    pub identity: Option<Identity>,
    pub quality: Option<Quality>,
    pub provenance: Vec<Provenance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sidecar {
    pub format: u32,
    pub doc_id: String,
    pub kind: DocKind,
    pub lines: u32,
    pub bytes: u64,
    pub title: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub language: Option<String>,
    #[serde(default)]
    pub identifiers: Identifiers,
    #[serde(default)]
    pub headings: Vec<Heading>,
    #[serde(default)]
    pub pages: Vec<PageMark>,
    #[serde(default)]
    pub blocks: Vec<BlockSpan>,
    pub code: Option<CodeInfo>,
    pub extraction: Extraction,
    pub quality: Option<Quality>,
    #[serde(default)]
    pub sources: Vec<SourceRef>,
    #[serde(default)]
    pub provenance: Vec<Provenance>,
    pub categories: Option<Categories>,
    pub identity: Option<Identity>,
    pub created: String,
    pub supersedes: Option<String>,
}

impl Sidecar {
    /// Names of the structural fields that differ between `self` and `other` (empty when they agree).
    pub fn structural_differences(&self, other: &Sidecar) -> Vec<&'static str> {
        let mut d = Vec::new();
        let mut check = |name: &'static str, same: bool| {
            if !same {
                d.push(name);
            }
        };
        check("format", self.format == other.format);
        check("doc_id", self.doc_id == other.doc_id);
        check("kind", self.kind == other.kind);
        check("lines", self.lines == other.lines);
        check("bytes", self.bytes == other.bytes);
        check("headings", self.headings == other.headings);
        check("pages", self.pages == other.pages);
        check("blocks", self.blocks == other.blocks);
        check("code", self.code == other.code);
        check("extraction", self.extraction == other.extraction);
        check("sources", self.sources == other.sources);
        check("supersedes", self.supersedes == other.supersedes);
        d
    }

    /// Refuses `next` if it changes any structural field; `created` always keeps its first value.
    pub fn check_replacement(&self, next: &Sidecar) -> Result<()> {
        let diff = self.structural_differences(next);
        if diff.is_empty() {
            Ok(())
        } else {
            Err(CoreError::SidecarImmutable { doc_id: self.doc_id.clone(), fields: diff.join(", ") })
        }
    }

    pub fn metadata(&self) -> SidecarMetadata {
        SidecarMetadata {
            title: self.title.clone(),
            authors: self.authors.clone(),
            year: self.year,
            language: self.language.clone(),
            identifiers: self.identifiers.clone(),
            categories: self.categories.clone(),
            identity: self.identity.clone(),
            quality: self.quality.clone(),
            provenance: self.provenance.clone(),
        }
    }

    /// The same sidecar with its metadata fields replaced; structure is untouched by construction.
    pub fn with_metadata(&self, m: SidecarMetadata) -> Sidecar {
        Sidecar {
            title: m.title,
            authors: m.authors,
            year: m.year,
            language: m.language,
            identifiers: m.identifiers,
            categories: m.categories,
            identity: m.identity,
            quality: m.quality,
            provenance: m.provenance,
            ..self.clone()
        }
    }

    pub fn to_json(&self) -> Result<Vec<u8>> {
        let mut v = serde_json::to_vec_pretty(self).map_err(|e| CoreError::json("serialise sidecar", e))?;
        v.push(b'\n');
        Ok(v)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Sidecar> {
        serde_json::from_slice(bytes).map_err(|e| CoreError::json("parse sidecar", e))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample() -> Sidecar {
        Sidecar {
            format: SIDECAR_FORMAT,
            doc_id: "3f9a1c07b2e4d55e-cowan".into(),
            kind: DocKind::Paper,
            lines: 10,
            bytes: 300,
            title: Some("The magical number 4".into()),
            authors: vec!["Nelson Cowan".into()],
            year: Some(2001),
            language: Some("en".into()),
            identifiers: Identifiers {
                doi: Some("10.1017/S0140525X01003922".into()),
                isbn: vec!["978-0".into()],
                other: [("pmid".to_string(), "11515286".to_string())].into(),
                ..Identifiers::default()
            },
            headings: vec![Heading { level: 1, text: "Intro".into(), start: 1, end: 10, page: Some("87".into()) }],
            pages: vec![PageMark { label: "87".into(), index: 1, line: 1 }],
            blocks: vec![BlockSpan { kind: "table".into(), start: 4, end: 6 }],
            code: None,
            extraction: Extraction {
                path: "jats".into(),
                tool: "t".into(),
                extract_fp: "fp".into(),
                encoding: "utf-8".into(),
            },
            quality: Some(Quality { score: 0.97, ..Quality::default() }),
            sources: vec![SourceRef { original: "ab".repeat(32), ext: "pdf".into(), bytes: 1234 }],
            provenance: vec![Provenance {
                batch: "b1".into(),
                received: "2026-10-02".into(),
                path: "papers/x.pdf".into(),
                manifest: serde_json::json!({"doi": "10.1017/S0140525X01003922", "extra_col": "kept"}),
            }],
            categories: Some(Categories {
                primary: Some("psych-cogsci".into()),
                areas: vec!["memory".into()],
                ..Categories::default()
            }),
            identity: Some(Identity { confidence: Some(0.95), ..Identity::default() }),
            created: "2026-10-02".into(),
            supersedes: None,
        }
    }

    #[test]
    fn round_trips_through_json_with_the_documented_keys() {
        let s = sample();
        let json = s.to_json().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&json).unwrap();
        for key in [
            "format",
            "doc_id",
            "kind",
            "lines",
            "bytes",
            "title",
            "authors",
            "year",
            "language",
            "identifiers",
            "headings",
            "pages",
            "blocks",
            "code",
            "extraction",
            "quality",
            "sources",
            "provenance",
            "categories",
            "identity",
            "created",
            "supersedes",
        ] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert_eq!(v["kind"], "paper");
        assert_eq!(v["identifiers"]["pmid"], "11515286", "unknown identifier schemes are kept");
        assert_eq!(Sidecar::from_json(&json).unwrap(), s);
        let pairs = s.identifiers.pairs();
        assert!(pairs.contains(&("doi".into(), "10.1017/S0140525X01003922".into())));
        assert!(pairs.contains(&("isbn".into(), "978-0".into())));
        assert!(pairs.contains(&("pmid".into(), "11515286".into())));
    }

    #[test]
    fn metadata_may_change_structure_may_not() {
        let s = sample();
        let mut m = s.metadata();
        m.title = Some("Renamed".into());
        m.categories = None;
        m.provenance.push(Provenance { batch: "b2".into(), ..Provenance::default() });
        let next = s.with_metadata(m);
        assert!(s.check_replacement(&next).is_ok());
        assert_eq!(next.title.as_deref(), Some("Renamed"));
        assert_eq!(next.headings, s.headings);

        for mutate in [
            (|x: &mut Sidecar| x.lines += 1) as fn(&mut Sidecar),
            |x| x.headings.clear(),
            |x| x.pages[0].line = 2,
            |x| x.blocks.clear(),
            |x| x.code = Some(CodeInfo::default()),
            |x| x.extraction.tool = "other".into(),
            |x| x.sources.clear(),
            |x| x.doc_id = "other".into(),
            |x| x.kind = DocKind::Book,
            |x| x.bytes = 0,
            |x| x.supersedes = Some("x".into()),
        ] {
            let mut bad = s.clone();
            mutate(&mut bad);
            assert!(matches!(s.check_replacement(&bad), Err(CoreError::SidecarImmutable { .. })));
        }
    }
}
