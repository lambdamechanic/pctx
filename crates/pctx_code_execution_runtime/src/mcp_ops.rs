//! Deno ops for MCP client functionality
//!
//! These ops expose the Rust MCP client to JavaScript

use deno_core::OpState;
use deno_core::op2;
use std::cell::RefCell;
use std::rc::Rc;

use crate::error::McpError;
use crate::fetch::{AllowedHosts, FetchOptions, FetchResponse};
use crate::mcp_registry::{CallMCPToolArgs, MCPRegistry};

/// Call an MCP tool (async op)
#[op2(async)]
#[serde]
pub(crate) async fn op_call_mcp_tool(
    state: Rc<RefCell<OpState>>,
    #[serde] args: CallMCPToolArgs,
) -> Result<serde_json::Value, McpError> {
    let registry = {
        let borrowed = state.borrow();
        borrowed.borrow::<MCPRegistry>().clone()
    };
    crate::mcp_registry::call_mcp_tool(&registry, args).await
}

/// Fetch with host-based permissions
#[op2(async)]
#[serde]
pub(crate) async fn op_fetch(
    state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[serde] options: Option<FetchOptions>,
) -> Result<FetchResponse, McpError> {
    let allowed_hosts = {
        let borrowed = state.borrow();
        borrowed.borrow::<AllowedHosts>().clone()
    };
    crate::fetch::fetch_with_permissions(url, options, &allowed_hosts).await
}
