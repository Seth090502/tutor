//! The gRPC face of a [`VaultApi`]: unary calls for the nine tools and server-streaming variants of the six
//! batched ones. See `proto/vault.proto`.

use crate::timed;
use futures::future::BoxFuture;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use vault_api as api;
use vault_api::VaultApi;
use vault_client::batch::Chunked;
use vault_client::convert::{ConvertError, status_from_api_error};
use vault_client::proto as pb;
use vault_client::proto::vault_server::{Vault, VaultServer};

/// Chunks of one streaming call that run at the same time.
pub const STREAM_CONCURRENCY: usize = 8;

/// Largest request message the server accepts.
pub const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

/// How a streaming call runs one chunk of its batch.
type ChunkCall<A> = fn(Arc<dyn VaultApi>, A) -> BoxFuture<'static, api::ApiResult<<A as Chunked>::Response>>;

#[derive(Clone)]
pub struct GrpcVault {
    api: Arc<dyn VaultApi>,
    timeout: Duration,
    stop: CancellationToken,
}

impl GrpcVault {
    pub fn new(api: Arc<dyn VaultApi>, timeout: Duration) -> Self {
        Self { api, timeout, stop: CancellationToken::new() }
    }

    /// Calls still running when `stop` is cancelled end with `Unavailable`.
    pub fn with_stop(mut self, stop: CancellationToken) -> Self {
        self.stop = stop;
        self
    }

    /// The tonic service with compression and size limits set. Wrap it in the auth layer before serving.
    pub fn into_service(self) -> VaultServer<Self> {
        use tonic::codec::CompressionEncoding::{Gzip, Zstd};
        VaultServer::new(self)
            .accept_compressed(Zstd)
            .accept_compressed(Gzip)
            .send_compressed(Zstd)
            .send_compressed(Gzip)
            .max_decoding_message_size(MAX_REQUEST_BYTES)
    }

    async fn unary<Q, A, R, P, Fut>(
        &self,
        name: &'static str,
        request: Request<Q>,
        call: impl FnOnce(Arc<dyn VaultApi>, A) -> Fut,
    ) -> Result<Response<P>, Status>
    where
        A: TryFrom<Q, Error = ConvertError>,
        P: From<R>,
        Fut: std::future::Future<Output = api::ApiResult<R>>,
    {
        let started = Instant::now();
        let request = A::try_from(request.into_inner())?;
        let outcome = timed(self.timeout, &self.stop, call(self.api.clone(), request)).await;
        tracing::info!(tool = name, ms = started.elapsed().as_millis() as u64, error = outcome.is_err(), "grpc call");
        outcome.map(|r| Response::new(P::from(r))).map_err(|e| status_from_api_error(&e))
    }

    /// Splits the batch into one-item requests, runs up to [`STREAM_CONCURRENCY`] at once, and streams each
    /// response in request order with its item indexes pointing into the original request.
    fn streamed<Q, A, P>(
        &self,
        name: &'static str,
        request: Request<Q>,
        call: ChunkCall<A>,
    ) -> Result<Response<ResponseStream<P>>, Status>
    where
        A: Chunked + TryFrom<Q, Error = ConvertError> + Send + 'static,
        A::Response: Send + 'static,
        P: From<A::Response> + Send + 'static,
    {
        let request = A::try_from(request.into_inner())?;
        let chunks = request.split();
        tracing::info!(tool = name, chunks = chunks.len(), "grpc stream started");
        let (api, timeout, stop) = (self.api.clone(), self.timeout, self.stop.clone());
        let stream = futures::stream::iter(chunks.into_iter().enumerate())
            .map(move |(index, chunk)| {
                let (api, stop) = (api.clone(), stop.clone());
                async move {
                    let mut response =
                        timed(timeout, &stop, call(api, chunk)).await.map_err(|e| status_from_api_error(&e))?;
                    A::rebase(&mut response, u32::try_from(index).unwrap_or(u32::MAX));
                    Ok(P::from(response))
                }
            })
            .buffered(STREAM_CONCURRENCY);
        Ok(Response::new(Box::pin(stream)))
    }
}

#[tonic::async_trait]
impl Vault for GrpcVault {
    async fn search(&self, request: Request<pb::SearchRequest>) -> Result<Response<pb::SearchResponse>, Status> {
        self.unary("search", request, |a, q: api::SearchRequest| async move { a.search(q).await }).await
    }

    async fn find_docs(&self, request: Request<pb::FindDocsRequest>) -> Result<Response<pb::FindDocsResponse>, Status> {
        self.unary("find_docs", request, |a, q: api::FindDocsRequest| async move { a.find_docs(q).await }).await
    }

    async fn workspace(
        &self,
        request: Request<pb::WorkspaceRequest>,
    ) -> Result<Response<pb::WorkspaceResponse>, Status> {
        self.unary("workspace", request, |a, q: api::WorkspaceRequest| async move { a.workspace(q).await }).await
    }

    async fn grep(&self, request: Request<pb::GrepRequest>) -> Result<Response<pb::GrepResponse>, Status> {
        self.unary("grep", request, |a, q: api::GrepRequest| async move { a.grep(q).await }).await
    }

    async fn read(&self, request: Request<pb::ReadRequest>) -> Result<Response<pb::ReadResponse>, Status> {
        self.unary("read", request, |a, q: api::ReadRequest| async move { a.read(q).await }).await
    }

    async fn outline(&self, request: Request<pb::OutlineRequest>) -> Result<Response<pb::OutlineResponse>, Status> {
        self.unary("outline", request, |a, q: api::OutlineRequest| async move { a.outline(q).await }).await
    }

    async fn citations(
        &self,
        request: Request<pb::CitationsRequest>,
    ) -> Result<Response<pb::CitationsResponse>, Status> {
        self.unary("citations", request, |a, q: api::CitationsRequest| async move { a.citations(q).await }).await
    }

    async fn define(&self, request: Request<pb::DefineRequest>) -> Result<Response<pb::DefineResponse>, Status> {
        self.unary("define", request, |a, q: api::DefineRequest| async move { a.define(q).await }).await
    }

    async fn vault_info(&self, request: Request<pb::VaultInfoRequest>) -> Result<Response<pb::VaultInfo>, Status> {
        struct NoArgs;
        impl TryFrom<pb::VaultInfoRequest> for NoArgs {
            type Error = ConvertError;
            fn try_from(_: pb::VaultInfoRequest) -> Result<Self, ConvertError> {
                Ok(NoArgs)
            }
        }
        self.unary("vault_info", request, |a, _: NoArgs| async move { a.vault_info().await }).await
    }

    type ReadStreamStream = ResponseStream<pb::ReadResponse>;
    async fn read_stream(&self, request: Request<pb::ReadRequest>) -> Result<Response<Self::ReadStreamStream>, Status> {
        self.streamed("read_stream", request, |a, q: api::ReadRequest| Box::pin(async move { a.read(q).await }))
    }

    type OutlineStreamStream = ResponseStream<pb::OutlineResponse>;
    async fn outline_stream(
        &self,
        request: Request<pb::OutlineRequest>,
    ) -> Result<Response<Self::OutlineStreamStream>, Status> {
        self.streamed("outline_stream", request, |a, q: api::OutlineRequest| {
            Box::pin(async move { a.outline(q).await })
        })
    }

    type GrepStreamStream = ResponseStream<pb::GrepResponse>;
    async fn grep_stream(&self, request: Request<pb::GrepRequest>) -> Result<Response<Self::GrepStreamStream>, Status> {
        self.streamed("grep_stream", request, |a, q: api::GrepRequest| Box::pin(async move { a.grep(q).await }))
    }

    type FindDocsStreamStream = ResponseStream<pb::FindDocsResponse>;
    async fn find_docs_stream(
        &self,
        request: Request<pb::FindDocsRequest>,
    ) -> Result<Response<Self::FindDocsStreamStream>, Status> {
        self.streamed("find_docs_stream", request, |a, q: api::FindDocsRequest| {
            Box::pin(async move { a.find_docs(q).await })
        })
    }

    type SearchStreamStream = ResponseStream<pb::SearchResponse>;
    async fn search_stream(
        &self,
        request: Request<pb::SearchRequest>,
    ) -> Result<Response<Self::SearchStreamStream>, Status> {
        self.streamed("search_stream", request, |a, q: api::SearchRequest| Box::pin(async move { a.search(q).await }))
    }

    type DefineStreamStream = ResponseStream<pb::DefineResponse>;
    async fn define_stream(
        &self,
        request: Request<pb::DefineRequest>,
    ) -> Result<Response<Self::DefineStreamStream>, Status> {
        self.streamed("define_stream", request, |a, q: api::DefineRequest| Box::pin(async move { a.define(q).await }))
    }
}
