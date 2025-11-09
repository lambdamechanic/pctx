//! PCTX Runtime - Custom Deno extension with MCP client built-in
//!
//! This crate provides a Deno extension that includes:
//! - MCP client functionality (implemented in Rust, exposed to JavaScript)
//! - Console output capturing utilities
//!
//! The extension is designed to be used with deno_executor.

mod error;
mod mcp_client;
pub mod ops;

#[cfg(test)]
mod tests;

pub use mcp_client::{MCPRegistry, MCPServerConfig};

// Define the pctx extension with MCP ops and runtime setup
// Note: op2 macro creates wrapper functions, we reference the wrappers here
deno_core::extension!(
    pctx_runtime,
    ops = [
        ops::op_register_mcp,
        ops::op_call_mcp_tool,
        ops::op_mcp_has,
        ops::op_mcp_get,
        ops::op_mcp_delete,
        ops::op_mcp_clear,
    ],
    esm_entry_point = "ext:pctx_runtime/runtime.js",
    esm = [ dir "src", "runtime.js" ],
    options = {
        registry: MCPRegistry,
    },
    state = |state, options| {
        state.put(options.registry);
    },
);

// The extension is available as pctx_runtime::pctx_runtime::init(registry)
// from code that depends on this crate
