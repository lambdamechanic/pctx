//! Deno ops for local tool callback functionality
//!
//! Local tools allow users to define runtime callbacks (JavaScript, Python, etc.) that can be invoked
//! from the sandbox.
//! - Callbacks are stored entirely on the runtime side (JavaScript Map, Python dict, etc.)
//! - Metadata is stored in Rust for validation and querying
//! - No V8 values or runtime-specific data cross the Rust boundary via ops

use deno_core::{OpState, op2};

use crate::error::McpError;
use crate::local_tool_registry::{LocalToolDefinition, LocalToolMetadata, LocalToolRegistry};

/// Register local tool metadata
///
/// The callback is stored on the runtime side; this just stores metadata.
#[op2]
#[serde]
pub(crate) fn op_register_local_tool_metadata(
    state: &mut OpState,
    #[serde] metadata: LocalToolMetadata,
) -> Result<(), McpError> {
    let registry = state.borrow::<LocalToolRegistry>();
    // We only store metadata; the callback is managed by the runtime
    registry.register_metadata_only(metadata)
}

/// Check if a local tool is registered
#[op2(fast)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_local_tool_has(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<LocalToolRegistry>();
    registry.has(&name)
}

/// Get local tool metadata
#[op2]
#[serde]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_local_tool_get(
    state: &mut OpState,
    #[string] name: String,
) -> Option<LocalToolMetadata> {
    let registry = state.borrow::<LocalToolRegistry>();
    registry.get_metadata(&name)
}

/// List all registered local tools
#[op2]
#[serde]
pub(crate) fn op_local_tool_list(state: &mut OpState) -> Vec<LocalToolMetadata> {
    let registry = state.borrow::<LocalToolRegistry>();
    registry.list()
}

/// Delete a local tool
#[op2(fast)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_local_tool_delete(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<LocalToolRegistry>();
    registry.delete(&name)
}

/// Clear all local tools
#[op2(fast)]
pub(crate) fn op_local_tool_clear(state: &mut OpState) {
    let registry = state.borrow::<LocalToolRegistry>();
    registry.clear();
}

/// Get pre-registered tools (called during runtime initialization)
#[op2]
#[serde]
pub(crate) fn op_get_pre_registered_tools(state: &mut OpState) -> Vec<LocalToolDefinition> {
    let registry = state.borrow::<LocalToolRegistry>();
    registry.get_pre_registered()
}
