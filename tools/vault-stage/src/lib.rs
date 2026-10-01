//! Deterministic byte staging. This module neither executes source material nor ingests it.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exclusion {
    path: String,
    kind: String,
    reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GitRecord {
    path: String,
    head: Option<String>,
    head_status: String,
    origin: Option<String>,
    origin_sha256: Option<String>,
    origin_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    files: Vec<Entry>,
    directories: Vec<String>,
    exclusions: Vec<Exclusion>,
    git: Vec<GitRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScannerRecord {
    binary_sha256: String,
    config_sha256: String,
    findings: Vec<String>,
    scope: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    status: String,
    source_root: String,
    inbox_root: String,
    entries: Vec<Entry>,
    directories: Vec<String>,
    inventory: Inventory,
    git_provenance_status: String,
    exclusions: Vec<Exclusion>,
    scanner: ScannerRecord,
    manifest_sha256: String,
    marker_sha256: String,
}

/// Arguments exclude the executable name. Results and errors contain no file contents.
pub fn run(args: &[String]) -> Result<Value> {
    if args == ["--version"] {
        return Ok(
            json!({"ok": true, "tool": "vault-stage", "version": "0.1.0", "schema_version": 1}),
        );
    }
    if args == ["--help"] {
        return Ok(
            json!({"ok": true, "tool": "vault-stage", "schema_version": 1,
            "commands": ["plan --source <dir>", "stage --source <dir> --work <new-dir> --inbox <dir> --scanner <exe> --scanner-config <trusted-toml>",
                "verify --work <dir>", "publish --work <dir> --inbox <dir> --name <YYYY-MM-DD-slug>"],
            "states": "inventoried, prepared, verified, published; never ingested"}),
        );
    }
    let (command, options) = parse(args)?;
    match command.as_str() {
        "plan" => {
            require_options(&options, &["source"])?;
            let source = existing_dir(Path::new(&options["source"]), "source")?;
            let inventory = inventory(&source, true)?;
            Ok(
                json!({"ok": true, "schema_version": 1, "status": "inventoried", "source": display(&source),
                "files": inventory.files.len(), "bytes": total_bytes(&inventory.files)?,
                "excluded": inventory.exclusions.len(), "exclusions": inventory.exclusions.iter().take(20).collect::<Vec<_>>(),
                "exclusions_truncated": inventory.exclusions.len() > 20,
                "directories": inventory.directories.len(), "scanner_completed": false,
                "pending_intake": false, "ingested": false}),
            )
        }
        "stage" => {
            require_options(
                &options,
                &["source", "work", "inbox", "scanner", "scanner-config"],
            )?;
            stage(&options)
        }
        "verify" => {
            require_options(&options, &["work"])?;
            let work = existing_dir(Path::new(&options["work"]), "work")?;
            let (receipt, batch) = verify(&work)?;
            response("verified", &work, &batch, &receipt)
        }
        "publish" => {
            require_options(&options, &["work", "inbox", "name"])?;
            publish(&options)
        }
        _ => Err("expected plan, stage, verify, or publish".into()),
    }
}

fn parse(args: &[String]) -> Result<(String, BTreeMap<String, String>)> {
    let command = args
        .first()
        .ok_or("missing command: plan, stage, verify, publish")?
        .clone();
    let mut options = BTreeMap::new();
    let rest = &args[1..];
    if !rest.len().is_multiple_of(2) {
        return Err("options require one explicit value each".into());
    }
    for pair in rest.as_chunks::<2>().0 {
        let key = pair[0]
            .strip_prefix("--")
            .ok_or("expected named --option")?;
        if key.is_empty()
            || pair[1].is_empty()
            || options.insert(key.to_owned(), pair[1].clone()).is_some()
        {
            return Err("empty or duplicate option".into());
        }
    }
    Ok((command, options))
}

fn require_options(options: &BTreeMap<String, String>, names: &[&str]) -> Result<()> {
    if options.len() != names.len() || names.iter().any(|name| !options.contains_key(*name)) {
        return Err(format!(
            "expected exactly these options: {}",
            names.join(", ")
        ));
    }
    Ok(())
}

fn stage(options: &BTreeMap<String, String>) -> Result<Value> {
    let source = existing_dir(Path::new(&options["source"]), "source")?;
    let inbox = existing_dir(Path::new(&options["inbox"]), "inbox")?;
    let work = new_dir_path(Path::new(&options["work"]))?;
    disjoint(&source, &inbox, "source and inbox")?;
    disjoint(&source, &work, "source and work")?;
    disjoint(&inbox, &work, "inbox and work")?;
    let scanner = existing_file(Path::new(&options["scanner"]), "scanner")?;
    let config = existing_file(Path::new(&options["scanner-config"]), "scanner config")?;
    if within(&config, &source) || within(&scanner, &source) {
        return Err("trusted scanner and config must be outside source".into());
    }
    if within(&config, &inbox) || within(&scanner, &inbox) {
        return Err("trusted scanner and config must be outside inbox".into());
    }
    if !same_volume(work.parent().ok_or("work has no parent")?, &inbox)? {
        return Err("work and inbox must be on the same volume for atomic publication".into());
    }
    let original = inventory(&source, true)?;
    check_free_space(work.parent().ok_or("work has no parent")?, &original)?;
    // Hash the actual locked handles, not a second path lookup. On Windows the
    // handles deny writes/deletes, including replacement while the child runs.
    let mut scanner_guard = open_read_locked(&scanner, "trusted scanner")?;
    let binary_hash = hash_open_file(&mut scanner_guard, "trusted scanner")?.1;
    let mut config_guard = open_read_locked(&config, "trusted scanner config")?;
    let mut config_bytes = Vec::new();
    (&mut config_guard)
        .take(1024 * 1024 + 1)
        .read_to_end(&mut config_bytes)
        .map_err(|e| io_error("read trusted scanner config snapshot", &e))?;
    if config_bytes.len() > 1024 * 1024 {
        return Err("trusted standalone scanner config exceeds 1 MiB limit".into());
    }
    validate_standalone_config(&config_bytes)?;
    let config_hash = hash_bytes(&config_bytes);
    fs::create_dir(&work).map_err(|e| io_error("create exclusive work directory", &e))?;
    let config_snapshot = work.join("scanner-config.toml");
    write_new(
        &config_snapshot,
        &config_bytes,
        "trusted scanner config snapshot",
    )?;
    let mut snapshot_guard = open_read_locked(&config_snapshot, "scanner config snapshot")?;
    if hash_open_file(&mut snapshot_guard, "scanner config snapshot")?.1 != config_hash {
        return Err("trusted scanner config snapshot differs from its pinned original".into());
    }

    // Quarantine copies are outside delivery. Always remove them before returning.
    let scan_result = scan_snapshot(&work, &source, &original.files, &scanner, &config_snapshot);
    let cleanup = cleanup_scan_inputs(&work);
    // Keep all three pins alive through process completion and scan verification.
    drop(snapshot_guard);
    drop(config_guard);
    drop(scanner_guard);
    let findings = match (scan_result, cleanup) {
        (Ok(value), Ok(())) => value,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup_error)) => return Err(format!("{error}; {cleanup_error}")),
    };
    if hash_file(&scanner, "scanner")?.1 != binary_hash
        || hash_file(&config, "scanner config")?.1 != config_hash
    {
        return Err("trusted scanner or config changed during scan".into());
    }
    if inventory(&source, true)? != original {
        return Err("source changed during scan; no complete batch was created".into());
    }
    let findings_set: BTreeSet<_> = findings.iter().cloned().collect();
    let entries: Vec<_> = original
        .files
        .iter()
        .filter(|entry| !findings_set.contains(&entry.path))
        .cloned()
        .collect();
    let mut exclusions = original.exclusions.clone();
    exclusions.extend(findings.iter().map(|path| Exclusion {
        path: path.clone(),
        kind: "file".into(),
        reason: "local secret scanner finding".into(),
    }));
    exclusions.sort_by(|a, b| a.path.cmp(&b.path));
    let batch = work.join("batch");
    let payload = batch.join("payload");
    fs::create_dir(&batch).map_err(|e| io_error("create batch", &e))?;
    fs::create_dir(&payload).map_err(|e| io_error("create payload", &e))?;
    for directory in &original.directories {
        fs::create_dir(payload.join(directory))
            .map_err(|e| io_error("create payload directory", &e))?;
    }
    for entry in &entries {
        copy_exact(&source.join(&entry.path), &payload.join(&entry.path), entry)?;
    }
    verify_payload(&payload, &entries, &original.directories)?;
    if inventory(&source, true)? != original {
        return Err("source changed while copying; batch has no readiness marker".into());
    }
    let manifest = manifest_bytes(&entries)?;
    let manifest_hash = hash_bytes(&manifest);
    let marker = marker_bytes(&manifest_hash, &entries)?;
    let receipt = Receipt {
        schema_version: 1, status: "prepared".into(), source_root: display(&source), inbox_root: display(&inbox),
        entries, directories: original.directories.clone(),
        git_provenance_status: if original.git.is_empty() {
            "unavailable: no safe Git metadata observed".into()
        } else { "bounded static metadata only; no clean-worktree claim".into() },
        inventory: original, exclusions,
        scanner: ScannerRecord { binary_sha256: binary_hash, config_sha256: config_hash, findings,
            scope: "local Gitleaks detection over opaque-name exact-byte snapshots; no semantic parsing or exhaustive secret guarantee".into() },
        manifest_sha256: manifest_hash, marker_sha256: hash_bytes(&marker),
    };
    write_new(&batch.join("manifest.jsonl"), &manifest, "manifest")?;
    write_new(
        &work.join("receipt.json"),
        &json_bytes(&receipt)?,
        "receipt",
    )?;
    // The marker is the LAST staged artifact, after source and destination checks.
    write_new(&batch.join("_BATCH_COMPLETE"), &marker, "readiness marker")?;
    response("prepared", &work, &batch, &receipt)
}

fn response(status: &str, work: &Path, batch: &Path, receipt: &Receipt) -> Result<Value> {
    Ok(
        json!({"ok": true, "schema_version": 1, "status": status, "work": display(work), "batch": display(batch),
        "receipt": display(&work.join("receipt.json")), "files": receipt.entries.len(),
        "bytes": total_bytes(&receipt.entries)?, "excluded": receipt.exclusions.len(),
        "exclusions": receipt.exclusions.iter().take(20).collect::<Vec<_>>(),
        "exclusions_truncated": receipt.exclusions.len() > 20,
        "scanner_completed": true, "pending_intake": true, "ingested": false}),
    )
}

fn verify(work: &Path) -> Result<(Receipt, PathBuf)> {
    let receipt_path = existing_file(&work.join("receipt.json"), "receipt")?;
    let receipt: Receipt = serde_json::from_reader(open_read(&receipt_path, "receipt")?)
        .map_err(|_| "receipt is not valid schema JSON".to_string())?;
    validate_receipt(&receipt)?;
    let source = existing_dir(Path::new(&receipt.source_root), "original source")?;
    let inbox = existing_dir(Path::new(&receipt.inbox_root), "receipt inbox")?;
    disjoint(&source, &inbox, "source and inbox")?;
    disjoint(&source, work, "source and work")?;
    disjoint(&inbox, work, "inbox and work")?;
    if inventory(&source, true)? != receipt.inventory {
        return Err("original source inventory changed since the verified scan".into());
    }
    let batch = locate_batch(work, &inbox)?;
    let mut root_names = BTreeSet::new();
    for child in fs::read_dir(&batch).map_err(|e| io_error("read batch root", &e))? {
        let child = child.map_err(|e| io_error("read batch entry", &e))?;
        let name = child
            .file_name()
            .to_str()
            .ok_or("non-UTF-8 batch entry")?
            .to_owned();
        if is_reparse(
            &fs::symlink_metadata(child.path()).map_err(|e| io_error("inspect batch entry", &e))?,
        ) {
            return Err("batch controls or payload are reparse entries".into());
        }
        root_names.insert(name);
    }
    let expected: BTreeSet<_> = ["payload", "manifest.jsonl", "_BATCH_COMPLETE"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    if root_names != expected {
        return Err("batch root has missing or extra artifacts".into());
    }
    verify_payload(
        &existing_dir(&batch.join("payload"), "payload")?,
        &receipt.entries,
        &receipt.directories,
    )?;
    let manifest = manifest_bytes(&receipt.entries)?;
    if hash_bytes(&manifest) != receipt.manifest_sha256
        || read_control(&batch.join("manifest.jsonl"), "manifest")? != manifest
    {
        return Err("generated manifest bytes do not match receipt and payload".into());
    }
    let marker = marker_bytes(&receipt.manifest_sha256, &receipt.entries)?;
    if hash_bytes(&marker) != receipt.marker_sha256
        || read_control(&batch.join("_BATCH_COMPLETE"), "readiness marker")? != marker
    {
        return Err("readiness marker does not match verified batch".into());
    }
    // A second original inventory catches writes during destination verification.
    if inventory(&source, true)? != receipt.inventory {
        return Err("original source changed during verification".into());
    }
    Ok((receipt, batch))
}

fn validate_receipt(receipt: &Receipt) -> Result<()> {
    if receipt.schema_version != 1 || receipt.status != "prepared" {
        return Err("unsupported receipt version or state".into());
    }
    validate_entries(&receipt.entries)?;
    validate_entries(&receipt.inventory.files)?;
    validate_paths(&receipt.directories)?;
    validate_paths(&receipt.inventory.directories)?;
    validate_paths(&receipt.scanner.findings)?;
    if receipt.directories != receipt.inventory.directories {
        return Err("receipt directory layout differs from source inventory".into());
    }
    for hash in [
        &receipt.scanner.binary_sha256,
        &receipt.scanner.config_sha256,
        &receipt.manifest_sha256,
        &receipt.marker_sha256,
    ] {
        if !valid_hash(hash) {
            return Err("receipt contains an invalid SHA-256".into());
        }
    }
    let finding_set: BTreeSet<_> = receipt.scanner.findings.iter().cloned().collect();
    let available: BTreeSet<_> = receipt
        .inventory
        .files
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    if !finding_set.is_subset(&available) {
        return Err("scanner findings refer to paths outside scanned inventory".into());
    }
    let expected_entries: Vec<_> = receipt
        .inventory
        .files
        .iter()
        .filter(|entry| !finding_set.contains(&entry.path))
        .cloned()
        .collect();
    if expected_entries != receipt.entries {
        return Err("receipt drops or changes source entries without a scanner disposition".into());
    }
    let mut exclusions = receipt.inventory.exclusions.clone();
    exclusions.extend(receipt.scanner.findings.iter().map(|path| Exclusion {
        path: path.clone(),
        kind: "file".into(),
        reason: "local secret scanner finding".into(),
    }));
    exclusions.sort_by(|a, b| a.path.cmp(&b.path));
    if exclusions != receipt.exclusions {
        return Err("receipt exclusion set is inconsistent".into());
    }
    Ok(())
}

fn publish(options: &BTreeMap<String, String>) -> Result<Value> {
    validate_batch_name(&options["name"])?;
    let work = existing_dir(Path::new(&options["work"]), "work")?;
    let inbox = existing_dir(Path::new(&options["inbox"]), "inbox")?;
    let (receipt, batch) = verify(&work)?;
    let bound_inbox = existing_dir(Path::new(&receipt.inbox_root), "receipt inbox")?;
    if !same_path(&inbox, &bound_inbox) {
        return Err("publish inbox differs from receipt inbox".into());
    }
    if !same_path(&batch, &work.join("batch")) {
        return Err("batch is already published; no overwrite or merge retry is permitted".into());
    }
    if !same_volume(&batch, &inbox)? {
        return Err("atomic publication requires the same volume".into());
    }
    let destination = inbox.join(&options["name"]);
    ensure_absent(&destination, "publication destination already exists")?;
    // Durable intent lets readonly verify locate the batch after the directory move.
    let intent = json!({"schema_version": 1, "status": "publish_intent", "name": options["name"],
        "inbox_root": display(&inbox), "manifest_sha256": receipt.manifest_sha256});
    write_new(
        &work.join("publish-intent.json"),
        &json_bytes(&intent)?,
        "publish intent (retries are disallowed)",
    )?;
    atomic_move_no_replace(&batch, &destination)?;
    response("published", &work, &destination, &receipt)
}

fn locate_batch(work: &Path, inbox: &Path) -> Result<PathBuf> {
    let batch = work.join("batch");
    match fs::symlink_metadata(&batch) {
        Ok(_) => existing_dir(&batch, "batch"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let intent: Value = serde_json::from_reader(open_read(
                &work.join("publish-intent.json"),
                "publish intent",
            )?)
            .map_err(|_| "batch is missing and publish intent is invalid".to_string())?;
            if intent.get("schema_version").and_then(Value::as_u64) != Some(1)
                || intent.get("status").and_then(Value::as_str) != Some("publish_intent")
                || intent.get("inbox_root").and_then(Value::as_str) != Some(display(inbox).as_str())
            {
                return Err("invalid publish intent".into());
            }
            let name = intent
                .get("name")
                .and_then(Value::as_str)
                .ok_or("publish intent lacks batch name")?;
            validate_batch_name(name)?;
            existing_dir(&inbox.join(name), "published batch")
        }
        Err(error) => Err(io_error("inspect batch", &error)),
    }
}

fn inventory(root: &Path, policy: bool) -> Result<Inventory> {
    let mut result = Inventory {
        files: vec![],
        directories: vec![],
        exclusions: vec![],
        git: vec![],
    };
    walk(root, "", policy, &mut result)?;
    result.files.sort_by(|a, b| a.path.cmp(&b.path));
    result.directories.sort();
    result.exclusions.sort_by(|a, b| a.path.cmp(&b.path));
    result.git.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

fn walk(root: &Path, relative: &str, policy: bool, result: &mut Inventory) -> Result<()> {
    let directory = root.join(relative);
    let metadata =
        fs::symlink_metadata(&directory).map_err(|e| io_error("inspect directory", &e))?;
    if !metadata.is_dir() || is_reparse(&metadata) {
        return Err(format!("directory changed type or is reparse: {relative}"));
    }
    let mut children = Vec::new();
    let mut folded = BTreeSet::new();
    for child in fs::read_dir(&directory).map_err(|e| io_error("enumerate directory", &e))? {
        let child = child.map_err(|e| io_error("enumerate child", &e))?;
        let name = child
            .file_name()
            .to_str()
            .ok_or("non-UTF-8 filename cannot be preserved safely")?
            .to_owned();
        validate_segment(&name)?;
        if !folded.insert(name.to_lowercase()) {
            return Err(format!("Windows case collision in directory: {relative}"));
        }
        children.push((name, child.path()));
    }
    children.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, path) in children {
        let rel = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| io_error("inspect source entry", &e))?;
        if is_reparse(&metadata) {
            if !policy {
                return Err(format!("payload contains reparse entry: {rel}"));
            }
            if name.eq_ignore_ascii_case(".git") {
                result.git.push(git_record(&path, relative));
            }
            result.exclusions.push(Exclusion {
                path: rel,
                kind: "reparse".into(),
                reason: "unsupported link/reparse entry; not traversed".into(),
            });
            continue;
        }
        let kind = if metadata.is_dir() {
            "directory"
        } else {
            "file"
        };
        if policy && let Some(reason) = excluded(&name, &path, &metadata)? {
            if name.eq_ignore_ascii_case(".git") {
                result.git.push(git_record(&path, relative));
            }
            result.exclusions.push(Exclusion {
                path: rel,
                kind: kind.into(),
                reason: reason.into(),
            });
            continue;
        }
        if metadata.is_dir() {
            result.directories.push(rel.clone());
            walk(root, &rel, policy, result)?;
        } else if metadata.is_file() {
            let (size, sha256) = hash_file(&path, &rel)?;
            result.files.push(Entry {
                path: rel,
                size,
                sha256,
            });
        } else {
            return Err(format!("unsupported non-regular source entry: {rel}"));
        }
    }
    Ok(())
}

fn excluded(name: &str, path: &Path, metadata: &Metadata) -> Result<Option<&'static str>> {
    let name = name.to_ascii_lowercase();
    if name == ".git"
        || (metadata.is_dir()
            && [".hg", ".svn", "node_modules", "bower_components"].contains(&name.as_str()))
    {
        return Ok(Some("VCS or dependency tree"));
    }
    if (metadata.is_dir() && [".aws", ".ssh", ".azure", ".kube", ".gnupg"].contains(&name.as_str()))
        || name == ".env"
        || name.starts_with(".env.")
        || [
            ".netrc",
            "_netrc",
            ".npmrc",
            ".pypirc",
            ".git-credentials",
            ".dockercfg",
            "credentials.json",
            "credentials.ini",
            "credentials.csv",
            "secrets.json",
            "secrets.toml",
            "secrets.yaml",
            "secrets.yml",
            "id_rsa",
            "id_dsa",
            "id_ecdsa",
            "id_ed25519",
        ]
        .contains(&name.as_str())
    {
        return Ok(Some("known credential path"));
    }
    if metadata.is_dir()
        && regular_exists(&path.join("pyvenv.cfg"))?
        && (interpreter_marker(path, "bin", "python")?
            || interpreter_marker(path, "Scripts", "python.exe")?)
    {
        return Ok(Some("Python virtual environment dependency tree"));
    }
    if metadata.is_dir() && name == "site-packages" && installed_distributions(path)? {
        return Ok(Some("Python installed dependency tree"));
    }
    Ok(None)
}

fn installed_distributions(path: &Path) -> Result<bool> {
    for child in
        fs::read_dir(path).map_err(|e| io_error("inspect installed dependency tree", &e))?
    {
        let child = child.map_err(|e| io_error("inspect installed dependency marker", &e))?;
        let metadata = fs::symlink_metadata(child.path())
            .map_err(|e| io_error("inspect installed dependency marker", &e))?;
        if !metadata.is_dir() || is_reparse(&metadata) {
            continue;
        }
        let name = child.file_name();
        let name = name.to_string_lossy();
        if (name.ends_with(".dist-info") && regular_exists(&child.path().join("METADATA"))?)
            || (name.ends_with(".egg-info") && regular_exists(&child.path().join("PKG-INFO"))?)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn interpreter_marker(root: &Path, directory: &str, name: &str) -> Result<bool> {
    let directory = root.join(directory);
    match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.is_dir() && !is_reparse(&metadata) => {
            regular_exists(&directory.join(name))
        }
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("inspect dependency interpreter directory", &error)),
    }
}

fn regular_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_file() && !is_reparse(&metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("inspect dependency marker", &error)),
    }
}

fn git_record(git: &Path, relative: &str) -> GitRecord {
    let mut record = GitRecord {
        path: if relative.is_empty() {
            ".".into()
        } else {
            relative.into()
        },
        head: None,
        head_status: "unavailable: bounded static metadata read".into(),
        origin: None,
        origin_sha256: None,
        origin_status: "unavailable: bounded static metadata read".into(),
    };
    if !fs::symlink_metadata(git).is_ok_and(|meta| meta.is_dir() && !is_reparse(&meta)) {
        return record;
    }
    if let Ok(head) = bounded_text(&git.join("HEAD"), 4096) {
        let value = head.trim();
        let commit = if let Some(reference) = value.strip_prefix("ref: ") {
            if reference.starts_with("refs/") && validate_relative(reference).is_ok() {
                bounded_text(&git.join(reference), 4096)
                    .ok()
                    .map(|text| text.trim().to_owned())
                    .or_else(|| {
                        bounded_text(&git.join("packed-refs"), 65536)
                            .ok()
                            .and_then(|text| {
                                text.lines().find_map(|line| {
                                    line.split_once(' ')
                                        .filter(|(_, name)| *name == reference)
                                        .map(|(hash, _)| hash.to_owned())
                                })
                            })
                    })
            } else {
                None
            }
        } else {
            Some(value.to_owned())
        };
        if let Some(commit) = commit.filter(|hash| {
            (hash.len() == 40 || hash.len() == 64) && hash.bytes().all(|c| c.is_ascii_hexdigit())
        }) {
            record.head = Some(commit.to_ascii_lowercase());
            record.head_status = "observed static Git HEAD".into();
        }
    }
    if let Ok(config) = bounded_text(&git.join("config"), 65536) {
        let mut origin_section = false;
        for line in config.lines().map(str::trim) {
            if line.starts_with('[') {
                origin_section = line == "[remote \"origin\"]";
            }
            if origin_section
                && let Some((key, value)) = line.split_once('=')
                && key.trim() == "url"
            {
                // Git remotes may carry passwords or tokens. Preserve identity
                // without persisting arbitrary credential-bearing source strings.
                record.origin_sha256 = Some(hash_bytes(value.trim().as_bytes()));
                record.origin_status = "observed static origin SHA-256; raw URL omitted".into();
                break;
            }
        }
    }
    record
}

fn bounded_text(path: &Path, limit: u64) -> Result<String> {
    let file = existing_file(path, "Git metadata")?;
    let input = open_read(&file, "Git metadata")?;
    let mut bytes = Vec::new();
    input
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("read Git metadata", &e))?;
    if bytes.len() as u64 > limit {
        return Err("Git metadata exceeds static read bound".into());
    }
    String::from_utf8(bytes).map_err(|_| "Git metadata is not UTF-8".into())
}

fn scan_snapshot(
    work: &Path,
    source: &Path,
    entries: &[Entry],
    scanner: &Path,
    config: &Path,
) -> Result<Vec<String>> {
    let scan_root = work.join("scan-input");
    fs::create_dir(&scan_root).map_err(|e| io_error("create scanner quarantine", &e))?;
    let mut aliases = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let alias = format!("{index:012}.txt");
        copy_exact(&source.join(&entry.path), &scan_root.join(&alias), entry)?;
        aliases.insert(alias, entry.path.clone());
    }
    let ignore = work.join("empty-gitleaks-ignore");
    write_new(&ignore, b"", "empty scanner ignore file")?;
    let output = Command::new(scanner)
        .arg("dir")
        .arg(&scan_root)
        .args(["--config"])
        .arg(config)
        .args(["--gitleaks-ignore-path"])
        .arg(&ignore)
        .args([
            "--ignore-gitleaks-allow",
            "--redact=100",
            "--no-banner",
            "--no-color",
            "--log-level",
            "error",
            "--report-format",
            "json",
            "--report-path",
            "-",
            "--exit-code",
            "10",
            "--max-target-megabytes",
            "0",
            "--max-decode-depth",
            "5",
            "--max-archive-depth",
            "0",
        ])
        .env_remove("GITLEAKS_CONFIG")
        .env_remove("GITLEAKS_CONFIG_TOML")
        .current_dir(work)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| io_error("launch local secret scanner", &e))?;
    let exit = output
        .status
        .code()
        .ok_or("local secret scanner terminated without an exit code")?;
    if exit != 0 && exit != 10 {
        return Err(format!(
            "local secret scanner failed with exit code {exit}; raw output withheld"
        ));
    }
    let report: Value = serde_json::from_slice(&output.stdout).map_err(|_| {
        "local secret scanner returned malformed JSON; raw output withheld".to_string()
    })?;
    let rows = report
        .as_array()
        .ok_or("local secret scanner report must be an array")?;
    if (exit == 0 && !rows.is_empty()) || (exit == 10 && rows.is_empty()) {
        return Err("local secret scanner exit code contradicts report".into());
    }
    let mut findings = BTreeSet::new();
    for row in rows {
        let object = row
            .as_object()
            .ok_or("malformed secret scanner finding object")?;
        let file = object
            .get("File")
            .and_then(Value::as_str)
            .ok_or("secret scanner finding lacks File")?;
        if object
            .get("RuleID")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err("secret scanner finding lacks RuleID".into());
        }
        let portable = file.replace('\\', "/");
        let alias = portable
            .rsplit('/')
            .next()
            .ok_or("invalid scanner finding path")?;
        let original = aliases
            .get(alias)
            .ok_or("scanner finding refers to an unknown snapshot file")?;
        let expected_file = display(&scan_root.join(alias)).replace('\\', "/");
        let normalize = |value: &str| value.strip_prefix("//?/").unwrap_or(value).to_lowercase();
        if portable != alias
            && portable != format!("scan-input/{alias}")
            && normalize(&portable) != normalize(&expected_file)
        {
            return Err("scanner finding path is outside the byte snapshot".into());
        }
        findings.insert(original.clone());
    }
    // Check every snapshot byte again, including scanner-negative files.
    let scanned = inventory(&scan_root, false)?;
    if !scanned.directories.is_empty() || scanned.files.len() != entries.len() {
        return Err("scanner snapshot path set changed during scan".into());
    }
    for (index, entry) in entries.iter().enumerate() {
        let scanned = &scanned.files[index];
        if scanned.path != format!("{index:012}.txt")
            || scanned.size != entry.size
            || scanned.sha256 != entry.sha256
        {
            return Err("scanner snapshot bytes changed during scan".into());
        }
    }
    Ok(findings.into_iter().collect())
}

fn cleanup_scan_inputs(work: &Path) -> Result<()> {
    let scan = work.join("scan-input");
    // The only recursively removed path is an exclusively created child of this work.
    let parent = existing_dir(
        scan.parent().ok_or("scanner quarantine has no parent")?,
        "quarantine parent",
    )?;
    if !same_path(&parent, work)
        || scan.file_name().and_then(|name| name.to_str()) != Some("scan-input")
    {
        return Err("refused scanner quarantine cleanup outside owned work".into());
    }
    match fs::symlink_metadata(&scan) {
        Ok(metadata) if is_reparse(&metadata) => {
            if metadata.is_dir() {
                fs::remove_dir(&scan)
            } else {
                fs::remove_file(&scan)
            }
            .map_err(|e| io_error("unlink reparse scanner quarantine", &e))?;
        }
        Ok(_) => fs::remove_dir_all(&scan)
            .map_err(|e| io_error("remove owned scanner quarantine", &e))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("inspect scanner quarantine cleanup", &error)),
    }
    Ok(())
}

fn verify_payload(payload: &Path, entries: &[Entry], directories: &[String]) -> Result<()> {
    let actual = inventory(payload, false)?;
    if actual.files != entries || actual.directories != directories || !actual.exclusions.is_empty()
    {
        return Err(
            "payload bytes, hashes, files, or directory path set differ from verified source"
                .into(),
        );
    }
    Ok(())
}

fn copy_exact(source: &Path, destination: &Path, expected: &Entry) -> Result<()> {
    let mut input = open_read(source, &expected.path)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| io_error("create exclusive byte copy", &e))?;
    let copied = (|| {
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let count = input
                .read(&mut buffer)
                .map_err(|e| io_error("read source while copying", &e))?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|e| io_error("write byte copy", &e))?;
            hasher.update(&buffer[..count]);
            size = size.checked_add(count as u64).ok_or("file size overflow")?;
        }
        output
            .sync_all()
            .map_err(|e| io_error("flush byte copy", &e))?;
        if size != expected.size || format!("{:x}", hasher.finalize()) != expected.sha256 {
            return Err(format!(
                "source bytes changed while copying: {}",
                expected.path
            ));
        }
        if hash_file(destination, &expected.path)? != (expected.size, expected.sha256.clone()) {
            return Err(format!(
                "independent copy verification failed: {}",
                expected.path
            ));
        }
        Ok(())
    })();
    drop(output);
    if let Err(error) = copied {
        // Only this exclusively created leaf is removed, never an existing file.
        fs::remove_file(destination).map_err(|cleanup| {
            format!(
                "{error}; {}",
                io_error("remove rejected partial copy", &cleanup)
            )
        })?;
        return Err(error);
    }
    Ok(())
}

fn hash_file(path: &Path, label: &str) -> Result<(u64, String)> {
    let mut input = open_read(path, label)?;
    hash_open_file(&mut input, label)
}

fn hash_open_file(input: &mut File, label: &str) -> Result<(u64, String)> {
    let initial = input
        .metadata()
        .map_err(|e| io_error("read file metadata", &e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0u64;
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|e| io_error("hash file", &e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        size = size.checked_add(count as u64).ok_or("file size overflow")?;
    }
    let after = input
        .metadata()
        .map_err(|e| io_error("read file metadata after hash", &e))?;
    if size != initial.len()
        || size != after.len()
        || initial.modified().ok() != after.modified().ok()
    {
        return Err(format!("file changed while hashing: {label}"));
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

fn open_read(path: &Path, label: &str) -> Result<File> {
    open_read_with_share(path, label, false)
}

fn open_read_locked(path: &Path, label: &str) -> Result<File> {
    open_read_with_share(path, label, true)
}

fn open_read_with_share(path: &Path, label: &str, pin: bool) -> Result<File> {
    let path = absolute_no_links(path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("inspect {label}: {}", io_error("filesystem", &e)))?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err(format!("not a regular non-reparse file: {label}"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
        if pin {
            options.share_mode(0x00000001); // FILE_SHARE_READ: deny writes/deletes.
        }
    }
    #[cfg(not(windows))]
    if pin {
        return Err("immutable scanner/config pins are currently supported only on Windows".into());
    }
    let file = options
        .open(&path)
        .map_err(|e| format!("open {label}: {}", io_error("filesystem", &e)))?;
    let metadata = file
        .metadata()
        .map_err(|e| io_error("inspect opened file", &e))?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err(format!(
            "opened file became reparse or non-regular: {label}"
        ));
    }
    Ok(file)
}

fn validate_standalone_config(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(|_| "trusted scanner config must be UTF-8")?;
    let mut extend = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            // Only the shipped standalone [extend] form is supported. Gitleaks
            // validates the rest of TOML; external imports are never relocated.
            if line
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace() && !"[]_.-".contains(c))
            {
                return Err("quoted, escaped, or non-ASCII scanner config table keys are unsupported in standalone snapshots".into());
            }
            let normalized: String = line
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_ascii_lowercase();
            let table = normalized.trim_matches(['[', ']']);
            if (table == "extend" || table.starts_with("extend.")) && line != "[extend]" {
                return Err("scanner config extension tables must use standalone [extend] syntax; external imports are unsupported".into());
            }
            extend = line == "[extend]";
        } else if let Some((key, value)) = line.split_once('=') {
            if key
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace() && !"_.-".contains(c))
            {
                return Err("quoted, escaped, or non-ASCII scanner config keys are unsupported in standalone snapshots".into());
            }
            let key: String = key
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_ascii_lowercase();
            if key == "extend" || key.starts_with("extend.") {
                return Err("inline or dotted scanner config extensions are unsupported; use standalone [extend] useDefault = true".into());
            }
            if extend && (key != "usedefault" || value.trim() != "true") {
                return Err("external scanner config imports and extension overrides are unsupported; only built-in useDefault = true is allowed".into());
            }
        } else if extend {
            return Err("unsupported standalone scanner config extension syntax".into());
        }
    }
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn total_bytes(entries: &[Entry]) -> Result<u64> {
    entries.iter().try_fold(0u64, |sum, entry| {
        sum.checked_add(entry.size)
            .ok_or("total byte count overflow".into())
    })
}

fn manifest_bytes(entries: &[Entry]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for entry in entries {
        serde_json::to_writer(&mut bytes, &json!({"file": format!("payload/{}", entry.path), "bytes": entry.size, "sha256": entry.sha256}))
            .map_err(|_| "could not serialize generated manifest".to_string())?;
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn marker_bytes(_manifest_hash: &str, _entries: &[Entry]) -> Result<Vec<u8>> {
    // The intake contract accepts an empty readiness marker. Evidence is external.
    Ok(Vec::new())
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| "could not serialize generated JSON".to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8], label: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create {label}: {}", io_error("filesystem", &e)))?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        let reason = io_error("write or flush generated artifact", &error);
        fs::remove_file(path).map_err(|cleanup| {
            format!(
                "{reason}; {}",
                io_error("remove rejected control artifact", &cleanup)
            )
        })?;
        return Err(reason);
    }
    Ok(())
}

fn read_control(path: &Path, label: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    open_read(path, label)?
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("read generated control", &e))?;
    Ok(bytes)
}

fn validate_entries(entries: &[Entry]) -> Result<()> {
    validate_paths(
        &entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>(),
    )?;
    if entries.iter().any(|entry| !valid_hash(&entry.sha256)) {
        return Err("invalid file SHA-256 in receipt".into());
    }
    Ok(())
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn validate_paths(paths: &[String]) -> Result<()> {
    let mut previous: Option<&str> = None;
    let mut folded = BTreeSet::new();
    for path in paths {
        validate_relative(path)?;
        if previous.is_some_and(|prev| prev >= path.as_str()) || !folded.insert(path.to_lowercase())
        {
            return Err("receipt paths are unsorted, duplicated, or collide on Windows".into());
        }
        previous = Some(path);
    }
    Ok(())
}

fn validate_relative(path: &str) -> Result<()> {
    if path.is_empty() || path.contains('\\') {
        return Err("unsafe relative path".into());
    }
    for part in path.split('/') {
        validate_segment(part)?;
    }
    Ok(())
}

fn validate_segment(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("filename contains an unsafe Windows path segment".into());
    }
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$"];
    if reserved.contains(&stem.as_str())
        || (stem.starts_with("COM") || stem.starts_with("LPT"))
            && ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&&stem[3..])
    {
        return Err("reserved Windows device filename".into());
    }
    Ok(())
}

fn validate_batch_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    if !(12..=40).contains(&bytes.len())
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'-')
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
        || !bytes[11..]
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        || !bytes[11].is_ascii_alphanumeric()
        || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
    {
        return Err(
            "batch name must be YYYY-MM-DD-lowercase-slug, 12 to 40 ASCII characters".into(),
        );
    }
    let year: u32 = name[..4].parse().map_err(|_| "invalid date year")?;
    let month: u32 = name[5..7].parse().map_err(|_| "invalid date month")?;
    let day: u32 = name[8..10].parse().map_err(|_| "invalid date day")?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days {
        return Err("batch name contains an invalid calendar date".into());
    }
    Ok(())
}

fn absolute_no_links(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| io_error("current directory", &e))?
            .join(path)
    };
    let mut walked = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => return Err("root paths may not contain '..'".into()),
            Component::CurDir => continue,
            Component::Normal(name) => {
                validate_segment(name.to_str().ok_or("non-UTF-8 root path")?)?
            }
            _ => {}
        }
        walked.push(component.as_os_str());
        // A drive prefix alone (C:) is drive-relative, not an ancestor directory.
        if matches!(component, Component::Prefix(_) | Component::RootDir) {
            continue;
        }
        match fs::symlink_metadata(&walked) {
            Ok(metadata) if is_reparse(&metadata) => {
                return Err("root or ancestor is a link/reparse entry".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("inspect root ancestor", &error)),
        }
    }
    Ok(walked)
}

fn existing_dir(path: &Path, label: &str) -> Result<PathBuf> {
    let path = absolute_no_links(path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("inspect {label}: {}", io_error("filesystem", &e)))?;
    if !metadata.is_dir() || is_reparse(&metadata) {
        return Err(format!("{label} must be an existing non-reparse directory"));
    }
    fs::canonicalize(path).map_err(|e| io_error("canonicalize directory", &e))
}

fn existing_file(path: &Path, label: &str) -> Result<PathBuf> {
    let path = absolute_no_links(path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("inspect {label}: {}", io_error("filesystem", &e)))?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err(format!("{label} must be an existing non-reparse file"));
    }
    fs::canonicalize(path).map_err(|e| io_error("canonicalize file", &e))
}

fn new_dir_path(path: &Path) -> Result<PathBuf> {
    let path = absolute_no_links(path)?;
    ensure_absent(
        &path,
        "work must be a new directory; retries cannot overwrite",
    )?;
    let parent = existing_dir(path.parent().ok_or("work has no parent")?, "work parent")?;
    Ok(parent.join(path.file_name().ok_or("work has no name")?))
}

fn ensure_absent(path: &Path, message: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(message.into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("inspect exclusive destination", &error)),
    }
}

fn path_key(path: &Path) -> Vec<String> {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}
fn same_path(a: &Path, b: &Path) -> bool {
    path_key(a) == path_key(b)
}
fn within(path: &Path, root: &Path) -> bool {
    path_key(path).starts_with(&path_key(root))
}
fn disjoint(a: &Path, b: &Path, label: &str) -> Result<()> {
    if within(a, b) || within(b, a) {
        Err(format!("{label} must be canonical disjoint roots"))
    } else {
        Ok(())
    }
}
fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn is_reparse(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 || metadata.file_type().is_symlink()
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn io_error(action: &str, error: &std::io::Error) -> String {
    format!(
        "{action}: {:?} (os code {:?})",
        error.kind(),
        error.raw_os_error()
    )
}

fn check_free_space(parent: &Path, inventory: &Inventory) -> Result<()> {
    let (available, cluster) = available_space(parent)?;
    let allocation = inventory.files.iter().try_fold(0u64, |sum, entry| {
        let rounded = entry
            .size
            .checked_add(cluster - 1)
            .ok_or("allocation estimate overflow")?
            / cluster
            * cluster;
        sum.checked_add(rounded)
            .ok_or("allocation estimate overflow")
    })?;
    // Scan inputs are removed before payload copying, so both complete trees do
    // not coexist. Include full receipt/manifest estimates and filesystem slack.
    let metadata = (json_bytes(inventory)?.len() as u64)
        .checked_mul(3)
        .ok_or("metadata estimate overflow")?
        .checked_add(manifest_bytes(&inventory.files)?.len() as u64)
        .ok_or("metadata estimate overflow")?;
    let overhead = (inventory.files.len() as u64 + inventory.directories.len() as u64 + 10)
        .checked_mul(cluster.max(4096))
        .ok_or("allocation overhead overflow")?;
    let required = allocation
        .checked_add(metadata)
        .and_then(|sum| sum.checked_add(overhead))
        .and_then(|sum| sum.checked_add(1024 * 1024))
        .ok_or("required space estimate overflow")?;
    if available < required {
        return Err(format!(
            "insufficient work volume space: required estimate {required} bytes, available {available} bytes"
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn available_space(path: &Path) -> Result<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            path: *const u16,
            available: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> i32;
        fn GetVolumePathNameW(file: *const u16, volume: *mut u16, length: u32) -> i32;
        fn GetDiskFreeSpaceW(
            root: *const u16,
            sectors: *mut u32,
            bytes: *mut u32,
            free: *mut u32,
            total: *mut u32,
        ) -> i32;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    if unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(io_error(
            "query work volume free space",
            &std::io::Error::last_os_error(),
        ));
    }
    let mut root = vec![0u16; 32768];
    if unsafe { GetVolumePathNameW(wide.as_ptr(), root.as_mut_ptr(), root.len() as u32) } == 0 {
        return Err(io_error(
            "query work volume root",
            &std::io::Error::last_os_error(),
        ));
    }
    let (mut sectors, mut bytes) = (0u32, 0u32);
    if unsafe {
        GetDiskFreeSpaceW(
            root.as_ptr(),
            &mut sectors,
            &mut bytes,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(io_error(
            "query work volume allocation unit",
            &std::io::Error::last_os_error(),
        ));
    }
    let cluster = u64::from(sectors) * u64::from(bytes);
    if cluster == 0 {
        return Err("invalid work volume allocation unit".into());
    }
    Ok((available, cluster))
}

#[cfg(not(windows))]
fn available_space(_path: &Path) -> Result<(u64, u64)> {
    Err("free-space staging preflight is currently supported only on Windows".into())
}

#[cfg(windows)]
fn same_volume(a: &Path, b: &Path) -> Result<bool> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetVolumePathNameW(file: *const u16, volume: *mut u16, length: u32) -> i32;
        fn GetVolumeNameForVolumeMountPointW(mount: *const u16, name: *mut u16, length: u32)
        -> i32;
    }
    fn volume(path: &Path) -> Result<Vec<u16>> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut mount = vec![0u16; 32768];
        if unsafe { GetVolumePathNameW(wide.as_ptr(), mount.as_mut_ptr(), mount.len() as u32) } == 0
        {
            return Err(io_error(
                "get publication volume path",
                &std::io::Error::last_os_error(),
            ));
        }
        let mut name = vec![0u16; 128];
        if unsafe {
            GetVolumeNameForVolumeMountPointW(mount.as_ptr(), name.as_mut_ptr(), name.len() as u32)
        } == 0
        {
            return Err(io_error(
                "get publication volume identity",
                &std::io::Error::last_os_error(),
            ));
        }
        name.truncate(
            name.iter()
                .position(|c| *c == 0)
                .ok_or("invalid volume identity")?,
        );
        Ok(name)
    }
    Ok(volume(a)? == volume(b)?)
}

#[cfg(unix)]
fn same_volume(a: &Path, b: &Path) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;
    Ok(fs::metadata(a)
        .map_err(|e| io_error("inspect work volume", &e))?
        .dev()
        == fs::metadata(b)
            .map_err(|e| io_error("inspect inbox volume", &e))?
            .dev())
}

#[cfg(windows)]
fn atomic_move_no_replace(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // Zero flags: neither MOVEFILE_REPLACE_EXISTING nor cross-volume COPY_ALLOWED.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) } == 0 {
        return Err(io_error(
            "atomic no-replace publication failed",
            &std::io::Error::last_os_error(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn atomic_move_no_replace(from: &Path, to: &Path) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        fn renameat2(olddir: i32, old: *const i8, newdir: i32, new: *const i8, flags: u32) -> i32;
    }
    let from = CString::new(from.as_os_str().as_bytes()).map_err(|_| "NUL in source path")?;
    let to = CString::new(to.as_os_str().as_bytes()).map_err(|_| "NUL in destination path")?;
    if unsafe { renameat2(-100, from.as_ptr(), -100, to.as_ptr(), 1) } != 0 {
        return Err(io_error(
            "atomic no-replace publication failed",
            &std::io::Error::last_os_error(),
        ));
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn atomic_move_no_replace(_from: &Path, _to: &Path) -> Result<()> {
    Err("atomic no-replace publication is supported only on Windows and Linux".into())
}
