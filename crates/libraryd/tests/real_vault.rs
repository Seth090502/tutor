//! libraryd over a real vault: a temporary vault is built through vault-core's public API and served through the
//! blocking adapter. Every line reads back exactly what vault-core stored, and MCP and gRPC agree.

mod common;

use common::real::{self, RealVault};
use common::*;
use libraryd::blocking::Blocking;
use std::sync::Arc;
use vault_api::*;
use vault_client::ClientError;
use vault_core::Vault;
use vault_core::canon::split_lines;

/// The vault opened as libraryd's binary opens it, and a server on ephemeral ports in front of it.
async fn serve(vault: &RealVault) -> (Arc<dyn VaultApi>, Harness) {
    let api: Arc<dyn VaultApi> = Arc::new(Blocking::new(Arc::new(Vault::open(vault.config.clone()).unwrap())));
    let harness = Harness::start_with(api.clone(), |options| options.vault_id = real::VAULT_ID.into()).await;
    (api, harness)
}

fn like<T: serde::de::DeserializeOwned>(_witness: &T, result: &rmcp::model::CallToolResult) -> T {
    structured(result)
}

/// The same request called directly, over gRPC and over MCP: the three structured answers must be equal.
macro_rules! parity {
    ($api:expr, $grpc:expr, $mcp:expr, $tool:literal, $method:ident, $req:expr) => {{
        let req = $req;
        let direct = $api.$method(req.clone()).await.expect(concat!($tool, " directly"));
        let over_grpc = $grpc.$method(req.clone()).await.expect(concat!($tool, " over gRPC"));
        let over_mcp = like(&direct, &mcp_call(&$mcp, $tool, &req).await);
        assert_eq!(over_grpc, direct, "{} over gRPC", $tool);
        assert_eq!(over_mcp, direct, "{} over MCP", $tool);
        direct
    }};
}

fn assert_exact(via: &str, response: &ReadResponse, doc: &DocId, expected: &[String]) {
    assert!(response.errors.is_empty() && response.truncated.is_none(), "{via} {doc}: {response:?}");
    let got: Vec<&str> = response.passages.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(got, expected, "{via} {doc}");
    assert!(response.passages.iter().all(|p| p.address.doc == *doc), "{via} {doc}: passages carry the document id");
}

#[tokio::test(flavor = "multi_thread")]
async fn every_line_of_every_document_reads_back_exactly_over_both_interfaces() {
    let vault = real::build();
    assert!(vault.docs.len() >= 3);
    let (_api, h) = serve(&vault).await;
    let (grpc, mcp) = (h.grpc(), h.mcp().await);

    for (doc, text) in &vault.docs {
        let lines: Vec<String> = split_lines(text).into_iter().map(String::from).collect();
        let by_line = ReadRequest {
            addresses: (1..=lines.len()).map(|n| format!("{doc}#L{n}")).collect(),
            expand: None,
            max_tokens: Some(20_000),
        };
        let whole = ReadRequest {
            addresses: vec![format!("{doc}#L1-L{}", lines.len())],
            expand: None,
            max_tokens: Some(20_000),
        };
        let joined = [lines.join("\n")];
        for (request, expected) in [(by_line, lines.as_slice()), (whole, joined.as_slice())] {
            assert_exact("gRPC", &grpc.read(request.clone()).await.unwrap(), doc, expected);
            assert_exact("MCP", &structured(&mcp_call(&mcp, "read", &request).await), doc, expected);
        }
    }
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn outline_find_docs_and_vault_info_are_identical_over_both_interfaces() {
    let vault = real::build();
    let (api, h) = serve(&vault).await;
    let (grpc, mcp) = (h.grpc(), h.mcp().await);
    let ids: Vec<String> = vault.docs.iter().map(|(doc, _)| doc.to_string()).collect();

    let outlines = parity!(
        api,
        grpc,
        mcp,
        "outline",
        outline,
        OutlineRequest {
            doc_ids: ids.iter().cloned().chain(["not-a-document".to_string()]).collect(),
            max_depth: None,
            max_tokens: None,
        }
    );
    assert_eq!((outlines.outlines.len(), outlines.errors.len()), (4, 1));
    let heading = &outlines.outlines[0].headings[0];
    assert_eq!((heading.level, heading.text.as_str(), heading.start), (1, "Working memory", 1));
    assert!(outlines.outlines[0].pages.len() >= 2, "the form feed made a second page");
    let symbols = &outlines.outlines[3].symbols;
    assert_eq!((symbols.len(), symbols[0].name.as_str()), (1, "main"));

    let found = parity!(
        api,
        grpc,
        mcp,
        "find_docs",
        find_docs,
        FindDocsRequest {
            queries: vec!["working memory".into(), "plain note".into(), "demo".into(), "no such words xyzzy".into()],
            filters: Filters::default(),
            k: Some(5),
            max_tokens: None,
        }
    );
    let top = |query: usize| found.results[query].first().map(|d| d.doc_id.to_string());
    assert_eq!(top(0), Some(ids[0].clone()));
    assert_eq!(top(1), Some(ids[2].clone()));
    assert_eq!(top(2), Some(ids[3].clone()));
    assert!(found.results[3].is_empty());

    // Free disk space can move between two calls; everything else in vault_info must agree.
    let mut direct = api.vault_info().await.unwrap();
    let mut over_grpc = grpc.vault_info().await.unwrap();
    let mut over_mcp: VaultInfo = structured(&mcp_call(&mcp, "vault_info", &serde_json::json!({})).await);
    let tiers = |info: &mut VaultInfo| info.disk.drain(..).map(|d| d.tier).collect::<Vec<_>>();
    let direct_tiers = tiers(&mut direct);
    assert_eq!(tiers(&mut over_grpc), direct_tiers);
    assert_eq!(tiers(&mut over_mcp), direct_tiers);
    assert_eq!(over_grpc, direct);
    assert_eq!(over_mcp, direct);
    assert_eq!((direct.vault_id.as_str(), direct.documents), (real::VAULT_ID, 4));
    assert!(direct.tools_available.iter().any(|t| t == "read"));
    assert!(!direct.tools_available.iter().any(|t| t == "grep"), "grep is not built yet");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tool_that_is_not_built_yet_is_not_available_over_both_interfaces() {
    let vault = real::build();
    let (api, h) = serve(&vault).await;
    let request = GrepRequest {
        patterns: vec!["magical".into()],
        scope: Scope::Vault,
        context_lines: 0,
        max_hits: None,
        case_insensitive: false,
        max_tokens: None,
    };

    let direct = api.grep(request.clone()).await.unwrap_err();
    assert!(matches!(direct, ApiError::NotYetAvailable(_)), "{direct:?}");
    match h.grpc().grep(request.clone()).await {
        Err(ClientError::Api(got)) => assert_eq!(got, direct),
        other => panic!("expected {direct:?} over gRPC, got {other:?}"),
    }
    let result = mcp_call(&h.mcp().await, "grep", &request).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.structured_content, Some(serde_json::to_value(&direct).unwrap()));
    h.server.shutdown().await.unwrap();
}

/// libraryd parses `[server]` itself and vault-core parses everything; both read the same file.
#[test]
fn a_config_valid_for_vault_core_is_valid_for_libraryd() {
    let vault = real::empty();
    let ours = libraryd::config::Config::load(&vault.config_path).expect("libraryd accepts the full SPEC config");
    let theirs = &vault.config.machine;
    assert_eq!(ours.stage(), theirs.stage.as_str());
    assert_eq!(ours.machine.as_deref(), Some(theirs.machine.as_str()));
    assert_eq!(ours.vault.root.as_deref(), Some(theirs.vault.root.as_path()));
    assert_eq!(ours.server.mcp_bind.to_string(), theirs.server.mcp_bind);
    assert_eq!(ours.server.grpc_bind.to_string(), theirs.server.grpc_bind);
    assert_eq!(ours.server.token_file, theirs.server.token_file);
    assert_eq!(ours.server.allowed_hosts, theirs.server.allowed_hosts);
}
