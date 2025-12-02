//! Deno ops for local tool callback functionality
//!
//! Local tools are pre-registered callbacks that execute synchronously.
//! Callbacks handle their own execution logic (WebSocket RPC, MCP calls, etc.)

use deno_core::{OpState, op2};

use crate::callable_tool_registry::{CallableToolMetadata, CallableToolRegistry};
use crate::error::McpError;

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

/// Execute a local tool via pre-registered callback
///
/// This op calls a callback that was registered before runtime creation.
/// The callback handles execution logic internally (WebSocket RPC, MCP, etc.)
///
/// # Arguments
/// * `name` - Name of the tool to execute (format: "namespace.toolName")
/// * `arguments` - Optional JSON arguments to pass to the callback
///
/// # Returns
/// The tool's result as JSON
///
/// # Errors
/// Returns error if:
/// - Tool is not found
/// - Callback execution fails
#[op2]
#[serde]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_execute_local_tool(
    state: &mut OpState,
    #[string] name: String,
    #[serde] arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    let registry = state.borrow::<CallableToolRegistry>();

    registry
        .execute(&name, arguments)
        .map_err(McpError::ExecutionError)
}
