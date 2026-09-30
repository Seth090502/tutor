//! Locating the Tutor repo root that holds the backend, rag-engine, evals and plan.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// `TUTOR_ROOT` if set, else the repo this crate was compiled from
/// (`CARGO_MANIFEST_DIR/../../..`). The chosen directory must look like the repo.
pub fn discover(tutor_root: Option<OsString>) -> Result<PathBuf, String> {
    let root = match tutor_root {
        Some(value) if !value.is_empty() => {
            std::path::absolute(PathBuf::from(value)).map_err(|e| format!("TUTOR_ROOT: {e}"))?
        }
        _ => compiled_root(),
    };
    validate(&root)?;
    Ok(root)
}

fn compiled_root() -> PathBuf {
    // apps/desktop/src-tauri -> repo root, without `..` components in the result.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("CARGO_MANIFEST_DIR is apps/desktop/src-tauri inside the repo")
        .to_path_buf()
}

fn validate(root: &Path) -> Result<(), String> {
    if root.join("deeptutor").is_dir() && root.join("Cargo.toml").is_file() {
        Ok(())
    } else {
        Err(format!(
            "{} is not the Tutor repo (it needs deeptutor/ and Cargo.toml); set TUTOR_ROOT to the repo",
            root.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_the_repo_this_crate_was_built_from() {
        let root = discover(None).unwrap();
        assert!(root.join("apps/desktop/src-tauri/Cargo.toml").is_file());
        assert_eq!(discover(Some(OsString::new())).unwrap(), root);
    }

    #[test]
    fn env_var_wins_when_it_points_at_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("deeptutor")).unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(discover(Some(dir.path().into())).unwrap(), dir.path());
    }

    #[test]
    fn env_var_pointing_elsewhere_is_rejected_not_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap(); // no deeptutor/
        let err = discover(Some(dir.path().into())).unwrap_err();
        assert!(err.contains(&dir.path().display().to_string()), "{err}");
        assert!(err.contains("TUTOR_ROOT"), "{err}");
    }
}
