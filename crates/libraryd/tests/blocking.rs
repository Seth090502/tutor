//! The blocking adapter: a call that blocks its thread must not stall the runtime, and a call that panics must
//! not leak its message.

use libraryd::blocking::Blocking;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use vault_api::*;

/// `vault_info` blocks its thread the way vault-core's SQLite and file reads do, and `read` panics.
struct Probe;

fn later<T>() -> ApiResult<T> {
    Err(ApiError::NotYetAvailable("later".into()))
}

#[async_trait::async_trait]
impl VaultApi for Probe {
    async fn search(&self, _: SearchRequest) -> ApiResult<SearchResponse> {
        later()
    }
    async fn find_docs(&self, _: FindDocsRequest) -> ApiResult<FindDocsResponse> {
        later()
    }
    async fn workspace(&self, _: WorkspaceRequest) -> ApiResult<WorkspaceResponse> {
        later()
    }
    async fn grep(&self, _: GrepRequest) -> ApiResult<GrepResponse> {
        later()
    }
    async fn read(&self, _: ReadRequest) -> ApiResult<ReadResponse> {
        panic!("secret detail: C:\\vault\\hot\\db\\vault.sqlite");
    }
    async fn outline(&self, _: OutlineRequest) -> ApiResult<OutlineResponse> {
        later()
    }
    async fn citations(&self, _: CitationsRequest) -> ApiResult<CitationsResponse> {
        later()
    }
    async fn define(&self, _: DefineRequest) -> ApiResult<DefineResponse> {
        later()
    }
    async fn vault_info(&self) -> ApiResult<VaultInfo> {
        std::thread::sleep(Duration::from_millis(300));
        Ok(VaultInfo {
            vault_id: "probe".into(),
            format_version: 1,
            stage: "local".into(),
            documents: 0,
            by_kind: vec![],
            quarantined: 0,
            repositories: 0,
            coverage: vec![],
            taxonomy_version: String::new(),
            index_version: String::new(),
            tools_available: vec![],
            disk: vec![],
        })
    }
}

/// One runtime thread, so a call that held it would freeze the ticker for the whole 300 ms.
#[tokio::test(flavor = "current_thread")]
async fn a_blocking_call_does_not_stall_the_runtime() {
    let ticks = Arc::new(AtomicUsize::new(0));
    let ticker = tokio::spawn({
        let ticks = ticks.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    let info = Blocking::new(Arc::new(Probe)).vault_info().await.unwrap();
    ticker.abort();
    assert_eq!(info.vault_id, "probe");
    let ticks = ticks.load(Ordering::SeqCst);
    assert!(ticks >= 5, "the runtime made only {ticks} ticks while the call blocked");
}

#[tokio::test(flavor = "multi_thread")]
async fn errors_pass_through_and_a_panic_becomes_a_generic_internal_error() {
    let api = Blocking::new(Arc::new(Probe));
    let grep = GrepRequest {
        patterns: vec!["x".into()],
        scope: Scope::Vault,
        context_lines: 0,
        max_hits: None,
        case_insensitive: false,
        max_tokens: None,
    };
    assert_eq!(api.grep(grep).await.unwrap_err(), ApiError::NotYetAvailable("later".into()));

    let read = ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L1".into()], expand: None, max_tokens: None };
    match api.read(read).await {
        Err(ApiError::Internal(message)) => assert!(!message.contains("secret"), "{message}"),
        other => panic!("expected Internal, got {other:?}"),
    }
}
