//! The vault's tool contract. libraryd exposes [`VaultApi`] over MCP and gRPC, vault-core implements it, and
//! vault-client calls it, so every interface returns identical results for identical requests.

pub mod address;
pub mod types;

pub use address::{Address, AddressError, DocId};
pub use types::*;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "message")]
pub enum ApiError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
    /// The tool exists in the contract but its build phase has not landed yet.
    #[error("not available yet: {0}")]
    NotYetAvailable(String),
    /// A dependency (index, sidecar, disk) is temporarily unavailable; retrying later may succeed.
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

/// The nine tools. Method names are the tool names; MCP and gRPC are thin layers over this trait.
#[async_trait::async_trait]
pub trait VaultApi: Send + Sync {
    async fn search(&self, req: SearchRequest) -> ApiResult<SearchResponse>;
    async fn find_docs(&self, req: FindDocsRequest) -> ApiResult<FindDocsResponse>;
    async fn workspace(&self, req: WorkspaceRequest) -> ApiResult<WorkspaceResponse>;
    async fn grep(&self, req: GrepRequest) -> ApiResult<GrepResponse>;
    async fn read(&self, req: ReadRequest) -> ApiResult<ReadResponse>;
    async fn outline(&self, req: OutlineRequest) -> ApiResult<OutlineResponse>;
    async fn citations(&self, req: CitationsRequest) -> ApiResult<CitationsResponse>;
    async fn define(&self, req: DefineRequest) -> ApiResult<DefineResponse>;
    async fn vault_info(&self) -> ApiResult<VaultInfo>;
}

/// Tool names exactly as exposed. Only `[A-Za-z0-9_-]`, which every runtime accepts.
pub const TOOL_NAMES: [&str; 9] =
    ["search", "find_docs", "workspace", "grep", "read", "outline", "citations", "define", "vault_info"];
