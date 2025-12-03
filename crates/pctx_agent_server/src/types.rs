use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Error response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorInfo,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// Request to register local tools
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterLocalToolsRequest {
    pub session_id: String,
    pub tools: Vec<pctx_code_mode::model::CallbackConfig>,
}

/// Response after registering local tools
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegisterLocalToolsResponse {
    pub registered: usize,
}

/// Request to register MCP servers
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterMcpServersRequest {
    pub servers: Vec<McpServerConfig>,
}

// TODO: de-dup with pctx_config
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct McpServerConfig {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub auth: Option<Value>,
}

/// Response after registering MCP servers
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegisterMcpServersResponse {
    pub registered: usize,
    pub failed: Vec<String>,
}
