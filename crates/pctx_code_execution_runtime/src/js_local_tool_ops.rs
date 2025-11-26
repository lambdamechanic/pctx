//! Deno ops for JS local tool callback functionality
//!
//! JS local tools allow users to define JavaScript callbacks that can be invoked
//! from the sandbox. The approach is simplified:
//! - Callbacks are stored entirely on the JavaScript side in a Map
//! - Metadata is stored in Rust for validation and querying
//! - No V8 values cross the Rust/JS boundary via ops

use deno_core::{OpState, op2};

use crate::error::McpError;
use crate::js_local_tool_registry::{
    JsLocalToolDefinition, JsLocalToolMetadata, JsLocalToolRegistry,
};

/// Register JS local tool metadata
///
/// The JavaScript callback is stored on the JS side; this just stores metadata.
#[op2]
#[serde]
pub(crate) fn op_register_js_local_tool_metadata(
    state: &mut OpState,
    #[serde] metadata: JsLocalToolMetadata,
) -> Result<(), McpError> {
    let registry = state.borrow::<JsLocalToolRegistry>();
    // We only store metadata; the callback is managed by JavaScript
    registry.register_metadata_only(metadata)
}

/// Check if a JS local tool is registered
#[op2(fast)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_js_local_tool_has(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<JsLocalToolRegistry>();
    registry.has(&name)
}

/// Get JS local tool metadata
#[op2]
#[serde]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_js_local_tool_get(
    state: &mut OpState,
    #[string] name: String,
) -> Option<JsLocalToolMetadata> {
    let registry = state.borrow::<JsLocalToolRegistry>();
    registry.get_metadata(&name)
}

/// List all registered JS local tools
#[op2]
#[serde]
pub(crate) fn op_js_local_tool_list(state: &mut OpState) -> Vec<JsLocalToolMetadata> {
    let registry = state.borrow::<JsLocalToolRegistry>();
    registry.list()
}

/// Delete a JS local tool
#[op2(fast)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_js_local_tool_delete(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<JsLocalToolRegistry>();
    registry.delete(&name)
}

/// Clear all JS local tools
#[op2(fast)]
pub(crate) fn op_js_local_tool_clear(state: &mut OpState) {
    let registry = state.borrow::<JsLocalToolRegistry>();
    registry.clear();
}

/// Get pre-registered tools (called during runtime initialization)
#[op2]
#[serde]
pub(crate) fn op_get_pre_registered_tools(state: &mut OpState) -> Vec<JsLocalToolDefinition> {
    let registry = state.borrow::<JsLocalToolRegistry>();
    registry.get_pre_registered()
}
