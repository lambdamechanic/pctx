//! Deno ops for local tool callback functionality
//!
//! Local tools allow users to define runtime callbacks (JavaScript, Python, etc.) that can be invoked
//! from the sandbox via WebSocket.
//!
//! ## WebSocket Execution Architecture
//! - Tool metadata stored in `CallableToolRegistry` (for listing/discovery)
//! - Actual execution happens via WebSocket RPC to connected clients
//! - JavaScript calls `op_execute_local_tool` which sends WebSocket message and awaits response

use deno_core::{op2, OpState};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::callable_tool_registry::{CallableToolMetadata, CallableToolRegistry};
use crate::error::McpError;
use pctx_websocket_server::SessionManager;

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
    // Check if tool exists in registry (metadata only)
    let has_tool = {
        let state_borrow = state.borrow();
        let registry = state_borrow.borrow::<CallableToolRegistry>();
        registry.has(&name)
    };

    if !has_tool {
        return Err(McpError::ExecutionError(format!(
            "Tool '{name}' not found"
        )));
    }

    // Get session manager and execute via WebSocket
    let session_manager = {
        let state_borrow = state.borrow();
        state_borrow.borrow::<Arc<SessionManager>>().clone()
    };

    // Generate unique request ID
    let request_id = uuid::Uuid::new_v4().to_string();

    // Execute tool via WebSocket
    session_manager
        .execute_tool(&name, arguments, serde_json::Value::String(request_id))
        .await
        .map_err(|e| McpError::ExecutionError(e.to_string()))
}
