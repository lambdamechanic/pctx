//! Deno ops for local tool callback functionality
//!
//! Callbacks are pre-registered callbacks that execute synchronously.
//! Callbacks handle their own execution logic (WebSocket RPC, MCP calls, etc.)

use deno_core::{OpState, op2};

use crate::{CallbackRegistry, error::McpError};

#[op2]
#[serde]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn op_invoke_callback(
    state: &mut OpState,
    #[string] id: String,
    #[serde] arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    let registry = state.borrow::<CallbackRegistry>();

    registry.invoke(&id, arguments)
}
