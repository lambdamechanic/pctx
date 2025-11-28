//! # PCTX Node.js SDK
//!
//! Node.js/TypeScript bindings for the complete PCTX toolkit.
//!
//! This crate provides a native Node.js addon that allows JavaScript/TypeScript code to:
//! - Register MCP servers
//! - Register local tool callbacks
//! - List available functions from all sources
//! - Get detailed function information
//! - Execute TypeScript code with full tool access
//!
//! ## Features
//!
//! - **Complete PCTX API**: All functionality from `pctx_core::PctxTools`
//! - **Native Performance**: Direct FFI calls with minimal overhead
//! - **TypeScript First**: Full TypeScript type definitions
//! - **Async/Await**: Native Promise support throughout
//! - **Comprehensive Errors**: JavaScript Error objects with context
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  TypeScript SDK (@pctx/sdk)                              │
//! │  - PctxTools class                                       │
//! │  - registerLocalTool(), addMcpServer()                   │
//! │  - listFunctions(), getFunctionDetails(), execute()      │
//! └─────────────────────┬────────────────────────────────────┘
//!                       │
//!                       ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │  Node.js Native Addon (this crate)                       │
//! │  - JsPctxTools: wraps pctx_core::PctxTools              │
//! │  - JsLocalToolRegistry: wraps LocalToolRegistry          │
//! └─────────────────────┬────────────────────────────────────┘
//!                       │
//!                       ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │  PCTX Core (pctx_core)                                   │
//! │  - MCP server management                                 │
//! │  - Local tool registry                                   │
//! │  - Code generation and execution                         │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example Usage (TypeScript)
//!
//! ```typescript
//! import { PctxTools } from '@pctx/sdk';
//!
//! const tools = new PctxTools();
//!
//! // Register an MCP server
//! await tools.addMcpServer({
//!   name: 'github',
//!   command: 'npx',
//!   args: ['-y', '@modelcontextprotocol/server-github'],
//!   env: { GITHUB_TOKEN: process.env.GITHUB_TOKEN }
//! });
//!
//! // Register a local tool
//! tools.registerLocalTool({
//!   name: 'getCurrentTime',
//!   description: 'Gets the current time',
//!   namespace: 'utils'
//! }, () => new Date().toISOString());
//!
//! // List all available functions
//! const functions = await tools.listFunctions();
//! console.log(functions.functions.map(f => `${f.namespace}.${f.name}`));
//!
//! // Get detailed information about specific functions
//! const details = await tools.getFunctionDetails({
//!   functions: ['github.createIssue', 'utils.getCurrentTime']
//! });
//!
//! // Execute TypeScript code with tool access
//! const result = await tools.execute({
//!   code: `
//!     async function run() {
//!       const time = await utils.getCurrentTime();
//!       const issue = await github.createIssue({
//!         owner: 'myorg',
//!         repo: 'myrepo',
//!         title: \`Issue created at \${time}\`
//!       });
//!       return { time, issue };
//!     }
//!   `
//! });
//! console.log(result.output);
//! ```

#![deny(clippy::all)]

use napi::{
    bindgen_prelude::*,
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
    JsFunction, JsObject,
};
use napi_derive::napi;
use pctx_code_execution_runtime::{LocalToolCallback, LocalToolMetadata, LocalToolRegistry};
use pctx_config::server::ServerConfig;
use pctx_core::{PctxTools, model::{ExecuteInput, GetFunctionDetailsInput}};
use std::sync::Arc;

// ==================== MCP Server Configuration ====================

/// Configuration for an MCP server
#[napi(object)]
pub struct McpServerConfig {
    /// Unique name for this server
    pub name: String,
    /// Command to execute (e.g., "npx", "python", "./my-server")
    pub command: String,
    /// Arguments to pass to the command
    pub args: Option<Vec<String>>,
    /// Environment variables to set
    pub env: Option<std::collections::HashMap<String, String>>,
}

impl From<McpServerConfig> for ServerConfig {
    fn from(config: McpServerConfig) -> Self {
        ServerConfig {
            name: config.name,
            command: config.command,
            args: config.args.unwrap_or_default(),
            env: config.env,
        }
    }
}

// ==================== Local Tool Options ====================

/// Options for registering a local tool
#[napi(object)]
pub struct LocalToolOptions {
    /// Name of the tool (must be unique within namespace)
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: Option<String>,
    /// JSON Schema for input validation
    pub input_schema: Option<serde_json::Value>,
    /// Namespace to organize tools (e.g., "math", "api", "db")
    pub namespace: String,
}

// ==================== Main PctxTools Class ====================

/// Main PCTX tools interface
///
/// This is the primary entry point for using PCTX from Node.js/TypeScript.
/// It provides access to all PCTX functionality including:
/// - MCP server management
/// - Local tool registration
/// - Function listing and introspection
/// - Code execution
#[napi]
pub struct JsPctxTools {
    inner: PctxTools,
}

#[napi]
impl JsPctxTools {
    /// Create a new PctxTools instance
    #[napi(constructor)]
    pub fn new() -> Self {
        tracing::debug!("Creating new PctxTools from Node.js");
        Self {
            inner: PctxTools::default(),
        }
    }

    // ==================== MCP Server Methods ====================

    /// Add an MCP server to the tools collection
    ///
    /// The server will be started and its tools will be available
    /// for listing and execution.
    ///
    /// # Example
    /// ```typescript
    /// await tools.addMcpServer({
    ///   name: 'github',
    ///   command: 'npx',
    ///   args: ['-y', '@modelcontextprotocol/server-github'],
    ///   env: { GITHUB_TOKEN: process.env.GITHUB_TOKEN }
    /// });
    /// ```
    #[napi]
    pub async fn add_mcp_server(&mut self, config: McpServerConfig) -> Result<()> {
        tracing::debug!(name = %config.name, "Adding MCP server from Node.js");

        self.inner.servers.push(config.into());
        Ok(())
    }

    /// List all configured MCP servers
    #[napi]
    pub fn list_mcp_servers(&self, env: Env) -> Result<Vec<JsObject>> {
        let mut result = Vec::new();

        for server in &self.inner.servers {
            let mut obj = env.create_object()?;
            obj.set("name", &server.name)?;
            obj.set("command", &server.command)?;
            obj.set("args", &server.args)?;

            result.push(obj);
        }

        Ok(result)
    }

    // ==================== Local Tool Methods ====================

    /// Register a local tool with a JavaScript callback
    ///
    /// The handler function will be called when the tool is executed.
    /// It can be either synchronous or async (returning a Promise).
    ///
    /// # Example
    /// ```typescript
    /// tools.registerLocalTool({
    ///   name: 'getCurrentTime',
    ///   description: 'Gets the current ISO timestamp',
    ///   namespace: 'utils'
    /// }, () => new Date().toISOString());
    /// ```
    #[napi]
    pub fn register_local_tool(
        &mut self,
        options: LocalToolOptions,
        handler: JsFunction,
    ) -> Result<()> {
        tracing::debug!(
            name = %options.name,
            namespace = %options.namespace,
            "Registering JavaScript local tool"
        );

        // Ensure we have a local registry
        if self.inner.local_registry.is_none() {
            self.inner.local_registry = Some(LocalToolRegistry::new());
        }

        let registry = self.inner.local_registry.as_ref().unwrap();

        // Create a threadsafe function that can be called from Rust threads
        let tsfn: ThreadsafeFunction<serde_json::Value, ErrorStrategy::CalleeHandled> =
            handler.create_threadsafe_function(0, |ctx| {
                // Convert Rust JSON value to JavaScript value
                ctx.env.to_js_value(&ctx.value).map(|v| vec![v])
            })?;

        // Wrap the threadsafe function in a LocalToolCallback
        let callback: LocalToolCallback = Arc::new(move |args: Option<serde_json::Value>| {
            let args_value = args.unwrap_or(serde_json::Value::Null);

            // Call the JavaScript function and wait for result
            tsfn.call_with_return_value(
                    args_value,
                    ThreadsafeFunctionCallMode::Blocking,
                    |result: serde_json::Value| Ok(result),
                )
                .map_err(|e| format!("JavaScript callback failed: {e}"))
        });

        // Register the callback in the registry
        registry
            .register_callback(
                LocalToolMetadata {
                    name: options.name.clone(),
                    description: options.description,
                    input_schema: options.input_schema,
                    namespace: options.namespace,
                },
                callback,
            )
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to register tool '{}': {}", options.name, e),
                )
            })
    }

    /// Check if a local tool is registered
    #[napi]
    pub fn has_local_tool(&self, name: String) -> bool {
        self.inner
            .local_registry
            .as_ref()
            .map_or(false, |reg| reg.has(&name))
    }

    /// Delete a local tool
    #[napi]
    pub fn delete_local_tool(&mut self, name: String) -> bool {
        self.inner
            .local_registry
            .as_mut()
            .map_or(false, |reg| reg.delete(&name))
    }

    /// Clear all local tools
    #[napi]
    pub fn clear_local_tools(&mut self) {
        if let Some(reg) = &self.inner.local_registry {
            reg.clear();
        }
    }

    // ==================== Function Discovery Methods ====================

    /// List all available functions from MCP servers and local tools
    ///
    /// Returns a list of functions with their names, namespaces, and descriptions.
    /// Also includes the generated TypeScript code for importing these functions.
    ///
    /// # Example
    /// ```typescript
    /// const result = await tools.listFunctions();
    /// console.log(result.functions); // Array of { namespace, name, description }
    /// console.log(result.code); // TypeScript import code
    /// ```
    #[napi]
    pub async fn list_functions(&self, env: Env) -> Result<JsObject> {
        tracing::debug!("Listing functions from Node.js");

        let output = self.inner.list_functions();

        let mut result = env.create_object()?;

        // Convert functions array
        let mut functions_arr = env.create_array(0)?;
        for (idx, func) in output.functions.iter().enumerate() {
            let mut func_obj = env.create_object()?;
            func_obj.set("namespace", &func.namespace)?;
            func_obj.set("name", &func.name)?;
            func_obj.set("description", &func.description)?;
            functions_arr.set(idx as u32, func_obj)?;
        }

        result.set("functions", functions_arr)?;
        result.set("code", output.code)?;

        Ok(result)
    }

    /// Get detailed information about specific functions
    ///
    /// Returns full TypeScript type definitions and signatures for the requested functions.
    ///
    /// # Arguments
    /// * `input` - Object with `functions` array of function IDs in format "namespace.name"
    ///
    /// # Example
    /// ```typescript
    /// const details = await tools.getFunctionDetails({
    ///   functions: ['github.createIssue', 'utils.getCurrentTime']
    /// });
    /// console.log(details.functions[0].inputType); // TypeScript input type
    /// console.log(details.functions[0].outputType); // TypeScript output type
    /// console.log(details.code); // Full TypeScript definitions
    /// ```
    #[napi]
    pub async fn get_function_details(&self, env: Env, input: JsObject) -> Result<JsObject> {
        tracing::debug!("Getting function details from Node.js");

        // Parse input
        let functions_arr: Vec<String> = input.get("functions")?.unwrap();

        let input_data = GetFunctionDetailsInput {
            functions: functions_arr
                .iter()
                .map(|s| {
                    let parts: Vec<&str> = s.splitn(2, '.').collect();
                    if parts.len() != 2 {
                        return Err(Error::new(
                            Status::InvalidArg,
                            format!("Invalid function ID format: '{}'. Expected 'namespace.name'", s),
                        ));
                    }
                    Ok(pctx_core::model::FunctionId {
                        mod_name: parts[0].to_string(),
                        fn_name: parts[1].to_string(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        };

        let output = self
            .inner
            .get_function_details(input_data)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

        let mut result = env.create_object()?;

        // Convert functions array
        let mut functions_arr = env.create_array(0)?;
        for (idx, func) in output.functions.iter().enumerate() {
            let mut func_obj = env.create_object()?;
            func_obj.set("namespace", &func.listed.namespace)?;
            func_obj.set("name", &func.listed.name)?;
            func_obj.set("description", &func.listed.description)?;
            func_obj.set("inputType", &func.input_type)?;
            func_obj.set("outputType", &func.output_type)?;
            func_obj.set("types", &func.types)?;
            functions_arr.set(idx as u32, func_obj)?;
        }

        result.set("functions", functions_arr)?;
        result.set("code", output.code)?;

        Ok(result)
    }

    // ==================== Code Execution ====================

    /// Execute TypeScript code with full access to tools
    ///
    /// The code must define an async `run()` function that returns a value.
    /// All registered MCP servers and local tools will be available.
    ///
    /// # Arguments
    /// * `input` - Object with `code` property containing the TypeScript code
    ///
    /// # Returns
    /// Object with execution result including:
    /// - `success`: Whether execution succeeded
    /// - `output`: The return value from `run()`
    /// - `stdout`: Standard output
    /// - `stderr`: Standard error
    ///
    /// # Example
    /// ```typescript
    /// const result = await tools.execute({
    ///   code: `
    ///     async function run() {
    ///       const time = await utils.getCurrentTime();
    ///       return { message: 'Current time is ' + time };
    ///     }
    ///   `
    /// });
    /// console.log(result.output); // { message: 'Current time is ...' }
    /// ```
    #[napi]
    pub async fn execute(&self, env: Env, input: JsObject) -> Result<JsObject> {
        tracing::debug!("Executing code from Node.js");

        let code: String = input.get("code")?.unwrap();

        let input_data = ExecuteInput { code };

        let output = self
            .inner
            .execute(input_data)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;

        let mut result = env.create_object()?;
        result.set("success", output.success)?;
        result.set("stdout", output.stdout)?;
        result.set("stderr", output.stderr)?;

        if let Some(output_value) = output.output {
            result.set("output", env.to_js_value(&output_value)?)?;
        } else {
            result.set("output", env.get_null()?)?;
        }

        Ok(result)
    }

    // ==================== Utility Methods ====================

    /// Get the internal local tool registry (for advanced use cases)
    ///
    /// This allows direct access to the underlying Rust registry.
    #[napi]
    pub fn get_local_registry(&self) -> Option<External<LocalToolRegistry>> {
        self.inner
            .local_registry
            .as_ref()
            .map(|reg| External::new(reg.clone()))
    }
}
