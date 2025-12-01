/// WebSocket protocol definitions for PCTX agent mode
///
/// Uses JSON-RPC 2.0 style messaging for request/response communication
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported JSON-RPC methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Client-to-server: Register tools
    RegisterTools,
    /// Server-to-client: Execute a registered tool on the client
    ExecuteTool,
    /// Unknown method (catch-all for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::RegisterTools => write!(f, "register_tools"),
            Method::ExecuteTool => write!(f, "execute_tool"),
            Method::Unknown => write!(f, "unknown"),
        }
    }
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: Method,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Value,
}

/// JSON-RPC 2.0 Response (success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    pub id: Value,
}

/// JSON-RPC 2.0 Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub error: JsonRpcError,
    pub id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 Notification (no id, no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Register tools request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterToolsParams {
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Execute tool request parameters (server → client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteToolParams {
    pub namespace: String,
    pub name: String,
    pub arguments: Value,
}

/// Session created notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreatedParams {
    pub session_id: String,
}

/// Standard error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // Application errors
    pub const TOOL_ALREADY_REGISTERED: i32 = -32000;
    pub const TOOL_NOT_FOUND: i32 = -32001;
    pub const EXECUTION_FAILED: i32 = -32002;
    pub const TIMEOUT: i32 = -32003;
}

impl JsonRpcRequest {
    pub fn new(method: Method, params: Option<Value>, id: impl Into<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id: id.into(),
        }
    }
}

impl JsonRpcResponse {
    pub fn success(result: Value, id: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            id,
        }
    }
}

impl JsonRpcErrorResponse {
    pub fn error(code: i32, message: impl Into<String>, id: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            error: JsonRpcError {
                code,
                message: message.into(),
                data: None,
            },
            id,
        }
    }
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}
