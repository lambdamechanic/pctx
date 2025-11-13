use pctx_config::auth::AuthConfig;
use pctx_config::server::{McpConnectionError, init_mcp_client as config_init_mcp_client};
use rmcp::{RoleClient, model::InitializeRequestParam, service::RunningService};
use url::Url;

/// Error types for MCP server connection failures
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InitMCPClientError {
    /// Server requires authentication (401 Unauthorized)
    #[error("Server requires OAuth authentication")]
    RequiresOAuth,
    /// Server requires authentication (401 Unauthorized)
    #[error("Server requires authentication")]
    RequiresAuth,
    /// Connection failed (network error, invalid URL, etc.)
    #[error("Failed to connect: {0}")]
    Failed(String),
}

impl From<McpConnectionError> for InitMCPClientError {
    fn from(err: McpConnectionError) -> Self {
        match err {
            McpConnectionError::RequiresOAuth => InitMCPClientError::RequiresOAuth,
            McpConnectionError::RequiresAuth => InitMCPClientError::RequiresAuth,
            McpConnectionError::Failed(msg) => InitMCPClientError::Failed(msg),
        }
    }
}

pub(crate) async fn init_mcp_client(
    url: &Url,
    auth: Option<&AuthConfig>,
) -> Result<RunningService<RoleClient, InitializeRequestParam>, InitMCPClientError> {
    config_init_mcp_client(url, auth, true)
        .await
        .map_err(Into::into)
}
