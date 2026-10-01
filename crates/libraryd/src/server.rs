//! Binds and runs the two listeners over one shared [`VaultApi`]: MCP (streamable HTTP at `/mcp`, plus an
//! unauthenticated `/healthz`) and gRPC.

use crate::auth::{BearerInterceptor, Token, require_bearer};
use crate::config::ServerSection;
use crate::grpc::GrpcVault;
use crate::mcp::VaultMcp;
use crate::tools::ToolsError;
use axum::Router;
use axum::extract::State;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;
use tonic::service::InterceptorLayer;
use vault_api::VaultApi;

/// Deadline for one tool call, on both interfaces.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// How long a shutdown waits for running calls and open connections before it aborts them.
pub const DEFAULT_SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// After the grace period, how long stopped calls get to close their connections before the listeners are aborted.
const HARD_STOP_WAIT: Duration = Duration::from_secs(2);

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("cannot listen on {addr} for {what}: {source}")]
    Bind { what: &'static str, addr: SocketAddr, source: std::io::Error },
    #[error(transparent)]
    Tools(#[from] ToolsError),
    #[error("a server task failed: {0}")]
    Task(String),
}

#[derive(Clone)]
pub struct ServerOptions {
    pub mcp_bind: SocketAddr,
    pub grpc_bind: SocketAddr,
    pub token: Token,
    pub allowed_hosts: Vec<String>,
    /// Reported by `/healthz`.
    pub stage: String,
    pub vault_id: String,
    pub request_timeout: Duration,
    pub shutdown_grace: Duration,
    /// Overrides the MCP response cap (characters); `None` uses the 20,000-token cap. For tests.
    pub mcp_max_chars: Option<usize>,
}

impl ServerOptions {
    /// The SPEC defaults (loopback, ports 7341 and 7342) for `token`. Override fields with struct update syntax.
    pub fn new(token: Token) -> Self {
        Self::from_config(&ServerSection::default(), token, "local", "")
    }

    pub fn from_config(section: &ServerSection, token: Token, stage: &str, vault_id: &str) -> Self {
        Self {
            mcp_bind: section.mcp_bind,
            grpc_bind: section.grpc_bind,
            token,
            allowed_hosts: section.allowed_hosts.clone(),
            stage: stage.to_string(),
            vault_id: vault_id.to_string(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            shutdown_grace: DEFAULT_SHUTDOWN_GRACE,
            mcp_max_chars: None,
        }
    }
}

#[derive(Clone)]
struct Health {
    stage: String,
    vault_id: String,
}

async fn healthz(State(health): State<Arc<Health>>) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true, "stage": health.stage, "vault_id": health.vault_id }))
}

/// Both listeners, running. Dropping this does not stop them; call [`RunningServer::shutdown`] or
/// [`RunningServer::run_until`].
pub struct RunningServer {
    pub mcp_addr: SocketAddr,
    pub grpc_addr: SocketAddr,
    shutdown: CancellationToken,
    hard_stop: CancellationToken,
    grace: Duration,
    mcp: JoinHandle<std::io::Result<()>>,
    grpc: JoinHandle<Result<(), tonic::transport::Error>>,
}

type Joined = (Result<std::io::Result<()>, JoinError>, Result<Result<(), tonic::transport::Error>, JoinError>);

fn finish((mcp, grpc): Joined) -> Result<(), ServerError> {
    mcp.map_err(|e| ServerError::Task(e.to_string()))?.map_err(|e| ServerError::Task(e.to_string()))?;
    grpc.map_err(|e| ServerError::Task(e.to_string()))?.map_err(|e| ServerError::Task(e.to_string()))
}

impl RunningServer {
    /// Serves until `signal` completes (or a listener fails), then stops accepting connections and lets running
    /// calls finish. Calls still running after the grace period are aborted.
    pub async fn run_until(self, signal: impl Future<Output = ()>) -> Result<(), ServerError> {
        let Self { shutdown, hard_stop, grace, mcp, grpc, .. } = self;
        let aborts = (mcp.abort_handle(), grpc.abort_handle());
        let mut joined = Box::pin(async move { tokio::join!(mcp, grpc) });
        tokio::select! {
            result = &mut joined => return finish(result),
            () = signal => shutdown.cancel(),
        }
        if let Ok(result) = tokio::time::timeout(grace, &mut joined).await {
            return finish(result);
        }
        // Out of patience: end the calls that are still running, so their connections can close.
        tracing::warn!(?grace, "shutdown grace period over; stopping calls that are still running");
        hard_stop.cancel();
        match tokio::time::timeout(HARD_STOP_WAIT, joined).await {
            Ok(result) => finish(result),
            Err(_) => {
                aborts.0.abort();
                aborts.1.abort();
                Ok(())
            }
        }
    }

    /// Stops accepting connections, lets running calls finish (up to the grace period), and stops both listeners.
    pub async fn shutdown(self) -> Result<(), ServerError> {
        self.run_until(std::future::ready(())).await
    }
}

async fn bind(what: &'static str, addr: SocketAddr) -> Result<TcpListener, ServerError> {
    TcpListener::bind(addr).await.map_err(|source| ServerError::Bind { what, addr, source })
}

/// Binds both ports (port 0 picks a free one) and starts serving.
pub async fn start(api: Arc<dyn VaultApi>, options: ServerOptions) -> Result<RunningServer, ServerError> {
    let mcp_listener = bind("MCP", options.mcp_bind).await?;
    let grpc_listener = bind("gRPC", options.grpc_bind).await?;
    let mcp_addr = mcp_listener.local_addr().map_err(|source| ServerError::Bind {
        what: "MCP",
        addr: options.mcp_bind,
        source,
    })?;
    let grpc_addr = grpc_listener.local_addr().map_err(|source| ServerError::Bind {
        what: "gRPC",
        addr: options.grpc_bind,
        source,
    })?;

    let shutdown = CancellationToken::new();
    let hard_stop = CancellationToken::new();

    // MCP: /mcp behind the bearer check; /healthz open.
    let mut handler = VaultMcp::new(api.clone(), options.request_timeout)?.with_stop(hard_stop.clone());
    if let Some(max) = options.mcp_max_chars {
        handler = handler.with_max_chars(max);
    }
    let service: StreamableHttpService<VaultMcp, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(handler.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_allowed_hosts(options.allowed_hosts.clone())
            .with_cancellation_token(shutdown.child_token()),
    );
    let protected =
        Router::new().nest_service("/mcp", service).layer(from_fn_with_state(options.token.clone(), require_bearer));
    let health = Arc::new(Health { stage: options.stage.clone(), vault_id: options.vault_id.clone() });
    let app = Router::new().route("/healthz", get(healthz)).with_state(health).merge(protected);
    let mcp = tokio::spawn({
        let stop = shutdown.clone();
        async move { axum::serve(mcp_listener, app).with_graceful_shutdown(stop.cancelled_owned()).await }
    });

    // gRPC: every call passes the interceptor.
    let grpc_service = GrpcVault::new(api, options.request_timeout).with_stop(hard_stop.clone()).into_service();
    let grpc = tokio::spawn({
        let stop = shutdown.clone();
        let interceptor = BearerInterceptor::new(options.token.clone());
        async move {
            tonic::transport::Server::builder()
                .tcp_nodelay(true)
                .http2_keepalive_interval(Some(Duration::from_secs(30)))
                .concurrency_limit_per_connection(256)
                .layer(InterceptorLayer::new(interceptor))
                .add_service(grpc_service)
                .serve_with_incoming_shutdown(TcpListenerStream::new(grpc_listener), stop.cancelled_owned())
                .await
        }
    });

    tracing::info!(%mcp_addr, %grpc_addr, stage = %options.stage, "libraryd listening");
    Ok(RunningServer { mcp_addr, grpc_addr, shutdown, hard_stop, grace: options.shutdown_grace, mcp, grpc })
}
