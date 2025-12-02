//! Deno ops for local tool callback functionality
//!
//! Local tools allow users to define runtime callbacks (JavaScript, Python, etc.) that can be invoked
//! from the sandbox via WebSocket.
//!
//! ## WebSocket Execution Architecture
//! - Tool metadata stored in `CallableToolRegistry` (for listing/discovery)
//! - Actual execution happens via WebSocket RPC to connected clients
//! - JavaScript calls `op_execute_local_tool` which sends WebSocket message and awaits response

use deno_core::{OpState, op2};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::callable_tool_registry::{CallableToolMetadata, CallableToolRegistry};
use crate::error::McpError;
use pctx_session_types::{SessionManager, SessionStorage, ToolCallRecord};

/// Check if a local tool is registered
#[op2(fast)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_local_tool_has(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<CallableToolRegistry>();
    registry.has(&name)
}

/// Get local tool metadata
#[op2]
#[serde]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_local_tool_get(
    state: &mut OpState,
    #[string] name: String,
) -> Option<CallableToolMetadata> {
    let registry = state.borrow::<CallableToolRegistry>();
    registry.get_metadata(&name)
}

/// List all registered local tools
#[op2]
#[serde]
pub(crate) fn op_local_tool_list(state: &mut OpState) -> Vec<CallableToolMetadata> {
    let registry = state.borrow::<CallableToolRegistry>();
    registry.list()
}

/// Delete a local tool
#[op2(fast)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_local_tool_delete(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<CallableToolRegistry>();
    registry.delete(&name)
}

/// Clear all local tools
#[op2(fast)]
pub(crate) fn op_local_tool_clear(state: &mut OpState) {
    let registry = state.borrow::<CallableToolRegistry>();
    registry.clear();
}

/// Execute a local tool via WebSocket RPC
///
/// This op sends an execution request to the WebSocket client that registered the tool,
/// then waits for the response. The execution happens asynchronously over WebSocket.
///
/// # Arguments
/// * `name` - Name of the tool to execute (format: "namespace.toolName")
/// * `arguments` - Optional JSON arguments to pass to the client
///
/// # Returns
/// The tool's result as JSON (returned from the client)
///
/// # Errors
/// Returns error if:
/// - Tool is not found
/// - Client disconnected
/// - Execution timeout (30s)
/// - Client returns error
#[op2(async)]
#[serde]
#[allow(clippy::needless_pass_by_value)]
pub(crate) async fn op_execute_local_tool(
    state: Rc<RefCell<OpState>>,
    #[string] name: String,
    #[serde] arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    // Get session manager
    let session_manager = {
        let state_borrow = state.borrow();
        state_borrow.borrow::<Arc<SessionManager>>().clone()
    };

    // Generate unique request ID
    let request_id = uuid::Uuid::new_v4().to_string();

    // Build execution request message
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "execute_tool",
        "params": {
            "name": name,
            "arguments": arguments
        },
        "id": request_id.clone()
    });

    // Get the session ID for this tool
    let session_id = session_manager
        .get_tool_session(&name)
        .await
        .ok_or_else(|| McpError::ExecutionError(format!("Tool not found: {}", name)))?;

    // Execute tool via WebSocket
    // The session manager will return ToolNotFound if the tool doesn't exist
    let start_time = chrono::Utc::now().timestamp_millis();
    let result = session_manager
        .execute_tool_raw(
            &name,
            pctx_session_types::OutgoingMessage::Response(request),
            serde_json::Value::String(request_id),
        )
        .await;

    // Record the tool call in session history
    let (namespace, tool_name_part) = name.split_once('.').unwrap_or(("", &name));
    let tool_call_record = ToolCallRecord {
        session_id: session_id.clone(),
        timestamp: start_time,
        tool_name: tool_name_part.to_string(),
        namespace: namespace.to_string(),
        arguments: arguments.unwrap_or(serde_json::Value::Null),
        result: result.as_ref().ok().cloned(),
        error: result.as_ref().err().map(|e| e.to_string()),
        code_snippet: None, // We don't have access to the source code here
    };

    // Try to save the tool call if session storage is available
    if let Some(session_storage) = state.borrow().try_borrow::<Arc<SessionStorage>>() {
        if let Ok(mut history) = session_storage.load_session(&session_id) {
            history.add_tool_call(tool_call_record);
            let _ = session_storage.save_session(&history);
        }
    }

    result.map_err(|e| McpError::ExecutionError(e.to_string()))
}
