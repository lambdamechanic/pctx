use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Request to list all available tools
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListToolsRequest {}

/// Tool information in list response
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ToolInfo {
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub source: ToolSource,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ToolSource {
    Mcp,
    Local,
}

/// Response with list of tools
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListToolsResponse {
    pub tools: Vec<ToolInfo>,
}

/// Request to get function details
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetFunctionDetailsRequest {
    pub namespace: String,
    pub name: String,
}

/// Response with function details
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetFunctionDetailsResponse {
    pub namespace: String,
    pub name: String,
    pub description: String,
    #[schema(value_type = Object)]
    pub parameters: Value,
    pub return_type: String,
}

/// Request to execute code
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecuteCodeRequest {
    pub code: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

/// Successful code execution response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecuteCodeResponse {
    #[schema(value_type = Object)]
    pub result: Value,
    pub execution_time_ms: u64,
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
    pub tools: Vec<LocalToolDefinition>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct LocalToolDefinition {
    pub namespace: String,
    pub name: String,
    pub description: String,
    #[schema(value_type = Object)]
    pub parameters: Value,
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
