//! The MCP face of a [`VaultApi`]: `tools/list` and `tools/call` for the nine vault tools.
//!
//! A tool call parses its arguments into the `vault-api` request type, calls the same trait method the gRPC
//! service calls, and renders the response (see [`crate::render`]). A tool failure is a tool-level error result
//! (`isError`) so the model reads the message and can correct itself; only an unknown tool is a protocol error.

use crate::render::{MAX_MCP_CHARS, ToolOutput, render};
use crate::tools;
use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, JsonObject,
    ListToolsResult, MetaObject, PaginatedRequestParams, ProtocolVersion, ResultType, ServerCapabilities, ServerConfig,
    Tool,
};
use rmcp::service::RequestContext;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use vault_api as api;
use vault_api::VaultApi;

#[derive(Clone)]
pub struct VaultMcp {
    api: Arc<dyn VaultApi>,
    tools: Arc<Vec<Tool>>,
    timeout: Duration,
    stop: CancellationToken,
    max_chars: usize,
}

/// `vault_info` takes no arguments; unknown fields are ignored like everywhere else.
#[derive(serde::Deserialize)]
struct NoArgs {}

impl VaultMcp {
    pub fn new(api: Arc<dyn VaultApi>, timeout: Duration) -> Result<Self, tools::ToolsError> {
        Ok(Self {
            api,
            tools: Arc::new(tools::definitions()?),
            timeout,
            stop: CancellationToken::new(),
            max_chars: MAX_MCP_CHARS,
        })
    }

    /// Calls still running when `stop` is cancelled end with an `unavailable` tool error.
    pub fn with_stop(mut self, stop: CancellationToken) -> Self {
        self.stop = stop;
        self
    }

    /// Overrides the response cap. Tests use it to exercise truncation with small responses.
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    async fn invoke<Q, R, F, Fut>(&self, tool: &'static str, arguments: JsonObject, call: F) -> CallToolResult
    where
        Q: DeserializeOwned,
        R: ToolOutput,
        F: FnOnce(Arc<dyn VaultApi>, Q) -> Fut,
        Fut: Future<Output = api::ApiResult<R>>,
    {
        let started = Instant::now();
        let request: Q = match serde_json::from_value(serde_json::Value::Object(arguments)) {
            Ok(r) => r,
            Err(e) => return failure(&api::ApiError::InvalidArgument(format!("{tool}: {e}"))),
        };
        let outcome = match crate::timed(self.timeout, &self.stop, call(self.api.clone(), request)).await {
            Ok(response) => self.success(response),
            Err(e) => failure(&e),
        };
        tracing::info!(
            tool,
            ms = started.elapsed().as_millis() as u64,
            error = outcome.is_error == Some(true),
            "mcp tool call"
        );
        outcome
    }

    fn success<R: ToolOutput>(&self, response: R) -> CallToolResult {
        match render(response, self.max_chars) {
            Ok(rendered) => {
                let mut result = CallToolResult::success(vec![ContentBlock::text(rendered.text)]);
                result.structured_content = rendered.structured;
                if let Some(notice) = rendered.cap_notice {
                    let mut meta = JsonObject::new();
                    meta.insert(
                        "vault/cap".into(),
                        serde_json::json!({ "reason": notice.reason, "next": notice.next }),
                    );
                    result.meta = Some(MetaObject(meta));
                }
                result
            }
            Err(e) => failure(&api::ApiError::Internal(format!("cannot serialise the response: {e}"))),
        }
    }
}

fn failure(error: &api::ApiError) -> CallToolResult {
    let structured = serde_json::to_value(error).unwrap_or(serde_json::Value::Null);
    let mut result = CallToolResult::structured_error(structured);
    result.content = vec![ContentBlock::text(format!("error: {error}"))];
    result
}

impl ServerHandler for VaultMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("libraryd", env!("CARGO_PKG_VERSION")))
            .with_instructions(tools::SERVER_INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let cache_hints = context.protocol_version().is_some_and(|v| v >= ProtocolVersion::V_2026_07_28);
        Ok(ListToolsResult {
            result_type: Some(ResultType::COMPLETE),
            tools: self.tools.as_ref().clone(),
            meta: None,
            next_cursor: None,
            ttl_ms: cache_hints.then_some(0),
            cache_scope: cache_hints.then_some(CacheScope::Public),
        })
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.iter().find(|t| t.name == name).cloned()
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let arguments = request.arguments.unwrap_or_default();
        let result = match request.name.as_ref() {
            "search" => {
                self.invoke("search", arguments, |a, q: api::SearchRequest| async move { a.search(q).await }).await
            }
            "find_docs" => {
                self.invoke("find_docs", arguments, |a, q: api::FindDocsRequest| async move { a.find_docs(q).await })
                    .await
            }
            "workspace" => {
                self.invoke("workspace", arguments, |a, q: api::WorkspaceRequest| async move { a.workspace(q).await })
                    .await
            }
            "grep" => self.invoke("grep", arguments, |a, q: api::GrepRequest| async move { a.grep(q).await }).await,
            "read" => self.invoke("read", arguments, |a, q: api::ReadRequest| async move { a.read(q).await }).await,
            "outline" => {
                self.invoke("outline", arguments, |a, q: api::OutlineRequest| async move { a.outline(q).await }).await
            }
            "citations" => {
                self.invoke("citations", arguments, |a, q: api::CitationsRequest| async move { a.citations(q).await })
                    .await
            }
            "define" => {
                self.invoke("define", arguments, |a, q: api::DefineRequest| async move { a.define(q).await }).await
            }
            "vault_info" => {
                self.invoke("vault_info", arguments, |a, _: NoArgs| async move { a.vault_info().await }).await
            }
            other => return Err(McpError::invalid_params(format!("unknown tool: {other}"), None)),
        };
        Ok(CallToolResponse::Complete(result))
    }
}
