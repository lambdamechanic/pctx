//! Deno ops for MCP client functionality
//!
//! These ops expose the Rust MCP client to JavaScript

use deno_core::op2;
use deno_core::OpState;
use std::cell::RefCell;
use std::rc::Rc;

use crate::error::McpError;
use crate::mcp_client::{CallMCPToolArgs, MCPRegistry, MCPServerConfig};

/// Register an MCP server
#[op2]
#[serde]
pub fn op_register_mcp(
    state: &mut OpState,
    #[serde] config: MCPServerConfig,
) -> Result<(), McpError> {
    let registry = state.borrow::<MCPRegistry>();
    registry.add(config)
}

/// Call an MCP tool (async op)
#[op2(async)]
#[serde]
pub async fn op_call_mcp_tool(
    state: Rc<RefCell<OpState>>,
    #[serde] args: CallMCPToolArgs,
) -> Result<serde_json::Value, McpError> {
    let registry = {
        let borrowed = state.borrow();
        borrowed.borrow::<MCPRegistry>().clone()
    };
    crate::mcp_client::call_mcp_tool(&registry, args).await
}

/// Check if an MCP server is registered
#[op2(fast)]
pub fn op_mcp_has(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<MCPRegistry>();
    registry.has(&name)
}

/// Get an MCP server configuration
#[op2]
#[serde]
pub fn op_mcp_get(
    state: &mut OpState,
    #[string] name: String,
) -> Option<MCPServerConfig> {
    let registry = state.borrow::<MCPRegistry>();
    registry.get(&name)
}

/// Delete an MCP server configuration
#[op2(fast)]
pub fn op_mcp_delete(state: &mut OpState, #[string] name: String) -> bool {
    let registry = state.borrow::<MCPRegistry>();
    registry.delete(&name)
}

/// Clear all MCP server configurations
#[op2(fast)]
pub fn op_mcp_clear(state: &mut OpState) {
    let registry = state.borrow::<MCPRegistry>();
    registry.clear();
}
