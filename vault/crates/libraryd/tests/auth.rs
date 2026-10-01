//! Authentication and host checks on both interfaces, `/healthz`, and that the token never reaches the logs.

mod common;

use common::*;
use vault_api::ReadRequest;
use vault_client::{ClientError, ClientOptions};

fn read_request() -> ReadRequest {
    ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L12".into()], expand: None, max_tokens: None }
}

#[tokio::test(flavor = "multi_thread")]
async fn mcp_rejects_missing_and_wrong_tokens() {
    let h = Harness::mock().await;
    let addr = h.server.mcp_addr;
    let good = format!("Bearer {}", h.token.expose());
    let wrong = format!("Bearer {}", "0".repeat(64));
    let almost = format!("Bearer {}x", h.token.expose());

    let mut headers = MCP_HEADERS.to_vec();
    let (status, head, _) = raw_http(addr, "POST", "/mcp", &headers, INITIALIZE).await;
    assert_eq!(status, 401, "no Authorization header");
    assert!(head.to_ascii_lowercase().contains("www-authenticate: bearer"));

    for bad in [wrong.as_str(), almost.as_str(), "Bearer ", "Basic abc", good.trim_start_matches("Bearer ")] {
        let mut h2 = headers.clone();
        h2.push(("Authorization", bad));
        let (status, _, body) = raw_http(addr, "POST", "/mcp", &h2, INITIALIZE).await;
        assert_eq!(status, 401, "{bad:?}");
        assert!(!body.contains(h.token.expose()));
    }

    headers.push(("Authorization", &good));
    let (status, _, body) = raw_http(addr, "POST", "/mcp", &headers, INITIALIZE).await;
    assert!(status == 200, "a valid token reaches the MCP service: {status} {body}");

    // GET and DELETE on /mcp are behind the same check.
    for method in ["GET", "DELETE"] {
        let (status, _, _) = raw_http(addr, method, "/mcp", &[("Accept", "text/event-stream")], "").await;
        assert_eq!(status, 401, "{method}");
    }
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_rmcp_client_cannot_connect_without_the_token() {
    let h = Harness::mock().await;
    let wrong = "f".repeat(64);
    let outcome = h.try_mcp(&wrong, rmcp::model::ClientConfig::default()).await;
    assert!(outcome.is_err(), "handshake with a wrong token must fail");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn grpc_rejects_missing_and_wrong_tokens() {
    let h = Harness::mock().await;
    let none = h.grpc_with(None, ClientOptions { max_retries: 0, ..Default::default() });
    let wrong = h.grpc_with(Some(&"0".repeat(64)), ClientOptions { max_retries: 0, ..Default::default() });
    for (name, client) in [("no token", none), ("wrong token", wrong)] {
        assert!(matches!(client.read(read_request()).await, Err(ClientError::Unauthenticated(_))), "{name}");
        assert!(matches!(client.vault_info().await, Err(ClientError::Unauthenticated(_))), "{name}");
        assert!(
            matches!(client.read_stream(read_request()).await, Err(ClientError::Unauthenticated(_))),
            "{name} (stream)"
        );
    }
    assert!(h.grpc().read(read_request()).await.is_ok());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn healthz_needs_no_token() {
    let h = Harness::mock().await;
    let (status, _, body) = raw_http(h.server.mcp_addr, "GET", "/healthz", &[], "").await;
    assert_eq!(status, 200);
    let json: serde_json::Value = serde_json::from_str(body.trim()).unwrap_or_else(|e| panic!("{e}: {body:?}"));
    assert_eq!(json, serde_json::json!({"ok": true, "stage": "local", "vault_id": libraryd::mock::MOCK_VAULT_ID}));
    // The health route does not open anything else.
    let (status, _, _) = raw_http(h.server.mcp_addr, "GET", "/nothing-here", &[], "").await;
    assert!(status == 404 || status == 401, "{status}");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_disallowed_host_header_is_rejected() {
    let h = Harness::mock().await;
    let auth = format!("Bearer {}", h.token.expose());
    let mut headers = MCP_HEADERS.to_vec();
    headers.push(("Authorization", &auth));

    for bad_host in ["evil.example", "evil.example:7341", "127.0.0.1.evil.example", "[::2]"] {
        let mut with_host = headers.clone();
        with_host.push(("Host", bad_host));
        let (status, _, body) = raw_http(h.server.mcp_addr, "POST", "/mcp", &with_host, INITIALIZE).await;
        assert_eq!(status, 403, "Host {bad_host:?} must be refused: {body}");
    }
    for good_host in ["127.0.0.1", "localhost", "localhost:7341"] {
        let mut with_host = headers.clone();
        with_host.push(("Host", good_host));
        let (status, _, body) = raw_http(h.server.mcp_addr, "POST", "/mcp", &with_host, INITIALIZE).await;
        assert_eq!(status, 200, "Host {good_host:?} is allowed: {body}");
    }
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn allowed_hosts_follows_the_config() {
    let h = Harness::start_with(std::sync::Arc::new(libraryd::mock::MockVault::new()), |o| {
        o.allowed_hosts = vec!["vault.lan".into()];
    })
    .await;
    let auth = format!("Bearer {}", h.token.expose());
    let mut headers = MCP_HEADERS.to_vec();
    headers.push(("Authorization", &auth));
    headers.push(("Host", "vault.lan"));
    assert_eq!(raw_http(h.server.mcp_addr, "POST", "/mcp", &headers, INITIALIZE).await.0, 200);
    headers.pop();
    headers.push(("Host", "localhost"));
    assert_eq!(raw_http(h.server.mcp_addr, "POST", "/mcp", &headers, INITIALIZE).await.0, 403);
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_token_never_appears_in_logs() {
    let logs = capture_logs();
    let h = Harness::mock().await;
    let token = h.token.expose().to_string();

    // Good and bad traffic on both interfaces.
    let mcp = h.mcp().await;
    mcp_call(&mcp, "vault_info", &serde_json::json!({})).await;
    let _ = h.try_mcp(&"e".repeat(64), rmcp::model::ClientConfig::default()).await;
    h.grpc().read(read_request()).await.unwrap();
    let _ =
        h.grpc_with(Some(&"d".repeat(64)), ClientOptions { max_retries: 0, ..Default::default() }).vault_info().await;
    let (status, _, _) =
        raw_http(h.server.mcp_addr, "POST", "/mcp", &[("Authorization", "Bearer ccc")], INITIALIZE).await;
    assert_eq!(status, 401);
    drop(mcp);
    h.server.shutdown().await.unwrap();

    let text = captured(&logs);
    assert!(text.contains("rejected"), "the rejections are logged");
    assert!(!text.contains(&token), "the token leaked into the logs");
    for wrong in ["e".repeat(64), "d".repeat(64), "ccc".to_string()] {
        assert!(!text.contains(&wrong), "a presented token leaked into the logs");
    }
}
