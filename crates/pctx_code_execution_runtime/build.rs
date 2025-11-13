//! Build script for `pctx_runtime`
//!
//! This script generates a V8 snapshot that includes the `pctx_runtime` extension
//! with all its JavaScript code pre-compiled. This snapshot can be loaded by
//! `pctx_executor` for faster startup times.

use std::env;
use std::path::PathBuf;

use deno_core::OpState;
use deno_core::extension;
use deno_core::snapshot::CreateSnapshotOptions;
use deno_core::snapshot::create_snapshot;

use pctx_config::server::ServerConfig;
use rmcp::model::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CallMCPToolArgs {
    pub name: String,
    pub tool: String,
    #[serde(default)]
    pub arguments: Option<JsonObject>,
}

#[derive(Debug, thiserror::Error)]
#[error("MCP error: {0}")]
struct McpError(String);

// Macro for implementing JsErrorClass (duplicated from src/js_error_impl.rs for build.rs)
macro_rules! impl_js_error_class {
    ($error_type:ty) => {
        impl deno_error::JsErrorClass for $error_type {
            fn get_class(&self) -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed("Error")
            }

            fn get_message(&self) -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Owned(self.to_string())
            }

            fn get_additional_properties(
                &self,
            ) -> Box<dyn Iterator<Item = (std::borrow::Cow<'static, str>, deno_error::PropertyValue)>>
            {
                Box::new(std::iter::empty())
            }

            fn get_ref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
                self
            }
        }
    };
}

impl_js_error_class!(McpError);

/// Register an MCP server (stub)
#[deno_core::op2]
#[serde]
fn op_register_mcp(_state: &mut OpState, #[serde] _config: ServerConfig) {}

/// Call an MCP tool (async stub)
#[deno_core::op2(async)]
#[serde]
#[allow(clippy::unused_async)]
async fn op_call_mcp_tool(#[serde] _args: CallMCPToolArgs) -> Result<serde_json::Value, McpError> {
    Ok(serde_json::Value::Null)
}

/// Check if an MCP server is registered (stub)
#[deno_core::op2(fast)]
fn op_mcp_has(_state: &mut OpState, #[string] _name: String) -> bool {
    false
}

/// Get an MCP server configuration (stub)
#[deno_core::op2]
#[serde]
fn op_mcp_get(_state: &mut OpState, #[string] _name: String) -> Option<ServerConfig> {
    None
}

/// Delete an MCP server configuration (stub)
#[deno_core::op2(fast)]
fn op_mcp_delete(_state: &mut OpState, #[string] _name: String) -> bool {
    false
}

/// Clear all MCP server configurations (stub)
#[deno_core::op2(fast)]
fn op_mcp_clear(_state: &mut OpState) {}

/// Fetch (stub)
#[deno_core::op2(async)]
#[serde]
#[allow(clippy::unused_async)]
async fn op_fetch(
    #[string] _url: String,
    #[serde] _options: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    Ok(serde_json::Value::Null)
}

// We need to define the extension here as well for snapshot creation
// The esm_entry_point tells deno_core to execute this module during snapshot creation
extension!(
    pctx_runtime_snapshot,
    ops = [
        // Op declarations - these will be registered but not executed during snapshot
        op_register_mcp,
        op_call_mcp_tool,
        op_mcp_has,
        op_mcp_get,
        op_mcp_delete,
        op_mcp_clear,
        op_fetch,
    ],
    esm_entry_point = "ext:pctx_runtime_snapshot/runtime.js",
    esm = [ dir "src", "runtime.js" ],
);

fn main() {
    // Tell cargo to rerun this build script if runtime.js changes
    println!("cargo:rerun-if-changed=src/runtime.js");

    // Get the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let snapshot_path = out_dir.join("PCTX_RUNTIME_SNAPSHOT.bin");

    // Create the snapshot
    let snapshot = create_snapshot(
        CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            startup_snapshot: None,
            skip_op_registration: false,
            extensions: vec![pctx_runtime_snapshot::init()],
            extension_transpiler: None,
            with_runtime_cb: None,
        },
        None, // No warmup script
    )
    .expect("Failed to create snapshot");

    // Write the snapshot to disk
    std::fs::write(&snapshot_path, snapshot.output).expect("Failed to write snapshot");

    println!(
        "cargo:rustc-env=PCTX_RUNTIME_SNAPSHOT={}",
        snapshot_path.display()
    );
    println!("Snapshot created at: {}", snapshot_path.display());
}
