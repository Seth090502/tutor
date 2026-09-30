//! Read-only data for the tracker screens: eval runs (contract C2) and the build plan.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct EvalRuns {
    pub runs: Vec<Value>,
    pub warnings: Vec<String>,
}

/// Every `<root>/evals/runs/*.json`, newest `created_at` first. Files that are unreadable or
/// lack a string `run_id` and numeric `created_at` are skipped and named in `warnings`.
/// A missing runs directory means no runs yet.
pub fn eval_runs(root: &Path) -> EvalRuns {
    let dir = root.join("evals").join("runs");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return EvalRuns {
                runs: Vec::new(),
                warnings: Vec::new(),
            };
        }
        Err(e) => {
            return EvalRuns {
                runs: Vec::new(),
                warnings: vec![format!("{}: {e}", dir.display())],
            }
        }
    };
    let mut runs = Vec::new();
    let mut warnings = Vec::new();
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(e) => {
                warnings.push(format!("{}: {e}", dir.display()));
                continue;
            }
        };
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        match read_run(&path) {
            Ok((created_at, run)) => runs.push((created_at, name, run)),
            Err(e) => warnings.push(format!("skipped {name}: {e}")),
        }
    }
    // Newest first; ties broken by file name so the order is stable.
    runs.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    warnings.sort();
    EvalRuns {
        runs: runs.into_iter().map(|(_, _, run)| run).collect(),
        warnings,
    }
}

fn read_run(path: &Path) -> Result<(f64, Value), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let run: Value = serde_json::from_str(&text).map_err(|e| format!("invalid JSON: {e}"))?;
    if !run.get("run_id").is_some_and(Value::is_string) {
        return Err("missing string run_id".into());
    }
    let created_at = run
        .get("created_at")
        .and_then(Value::as_f64)
        .ok_or("missing numeric created_at")?;
    Ok((created_at, run))
}

#[derive(Debug, Serialize)]
pub struct Plan {
    pub markdown: String,
}

pub fn plan_markdown(root: &Path) -> Result<Plan, String> {
    let path = root.join("docs").join("tutor").join("PLAN.md");
    std::fs::read_to_string(&path)
        .map(|markdown| Plan { markdown })
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write(dir: &Path, name: &str, text: &str) {
        std::fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn newest_first_and_bad_files_become_warnings() {
        let root = tempfile::tempdir().unwrap();
        let runs = root.path().join("evals").join("runs");
        std::fs::create_dir_all(&runs).unwrap();
        write(
            &runs,
            "a.json",
            r#"{"run_id": "old", "created_at": 1790000000.0, "decision": "baseline"}"#,
        );
        write(
            &runs,
            "b.json",
            r#"{"run_id": "new", "created_at": 1790000500.5}"#,
        );
        write(
            &runs,
            "c.json",
            r#"{"run_id": "mid", "created_at": 1790000100}"#,
        );
        write(&runs, "broken.json", "{\"run_id\": ");
        write(&runs, "no_time.json", r#"{"run_id": "x"}"#);
        write(&runs, "no_id.json", r#"{"created_at": 1.0}"#);
        write(&runs, "notes.txt", "ignored");

        let out = eval_runs(root.path());
        let ids: Vec<_> = out
            .runs
            .iter()
            .map(|r| r["run_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["new", "mid", "old"]);
        assert_eq!(
            out.runs[2]["decision"],
            json!("baseline"),
            "runs pass through unchanged"
        );
        assert_eq!(out.warnings.len(), 3, "{:?}", out.warnings);
        assert!(out.warnings[0].starts_with("skipped broken.json: invalid JSON"));
        assert_eq!(out.warnings[1], "skipped no_id.json: missing string run_id");
        assert_eq!(
            out.warnings[2],
            "skipped no_time.json: missing numeric created_at"
        );
    }

    #[test]
    fn no_runs_directory_means_no_runs() {
        let root = tempfile::tempdir().unwrap();
        let out = eval_runs(root.path());
        assert!(out.runs.is_empty() && out.warnings.is_empty());
    }

    #[test]
    fn serializes_the_contract_shape() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            serde_json::to_value(eval_runs(root.path())).unwrap(),
            json!({"runs": [], "warnings": []})
        );
    }

    #[test]
    fn plan_reads_the_tracker_file() {
        let root = tempfile::tempdir().unwrap();
        assert!(plan_markdown(root.path()).unwrap_err().contains("PLAN.md"));
        std::fs::create_dir_all(root.path().join("docs/tutor")).unwrap();
        write(&root.path().join("docs/tutor"), "PLAN.md", "# Plan\n");
        assert_eq!(plan_markdown(root.path()).unwrap().markdown, "# Plan\n");
    }
}
