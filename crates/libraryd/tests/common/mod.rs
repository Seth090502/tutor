//! Shared test harness: a libraryd on ephemeral ports over a (possibly misbehaving) VaultApi, an rmcp client, a
//! vault-client, raw HTTP, and a log capture.
#![allow(dead_code)]

use libraryd::auth::Token;
use libraryd::mock::MockVault;
use libraryd::server::{RunningServer, ServerOptions, start};
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, ClientConfig};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde::de::DeserializeOwned;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use vault_api::*;
use vault_client::{ClientOptions, Secret, VaultClient};

pub type McpClient = RunningService<rmcp::RoleClient, ClientConfig>;

pub struct Harness {
    pub server: RunningServer,
    pub token: Token,
}

impl Harness {
    pub async fn start(api: Arc<dyn VaultApi>) -> Self {
        Self::start_with(api, |_| {}).await
    }

    pub async fn mock() -> Self {
        Self::start(Arc::new(MockVault::new())).await
    }

    pub async fn start_with(api: Arc<dyn VaultApi>, tweak: impl FnOnce(&mut ServerOptions)) -> Self {
        let token = Token::generate();
        let mut options = ServerOptions {
            mcp_bind: "127.0.0.1:0".parse().unwrap(),
            grpc_bind: "127.0.0.1:0".parse().unwrap(),
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            vault_id: libraryd::mock::MOCK_VAULT_ID.into(),
            request_timeout: Duration::from_secs(30),
            ..ServerOptions::new(token.clone())
        };
        tweak(&mut options);
        let server = start(api, options).await.expect("server starts");
        Self { server, token }
    }

    pub fn mcp_url(&self) -> String {
        format!("http://{}/mcp", self.server.mcp_addr)
    }

    pub fn grpc_url(&self) -> String {
        format!("http://{}", self.server.grpc_addr)
    }

    pub fn grpc(&self) -> VaultClient {
        self.grpc_with(Some(self.token.expose()), ClientOptions::default())
    }

    pub fn grpc_with(&self, token: Option<&str>, options: ClientOptions) -> VaultClient {
        VaultClient::connect_lazy(&self.grpc_url(), token.map(Secret::new), options).expect("valid endpoint")
    }

    pub async fn mcp(&self) -> McpClient {
        self.mcp_with_token(self.token.expose()).await
    }

    pub async fn mcp_with_token(&self, token: &str) -> McpClient {
        self.try_mcp(token, ClientConfig::default()).await.expect("MCP handshake")
    }

    pub async fn try_mcp(&self, token: &str, config: ClientConfig) -> Result<McpClient, String> {
        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(self.mcp_url()).auth_header(token),
        );
        config.serve(transport).await.map_err(|e| e.to_string())
    }
}

/// Calls an MCP tool with `args` serialised from a request type.
pub async fn mcp_call(client: &McpClient, tool: &str, args: &impl serde::Serialize) -> CallToolResult {
    let arguments = serde_json::to_value(args).unwrap().as_object().cloned().unwrap_or_default();
    client
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
        .unwrap_or_else(|e| panic!("{tool}: {e}"))
}

/// The structured content of a successful result, as the typed response.
pub fn structured<T: DeserializeOwned>(result: &CallToolResult) -> T {
    assert_ne!(result.is_error, Some(true), "tool failed: {:?}", result.content);
    let value = result.structured_content.clone().expect("structuredContent");
    serde_json::from_value(value).expect("structured content matches the response type")
}

pub fn text_of(result: &CallToolResult) -> String {
    result.content.iter().filter_map(|c| c.as_text()).map(|t| t.text.clone()).collect::<Vec<_>>().join("\n")
}

/// One HTTP/1.1 exchange on a fresh connection: (status, header block, body).
pub async fn raw_http(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut request = format!("{method} {path} HTTP/1.1\r\nConnection: close\r\nContent-Length: {}\r\n", body.len());
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        request.push_str(&format!("Host: {addr}\r\n"));
    }
    for (k, v) in headers {
        request.push_str(&format!("{k}: {v}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(body);
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(10), stream.read_to_end(&mut raw)).await;
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    (status, head.to_string(), body.to_string())
}

pub const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"raw","version":"0"}}}"#;

pub const MCP_HEADERS: [(&str, &str); 2] =
    [("Content-Type", "application/json"), ("Accept", "application/json, text/event-stream")];

// ----------------------------------------------------------------------------------------- test doubles

/// The mock, with switches for failures, delays and oversized answers.
#[derive(Default)]
pub struct Scripted {
    pub inner: MockVault,
    pub calls: AtomicUsize,
    /// Fail the first N calls (of any tool) with `error`; `usize::MAX` fails every call.
    pub fail_first: usize,
    pub error: Option<ApiError>,
    pub flat_delay: Duration,
    /// `read` sleeps (9 - first line number) of these, so earlier addresses finish later.
    pub read_delay_unit: Duration,
    /// `read` ignores its budget and returns this many passages of this many characters.
    pub huge_read: Option<(usize, usize)>,
}

impl Scripted {
    pub fn failing(error: ApiError, first: usize) -> Self {
        Self { error: Some(error), fail_first: first, ..Self::default() }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    async fn gate(&self) -> ApiResult<()> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.flat_delay.is_zero() {
            tokio::time::sleep(self.flat_delay).await;
        }
        match &self.error {
            Some(e) if n < self.fail_first => Err(e.clone()),
            _ => Ok(()),
        }
    }
}

#[async_trait::async_trait]
impl VaultApi for Scripted {
    async fn search(&self, req: SearchRequest) -> ApiResult<SearchResponse> {
        self.gate().await?;
        self.inner.search(req).await
    }
    async fn find_docs(&self, req: FindDocsRequest) -> ApiResult<FindDocsResponse> {
        self.gate().await?;
        self.inner.find_docs(req).await
    }
    async fn workspace(&self, req: WorkspaceRequest) -> ApiResult<WorkspaceResponse> {
        self.gate().await?;
        self.inner.workspace(req).await
    }
    async fn grep(&self, req: GrepRequest) -> ApiResult<GrepResponse> {
        self.gate().await?;
        self.inner.grep(req).await
    }
    async fn read(&self, req: ReadRequest) -> ApiResult<ReadResponse> {
        self.gate().await?;
        if let Some((count, size)) = self.huge_read {
            let address: Address = "3f9a1c07b2e4d55e#L1".parse().unwrap();
            let passage = Passage {
                address,
                title: Some("Huge".into()),
                kind: DocKind::Book,
                page: Some("1".into()),
                section: None,
                text: "x".repeat(size),
                superseded_by: None,
            };
            return Ok(ReadResponse { passages: vec![passage; count], errors: vec![], truncated: None });
        }
        if !self.read_delay_unit.is_zero() {
            let line = req
                .addresses
                .first()
                .and_then(|a| a.split("#L").nth(1))
                .and_then(|l| l.parse::<u32>().ok())
                .unwrap_or(1);
            tokio::time::sleep(self.read_delay_unit * 9u32.saturating_sub(line)).await;
        }
        self.inner.read(req).await
    }
    async fn outline(&self, req: OutlineRequest) -> ApiResult<OutlineResponse> {
        self.gate().await?;
        self.inner.outline(req).await
    }
    async fn citations(&self, req: CitationsRequest) -> ApiResult<CitationsResponse> {
        self.gate().await?;
        self.inner.citations(req).await
    }
    async fn define(&self, req: DefineRequest) -> ApiResult<DefineResponse> {
        self.gate().await?;
        self.inner.define(req).await
    }
    async fn vault_info(&self) -> ApiResult<VaultInfo> {
        self.gate().await?;
        self.inner.vault_info().await
    }
}

// ------------------------------------------------------------------------------------------- log capture

struct LogWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut b) = self.0.lock() {
            b.extend_from_slice(buf);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct LogMaker(Arc<Mutex<Vec<u8>>>);

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogMaker {
    type Writer = LogWriter;
    fn make_writer(&'a self) -> LogWriter {
        LogWriter(self.0.clone())
    }
}

/// Installs a global subscriber at TRACE level (once per test binary) and returns its buffer.
pub fn capture_logs() -> Arc<Mutex<Vec<u8>>> {
    static LOGS: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();
    LOGS.get_or_init(|| {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(LogMaker(buffer.clone()))
            .try_init();
        buffer
    })
    .clone()
}

pub fn captured(buffer: &Arc<Mutex<Vec<u8>>>) -> String {
    String::from_utf8_lossy(&buffer.lock().unwrap()).to_string()
}
