//! What MCP clients receive: wrapped passages with fresh ids, structured content beside the text, and the
//! 20,000-token cap.

mod common;

use common::*;
use libraryd::mock::MockVault;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use vault_api::*;

/// (address attribute, page attribute, id, body) of every wrapper block, checking the closing tag as it goes.
fn blocks(text: &str) -> Vec<(String, Option<String>, String, String)> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<vault_passage ") {
        let after = &rest[start..];
        let open_end = after.find(">\n").expect("opening tag ends with a newline");
        let open = &after[..open_end];
        let attr =
            |name: &str| open.split(&format!(" {name}=\"")).nth(1).map(|v| v.split('"').next().unwrap().to_string());
        let id = attr("id").expect("id attribute");
        assert_eq!(id.len(), 4, "{open}");
        assert!(id.bytes().all(|b| b.is_ascii_alphanumeric()));
        let close = format!("\n</vault_passage id=\"{id}\">");
        let body_start = open_end + 2;
        let close_at = after[body_start..].find(&close).expect("the closing tag repeats the id") + body_start;
        found.push((attr("address").unwrap(), attr("page"), id, after[body_start..close_at].to_string()));
        rest = &after[close_at + close.len()..];
    }
    found
}

#[tokio::test(flavor = "multi_thread")]
async fn passages_are_wrapped_with_unique_ids_and_matching_structured_content() {
    let h = Harness::mock().await;
    let mcp = h.mcp().await;
    // Five reads of the same text in two calls: every block gets its own id.
    let request = ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L12".into(); 5], expand: None, max_tokens: None };
    let first = mcp_call(&mcp, "read", &request).await;
    let second = mcp_call(&mcp, "read", &request).await;

    let mut ids = HashSet::new();
    for result in [&first, &second] {
        let text = text_of(result);
        let found = blocks(&text);
        assert_eq!(found.len(), 5);
        for (address, page, id, body) in &found {
            assert_eq!(address, "3f9a1c07b2e4d55e-cowan-magical-number-four#L12");
            assert_eq!(page.as_deref(), Some("87"));
            assert_eq!(body, "The capacity limit of working memory is about four chunks.");
            ids.insert(id.clone());
        }
        // The structured content is the same data, not a different rendering.
        let typed: ReadResponse = structured(result);
        assert_eq!(typed.passages.len(), 5);
        assert!(text.contains("source: The magical number 4 in short-term memory | section: 1 Introduction"));
    }
    assert!(ids.len() >= 9, "ids are random per block, got {} distinct of 10", ids.len());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn every_passage_bearing_tool_wraps_its_text() {
    let h = Harness::mock().await;
    let mcp = h.mcp().await;

    let grep = mcp_call(
        &mcp,
        "grep",
        &GrepRequest {
            patterns: vec!["magical number".into()],
            scope: Scope::Vault,
            context_lines: 1,
            max_hits: None,
            case_insensitive: true,
            max_tokens: None,
        },
    )
    .await;
    let found = blocks(&text_of(&grep));
    assert!(!found.is_empty());
    // A grep block holds the context lines around the match, and its address is the matching line's.
    assert!(found.iter().any(|(a, _, _, body)| a.ends_with("#L30") && body.lines().count() == 3));

    let search = mcp_call(
        &mcp,
        "search",
        &SearchRequest {
            queries: vec!["working memory capacity".into()],
            filters: Filters::default(),
            k: Some(3),
            max_tokens: None,
        },
    )
    .await;
    assert!(!blocks(&text_of(&search)).is_empty());

    let define = mcp_call(
        &mcp,
        "define",
        &DefineRequest { terms: vec!["chunking".into()], filters: Filters::default(), k: None, max_tokens: None },
    )
    .await;
    let found = blocks(&text_of(&define));
    assert_eq!(found.len(), 1);
    assert!(found[0].3.starts_with("Chunking:"));

    // Tools without passage text still answer in text, and carry structured content.
    let info = mcp_call(&mcp, "vault_info", &serde_json::json!({})).await;
    assert!(text_of(&info).contains("vault mock-vault-0001"));
    assert!(info.structured_content.is_some());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_20k_token_cap_truncates_a_misbehaving_response() {
    // 100 passages of 20,000 characters each: 2,000,000 characters, 25 times the cap, ignoring max_tokens.
    let greedy = Scripted { huge_read: Some((100, 20_000)), ..Scripted::default() };
    let h = Harness::start(Arc::new(greedy)).await;
    let mcp = h.mcp().await;
    let request = ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L1".into()], expand: None, max_tokens: Some(1_000) };

    let result = mcp_call(&mcp, "read", &request).await;
    assert_ne!(result.is_error, Some(true));
    let text = text_of(&result);
    let structured_json = result.structured_content.clone().expect("structured content").to_string();
    let total_chars = text.chars().count() + structured_json.chars().count();
    assert!(total_chars <= 80_000, "{total_chars} characters is over 20,000 tokens");
    assert!(total_chars > 40_000, "the cap should keep as much as fits, kept only {total_chars}");

    let typed: ReadResponse = structured(&result);
    let kept = typed.passages.len();
    assert!((1..100).contains(&kept), "kept {kept}");
    assert_eq!(blocks(&text).len(), kept, "text and structured content keep the same passages");
    let notice = typed.truncated.expect("the response says it was truncated");
    assert!(
        notice.reason.contains("20000 tokens") && notice.reason.contains(&format!("kept {kept} of 100")),
        "{notice:?}"
    );
    assert!(notice.next.contains("fewer"), "{notice:?}");
    assert!(text.contains("[truncated: MCP response cap of 20000 tokens"), "the model sees the note too");
    assert!(result.meta.is_some(), "the cap is also recorded in _meta");

    // gRPC has no such cap: the same call returns everything the API produced.
    let all = h.grpc().read(request).await.unwrap();
    assert_eq!(all.passages.len(), 100);
    assert!(all.truncated.is_none());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn responses_under_the_cap_are_untouched() {
    let h = Harness::start(Arc::new(MockVault::new())).await;
    let mcp = h.mcp().await;
    let request =
        ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L1-L60".into()], expand: None, max_tokens: Some(8_000) };
    let result = mcp_call(&mcp, "read", &request).await;
    assert!(result.meta.is_none());
    let typed: ReadResponse = structured(&result);
    assert_eq!(typed, h.grpc().read(request).await.unwrap());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_small_configured_cap_applies_to_every_tool() {
    let h = Harness::start_with(Arc::new(MockVault::new()), |o| o.mcp_max_chars = Some(1_500)).await;
    let mcp = h.mcp().await;
    let result = mcp_call(
        &mcp,
        "grep",
        &GrepRequest {
            patterns: vec!["line".into()],
            scope: Scope::Vault,
            context_lines: 0,
            max_hits: Some(50),
            case_insensitive: false,
            max_tokens: None,
        },
    )
    .await;
    let total = text_of(&result).chars().count() + result.structured_content.unwrap().to_string().chars().count();
    assert!(total <= 1_500, "{total}");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_does_not_wait_for_open_mcp_sessions() {
    // An MCP client keeps a session (and an event stream) open; shutting down must still be quick.
    let h = Harness::mock().await;
    let mcp = h.mcp().await;
    mcp_call(&mcp, "vault_info", &serde_json::json!({})).await;
    let started = Instant::now();
    tokio::time::timeout(Duration::from_secs(5), h.server.shutdown()).await.expect("shutdown returns").unwrap();
    assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
    drop(mcp);
}
