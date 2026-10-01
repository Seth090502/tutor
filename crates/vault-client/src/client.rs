//! The async gRPC client: one multiplexed HTTP/2 connection shared by every call, per-call deadlines, limited
//! retries for idempotent calls, and message compression.

use crate::batch::Chunked;
use crate::convert::{ConvertError, api_error_from_status};
use crate::proto as pb;
use crate::proto::vault_client::VaultClient as Grpc;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use std::fmt;
use std::time::Duration;
use tonic::codec::CompressionEncoding;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Request, Status, Streaming};
use vault_api as api;

/// Which compression the client asks the server to use for responses, and uses for its own requests.
/// The client always accepts both encodings; this picks the preferred one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Zstd,
}

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub connect_timeout: Duration,
    /// Deadline for one unary call, sent to the server as `grpc-timeout`.
    pub request_timeout: Duration,
    /// Deadline for a whole streaming call.
    pub stream_timeout: Duration,
    /// Extra attempts after the first for calls that are safe to repeat.
    pub max_retries: u32,
    /// Delay before the first retry; it doubles for each further retry, up to two seconds.
    pub retry_backoff: Duration,
    pub compression: Compression,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(60),
            stream_timeout: Duration::from_secs(300),
            max_retries: 2,
            retry_backoff: Duration::from_millis(50),
            compression: Compression::Zstd,
        }
    }
}

/// A bearer token. It never appears in `Debug` output or logs.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("invalid endpoint {endpoint:?}: {message}")]
    Endpoint { endpoint: String, message: String },
    #[error("cannot connect to {endpoint}: {message}")]
    Connect { endpoint: String, message: String },
    #[error("the server rejected the token: {0}")]
    Unauthenticated(String),
    #[error(transparent)]
    Api(#[from] api::ApiError),
    #[error("the server sent a message that is not valid vault data: {0}")]
    Protocol(#[from] ConvertError),
}

impl From<ClientError> for api::ApiError {
    fn from(e: ClientError) -> Self {
        match e {
            ClientError::Api(e) => e,
            ClientError::Protocol(e) => api::ApiError::Internal(e.to_string()),
            e @ (ClientError::Endpoint { .. } | ClientError::Connect { .. } | ClientError::Unauthenticated(_)) => {
                api::ApiError::Unavailable(e.to_string())
            }
        }
    }
}

fn client_error(status: Status) -> ClientError {
    match status.code() {
        Code::Unauthenticated | Code::PermissionDenied => ClientError::Unauthenticated(status.message().to_string()),
        _ => ClientError::Api(api_error_from_status(&status)),
    }
}

/// Only an unavailable server or an aborted call is worth repeating; the other codes would fail the same way.
fn retryable(status: &Status) -> bool {
    matches!(status.code(), Code::Unavailable | Code::Aborted)
}

/// A handle to libraryd's gRPC API. Cloning is cheap and clones share one connection.
#[derive(Clone)]
pub struct VaultClient {
    channel: Channel,
    token: Option<Secret>,
    options: ClientOptions,
}

impl fmt::Debug for VaultClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultClient").field("token", &self.token).field("options", &self.options).finish()
    }
}

type Resp<T> = Result<tonic::Response<T>, Status>;

impl VaultClient {
    fn endpoint(endpoint: &str, options: &ClientOptions) -> Result<Endpoint, ClientError> {
        Endpoint::from_shared(endpoint.to_string())
            .map(|e| {
                e.connect_timeout(options.connect_timeout)
                    .tcp_nodelay(true)
                    .http2_keep_alive_interval(Duration::from_secs(30))
                    .keep_alive_while_idle(true)
            })
            .map_err(|e| ClientError::Endpoint { endpoint: endpoint.to_string(), message: e.to_string() })
    }

    /// Connects now and fails if the server cannot be reached. `endpoint` looks like `http://127.0.0.1:7342`.
    pub async fn connect(endpoint: &str, token: Option<Secret>, options: ClientOptions) -> Result<Self, ClientError> {
        let channel = Self::endpoint(endpoint, &options)?
            .connect()
            .await
            .map_err(|e| ClientError::Connect { endpoint: endpoint.to_string(), message: e.to_string() })?;
        Ok(Self { channel, token, options })
    }

    /// Connects on the first call, and reconnects by itself whenever the connection drops.
    pub fn connect_lazy(endpoint: &str, token: Option<Secret>, options: ClientOptions) -> Result<Self, ClientError> {
        let channel = Self::endpoint(endpoint, &options)?.connect_lazy();
        Ok(Self { channel, token, options })
    }

    fn grpc(&self) -> Grpc<Channel> {
        let mut grpc = Grpc::new(self.channel.clone());
        grpc = grpc.accept_compressed(CompressionEncoding::Zstd).accept_compressed(CompressionEncoding::Gzip);
        match self.options.compression {
            Compression::None => grpc,
            Compression::Gzip => grpc.send_compressed(CompressionEncoding::Gzip),
            Compression::Zstd => grpc.send_compressed(CompressionEncoding::Zstd),
        }
    }

    fn request<T>(&self, message: T, timeout: Duration) -> Request<T> {
        let mut request = Request::new(message);
        request.set_timeout(timeout);
        if let Some(Secret(token)) = &self.token {
            // A token with characters that are not valid in a header is simply not sent; the server then
            // answers `unauthenticated`, which names the problem.
            if let Ok(mut value) = MetadataValue::try_from(format!("Bearer {token}")) {
                value.set_sensitive(true);
                request.metadata_mut().insert("authorization", value);
            }
        }
        request
    }

    async fn retrying<M: Clone, R>(
        &self,
        name: &'static str,
        idempotent: bool,
        timeout: Duration,
        message: M,
        call: impl Fn(Grpc<Channel>, Request<M>) -> BoxFuture<'static, Resp<R>>,
    ) -> Result<R, ClientError> {
        let attempts = if idempotent { self.options.max_retries } else { 0 };
        let mut delay = self.options.retry_backoff;
        let mut attempt = 0;
        loop {
            match call(self.grpc(), self.request(message.clone(), timeout)).await {
                Ok(response) => return Ok(response.into_inner()),
                Err(status) if attempt < attempts && retryable(&status) => {
                    attempt += 1;
                    tracing::warn!(call = name, attempt, code = ?status.code(), "retrying after a transient failure");
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(2));
                }
                Err(status) => return Err(client_error(status)),
            }
        }
    }

    async fn unary<M: Clone, R>(
        &self,
        name: &'static str,
        idempotent: bool,
        message: M,
        call: impl Fn(Grpc<Channel>, Request<M>) -> BoxFuture<'static, Resp<R>>,
    ) -> Result<R, ClientError> {
        self.retrying(name, idempotent, self.options.request_timeout, message, call).await
    }

    async fn streaming<M: Clone, R, T>(
        &self,
        name: &'static str,
        message: M,
        call: impl Fn(Grpc<Channel>, Request<M>) -> BoxFuture<'static, Resp<Streaming<R>>>,
    ) -> Result<BoxStream<'static, Result<T, ClientError>>, ClientError>
    where
        R: Send + 'static,
        T: TryFrom<R, Error = ConvertError> + Send + 'static,
    {
        let stream = self.retrying(name, true, self.options.stream_timeout, message, call).await?;
        Ok(stream.map_err(client_error).and_then(|m| async move { T::try_from(m).map_err(ClientError::from) }).boxed())
    }

    pub async fn search(&self, req: api::SearchRequest) -> Result<api::SearchResponse, ClientError> {
        let m = pb::SearchRequest::from(req);
        let r = self.unary("search", true, m, |mut c, r| Box::pin(async move { c.search(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn find_docs(&self, req: api::FindDocsRequest) -> Result<api::FindDocsResponse, ClientError> {
        let m = pb::FindDocsRequest::from(req);
        let r = self.unary("find_docs", true, m, |mut c, r| Box::pin(async move { c.find_docs(r).await })).await?;
        Ok(r.try_into()?)
    }

    /// `add` and `remove` are retried too: both are set operations, so repeating one changes nothing.
    pub async fn workspace(&self, req: api::WorkspaceRequest) -> Result<api::WorkspaceResponse, ClientError> {
        let m = pb::WorkspaceRequest::from(req);
        let r = self.unary("workspace", true, m, |mut c, r| Box::pin(async move { c.workspace(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn grep(&self, req: api::GrepRequest) -> Result<api::GrepResponse, ClientError> {
        let m = pb::GrepRequest::from(req);
        let r = self.unary("grep", true, m, |mut c, r| Box::pin(async move { c.grep(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn read(&self, req: api::ReadRequest) -> Result<api::ReadResponse, ClientError> {
        let m = pb::ReadRequest::from(req);
        let r = self.unary("read", true, m, |mut c, r| Box::pin(async move { c.read(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn outline(&self, req: api::OutlineRequest) -> Result<api::OutlineResponse, ClientError> {
        let m = pb::OutlineRequest::from(req);
        let r = self.unary("outline", true, m, |mut c, r| Box::pin(async move { c.outline(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn citations(&self, req: api::CitationsRequest) -> Result<api::CitationsResponse, ClientError> {
        let m = pb::CitationsRequest::from(req);
        let r = self.unary("citations", true, m, |mut c, r| Box::pin(async move { c.citations(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn define(&self, req: api::DefineRequest) -> Result<api::DefineResponse, ClientError> {
        let m = pb::DefineRequest::from(req);
        let r = self.unary("define", true, m, |mut c, r| Box::pin(async move { c.define(r).await })).await?;
        Ok(r.try_into()?)
    }

    pub async fn vault_info(&self) -> Result<api::VaultInfo, ClientError> {
        let m = pb::VaultInfoRequest {};
        let r = self.unary("vault_info", true, m, |mut c, r| Box::pin(async move { c.vault_info(r).await })).await?;
        Ok(r.try_into()?)
    }

    /// One response per address, in request order, as the server finishes them. See `proto/vault.proto`.
    pub async fn read_stream(
        &self,
        req: api::ReadRequest,
    ) -> Result<BoxStream<'static, Result<api::ReadResponse, ClientError>>, ClientError> {
        let m = pb::ReadRequest::from(req);
        self.streaming("read_stream", m, |mut c, r| Box::pin(async move { c.read_stream(r).await })).await
    }

    pub async fn outline_stream(
        &self,
        req: api::OutlineRequest,
    ) -> Result<BoxStream<'static, Result<api::OutlineResponse, ClientError>>, ClientError> {
        let m = pb::OutlineRequest::from(req);
        self.streaming("outline_stream", m, |mut c, r| Box::pin(async move { c.outline_stream(r).await })).await
    }

    pub async fn grep_stream(
        &self,
        req: api::GrepRequest,
    ) -> Result<BoxStream<'static, Result<api::GrepResponse, ClientError>>, ClientError> {
        let m = pb::GrepRequest::from(req);
        self.streaming("grep_stream", m, |mut c, r| Box::pin(async move { c.grep_stream(r).await })).await
    }

    pub async fn find_docs_stream(
        &self,
        req: api::FindDocsRequest,
    ) -> Result<BoxStream<'static, Result<api::FindDocsResponse, ClientError>>, ClientError> {
        let m = pb::FindDocsRequest::from(req);
        self.streaming("find_docs_stream", m, |mut c, r| Box::pin(async move { c.find_docs_stream(r).await })).await
    }

    pub async fn search_stream(
        &self,
        req: api::SearchRequest,
    ) -> Result<BoxStream<'static, Result<api::SearchResponse, ClientError>>, ClientError> {
        let m = pb::SearchRequest::from(req);
        self.streaming("search_stream", m, |mut c, r| Box::pin(async move { c.search_stream(r).await })).await
    }

    pub async fn define_stream(
        &self,
        req: api::DefineRequest,
    ) -> Result<BoxStream<'static, Result<api::DefineResponse, ClientError>>, ClientError> {
        let m = pb::DefineRequest::from(req);
        self.streaming("define_stream", m, |mut c, r| Box::pin(async move { c.define_stream(r).await })).await
    }
}

/// Reassembles a stream of chunk responses into the single response the unary call would have returned.
/// `R` is the request type, for example `merge_stream::<api::ReadRequest>(stream)`.
pub async fn merge_stream<R: Chunked>(
    stream: BoxStream<'static, Result<R::Response, ClientError>>,
) -> Result<R::Response, ClientError> {
    Ok(R::merge(stream.try_collect().await?))
}

#[async_trait::async_trait]
impl api::VaultApi for VaultClient {
    async fn search(&self, req: api::SearchRequest) -> api::ApiResult<api::SearchResponse> {
        Ok(VaultClient::search(self, req).await?)
    }
    async fn find_docs(&self, req: api::FindDocsRequest) -> api::ApiResult<api::FindDocsResponse> {
        Ok(VaultClient::find_docs(self, req).await?)
    }
    async fn workspace(&self, req: api::WorkspaceRequest) -> api::ApiResult<api::WorkspaceResponse> {
        Ok(VaultClient::workspace(self, req).await?)
    }
    async fn grep(&self, req: api::GrepRequest) -> api::ApiResult<api::GrepResponse> {
        Ok(VaultClient::grep(self, req).await?)
    }
    async fn read(&self, req: api::ReadRequest) -> api::ApiResult<api::ReadResponse> {
        Ok(VaultClient::read(self, req).await?)
    }
    async fn outline(&self, req: api::OutlineRequest) -> api::ApiResult<api::OutlineResponse> {
        Ok(VaultClient::outline(self, req).await?)
    }
    async fn citations(&self, req: api::CitationsRequest) -> api::ApiResult<api::CitationsResponse> {
        Ok(VaultClient::citations(self, req).await?)
    }
    async fn define(&self, req: api::DefineRequest) -> api::ApiResult<api::DefineResponse> {
        Ok(VaultClient::define(self, req).await?)
    }
    async fn vault_info(&self) -> api::ApiResult<api::VaultInfo> {
        Ok(VaultClient::vault_info(self).await?)
    }
}
