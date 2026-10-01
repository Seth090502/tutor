//! The MCP tool list (names, descriptions, schemas, annotations) is byte-stable: this fails on any change.
//!
//! After an intended change, regenerate the snapshot:
//! `UPDATE_SNAPSHOT=1 cargo test -p libraryd --test tool_snapshot`

use std::path::PathBuf;

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("snapshots").join("tools.json")
}

#[test]
fn tool_definitions_match_the_snapshot() {
    let tools = libraryd::tools::definitions().expect("definitions build");
    let mut current = serde_json::to_string_pretty(&tools).expect("tools serialise");
    current.push('\n');

    // Building twice gives the same bytes: nothing in the list depends on the run.
    let again = serde_json::to_string_pretty(&libraryd::tools::definitions().unwrap()).unwrap() + "\n";
    assert_eq!(current, again);

    let path = snapshot_path();
    if std::env::var_os("UPDATE_SNAPSHOT").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &current).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing snapshot {}: {e}; run with UPDATE_SNAPSHOT=1", path.display()))
        .replace("\r\n", "\n");
    assert!(
        expected == current,
        "the MCP tool definitions changed; if that is intended, regenerate with UPDATE_SNAPSHOT=1 and review the diff of {}",
        path.display()
    );
}

/// The JSON Schema patterns for ids and addresses (in vault-api's `schema` feature) agree with the real parser.
#[test]
fn schema_patterns_agree_with_the_address_parser() {
    use rmcp::schemars::schema_for;
    use vault_api::{Address, DocId};

    let pattern = |schema: rmcp::schemars::Schema| {
        let value = serde_json::to_value(schema).unwrap();
        regex::Regex::new(value["pattern"].as_str().expect("a pattern")).unwrap()
    };
    let doc_pattern = pattern(schema_for!(DocId));
    let address_pattern = pattern(schema_for!(Address));

    for s in [
        "3f9a1c07b2e4d55e",
        "3f9a1c07b2e4d55e-cowan-magical-number-four",
        "00000000000000ff-a",
        "3f9a1c07b2e4d55e-",
        "3F9A1C07B2E4D55E",
        "3f9a1c07b2e4d55",
        "3f9a1c07b2e4d55e--x",
        "3f9a1c07b2e4d55e-Bad",
        "x",
    ] {
        assert_eq!(doc_pattern.is_match(s), s.parse::<DocId>().is_ok(), "doc id {s:?}");
    }
    for s in [
        "3f9a1c07b2e4d55e#L7",
        "3f9a1c07b2e4d55e#L120-L148",
        "3f9a1c07b2e4d55e-cowan-magical-number-four#L120-L148",
        "3f9a1c07b2e4d55e#L0",
        "3f9a1c07b2e4d55e#L05",
        "3f9a1c07b2e4d55e#120-148",
        "3f9a1c07b2e4d55e#L7-",
        "3f9a1c07b2e4d55e",
        r"C:\vault\text\3f#L1",
    ] {
        // Not covered: the pattern cannot say that the range must not run backwards (`#L5-L4`).
        let parsed = s.parse::<Address>().is_ok();
        assert_eq!(address_pattern.is_match(s), parsed, "address {s:?}");
    }
}
