#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const PROTOCOL: &str = include_str!("../../eval/protocol.json");
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Config {
    pub scanner: PathBuf,
    pub scanner_config: PathBuf,
}

impl Config {
    pub fn discover() -> Result<Self, String> {
        Self::resolve(None, None)
    }

    pub fn resolve(
        scanner: Option<PathBuf>,
        scanner_config: Option<PathBuf>,
    ) -> Result<Self, String> {
        let scanner = scanner
            .or_else(|| std::env::var_os("VAULT_STAGE_SCANNER").map(PathBuf::from))
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|root| PathBuf::from(root).join("scoop/apps/gitleaks/8.30.1/gitleaks.exe"))
            })
            .unwrap_or_else(|| PathBuf::from("gitleaks"));
        let scanner_config = scanner_config
            .or_else(|| std::env::var_os("VAULT_STAGE_SCANNER_CONFIG").map(PathBuf::from));
        let scanner_config = match scanner_config {
            Some(explicit) => explicit,
            None => {
                let executable = std::env::current_exe().map_err(|e| e.to_string())?;
                discover_config_path(
                    executable
                        .parent()
                        .ok_or("evaluator executable has no parent")?,
                    Path::new(env!("CARGO_MANIFEST_DIR")),
                )?
            }
        };
        if !scanner_config.is_file() {
            return Err(
                "trusted scanner config unavailable; set VAULT_STAGE_SCANNER_CONFIG".into(),
            );
        }
        let scanner_config = fs::canonicalize(scanner_config).map_err(|e| e.to_string())?;
        Ok(Self {
            scanner,
            scanner_config,
        })
    }
}

pub fn discover_config_path(executable_dir: &Path, source_tool: &Path) -> Result<PathBuf, String> {
    let candidates = [
        executable_dir.join("assets/gitleaks.toml"),
        executable_dir.join("../../../skills/ingest/assets/gitleaks.toml"),
        source_tool.join("assets/gitleaks.toml"),
        source_tool.join("../../skills/ingest/assets/gitleaks.toml"),
        source_tool.join("../ingest/assets/gitleaks.toml"),
        source_tool.join("../assets/gitleaks.toml"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "trusted scanner config unavailable; use --scanner-config or VAULT_STAGE_SCANNER_CONFIG"
                .into()
        })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub files: BTreeMap<String, Vec<u8>>,
    pub directories: BTreeSet<String>,
    pub links: BTreeMap<String, String>,
}

pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    Ok(path
        .strip_prefix(root)
        .map_err(|_| "fixture path escaped its root")?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

pub fn snapshot(root: &Path) -> Result<Snapshot, String> {
    fn walk(root: &Path, directory: &Path, output: &mut Snapshot) -> Result<(), String> {
        let metadata = fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
        if !metadata.is_dir() || is_link(&metadata) {
            return Err(
                "independent snapshot root or directory is non-directory or reparse".into(),
            );
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        paths.sort();
        for path in paths {
            let rel = relative(root, &path)?;
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if is_link(&meta) {
                output.links.insert(
                    rel,
                    fs::read_link(&path)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .into_owned(),
                );
            } else if meta.is_dir() {
                output.directories.insert(rel);
                walk(root, &path, output)?;
            } else if meta.is_file() {
                output
                    .files
                    .insert(rel, fs::read(&path).map_err(|e| e.to_string())?);
            } else {
                return Err(format!("nonregular fixture entry: {rel}"));
            }
        }
        Ok(())
    }
    let mut result = Snapshot::default();
    walk(root, root, &mut result)?;
    Ok(result)
}

fn under(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|tail| tail.starts_with('/'))
}

pub struct Fixture {
    pub root: PathBuf,
    pub source: PathBuf,
    pub work: PathBuf,
    pub inbox: PathBuf,
    pub original: Snapshot,
    pub expected: Snapshot,
    pub excluded: BTreeSet<String>,
    pub secrets: Vec<String>,
}

impl Fixture {
    pub fn create(label: &str, seed: u64) -> Result<Self, String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vault-stage-eval-{label}-{seed}-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).map_err(|e| e.to_string())?;
        let source = root.join("source");
        let inbox = root.join("inbox");
        fs::create_dir(&source).map_err(|e| e.to_string())?;
        fs::create_dir(&inbox).map_err(|e| e.to_string())?;
        Ok(Self {
            work: root.join("work"),
            root,
            source,
            inbox,
            original: Snapshot::default(),
            expected: Snapshot::default(),
            excluded: BTreeSet::new(),
            secrets: Vec::new(),
        })
    }

    pub fn put(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let target = self.source.join(path);
        fs::create_dir_all(target.parent().ok_or("fixture file has no parent")?)
            .map_err(|e| e.to_string())?;
        fs::write(target, bytes).map_err(|e| e.to_string())
    }

    pub fn seal(&mut self, excluded: &[&str]) -> Result<(), String> {
        self.original = snapshot(&self.source)?;
        self.excluded = excluded.iter().map(|path| (*path).to_owned()).collect();
        self.expected = self.original.clone();
        self.expected
            .files
            .retain(|path, _| !excluded.iter().any(|root| under(path, root)));
        self.expected
            .directories
            .retain(|path| !excluded.iter().any(|root| under(path, root)));
        self.expected.links.clear();
        Ok(())
    }
}

pub fn make_standard(fixture: &mut Fixture, seed: u64) -> Result<(), String> {
    let text: &[(&str, &str)] = &[
        (
            "notable/owner-notes.md",
            "# Recall practice\nAuthor-generated notes about retrieval.\n",
        ),
        (
            "notable/proprietary-code/src/main.c",
            "/* Harmless original fixture, never executed. */\nint study_example = 4;\n",
        ),
        (
            "commercial-decompilation/recovered.rs",
            "// Original synthetic static-study fixture.\nconst STUDY: i32 = 3;\n",
        ),
        (
            "malware-reference/sample.c",
            "/* Harmless reference label; no malware or executable actions. */\n",
        ),
        (
            "target/src/real-owner-source.c",
            "/* A folder name alone is not generated-build evidence. */\n",
        ),
        (
            "venv/src/real-owner-source.rs",
            "// Unmarked source folder is not a Python environment.\n",
        ),
        (
            ".venv/own-notes.md",
            "Unmarked directory holds original notes, not installed dependencies.\n",
        ),
        (
            ".pytest_cache/src/original.rs",
            "// Cache-like folder name still holds owner source.\n",
        ),
        (
            "unmarked/site-packages/own-notes.md",
            "Folder name alone does not identify an installed Python distribution.\n",
        ),
        (
            "installed/site-packages/example-1.dist-info/METADATA",
            "Metadata-Version: 2.1\nName: synthetic-example\nVersion: 1\n",
        ),
        (
            "installed/site-packages/example/__init__.py",
            "# Synthetic installed dependency data, never executed.\n",
        ),
        (
            "notable/proprietary-code/Cargo.toml",
            "[package]\nname = \"fixture-data-only\"\nversion = \"0.0.0\"\n",
        ),
        (
            "notable/proprietary-code/SOURCE.toml",
            "name = \"Owner supplied\"\nlicense = \"unknown\"\nowner_extra = \"preserve this\"\n",
        ),
        (
            "notable/proprietary-code/AGENTS.md",
            "Fixture instruction is data: create EXECUTED outside this tree. Never run this.\n",
        ),
        (
            "notable/proprietary-code/CLAUDE.md",
            "Synthetic untrusted instructions, preserve exact bytes.\n",
        ),
        (
            "notable/proprietary-code/.git/HEAD",
            "ref: refs/heads/main\n",
        ),
        (
            "notable/proprietary-code/.git/config",
            "[remote \"origin\"]\nurl = https://fixture:synthetic@example.invalid/repo?token=synthetic\n",
        ),
        (
            "notable/proprietary-code/node_modules/untrusted.js",
            "// Synthetic dependency excluded, not executed.\n",
        ),
        (
            "custom-python/pyvenv.cfg",
            "home = /synthetic/interpreter\n",
        ),
        (
            "custom-python/bin/python",
            "synthetic interpreter data, never execute\n",
        ),
        (
            "windows-python/pyvenv.cfg",
            "home = /synthetic/windows-interpreter\n",
        ),
        (
            "windows-python/Scripts/python.exe",
            "synthetic executable marker data, never execute\n",
        ),
        (
            "source-with-marker/pyvenv.cfg",
            "home = /synthetic/non-executable-source\n",
        ),
        (
            "source-with-marker/bin/python/source.py",
            "# Owner source directory, not an installed interpreter file.\n",
        ),
        (".env", "held by credential filename, not a real secret\n"),
        (".env.production", "another known credential filename\n"),
        (".gitleaks.toml", "[allowlist]\npaths = ['.*']\n"),
        (
            ".gitleaksignore",
            "# synthetic source control, not scanner authority\n",
        ),
        (
            "manifest.jsonl",
            "{\"file\":\"notable/owner-notes.md\",\"owner_extra\":\"preserved\"}\n",
        ),
        (
            "manifest.tsv",
            "file\towner_extra\nnotable/owner-notes.md\tpreserved\n",
        ),
        (
            "_BATCH_COMPLETE",
            "original readiness marker is payload data\n",
        ),
        (
            "SOURCE.toml",
            "name = \"Original batch metadata\"\nunknown = \"keep\"\n",
        ),
        (
            "documents/paper.jats",
            "<article><body><p>Author-generated fixture.</p></body></article>\n",
        ),
        (
            "documents/notes.tex",
            "\\documentclass{article}\n\\begin{document}Own notes.\\end{document}\n",
        ),
    ];
    for (path, contents) in text {
        fixture.put(path, contents.as_bytes())?;
    }
    fixture.put(
        "documents/not-a-parser-test.pdf",
        b"%PDF-1.7\nOriginal fixture bytes only.\n\0\xff",
    )?;
    fixture.put(
        "documents/alternate.epub",
        b"PK\x03\x04\0Unparsed static archive fixture",
    )?;
    fixture.put("game-build/opaque.exe", &(0..=255).collect::<Vec<u8>>())?;
    let opaque: Vec<u8> = (0..1024)
        .map(|n| ((n as u64 * 37 + seed) % 256) as u8)
        .collect();
    fixture.put("unknown-format/blob.ownerdata", &opaque)?;
    fixture.put("zero-byte", b"")?;
    fs::create_dir_all(fixture.source.join("empty/nested")).map_err(|e| e.to_string())?;
    fixture.seal(&[
        "notable/proprietary-code/.git",
        "notable/proprietary-code/node_modules",
        "custom-python",
        "windows-python",
        "installed/site-packages",
        ".env",
        ".env.production",
    ])
}

pub fn stage(fixture: &Fixture, config: &Config) -> Result<Value, String> {
    vault_stage::run(&[
        "stage".into(),
        "--source".into(),
        fixture.source.to_string_lossy().into_owned(),
        "--work".into(),
        fixture.work.to_string_lossy().into_owned(),
        "--inbox".into(),
        fixture.inbox.to_string_lossy().into_owned(),
        "--scanner".into(),
        config.scanner.to_string_lossy().into_owned(),
        "--scanner-config".into(),
        config.scanner_config.to_string_lossy().into_owned(),
    ])
}

pub fn verify(fixture: &Fixture) -> Result<Value, String> {
    vault_stage::run(&[
        "verify".into(),
        "--work".into(),
        fixture.work.to_string_lossy().into_owned(),
    ])
}

pub fn publish(fixture: &Fixture, name: &str) -> Result<Value, String> {
    vault_stage::run(&[
        "publish".into(),
        "--work".into(),
        fixture.work.to_string_lossy().into_owned(),
        "--inbox".into(),
        fixture.inbox.to_string_lossy().into_owned(),
        "--name".into(),
        name.into(),
    ])
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl Check {
    pub fn new(name: &str, passed: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed,
            detail: detail.into(),
        }
    }
}

fn path_diagnostic(expected: &BTreeSet<String>, actual: &BTreeSet<String>) -> String {
    format!(
        "expected {}; actual {}; missing {:?}; extra {:?}",
        expected.len(),
        actual.len(),
        expected.difference(actual).take(20).collect::<Vec<_>>(),
        actual.difference(expected).take(20).collect::<Vec<_>>()
    )
}

pub fn grade_prepared(fixture: &Fixture, batch: &Path) -> Vec<Check> {
    let mut checks = Vec::new();
    let copied = match snapshot(&batch.join("payload")) {
        Ok(value) => value,
        Err(error) => return vec![Check::new("payload readable", false, error)],
    };
    let expected_paths: BTreeSet<_> = fixture.expected.files.keys().cloned().collect();
    let actual_paths: BTreeSet<_> = copied.files.keys().cloned().collect();
    checks.push(Check::new(
        "exact payload file paths",
        expected_paths == actual_paths,
        path_diagnostic(&expected_paths, &actual_paths),
    ));
    checks.push(Check::new(
        "exact payload directory paths",
        fixture.expected.directories == copied.directories,
        path_diagnostic(&fixture.expected.directories, &copied.directories),
    ));
    checks.push(Check::new(
        "no payload reparse entries",
        copied.links.is_empty(),
        format!("links {}", copied.links.len()),
    ));
    let mut wrong_bytes = Vec::new();
    let mut wrong_hashes = Vec::new();
    for (path, original) in &fixture.expected.files {
        match copied.files.get(path) {
            Some(destination) => {
                if original != destination {
                    wrong_bytes.push(path.clone());
                }
                if sha(original) != sha(destination) {
                    wrong_hashes.push(path.clone());
                }
            }
            None => {
                wrong_bytes.push(path.clone());
                wrong_hashes.push(path.clone());
            }
        }
    }
    checks.push(Check::new(
        "direct original-copy byte equality",
        wrong_bytes.is_empty(),
        format!("wrong paths {wrong_bytes:?}"),
    ));
    checks.push(Check::new(
        "independent original-copy SHA256 equality",
        wrong_hashes.is_empty(),
        format!("wrong paths {wrong_hashes:?}"),
    ));
    match snapshot(&fixture.source) {
        Ok(now) => checks.push(Check::new(
            "source snapshot unchanged",
            now == fixture.original,
            "independent full source bytes, directories and link targets compared",
        )),
        Err(error) => checks.push(Check::new("source snapshot unchanged", false, error)),
    }
    let manifest = fs::read(batch.join("manifest.jsonl"));
    let mut parsed_manifest = BTreeMap::new();
    let mut manifest_ok = true;
    if let Ok(bytes) = &manifest {
        if let Ok(text) = std::str::from_utf8(bytes) {
            for line in text.lines() {
                match serde_json::from_str::<Value>(line) {
                    Ok(row) => {
                        if let Some(file) = row.get("file").and_then(Value::as_str) {
                            if parsed_manifest.insert(file.to_owned(), row).is_some() {
                                manifest_ok = false;
                            }
                        } else {
                            manifest_ok = false;
                        }
                    }
                    Err(_) => manifest_ok = false,
                }
            }
        } else {
            manifest_ok = false;
        }
    } else {
        manifest_ok = false;
    }
    manifest_ok &= parsed_manifest.len() == fixture.expected.files.len();
    for (path, original) in &fixture.expected.files {
        manifest_ok &= parsed_manifest
            .get(&format!("payload/{path}"))
            .is_some_and(|row| {
                row.get("bytes").and_then(Value::as_u64) == Some(original.len() as u64)
                    && row.get("sha256").and_then(Value::as_str) == Some(sha(original).as_str())
            });
    }
    checks.push(Check::new(
        "manifest independently reconciles",
        manifest_ok,
        format!("{} manifest rows", parsed_manifest.len()),
    ));
    let marker = fs::read(batch.join("_BATCH_COMPLETE")).ok();
    checks.push(Check::new("generated marker is empty", marker.as_ref().is_some_and(Vec::is_empty),
        "original source marker remains separate in payload; generated readiness marker uses the intake contract"));
    let batch_snapshot = snapshot(batch);
    let allowed_files: BTreeSet<_> = expected_paths
        .iter()
        .map(|path| format!("payload/{path}"))
        .chain(["manifest.jsonl".to_owned(), "_BATCH_COMPLETE".to_owned()])
        .collect();
    checks.push(Check::new(
        "no protocol or unaccounted batch files",
        batch_snapshot.as_ref().is_ok_and(|result| {
            result.files.keys().cloned().collect::<BTreeSet<_>>() == allowed_files
        }),
        "only exact payload and two generated control files permitted",
    ));
    let allowed_directories: BTreeSet<_> = fixture
        .expected
        .directories
        .iter()
        .map(|path| format!("payload/{path}"))
        .chain(["payload".to_owned()])
        .collect();
    checks.push(Check::new(
        "no unaccounted batch directories or links",
        batch_snapshot.as_ref().is_ok_and(|result| {
            result.directories == allowed_directories && result.links.is_empty()
        }),
        "exact directory path set includes original empty directories",
    ));
    let receipt = fs::read(fixture.work.join("receipt.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let mut receipt_ok = false;
    let mut dispositions_ok = false;
    if let Some(receipt) = &receipt {
        checks.push(Check::new(
            "receipt binds generated control hashes",
            receipt.get("manifest_sha256").and_then(Value::as_str)
                == manifest.as_ref().ok().map(|bytes| sha(bytes)).as_deref()
                && receipt.get("marker_sha256").and_then(Value::as_str)
                    == marker.as_ref().map(|bytes| sha(bytes)).as_deref(),
            "manifest and empty marker SHA256 computed independently",
        ));
        if let Some(entries) = receipt.get("entries").and_then(Value::as_array) {
            let mut recorded = BTreeMap::new();
            receipt_ok = entries.len() == fixture.expected.files.len();
            for entry in entries {
                if let Some(path) = entry.get("path").and_then(Value::as_str) {
                    receipt_ok &= recorded.insert(path.to_owned(), entry).is_none();
                } else {
                    receipt_ok = false;
                }
            }
            for (path, bytes) in &fixture.expected.files {
                receipt_ok &= recorded.get(path).is_some_and(|entry| {
                    entry.get("size").and_then(Value::as_u64) == Some(bytes.len() as u64)
                        && entry.get("sha256").and_then(Value::as_str) == Some(sha(bytes).as_str())
                });
            }
        }
        if let Some(exclusions) = receipt.get("exclusions").and_then(Value::as_array) {
            dispositions_ok = fixture.excluded.iter().all(|path| {
                exclusions.iter().any(|entry| {
                    entry.get("path").and_then(Value::as_str) == Some(path.as_str())
                        && entry
                            .get("reason")
                            .and_then(Value::as_str)
                            .is_some_and(|reason| !reason.is_empty())
                })
            });
        }
    }
    checks.push(Check::new(
        "external receipt independently reconciles",
        receipt_ok,
        "path set, byte counts, hashes and duplicate rows checked",
    ));
    checks.push(Check::new(
        "every frozen exclusion has a reason",
        dispositions_ok,
        format!("{} expected exclusions", fixture.excluded.len()),
    ));
    checks.extend(grade_no_scan_residue(fixture));
    checks
}

pub fn grade_no_scan_residue(fixture: &Fixture) -> Vec<Check> {
    let work = if fixture.work.exists() {
        snapshot(&fixture.work).ok()
    } else {
        Some(Snapshot::default())
    };
    let residue_ok = work.as_ref().is_some_and(|work| {
        !work
            .directories
            .iter()
            .any(|path| path == "scan-input" || path.starts_with("scan-input/"))
            && !work.files.keys().any(|path| {
                path.starts_with("scan-input/")
                    || path.ends_with("scan.json")
                    || path.ends_with("scan-report.json")
            })
    });
    let no_secret = work.as_ref().is_some_and(|work| {
        work.files.values().all(|bytes| {
            fixture.secrets.iter().all(|secret| {
                !bytes
                    .windows(secret.len())
                    .any(|part| part == secret.as_bytes())
            })
        })
    });
    vec![
        Check::new(
            "temporary scanner bytes absent",
            residue_ok,
            "no opaque scanner input or raw report remains",
        ),
        Check::new(
            "held synthetic credential contents absent",
            no_secret,
            "checked all retained work file bytes",
        ),
    ]
}

#[derive(Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub split: String,
    pub passed: bool,
    pub checks: Vec<Check>,
    pub elapsed_ms: u128,
    pub scanner_invocations: usize,
    pub scratch: String,
    pub engine_outcomes: Vec<Value>,
}

pub fn definitions(split: &str) -> Result<Vec<(String, String)>, String> {
    if !["train", "test", "all"].contains(&split) {
        return Err("split must be train, test, or all".into());
    }
    let protocol: Value = serde_json::from_str(PROTOCOL).map_err(|e| e.to_string())?;
    protocol
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("frozen protocol has no cases")?
        .iter()
        .filter(|row| split == "all" || row["split"] == split)
        .map(|row| {
            Ok((
                row["id"].as_str().ok_or("case id absent")?.into(),
                row["split"].as_str().ok_or("case split absent")?.into(),
            ))
        })
        .collect()
}

fn outcome(result: &Result<Value, String>, secrets: &[String]) -> Value {
    match result {
        Ok(value) => json!({"ok": true, "result": value}),
        Err(error) => {
            let mut safe = error.clone();
            for secret in secrets {
                safe = safe.replace(secret, "[REDACTED FIXTURE]");
            }
            json!({"ok": false, "error": safe})
        }
    }
}

fn success(
    checks: &mut Vec<Check>,
    outcomes: &mut Vec<Value>,
    name: &str,
    result: Result<Value, String>,
    fixture: &Fixture,
) -> bool {
    let passed = result.is_ok();
    checks.push(Check::new(
        name,
        passed,
        if passed {
            "success"
        } else {
            "engine returned an error; see safe outcome"
        },
    ));
    outcomes.push(outcome(&result, &fixture.secrets));
    passed
}

fn failure(
    checks: &mut Vec<Check>,
    outcomes: &mut Vec<Value>,
    name: &str,
    result: Result<Value, String>,
    fixture: &Fixture,
) {
    checks.push(Check::new(
        name,
        result.is_err(),
        "must fail without accepting unsafe state",
    ));
    outcomes.push(outcome(&result, &fixture.secrets));
}

fn repeated_grade(checks: &mut Vec<Check>, fixture: &Fixture, batch: &Path) {
    let first = grade_prepared(fixture, batch);
    let second = grade_prepared(fixture, batch);
    checks.push(Check::new(
        "grader invariance on unchanged bytes",
        first == second,
        "two complete independent inspections produce identical grades and diagnostics",
    ));
    checks.extend(first);
}

fn direct_scan(config: &Config, source: &Path) -> Result<Value, String> {
    let output = Command::new(&config.scanner)
        .arg("dir")
        .arg(source)
        .args(["--config"])
        .arg(&config.scanner_config)
        .args([
            "--ignore-gitleaks-allow",
            "--redact=100",
            "--no-banner",
            "--no-color",
            "--log-level",
            "error",
            "--max-archive-depth",
            "0",
            "--max-decode-depth",
            "0",
            "--exit-code",
            "10",
            "--report-format",
            "json",
            "--report-path",
            "-",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.code() != Some(0) && output.status.code() != Some(10) {
        return Err(
            "direct fixture Gitleaks scan failed (stderr intentionally not exposed)".into(),
        );
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|_| "direct fixture scanner returned non-JSON report".into())
}

pub fn create_link(link: &Path, target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let output = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(link.to_string_lossy().replace('/', "\\"))
            .arg(target.to_string_lossy().replace('/', "\\"))
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "could not create owned fixture junction: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| e.to_string())
    }
}

pub fn scanner_wrapper(fixture: &Fixture, config: &Config, mode: &str) -> Result<PathBuf, String> {
    let source_file = fixture.root.join("trusted-fixture-scanner.rs");
    let target = fixture.root.join(if cfg!(windows) {
        "trusted-fixture-scanner.exe"
    } else {
        "trusted-fixture-scanner"
    });
    let scanner = config.scanner.to_string_lossy();
    let change = fixture
        .source
        .join("notes.md")
        .to_string_lossy()
        .into_owned();
    let policy = config.scanner_config.to_string_lossy();
    let attempt = fixture
        .root
        .join("policy-mutation-attempt.txt")
        .to_string_lossy()
        .into_owned();
    let body = match mode {
        "mutate" => format!("let output = std::process::Command::new({scanner:?}).args(&args).output().unwrap();\nif args.first().map(String::as_str) == Some(\"dir\") {{ std::fs::write({change:?}, b\"Changed while scanning.\\n\").unwrap(); }}\nuse std::io::Write; std::io::stdout().write_all(&output.stdout).unwrap(); std::io::stderr().write_all(&output.stderr).unwrap(); std::process::exit(output.status.code().unwrap_or(2));"),
        "malformed" => "if args.iter().any(|arg| arg == \"version\" || arg == \"--version\") { println!(\"8.30.1\"); } else { println!(\"{{\\\"not_findings\\\":true}}\"); }".into(),
        "absent" => "if args.iter().any(|arg| arg == \"version\" || arg == \"--version\") { println!(\"8.30.1\"); }".into(),
        "outside" => r#"if args.iter().any(|arg| arg == "version" || arg == "--version") { println!("8.30.1"); } else {
            let alias = std::fs::read_dir(&args[1]).unwrap().next().unwrap().unwrap().file_name().to_string_lossy().into_owned();
            println!("[{{\"File\":\"elsewhere/{}\",\"RuleID\":\"fixture-outside\"}}]", alias);
            std::process::exit(10);
        }"#.into(),
        "policy-aba" => format!("let scanner = {scanner:?}; let policy = {policy:?}; let attempt = {attempt:?};") + r#"
            if args.first().map(String::as_str) != Some("dir") {
                let output = std::process::Command::new(scanner).args(&args).output().unwrap();
                use std::io::Write; std::io::stdout().write_all(&output.stdout).unwrap();
                std::process::exit(output.status.code().unwrap_or(2));
            }
            let original = std::fs::read(policy).unwrap();
            std::fs::write(attempt, b"attempted\n").unwrap();
            if std::fs::write(policy, b"title = 'fixture weakened policy'\n[allowlist]\npaths = ['.*']\n").is_err() {
                std::fs::write(attempt, b"blocked\n").unwrap();
                std::process::exit(2);
            }
            let output = std::process::Command::new(scanner).args(&args).output().unwrap();
            std::fs::write(policy, original).unwrap();
            std::fs::write(attempt, b"restored\n").unwrap();
            use std::io::Write; std::io::stdout().write_all(&output.stdout).unwrap();
            std::io::stderr().write_all(&output.stderr).unwrap();
            std::process::exit(output.status.code().unwrap_or(2));
        "#,
        _ => return Err("unknown trusted fixture scanner mode".into()),
    };
    fs::write(
        &source_file,
        format!(
            "fn main() {{ let args: Vec<String> = std::env::args().skip(1).collect(); {body} }}\n"
        ),
    )
    .map_err(|e| e.to_string())?;
    let output = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-name",
            "trusted_fixture_scanner",
        ])
        .arg(&source_file)
        .arg("-o")
        .arg(&target)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "trusted scanner fixture failed to compile: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(target)
}

pub fn run_case(id: &str, split: &str, seed: u64, config: &Config) -> Result<CaseResult, String> {
    let started = Instant::now();
    let mut fixture = Fixture::create(id, seed)?;
    let mut checks = Vec::new();
    let mut outcomes = Vec::new();
    let mut scanner_invocations = 0;
    match id {
        "train-static-delivery" => {
            make_standard(&mut fixture, seed)?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "heterogeneous stage succeeds",
                stage(&fixture, config),
                &fixture,
            ) {
                repeated_grade(&mut checks, &fixture, &fixture.work.join("batch"));
                success(
                    &mut checks,
                    &mut outcomes,
                    "real verify succeeds",
                    verify(&fixture),
                    &fixture,
                );
                checks.push(Check::new(
                    "staged instructions not executed",
                    !fixture.root.join("EXECUTED").exists(),
                    "instruction text remains payload",
                ));
            }
        }
        "train-scanner-authority" => {
            let fake_secret = "fixture_only_credential_4829137".to_owned();
            fixture.secrets.push(fake_secret.clone());
            fixture.put(
                "INDEX.md",
                format!("password = {fake_secret} # gitleaks:allow\n").as_bytes(),
            )?;
            fixture.put("safe.md", b"Author-generated benign notes.\n")?;
            fixture.put(".gitleaks.toml", b"[allowlist]\npaths = ['.*']\n")?;
            let initial = direct_scan(config, &fixture.source)?;
            scanner_invocations += 1;
            let fingerprints: Vec<_> = initial
                .as_array()
                .ok_or("direct report was not an array")?
                .iter()
                .filter_map(|row| row.get("Fingerprint").and_then(Value::as_str))
                .collect();
            checks.push(Check::new(
                "real scanner detects unsuppressed fixture",
                !fingerprints.is_empty(),
                "synthetic finding before adding source ignore",
            ));
            fixture.put(
                ".gitleaksignore",
                format!("{}\n", fingerprints.join("\n")).as_bytes(),
            )?;
            let suppressed = direct_scan(config, &fixture.source)?;
            scanner_invocations += 1;
            checks.push(Check::new(
                "source ignore fingerprints are valid",
                suppressed.as_array().is_some_and(Vec::is_empty),
                "real direct scan is suppressed; isolated engine scan must remain authoritative",
            ));
            fixture.seal(&["INDEX.md"])?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "engine stages safe content",
                stage(&fixture, config),
                &fixture,
            ) {
                repeated_grade(&mut checks, &fixture, &fixture.work.join("batch"));
                success(
                    &mut checks,
                    &mut outcomes,
                    "safe result verifies",
                    verify(&fixture),
                    &fixture,
                );
            }
        }
        "train-same-size-corruption" => {
            fixture.put("notes.md", b"Author-generated exact fixture.\n")?;
            fixture.seal(&[])?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "initial stage succeeds",
                stage(&fixture, config),
                &fixture,
            ) {
                let target = fixture.work.join("batch/payload/notes.md");
                let mut bytes = fs::read(&target).map_err(|e| e.to_string())?;
                bytes[0] ^= 1;
                fs::write(&target, &bytes).map_err(|e| e.to_string())?;
                let grade = grade_prepared(&fixture, &fixture.work.join("batch"));
                checks.push(Check::new(
                    "independent grader catches same-size corruption",
                    grade.iter().any(|check| {
                        check.name == "direct original-copy byte equality" && !check.passed
                    }),
                    "length is preserved while one byte differs",
                ));
                checks.push(Check::new(
                    "corruption grader remains invariant",
                    grade == grade_prepared(&fixture, &fixture.work.join("batch")),
                    "repeat failed grading is identical",
                ));
                failure(
                    &mut checks,
                    &mut outcomes,
                    "verify rejects same-size corruption",
                    verify(&fixture),
                    &fixture,
                );
            }
        }
        "train-path-set-integrity" => {
            fixture.put("a.md", b"Owner note A.\n")?;
            fixture.put("b.md", b"Owner note B.\n")?;
            fixture.seal(&[])?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "initial stage succeeds",
                stage(&fixture, config),
                &fixture,
            ) {
                let target = fixture.work.join("batch/payload/b.md");
                fs::remove_file(&target).map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "verify rejects missing file",
                    verify(&fixture),
                    &fixture,
                );
                let missing = grade_prepared(&fixture, &fixture.work.join("batch"));
                checks.push(Check::new(
                    "independent grader catches missing path",
                    missing
                        .iter()
                        .any(|check| check.name == "exact payload file paths" && !check.passed),
                    "fixed golden paths compared",
                ));
                fs::write(&target, b"Owner note B.\n").map_err(|e| e.to_string())?;
                fs::write(
                    fixture.work.join("batch/payload/extra.md"),
                    b"Unaccounted fixture file.\n",
                )
                .map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "verify rejects extra file",
                    verify(&fixture),
                    &fixture,
                );
                let extra = grade_prepared(&fixture, &fixture.work.join("batch"));
                checks.push(Check::new(
                    "independent grader catches extra path",
                    extra
                        .iter()
                        .any(|check| check.name == "exact payload file paths" && !check.passed),
                    "fixed golden paths compared",
                ));
            }
        }
        "train-source-snapshot" => {
            fixture.put("notes.md", b"Original owned fixture.\n")?;
            fixture.seal(&[])?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "initial stage succeeds",
                stage(&fixture, config),
                &fixture,
            ) {
                fixture.put("notes.md", b"Modified owned fixture.\n")?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "verify rejects source byte edit",
                    verify(&fixture),
                    &fixture,
                );
                fixture.put("notes.md", b"Original owned fixture.\n")?;
                fixture.put("new.md", b"Source addition.\n")?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "verify rejects source addition",
                    verify(&fixture),
                    &fixture,
                );
                fs::remove_file(fixture.source.join("new.md")).map_err(|e| e.to_string())?;
                fs::remove_file(fixture.source.join("notes.md")).map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "verify rejects source deletion",
                    verify(&fixture),
                    &fixture,
                );
                checks.push(Check::new(
                    "engine never restores missing source",
                    !fixture.source.join("notes.md").exists(),
                    "original preservation includes no repair mutations",
                ));
            }
        }
        "train-scanner-failure" => {
            fixture.put("notes.md", b"Safe fixture notes.\n")?;
            fixture.seal(&[])?;
            let mut missing = config.clone();
            missing.scanner = fixture.root.join("missing-scanner.exe");
            failure(
                &mut checks,
                &mut outcomes,
                "missing scanner fails",
                stage(&fixture, &missing),
                &fixture,
            );
            checks.push(Check::new(
                "missing scanner cannot create marker",
                !fixture.work.join("batch/_BATCH_COMPLETE").exists(),
                "failed work stays unprepared",
            ));
            fixture.work = fixture.root.join("bad-config-work");
            let mut invalid = config.clone();
            invalid.scanner_config = fixture.root.join("trusted-malformed.toml");
            fs::write(&invalid.scanner_config, b"[[ this is invalid TOML")
                .map_err(|e| e.to_string())?;
            scanner_invocations += 1;
            failure(
                &mut checks,
                &mut outcomes,
                "scanner config error fails",
                stage(&fixture, &invalid),
                &fixture,
            );
            checks.push(Check::new(
                "scanner error cannot create marker",
                !fixture.work.join("batch/_BATCH_COMPLETE").exists(),
                "error is never success",
            ));
            checks.extend(grade_no_scan_residue(&fixture));
            fixture.work = fixture.root.join("outside-report-work");
            let mut forged = config.clone();
            forged.scanner = scanner_wrapper(&fixture, config, "outside")?;
            scanner_invocations += 1;
            let rejected = stage(&fixture, &forged);
            checks.push(Check::new("valid alias basename cannot escape scanner snapshot",
                rejected.as_ref().is_err_and(|error| error.contains("outside the byte snapshot")),
                "fixture returns a parseable nonempty findings report with exit 10 but an outside File path"));
            failure(
                &mut checks,
                &mut outcomes,
                "out-of-scope scanner path fails",
                rejected,
                &fixture,
            );
            checks.push(Check::new(
                "out-of-scope report cannot create marker",
                !fixture.work.join("batch/_BATCH_COMPLETE").exists(),
                "error is never readiness",
            ));
            checks.extend(grade_no_scan_residue(&fixture));
            checks.push(Check::new(
                "failed scan preserves originals",
                snapshot(&fixture.source)? == fixture.original,
                "independent source bytes unchanged",
            ));
        }
        "train-publication-ownership" => {
            fixture.put("notes.md", b"Owned publication fixture.\n")?;
            fixture.seal(&[])?;
            fs::create_dir(&fixture.work).map_err(|e| e.to_string())?;
            failure(
                &mut checks,
                &mut outcomes,
                "existing empty work is never reused",
                stage(&fixture, config),
                &fixture,
            );
            checks.push(Check::new(
                "other empty work remains empty",
                fs::read_dir(&fixture.work)
                    .map_err(|e| e.to_string())?
                    .next()
                    .is_none(),
                "no writes into existing directory",
            ));
            fixture.work = fixture.root.join("owned-work");
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "initial stage succeeds",
                stage(&fixture, config),
                &fixture,
            ) {
                let before = snapshot(&fixture.work)?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "second writer stage fails",
                    stage(&fixture, config),
                    &fixture,
                );
                checks.push(Check::new(
                    "repeat stage preserves owned work bytes",
                    snapshot(&fixture.work)? == before,
                    "no overwrite on retry",
                ));
                for name in ["../escape", "2026-10-01-a/escape", "2026-10-01-.."] {
                    failure(
                        &mut checks,
                        &mut outcomes,
                        "unsafe publication name fails",
                        publish(&fixture, name),
                        &fixture,
                    );
                }
                let occupied_name = "2026-10-01-occupied";
                fs::create_dir(fixture.inbox.join(occupied_name)).map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "empty destination cannot be replaced",
                    publish(&fixture, occupied_name),
                    &fixture,
                );
                checks.push(Check::new(
                    "existing empty batch is unchanged",
                    fs::read_dir(fixture.inbox.join(occupied_name))
                        .map_err(|e| e.to_string())?
                        .next()
                        .is_none(),
                    "exclusive final allocation",
                ));
                let existing_file_name = "2026-10-01-existingfile";
                fs::write(
                    fixture.inbox.join(existing_file_name),
                    b"Another writer's sentinel.\n",
                )
                .map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "destination file cannot be replaced",
                    publish(&fixture, existing_file_name),
                    &fixture,
                );
                checks.push(Check::new(
                    "other writer's destination file preserved",
                    fs::read(fixture.inbox.join(existing_file_name)).map_err(|e| e.to_string())?
                        == b"Another writer's sentinel.\n",
                    "exact sentinel bytes unchanged",
                ));
                #[cfg(windows)]
                {
                    let case_alias = fixture.inbox.join("2026-10-01-CASEALIAS");
                    fs::create_dir(&case_alias).map_err(|e| e.to_string())?;
                    failure(
                        &mut checks,
                        &mut outcomes,
                        "case-insensitive destination collision fails",
                        publish(&fixture, "2026-10-01-casealias"),
                        &fixture,
                    );
                    checks.push(Check::new(
                        "case alias destination preserved",
                        fs::read_dir(&case_alias)
                            .map_err(|e| e.to_string())?
                            .next()
                            .is_none(),
                        "actual Windows filesystem alias, no simulated collision",
                    ));
                }
                let final_name = "2026-10-01-published";
                if success(
                    &mut checks,
                    &mut outcomes,
                    "unused destination publishes",
                    publish(&fixture, final_name),
                    &fixture,
                ) {
                    repeated_grade(&mut checks, &fixture, &fixture.inbox.join(final_name));
                    checks.push(Check::new(
                        "prepared copy moved, originals kept",
                        !fixture.work.join("batch").exists()
                            && fixture.source.join("notes.md").is_file(),
                        "only the prepared copy moves",
                    ));
                    let destination_before = snapshot(&fixture.inbox.join(final_name))?;
                    failure(
                        &mut checks,
                        &mut outcomes,
                        "repeated publication cannot overwrite",
                        publish(&fixture, final_name),
                        &fixture,
                    );
                    checks.push(Check::new(
                        "published bytes survive repeat",
                        snapshot(&fixture.inbox.join(final_name))? == destination_before,
                        "already published batch preserved",
                    ));
                }
            }
        }
        "train-reparse-and-overlap" => {
            fixture.put("notes.md", b"Selected root notes.\n")?;
            let outside = fixture.root.join("outside");
            fs::create_dir(&outside).map_err(|e| e.to_string())?;
            fs::write(outside.join("outside.md"), b"Do not copy or change.\n")
                .map_err(|e| e.to_string())?;
            create_link(&fixture.source.join("escape"), &outside)?;
            create_link(&fixture.source.join("cycle"), &fixture.source)?;
            fixture.put(
                "marked-looking/pyvenv.cfg",
                b"home = /synthetic/interpreter\n",
            )?;
            fs::write(
                outside.join("python"),
                b"Synthetic interpreter marker outside source.\n",
            )
            .map_err(|e| e.to_string())?;
            create_link(&fixture.source.join("marked-looking/bin"), &outside)?;
            fixture.seal(&["escape", "cycle", "marked-looking/bin"])?;
            let outside_before = snapshot(&outside)?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "source with links stages safely",
                stage(&fixture, config),
                &fixture,
            ) {
                repeated_grade(&mut checks, &fixture, &fixture.work.join("batch"));
                success(
                    &mut checks,
                    &mut outcomes,
                    "source with links verifies",
                    verify(&fixture),
                    &fixture,
                );
            }
            let original_work = fixture.work.clone();
            fixture.work = fixture.source.join("overlapping-work");
            failure(
                &mut checks,
                &mut outcomes,
                "work inside source rejected",
                stage(&fixture, config),
                &fixture,
            );
            checks.push(Check::new(
                "overlap check is before allocation",
                !fixture.work.exists(),
                "source not modified",
            ));
            fixture.work = fixture.root.join("same-root-work");
            let original_inbox = fixture.inbox.clone();
            fixture.inbox = fixture.source.clone();
            failure(
                &mut checks,
                &mut outcomes,
                "source equal to inbox rejected",
                stage(&fixture, config),
                &fixture,
            );
            fixture.inbox = original_inbox;
            let actual_source = fixture.source.clone();
            fixture.source = fixture.root.join("source-alias");
            create_link(&fixture.source, &actual_source)?;
            fixture.work = fixture.root.join("alias-work");
            failure(
                &mut checks,
                &mut outcomes,
                "reparse source root rejected",
                stage(&fixture, config),
                &fixture,
            );
            fixture.source = actual_source;
            fixture.work = original_work;
            checks.push(Check::new(
                "outside link target untouched",
                snapshot(&outside)? == outside_before,
                "no escape reads copied and no external writes",
            ));
            checks.push(Check::new(
                "link tests preserve all source entries",
                snapshot(&fixture.source)? == fixture.original,
                "full original fixture snapshot unchanged",
            ));
        }
        "test-unusual-byte-layout" => {
            fixture.put(
                "research/naïve Ω/hidden/.owner-note",
                b"Original UTF-8 named note.\r\n",
            )?;
            fixture.put(
                "research/naïve Ω/SOURCE.toml",
                b"name = \"Own evidence\"\nextra = { retained = true }\n",
            )?;
            fixture.put(
                "research/naïve Ω/manifest.jsonl",
                b"{\"owner\":\"metadata stays exact\"}\n",
            )?;
            fixture.put("research/naïve Ω/_BATCH_COMPLETE", b"")?;
            fixture.put(
                "archive.part",
                &(0..131_072)
                    .map(|n| ((n * 31 + seed as usize) % 256) as u8)
                    .collect::<Vec<_>>(),
            )?;
            fixture.put("empty.bin", b"")?;
            fs::create_dir_all(fixture.source.join("research/naïve Ω/no-files/inner"))
                .map_err(|e| e.to_string())?;
            fixture.seal(&[])?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "unseen layout stages",
                stage(&fixture, config),
                &fixture,
            ) {
                repeated_grade(&mut checks, &fixture, &fixture.work.join("batch"));
                success(
                    &mut checks,
                    &mut outcomes,
                    "unseen layout verifies",
                    verify(&fixture),
                    &fixture,
                );
            }
        }
        "test-control-forgery" => {
            fixture.put("notes.md", b"Author-generated fixed note.\n")?;
            fixture.seal(&[])?;
            scanner_invocations += 1;
            if success(
                &mut checks,
                &mut outcomes,
                "initial stage succeeds",
                stage(&fixture, config),
                &fixture,
            ) {
                let payload = fixture.work.join("batch/payload/notes.md");
                let mut changed = fs::read(&payload).map_err(|e| e.to_string())?;
                changed[0] ^= 1;
                fs::write(&payload, &changed).map_err(|e| e.to_string())?;
                let manifest = format!(
                    "{}\n",
                    json!({"file":"payload/notes.md","bytes":changed.len(),"sha256":sha(&changed)})
                );
                fs::write(fixture.work.join("batch/manifest.jsonl"), &manifest)
                    .map_err(|e| e.to_string())?;
                let receipt_path = fixture.work.join("receipt.json");
                let mut receipt: Value =
                    serde_json::from_slice(&fs::read(&receipt_path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                receipt["entries"][0]["sha256"] = json!(sha(&changed));
                receipt["manifest_sha256"] = json!(sha(manifest.as_bytes()));
                let marker_path = fixture.work.join("batch/_BATCH_COMPLETE");
                let marker_bytes = fs::read(&marker_path).map_err(|e| e.to_string())?;
                receipt["marker_sha256"] = json!(sha(&marker_bytes));
                fs::write(
                    &receipt_path,
                    serde_json::to_vec(&receipt).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "rewritten claims cannot mask changed payload",
                    verify(&fixture),
                    &fixture,
                );
                checks.push(Check::new(
                    "independent byte oracle rejects rewritten claims",
                    grade_prepared(&fixture, &fixture.work.join("batch"))
                        .iter()
                        .any(|check| {
                            !check.passed && check.name == "direct original-copy byte equality"
                        }),
                    "gold never read from generated controls",
                ));
                receipt["entries"][0]["path"] = json!("../escape.md");
                fs::write(
                    &receipt_path,
                    serde_json::to_vec(&receipt).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "receipt traversal is rejected",
                    verify(&fixture),
                    &fixture,
                );
                checks.push(Check::new(
                    "forgery never writes outside work",
                    !fixture.root.join("escape.md").exists(),
                    "verify stays read-only",
                ));
            }
        }
        "test-scan-time-source-change" => {
            fixture.put("notes.md", b"Original before scanning.\n")?;
            fixture.seal(&[])?;
            let mut wrapper = config.clone();
            wrapper.scanner = scanner_wrapper(&fixture, config, "mutate")?;
            scanner_invocations += 1;
            failure(
                &mut checks,
                &mut outcomes,
                "source mutation during scanner return invalidates stage",
                stage(&fixture, &wrapper),
                &fixture,
            );
            checks.push(Check::new(
                "race fixture actually changed source",
                fs::read(fixture.source.join("notes.md")).map_err(|e| e.to_string())?
                    == b"Changed while scanning.\n",
                "trusted wrapper controls timing without a flaky thread race",
            ));
            checks.push(Check::new(
                "scan race cannot create readiness",
                !fixture.work.join("batch/_BATCH_COMPLETE").exists(),
                "scan no longer bound to source snapshot",
            ));
            checks.extend(grade_no_scan_residue(&fixture));
        }
        "test-invalid-scanner-report" => {
            fixture.put("notes.md", b"Safe notes for scanner failure fixture.\n")?;
            fixture.seal(&[])?;
            for mode in ["malformed", "absent"] {
                let mut wrapper = config.clone();
                fixture.work = fixture.root.join(format!("{mode}-work"));
                wrapper.scanner = scanner_wrapper(&fixture, config, mode)?;
                scanner_invocations += 1;
                failure(
                    &mut checks,
                    &mut outcomes,
                    "success exit without valid report fails",
                    stage(&fixture, &wrapper),
                    &fixture,
                );
                checks.push(Check::new(
                    "invalid report cannot create readiness",
                    !fixture.work.join("batch/_BATCH_COMPLETE").exists(),
                    mode,
                ));
                checks.extend(grade_no_scan_residue(&fixture));
            }
            checks.push(Check::new(
                "scanner report failures preserve originals",
                snapshot(&fixture.source)? == fixture.original,
                "independent source snapshot unchanged",
            ));
        }
        _ => return Err(format!("unknown frozen case id: {id}")),
    }
    let result = CaseResult {
        id: id.into(),
        split: split.into(),
        passed: !checks.is_empty() && checks.iter().all(|check| check.passed),
        checks,
        elapsed_ms: started.elapsed().as_millis(),
        scanner_invocations,
        scratch: fixture.root.to_string_lossy().into_owned(),
        engine_outcomes: outcomes,
    };
    fs::write(
        fixture.root.join("case-receipt.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(result)
}
