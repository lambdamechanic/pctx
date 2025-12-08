use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorData {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidSession,
    Internal,
    Execution,
}

/// Request to register tools
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterToolsRequest {
    pub tools: Vec<pctx_code_mode::model::CallbackConfig>,
}

/// Response to registering tools
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegisterToolsResponse {
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

/// Response after creating a new `CodeMode` session
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionResponse {
    #[schema(value_type = String)]
    pub session_id: Uuid,
}
/// Response after closing a `CodeMode` session
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CloseSessionResponse {
    pub success: bool,
}

/// Messages that can be sent to a WebSocket client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsMessage {
    ExecuteTool(WsExecuteTool),
    ExecuteCodeResponse(WsExecuteCodeResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsExecuteTool {
    pub id: Uuid,
    pub namespace: String,
    pub name: String,
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsExecuteToolResult {
    pub output: Option<serde_json::Value>,
}

/// Execute code request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsExecuteCodeRequest {
    pub id: serde_json::Value, // String or Number (JSON-RPC allows both)
    pub code: String,
}

/// Execute code response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsExecuteCodeResponse {
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<pctx_code_mode::model::ExecuteOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorData>,
}
