#[path = "../../tests/common/mod.rs"]
mod common;

use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

fn run() -> Result<bool, String> {
    let mut output = None;
    let mut split = "train".to_owned();
    let mut seed = 41u64;
    let mut repeat = 1usize;
    let mut scanner = None;
    let mut scanner_config = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(flag) = arguments.next() {
        if flag == "--help" || flag == "-h" {
            println!(
                "vault-stage-eval --output <new.json> [--split train|test|all] [--seed N] [--repeat N] [--scanner <exe>] [--scanner-config <toml>]\nDefault split is train. Fixed test membership is never changed by seed. Test outcomes must stay hidden during hillclimbing. Runs synthetic fixtures in fresh scratch, outside the vault."
            );
            return Ok(true);
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--output" => output = Some(PathBuf::from(value)),
            "--split" => split = value,
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| "seed must be an unsigned integer")?
            }
            "--repeat" => {
                repeat = value
                    .parse()
                    .map_err(|_| "repeat must be a positive integer")?
            }
            "--scanner" => scanner = Some(PathBuf::from(value)),
            "--scanner-config" => scanner_config = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    if repeat == 0 || repeat > 100 {
        return Err("repeat must be between 1 and 100".into());
    }
    let output = output.ok_or("--output is required")?;
    let cases = common::definitions(&split)?;
    let config = common::Config::resolve(scanner, scanner_config)?;
    let mut report_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|e| format!("cannot exclusively create output: {e}"))?;
    let started = Instant::now();
    let mut results = Vec::new();
    let mut errors = Vec::new();
    for iteration in 0..repeat {
        for (id, case_split) in &cases {
            match common::run_case(id, case_split, seed.wrapping_add(iteration as u64), &config) {
                Ok(result) => results.push(result),
                Err(error) => errors.push(json!({"id": id, "split": case_split, "repeat": iteration, "passed": false, "fixture_error": error})),
            }
        }
    }
    let total = cases.len() * repeat;
    let passed = results.iter().filter(|result| result.passed).count();
    let assertions = results
        .iter()
        .map(|result| result.checks.len())
        .sum::<usize>();
    let passed_assertions = results
        .iter()
        .flat_map(|result| &result.checks)
        .filter(|check| check.passed)
        .count();
    let scanner_invocations = results
        .iter()
        .map(|result| result.scanner_invocations)
        .sum::<usize>();
    let report = json!({
        "schema_version": 1,
        "protocol": "vault-stage-byte-fidelity-v1",
        "protocol_sha256": common::sha(common::PROTOCOL.as_bytes()),
        "evidence_kind": "synthetic mechanical end-to-end; no local model invoked",
        "split": split,
        "seed": seed,
        "repeat": repeat,
        "passed": passed == total && errors.is_empty(),
        "score": {"passing_cases":passed,"total_cases":total,"fraction":passed as f64 / total as f64,
            "passing_assertions":passed_assertions,"total_assertions":assertions},
        "cost": {"wall_ms":started.elapsed().as_millis(),"scanner_invocations":scanner_invocations},
        "cases": results,
        "fixture_errors": errors,
        "limitations": ["Tests use author-generated synthetic files and actual local Gitleaks. No staging originals or live vault are touched.",
            "Mechanical fidelity is independent of model reliability. Qwen performance requires separately running the optional candidate harness against an explicitly approved local endpoint.",
            "Seed varies harmless bytes; fixed test split is reserved for one final independent result. Reusing test feedback during iteration consumes the holdout."]
    });
    report_file
        .write_all(
            serde_json::to_string_pretty(&report)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"passed":passed == total && errors.is_empty(),"passing_cases":passed,"total_cases":total,"report":output})
    );
    Ok(passed == total && errors.is_empty())
}

fn main() {
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{}", json!({"passed":false,"error":error}));
            std::process::exit(2);
        }
    }
}
