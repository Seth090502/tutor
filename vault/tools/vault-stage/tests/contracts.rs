mod common;

fn check_case(id: &str) {
    let config = common::Config::discover().expect("trusted fixture scanner configuration");
    let result =
        common::run_case(id, "train", 41, &config).expect("fixture and actual tool invocation");
    let failures: Vec<_> = result.checks.iter().filter(|check| !check.passed).collect();
    assert!(
        result.passed,
        "{id} failed: {failures:?}; receipt in {}",
        result.scratch
    );
}

#[test]
fn heterogeneous_byte_fidelity() {
    check_case("train-static-delivery");
}

#[test]
fn trusted_scanner_ignores_source_suppression() {
    check_case("train-scanner-authority");
}

#[test]
fn same_size_corruption_is_detected() {
    check_case("train-same-size-corruption");
}

#[test]
fn missing_and_extra_paths_are_detected() {
    check_case("train-path-set-integrity");
}

#[test]
fn source_edits_additions_and_deletions_invalidate_verification() {
    check_case("train-source-snapshot");
}

#[test]
fn scanner_errors_never_prepare_a_batch() {
    check_case("train-scanner-failure");
}

#[test]
fn exclusive_allocation_and_publication() {
    check_case("train-publication-ownership");
}

#[test]
fn reparse_cycles_escapes_and_root_overlap() {
    check_case("train-reparse-and-overlap");
}

#[test]
fn splits_are_frozen_and_seed_independent() {
    let train = common::definitions("train").unwrap();
    let test = common::definitions("test").unwrap();
    assert!(!train.is_empty() && !test.is_empty());
    assert!(
        train
            .iter()
            .all(|(id, _)| !test.iter().any(|(other, _)| other == id))
    );
    assert_eq!(train, common::definitions("train").unwrap());
    assert!(common::definitions("unknown").is_err());
}

#[test]
fn inventory_is_read_only_and_does_not_claim_preparation() {
    let mut fixture = common::Fixture::create("plan-preview", 41).unwrap();
    common::make_standard(&mut fixture, 41).unwrap();
    let before = common::snapshot(&fixture.root).unwrap();
    let output = vault_stage::run(&[
        "plan".into(),
        "--source".into(),
        fixture.source.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(output["status"], "inventoried");
    assert_eq!(output["scanner_completed"], false);
    assert_eq!(output["ingested"], false);
    assert_eq!(common::snapshot(&fixture.root).unwrap(), before);
    assert!(!fixture.work.exists());
}

#[test]
fn arguments_and_help_report_actual_supported_capabilities() {
    assert_eq!(
        vault_stage::run(&["--version".into()]).unwrap()["tool"],
        "vault-stage"
    );
    assert!(vault_stage::run(&["--help".into()]).unwrap()["commands"].is_array());
    for arguments in [
        vec!["stage"],
        vec!["verify", "--work"],
        vec!["plan", "--source", "x", "--source", "y"],
        vec!["invented", "--source", "x"],
        vec!["plan", "--source", "x", "--unknown", "y"],
    ] {
        assert!(
            vault_stage::run(&arguments.into_iter().map(String::from).collect::<Vec<_>>()).is_err()
        );
    }
}

#[test]
fn deployed_config_discovery_is_independent_of_build_scratch() {
    let fixture = common::Fixture::create("deployed-config", 41).unwrap();
    let source_tool = fixture.root.join("repository/tools/vault-stage");
    let executable_dir = source_tool.join("bin");
    let canonical_assets = fixture.root.join("repository/skills/ingest/assets");
    std::fs::create_dir_all(&executable_dir).unwrap();
    std::fs::create_dir_all(&canonical_assets).unwrap();
    let canonical_config = canonical_assets.join("gitleaks.toml");
    std::fs::write(&canonical_config, b"title = 'known trusted test asset'\n").unwrap();
    let missing_build_scratch = fixture.root.join("removed-build-scratch");
    assert_eq!(
        std::fs::canonicalize(
            common::discover_config_path(&executable_dir, &missing_build_scratch).unwrap()
        )
        .unwrap(),
        std::fs::canonicalize(&canonical_config).unwrap()
    );
    assert_eq!(
        std::fs::canonicalize(
            common::discover_config_path(
                &fixture.root.join("unrelated/build-target"),
                &source_tool
            )
            .unwrap()
        )
        .unwrap(),
        std::fs::canonicalize(&canonical_config).unwrap()
    );
    let bundled_assets = executable_dir.join("assets");
    std::fs::create_dir(&bundled_assets).unwrap();
    let bundled_config = bundled_assets.join("gitleaks.toml");
    std::fs::write(&bundled_config, b"title = 'bundled trusted test asset'\n").unwrap();
    assert_eq!(
        common::discover_config_path(&executable_dir, &missing_build_scratch).unwrap(),
        bundled_config
    );
}

#[test]
fn evaluator_cli_explicit_overrides_bypass_invalid_environment_defaults() {
    let fixture = common::Fixture::create("explicit-config-cli", 41).unwrap();
    let config = common::Config::discover().unwrap();
    let report = fixture.root.join("train-report.json");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_vault-stage-eval"))
        .args(["--output"])
        .arg(&report)
        .args([
            "--split",
            "train",
            "--seed",
            "41",
            "--repeat",
            "1",
            "--scanner",
        ])
        .arg(&config.scanner)
        .arg("--scanner-config")
        .arg(&config.scanner_config)
        .env(
            "VAULT_STAGE_SCANNER",
            fixture.root.join("invalid-environment-scanner.exe"),
        )
        .env(
            "VAULT_STAGE_SCANNER_CONFIG",
            fixture.root.join("invalid-environment-config.toml"),
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "explicit settings must be resolved before defaults; {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert_eq!(report["passed"], true);
    assert!(report["fixture_errors"].as_array().unwrap().is_empty());
    assert!(
        common::Config::resolve(
            Some(config.scanner),
            Some(fixture.root.join("explicit-missing.toml"))
        )
        .is_err()
    );
}

#[test]
fn independent_grader_rejects_a_reparse_root_with_identical_bytes() {
    let fixture = common::Fixture::create("oracle-root-reparse", 41).unwrap();
    fixture
        .put("notes.md", b"Original source bytes.\n")
        .unwrap();
    let root_alias = fixture.root.join("source-root-alias");
    common::create_link(&root_alias, &fixture.source).unwrap();
    assert!(common::snapshot(&fixture.source).is_ok());
    assert!(common::snapshot(&root_alias).is_err());
    let ordinary_file = fixture.source.join("notes.md");
    assert!(common::snapshot(&ordinary_file).is_err());
}

#[test]
fn trusted_scanner_policy_snapshot_resists_reverted_config_mutation() {
    let mut fixture = common::Fixture::create("trusted-policy-race", 41).unwrap();
    let mut config = common::Config::discover().unwrap();
    let original_policy = std::fs::read(&config.scanner_config).unwrap();
    config.scanner_config = fixture.root.join("trusted-config.toml");
    std::fs::write(&config.scanner_config, &original_policy).unwrap();
    let synthetic_secret = "fixture_only_policy_race_5931472".to_owned();
    fixture.secrets.push(synthetic_secret.clone());
    fixture
        .put(
            "notes-with-credential.md",
            format!("password = {synthetic_secret}\n").as_bytes(),
        )
        .unwrap();
    fixture
        .put("safe.md", b"Original harmless source note.\n")
        .unwrap();
    fixture.seal(&["notes-with-credential.md"]).unwrap();
    config.scanner = common::scanner_wrapper(&fixture, &config, "policy-aba").unwrap();
    let result = common::stage(&fixture, &config);
    let attempt = std::fs::read(fixture.root.join("policy-mutation-attempt.txt"))
        .expect("trusted wrapper attempted the timed mutation");
    assert!(attempt == b"blocked\n" || attempt == b"restored\n");
    assert_eq!(
        std::fs::read(&config.scanner_config).unwrap(),
        original_policy,
        "original config bytes must be restored or write-blocked"
    );
    let checks = if result.is_ok() {
        common::grade_prepared(&fixture, &fixture.work.join("batch"))
    } else {
        assert!(
            !fixture.work.join("batch/_BATCH_COMPLETE").exists(),
            "rejected mutation cannot become prepared"
        );
        assert_eq!(common::snapshot(&fixture.source).unwrap(), fixture.original);
        common::grade_no_scan_residue(&fixture)
    };
    let failures: Vec<_> = checks.iter().filter(|check| !check.passed).collect();
    assert!(
        failures.is_empty(),
        "mutable or reverted trusted policy cannot admit held bytes: {failures:?}"
    );
}
