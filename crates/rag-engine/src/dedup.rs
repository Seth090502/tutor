//! Exact duplicate detection by BLAKE3 content hash.

use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn hash_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("cannot open file: {e}"))?;
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(file)
        .map_err(|e| format!("cannot read file: {e}"))?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Hash every path in parallel, keeping input order. `progress` is called with the number of
/// files finished so far.
pub fn hash_all<P: AsRef<Path> + Sync>(
    paths: &[P],
    progress: impl Fn(usize) + Sync,
) -> Vec<Result<String, String>> {
    let done = AtomicUsize::new(0);
    paths
        .par_iter()
        .map(|p| {
            let result = hash_file(p.as_ref());
            progress(done.fetch_add(1, Ordering::Relaxed) + 1);
            result
        })
        .collect()
}

/// Group indices of equal hashes. Returns only groups with two or more members, each in input
/// order, and the groups ordered by their first member. `None` (unhashed) never matches.
pub fn duplicate_groups(hashes: &[Option<&str>]) -> Vec<Vec<usize>> {
    let mut by_hash: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, hash) in hashes.iter().enumerate() {
        if let Some(hash) = hash {
            by_hash.entry(hash).or_default().push(i);
        }
    }
    let mut groups: Vec<Vec<usize>> = by_hash.into_values().filter(|g| g.len() > 1).collect();
    groups.sort_by_key(|g| g[0]);
    groups
}

/// Bytes removed by keeping one copy per group.
pub fn duplicate_bytes(groups: &[Vec<usize>], bytes: &[u64]) -> u64 {
    groups
        .iter()
        .map(|g| (g.len() as u64 - 1) * bytes[g[0]])
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn groups_equal_hashes_in_input_order() {
        let hashes = [
            Some("b"),
            Some("a"),
            None,
            Some("b"),
            Some("c"),
            Some("a"),
            Some("b"),
            None,
        ];
        assert_eq!(duplicate_groups(&hashes), vec![vec![0, 3, 6], vec![1, 5]]);
    }

    #[test]
    fn no_duplicates_means_no_groups() {
        assert!(duplicate_groups(&[Some("a"), Some("b"), None, None]).is_empty());
        assert!(duplicate_groups(&[]).is_empty());
    }

    #[test]
    fn duplicate_bytes_counts_all_but_one_copy() {
        let groups = vec![vec![0, 3, 6], vec![1, 5]];
        let bytes = [100, 7, 0, 100, 1, 7, 100];
        assert_eq!(duplicate_bytes(&groups, &bytes), 200 + 7);
    }

    #[test]
    fn hash_all_matches_blake3_and_reports_errors_in_order() {
        let dir = std::env::temp_dir().join(format!("rag-engine-dedup-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.bin");
        fs::write(&a, b"same bytes").unwrap();
        let missing = dir.join("missing.bin");
        let calls = AtomicUsize::new(0);

        let got = hash_all(&[a.clone(), missing, a], |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        });
        fs::remove_dir_all(&dir).unwrap();

        let expected = blake3::hash(b"same bytes").to_hex().to_string();
        assert_eq!(got[0].as_deref(), Ok(expected.as_str()));
        assert!(got[1].is_err());
        assert_eq!(got[2].as_deref(), Ok(expected.as_str()));
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }
}
