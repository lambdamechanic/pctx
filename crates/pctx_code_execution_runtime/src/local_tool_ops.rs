//! Deno ops for local tool callback functionality
//!
//! Local tools allow users to define runtime callbacks (JavaScript, Python, etc.) that can be invoked
//! from the sandbox.
//!
//! ## Unified Callback Architecture
//! - All callbacks (Python, JS, Rust) are stored as Rust closures in `CallableToolRegistry`
//! - JavaScript calls `op_execute_local_tool` which executes the closure
//! - No distinction between Python/JS at the op level - all are just callbacks!

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

/// Execute a local tool callback (UNIFIED API - works for Python, JS, anything!)
///
/// This op executes a callback stored in the `CallableToolRegistry`. The callback
/// can be from any source language (Python, JavaScript, Rust native, etc.) -
/// from this op's perspective, it's just a Rust closure.
///
/// # Arguments
/// * `name` - Name of the tool to execute
/// * `arguments` - Optional JSON arguments to pass to the callback
///
/// # Returns
/// The callback's result as JSON
///
/// # Errors
/// Returns error if the tool is not found or the callback fails
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
