//! Report schema and extrapolation math.

use crate::discover::{DocKind, FileTotals};
use crate::triage::TriageCounts;
use serde::Serialize;
use std::collections::BTreeMap;

/// Decimal gigabyte, matching how drive and corpus sizes are quoted.
pub const BYTES_PER_GB: f64 = 1e9;

/// Corpus sizes (GB) the measured ratios are projected to.
pub const PROJECTION_GB: [u64; 2] = [50, 250];

#[derive(Debug, Serialize)]
pub struct Report {
    pub root: String,
    pub triage_thresholds: TriageThresholds,
    pub totals: Totals,
    pub extrapolation: Extrapolation,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub walk_errors: Vec<String>,
    pub files: Vec<FileRecord>,
}

#[derive(Debug, Serialize)]
pub struct TriageThresholds {
    pub min_text_chars_per_page: usize,
    pub math_char_ratio_min: f64,
    pub short_token_share_min: f64,
}

/// `files`, `bytes` and `by_type` count every document copy. Everything from `unique_files`
/// down is measured once per distinct content hash.
#[derive(Debug, Default, Serialize)]
pub struct Totals {
    pub files: u64,
    pub bytes: u64,
    pub by_type: BTreeMap<DocKind, FileTotals>,
    /// Non-document files, counted from metadata only.
    pub other_by_extension: BTreeMap<String, FileTotals>,
    pub unique_files: u64,
    pub unique_bytes: u64,
    pub duplicate_groups: u64,
    /// Extra copies beyond the first in each duplicate group.
    pub duplicate_files: u64,
    pub duplicate_bytes: u64,
    pub pages: u64,
    pub pages_by_class: TriageCounts,
    pub characters: u64,
    pub whitespace_tokens: u64,
    pub tokens_est: u64,
    /// Unique documents with an error, plus directory-walk errors.
    pub errors: u64,
    pub error_bytes: u64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Extrapolation {
    pub basis: &'static str,
    pub measured_bytes: u64,
    pub tokens_est_per_gb: Option<f64>,
    pub pages_per_gb: Option<f64>,
    pub projections: Vec<Projection>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Projection {
    pub corpus_gb: u64,
    pub tokens_est: Option<u64>,
    pub pages: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateGroup {
    pub blake3: String,
    pub bytes_each: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileRecord {
    pub path: String,
    pub kind: DocKind,
    pub bytes: u64,
    pub blake3: Option<String>,
    /// Set on every copy after the first; measurements are copied from that file.
    pub duplicate_of: Option<String>,
    pub pages: Option<u64>,
    pub triage: Option<TriageCounts>,
    pub characters: Option<u64>,
    pub whitespace_tokens: Option<u64>,
    pub tokens_est: Option<u64>,
    pub error: Option<String>,
}

/// Ratios per GB of `measured_bytes` (unique documents measured without error), projected to
/// each of [`PROJECTION_GB`]. Ratios are `None` when nothing was measured.
pub fn extrapolate(measured_bytes: u64, tokens_est: u64, pages: u64) -> Extrapolation {
    let per_gb = |count: u64| {
        (measured_bytes > 0).then(|| count as f64 * BYTES_PER_GB / measured_bytes as f64)
    };
    let tokens_est_per_gb = per_gb(tokens_est);
    let pages_per_gb = per_gb(pages);
    let project = |ratio: Option<f64>, gb: u64| ratio.map(|r| (r * gb as f64).round() as u64);
    Extrapolation {
        basis: "unique documents measured without error; GB = 1e9 bytes",
        measured_bytes,
        tokens_est_per_gb,
        pages_per_gb,
        projections: PROJECTION_GB
            .iter()
            .map(|&gb| Projection {
                corpus_gb: gb,
                tokens_est: project(tokens_est_per_gb, gb),
                pages: project(pages_per_gb, gb),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrapolates_per_gb_and_projects() {
        // 0.5 GB measured, 125M tokens, 200k pages -> 250M tokens/GB, 400k pages/GB.
        let e = extrapolate(500_000_000, 125_000_000, 200_000);
        assert_eq!(e.measured_bytes, 500_000_000);
        assert_eq!(e.tokens_est_per_gb, Some(250_000_000.0));
        assert_eq!(e.pages_per_gb, Some(400_000.0));
        assert_eq!(
            e.projections,
            vec![
                Projection { corpus_gb: 50, tokens_est: Some(12_500_000_000), pages: Some(20_000_000) },
                Projection { corpus_gb: 250, tokens_est: Some(62_500_000_000), pages: Some(100_000_000) },
            ]
        );
    }

    #[test]
    fn projections_round_to_nearest() {
        // 3 bytes -> 1/3 token per byte; 50 GB -> 16,666,666,666.67 tokens.
        let e = extrapolate(3, 1, 0);
        assert_eq!(e.projections[0].tokens_est, Some(16_666_666_667));
        assert_eq!(e.projections[0].pages, Some(0));
    }

    #[test]
    fn nothing_measured_gives_no_ratios() {
        let e = extrapolate(0, 0, 0);
        assert_eq!(e.tokens_est_per_gb, None);
        assert_eq!(e.pages_per_gb, None);
        assert_eq!(e.projections.len(), PROJECTION_GB.len());
        assert!(e.projections.iter().all(|p| p.tokens_est.is_none() && p.pages.is_none()));
    }
}
