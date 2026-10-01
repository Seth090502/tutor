//! Interface parity: the same request through MCP and through gRPC yields the same structured result, which is
//! also exactly what the VaultApi returns when called directly.

mod common;

use common::*;
use libraryd::mock::{CODE, COWAN, MockVault, TEXTBOOK};
use rmcp::model::{ClientConfig, ProtocolVersion};
use std::sync::Arc;
use vault_api::*;
use vault_client::ClientError;

fn same_as<T: serde::de::DeserializeOwned>(_witness: &T, result: &rmcp::model::CallToolResult) -> T {
    structured(result)
}

macro_rules! parity {
    ($api:expr, $grpc:expr, $mcp:expr, $tool:literal, $method:ident, $req:expr) => {{
        let req = $req;
        let direct = $api.$method(req.clone()).await.expect(concat!($tool, " directly"));
        let over_grpc = $grpc.$method(req.clone()).await.expect(concat!($tool, " over gRPC"));
        let result = mcp_call(&$mcp, $tool, &req).await;
        let over_mcp = same_as(&direct, &result);
        assert_eq!(over_grpc, direct, "{} over gRPC", $tool);
        assert_eq!(over_mcp, direct, "{} over MCP", $tool);
        direct
    }};
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn every_tool_returns_identical_structured_results_over_mcp_and_grpc() {
    let api = Arc::new(MockVault::new());
    let h = Harness::start(api.clone()).await;
    let (grpc, mcp) = (h.grpc(), h.mcp().await);

    let info = {
        let direct = api.vault_info().await.unwrap();
        assert_eq!(grpc.vault_info().await.unwrap(), direct);
        let result = mcp_call(&mcp, "vault_info", &serde_json::json!({})).await;
        assert_eq!(same_as(&direct, &result), direct);
        direct
    };
    assert_eq!(info.tools_available.len(), 9);

    let found = parity!(
        api,
        grpc,
        mcp,
        "find_docs",
        find_docs,
        FindDocsRequest {
            queries: s(&["cowan magical number", "textbook memory", "nothing matches this"]),
            filters: Filters { category: Some("psych-cogsci".into()), ..Filters::default() },
            k: Some(5),
            max_tokens: None,
        }
    );
    assert_eq!(found.results.len(), 3);
    assert!(found.results[2].is_empty());

    let searched = parity!(
        api,
        grpc,
        mcp,
        "search",
        search,
        SearchRequest {
            queries: s(&["capacity of working memory", "chunking"]),
            filters: Filters::default(),
            k: Some(4),
            max_tokens: Some(2_000),
        }
    );
    assert!(!searched.results[0].is_empty());

    let added = parity!(
        api,
        grpc,
        mcp,
        "workspace",
        workspace,
        WorkspaceRequest {
            action: WorkspaceAction::Add,
            doc_ids: s(&[COWAN, TEXTBOOK, "not-a-doc"]),
            workspace: Some("parity".into()),
        }
    );
    assert_eq!((added.docs.len(), added.errors.len()), (2, 1));
    parity!(
        api,
        grpc,
        mcp,
        "workspace",
        workspace,
        WorkspaceRequest { action: WorkspaceAction::List, doc_ids: vec![], workspace: Some("parity".into()) }
    );

    let grepped = parity!(
        api,
        grpc,
        mcp,
        "grep",
        grep,
        GrepRequest {
            patterns: s(&["magical number (seven|four)", "(unclosed", "fn \\w+"]),
            scope: Scope::Vault,
            context_lines: 1,
            max_hits: Some(20),
            case_insensitive: true,
            max_tokens: None,
        }
    );
    assert!(grepped.hits.iter().any(|h| h.pattern == 2), "the code file matches fn");
    assert_eq!(grepped.errors.len(), 1);
    parity!(
        api,
        grpc,
        mcp,
        "grep",
        grep,
        GrepRequest {
            patterns: s(&["four"]),
            scope: Scope::Workspace { workspace: Some("parity".into()) },
            context_lines: 0,
            max_hits: None,
            case_insensitive: false,
            max_tokens: None,
        }
    );

    let read = parity!(
        api,
        grpc,
        mcp,
        "read",
        read,
        ReadRequest {
            addresses: s(&["3f9a1c07b2e4d55e#L30", "9c1d2e3f4a5b6c7d#L5-L6", "3f9a1c07b2e4d55e#L999", "garbage"]),
            expand: Some(Expand::Section),
            max_tokens: None,
        }
    );
    assert_eq!((read.passages.len(), read.errors.len()), (2, 2));
    parity!(
        api,
        grpc,
        mcp,
        "read",
        read,
        ReadRequest {
            addresses: s(&["3f9a1c07b2e4d55e#L30"]),
            expand: Some(Expand::Lines { before: 2, after: 1 }),
            max_tokens: Some(100),
        }
    );
    let truncated = parity!(
        api,
        grpc,
        mcp,
        "read",
        read,
        ReadRequest {
            addresses: s(&["3f9a1c07b2e4d55e#L1-L60", "9c1d2e3f4a5b6c7d#L1-L80"]),
            expand: None,
            max_tokens: Some(700),
        }
    );
    assert!(truncated.truncated.is_some(), "the tool's own truncation notice crosses both interfaces");

    let outlines = parity!(
        api,
        grpc,
        mcp,
        "outline",
        outline,
        OutlineRequest { doc_ids: s(&[COWAN, CODE, "bad"]), max_depth: Some(2), max_tokens: None }
    );
    assert_eq!((outlines.outlines.len(), outlines.errors.len()), (2, 1));

    let cites = parity!(
        api,
        grpc,
        mcp,
        "citations",
        citations,
        CitationsRequest { doc_ids: s(&[COWAN]), direction: Direction::Both, k: None, max_tokens: None }
    );
    assert!(cites.edges.iter().any(|e| e.external));
    parity!(
        api,
        grpc,
        mcp,
        "citations",
        citations,
        CitationsRequest { doc_ids: s(&[TEXTBOOK]), direction: Direction::CitedBy, k: Some(1), max_tokens: None }
    );

    let defined = parity!(
        api,
        grpc,
        mcp,
        "define",
        define,
        DefineRequest {
            terms: s(&["working memory", "chunking", "unobtainium"]),
            filters: Filters::default(),
            k: None,
            max_tokens: None,
        }
    );
    assert_eq!((defined.definitions.len(), defined.errors.len()), (2, 1));

    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_errors_are_identical_over_both_interfaces() {
    for error in [
        ApiError::InvalidArgument("queries must not be empty".into()),
        ApiError::NotFound("no such document".into()),
        ApiError::NotYetAvailable("search arrives in phase 3".into()),
        ApiError::Unavailable("the index is warming up".into()),
        ApiError::Internal("disk read failed".into()),
    ] {
        let h = Harness::start(Arc::new(Scripted::failing(error.clone(), usize::MAX))).await;
        let request = ReadRequest { addresses: s(&["3f9a1c07b2e4d55e#L1"]), expand: None, max_tokens: None };

        let result = mcp_call(&h.mcp().await, "read", &request).await;
        assert_eq!(result.is_error, Some(true), "{error:?}");
        assert_eq!(result.structured_content, Some(serde_json::to_value(&error).unwrap()), "{error:?}");
        assert!(text_of(&result).contains(&error.to_string()), "the model reads the message: {}", text_of(&result));

        // No retries here: Unavailable is retryable, and the counter is not what this test is about.
        let client =
            h.grpc_with(Some(h.token.expose()), vault_client::ClientOptions { max_retries: 0, ..Default::default() });
        match client.read(request).await {
            Err(ClientError::Api(got)) => assert_eq!(got, error),
            other => panic!("expected {error:?}, got {other:?}"),
        }
        h.server.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_arguments_and_unknown_tools_over_mcp() {
    let h = Harness::mock().await;
    let mcp = h.mcp().await;

    let wrong_type = mcp_call(&mcp, "read", &serde_json::json!({"addresses": 5})).await;
    assert_eq!(wrong_type.is_error, Some(true));
    let kind = wrong_type.structured_content.as_ref().and_then(|v| v["kind"].as_str());
    assert_eq!(kind, Some("invalid_argument"));
    assert!(text_of(&wrong_type).contains("read"), "{}", text_of(&wrong_type));

    let missing = mcp_call(&mcp, "grep", &serde_json::json!({"patterns": ["x"]})).await;
    assert_eq!(missing.is_error, Some(true), "scope is required");

    let unknown = mcp.call_tool(rmcp::model::CallToolRequestParams::new("delete_everything")).await;
    assert!(unknown.is_err(), "an unknown tool is a protocol error: {unknown:?}");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_tool_list_is_the_contract() {
    let h = Harness::mock().await;
    let mcp = h.mcp().await;
    let listed = mcp.list_all_tools().await.unwrap();
    let names: Vec<&str> = listed.iter().map(|t| t.name.as_ref()).collect();
    assert_eq!(names, TOOL_NAMES);
    let defined = libraryd::tools::definitions().unwrap();
    for (got, want) in listed.iter().zip(&defined) {
        assert_eq!(got.description, want.description, "{}", got.name);
        assert_eq!(got.input_schema, want.input_schema, "{}", got.name);
        assert_eq!(got.output_schema, want.output_schema, "{}", got.name);
    }
    let info = mcp.peer_info().expect("server info after the handshake");
    assert!(info.instructions.as_deref().unwrap_or("").contains("vault_passage"));
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_on_the_previous_protocol_version_works_too() {
    let h = Harness::mock().await;
    let legacy = ClientConfig::default().with_protocol_version(ProtocolVersion::V_2025_11_25);
    let mcp = h.try_mcp(h.token.expose(), legacy).await.expect("legacy handshake");
    let request = ReadRequest { addresses: s(&["3f9a1c07b2e4d55e#L12"]), expand: None, max_tokens: None };
    let result = mcp_call(&mcp, "read", &request).await;
    let got: ReadResponse = structured(&result);
    assert_eq!(got, h.grpc().read(request).await.unwrap());
    h.server.shutdown().await.unwrap();
}
