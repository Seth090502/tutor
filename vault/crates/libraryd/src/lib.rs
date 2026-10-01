//! libraryd: one `Arc<dyn VaultApi>` served identically over MCP (streamable HTTP, rmcp) and gRPC (tonic), with
//! bearer-token auth on both. See docs/phase1/SPEC.md section 11.

pub mod auth;
pub mod blocking;
pub mod config;
pub mod grpc;
pub mod mcp;
#[cfg(feature = "mock")]
pub mod mock;
pub mod render;
pub mod server;
pub mod tools;

use std::future::Future;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use vault_api::{ApiError, ApiResult};

/// Runs a tool call under a deadline, and gives it up when `stop` is cancelled (a shutdown that ran out of
/// patience). A call that overruns, or is stopped, becomes `Unavailable`, which clients may retry.
pub(crate) async fn timed<T>(
    limit: Duration,
    stop: &CancellationToken,
    call: impl Future<Output = ApiResult<T>>,
) -> ApiResult<T> {
    tokio::select! {
        outcome = tokio::time::timeout(limit, call) => match outcome {
            Ok(result) => result,
            Err(_) => Err(ApiError::Unavailable(format!("the call took longer than {limit:?}"))),
        },
        () = stop.cancelled() => Err(ApiError::Unavailable("the server is shutting down".into())),
    }
}
