//! PCTX Runtime - Custom Deno extension with MCP client built-in
//!
//! This crate provides a Deno extension that includes:
//! - MCP client functionality (implemented in Rust, exposed to JavaScript)
//! - Console output capturing utilities
//!
//! The extension is designed to be used with `deno_executor`.

mod error;
mod fetch;
mod mcp_client;
pub mod ops;

#[cfg(test)]
mod tests;

pub use fetch::AllowedHosts;
pub use mcp_client::{MCPRegistry, MCPServerConfig};

// Export the snapshot that was created by the build script
// This can be used by other crates like deno_executor
pub static RUNTIME_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/PCTX_RUNTIME_SNAPSHOT.bin"));

// Define the pctx extension with MCP ops and runtime setup
// Note: op2 macro creates wrapper functions, we reference the wrappers here
// IMPORTANT: The extension name must match the snapshot extension name (pctx_runtime_snapshot)
// for the snapshot to load correctly
deno_core::extension!(
    pctx_runtime_snapshot,
    ops = [
        ops::op_register_mcp,
        ops::op_call_mcp_tool,
        ops::op_mcp_has,
        ops::op_mcp_get,
        ops::op_mcp_delete,
        ops::op_mcp_clear,
        ops::op_fetch,
    ],
    esm_entry_point = "ext:pctx_runtime_snapshot/runtime.js",
    esm = [ dir "src", "runtime.js" ],
    options = {
        registry: MCPRegistry,
        allowed_hosts: AllowedHosts,
    },
    state = |state, options| {
        state.put(options.registry);
        state.put(options.allowed_hosts);
    },
);

// The extension is available as pctx_runtime::pctx_runtime_snapshot::init(registry)
// from code that depends on this crate
