//! Runs a [`VaultApi`] whose methods do blocking work (SQLite queries, file reads) on tokio's blocking pool.
//!
//! `vault_core::Vault` implements the trait with `async fn`s that never wait: they query SQLite and read files
//! inline. Called straight from a request handler, each call would hold a runtime worker thread for its whole
//! duration and stall every other connection scheduled on it. [`Blocking`] moves each call to `spawn_blocking`
//! and drives the inner future to completion there.

use std::future::Future;
use std::sync::Arc;
use tokio::runtime::Handle;
use vault_api::*;

pub struct Blocking<A>(Arc<A>);

impl<A: VaultApi + 'static> Blocking<A> {
    pub fn new(api: Arc<A>) -> Self {
        Self(api)
    }

    /// Runs `call` on the blocking pool. A panic inside it, or a runtime shutting down under it, is logged and
    /// reported as `Internal` without detail: the caller learns nothing about the vault's internals.
    async fn run<T, F>(&self, call: impl FnOnce(Arc<A>) -> F + Send + 'static) -> ApiResult<T>
    where
        F: Future<Output = ApiResult<T>>,
        T: Send + 'static,
    {
        let api = self.0.clone();
        let runtime = Handle::current();
        match tokio::task::spawn_blocking(move || runtime.block_on(call(api))).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = %e, "a vault call did not finish");
                Err(ApiError::Internal(
                    "the vault could not complete this request; the server log has the details".into(),
                ))
            }
        }
    }
}

#[async_trait::async_trait]
impl<A: VaultApi + 'static> VaultApi for Blocking<A> {
    async fn search(&self, req: SearchRequest) -> ApiResult<SearchResponse> {
        self.run(move |api| async move { api.search(req).await }).await
    }

    async fn find_docs(&self, req: FindDocsRequest) -> ApiResult<FindDocsResponse> {
        self.run(move |api| async move { api.find_docs(req).await }).await
    }

    async fn workspace(&self, req: WorkspaceRequest) -> ApiResult<WorkspaceResponse> {
        self.run(move |api| async move { api.workspace(req).await }).await
    }

    async fn grep(&self, req: GrepRequest) -> ApiResult<GrepResponse> {
        self.run(move |api| async move { api.grep(req).await }).await
    }

    async fn read(&self, req: ReadRequest) -> ApiResult<ReadResponse> {
        self.run(move |api| async move { api.read(req).await }).await
    }

    async fn outline(&self, req: OutlineRequest) -> ApiResult<OutlineResponse> {
        self.run(move |api| async move { api.outline(req).await }).await
    }

    async fn citations(&self, req: CitationsRequest) -> ApiResult<CitationsResponse> {
        self.run(move |api| async move { api.citations(req).await }).await
    }

    async fn define(&self, req: DefineRequest) -> ApiResult<DefineResponse> {
        self.run(move |api| async move { api.define(req).await }).await
    }

    async fn vault_info(&self) -> ApiResult<VaultInfo> {
        self.run(|api| async move { api.vault_info().await }).await
    }
}
