//! The stdin/stdout contract tutor/corpus.py relies on: one JSON line per query line.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn one_json_line_per_query() {
    let root = std::env::temp_dir().join(format!("tutor-search-proto-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("corpus")).unwrap();
    fs::write(root.join("corpus").join("n.md"), "Newton: F=ma; action and reaction.").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tutor-search"))
        .args(["--root", root.to_str().unwrap(), "--limit", "3"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"newton reaction\r\n\nzebra\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());

    let lines: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0][0]["path"], "corpus/n.md");
    let score = lines[0][0]["score"].as_f64().unwrap();
    assert!(score > 0.0 && score == (score * 1000.0).round() / 1000.0);
    assert_eq!(lines[1], serde_json::json!([]));
    assert_eq!(lines[2], serde_json::json!([]));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn bad_limit_fails_loudly() {
    let out = Command::new(env!("CARGO_BIN_EXE_tutor-search"))
        .args(["--limit", "many"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--limit"));
}
