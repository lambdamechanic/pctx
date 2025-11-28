//! # PCTX Python SDK
//!
//! Python bindings for the complete PCTX toolkit.
//!
//! This crate provides a Python module that allows Python code to:
//! - Register MCP servers
//! - Register local tool callbacks
//! - List available functions from all sources
//! - Get detailed function information
//! - Execute TypeScript code with full tool access
//!
//! ## Features
//!
//! - **Complete PCTX API**: All functionality from `pctx_core::PctxTools`
//! - **Native Performance**: Direct FFI calls via PyO3
//! - **Pythonic API**: Idiomatic Python interfaces with type hints
//! - **Async Support**: Compatible with asyncio
//! - **Type Safety**: Full type stubs for IDE support
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  Python SDK (pctx package)                               │
//! │  - PctxTools class                                       │
//! │  - register_local_tool(), add_mcp_server()              │
//! │  - list_functions(), get_function_details(), execute()   │
//! └─────────────────────┬────────────────────────────────────┘
//!                       │
//!                       ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │  Python Extension Module (this crate)                    │
//! │  - PyPctxTools: wraps pctx_core::PctxTools              │
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
//! ## Example Usage (Python)
//!
//! ```python
//! from pctx import PctxTools
//! import asyncio
//!
//! async def main():
//!     tools = PctxTools()
//!
//!     # Register an MCP server
//!     await tools.add_mcp_server(
//!         name='github',
//!         command='npx',
//!         args=['-y', '@modelcontextprotocol/server-github'],
//!         env={'GITHUB_TOKEN': os.environ['GITHUB_TOKEN']}
//!     )
//!
//!     # Register a local tool
//!     def get_current_time(args):
//!         from datetime import datetime
//!         return datetime.now().isoformat()
//!
//!     tools.register_local_tool(
//!         name='getCurrentTime',
//!         description='Gets the current time',
//!         namespace='utils',
//!         handler=get_current_time
//!     )
//!
//!     # List all available functions
//!     functions = await tools.list_functions()
//!     print([f"{f['namespace']}.{f['name']}" for f in functions['functions']])
//!
//!     # Get detailed information
//!     details = await tools.get_function_details(
//!         functions=['github.createIssue', 'utils.getCurrentTime']
//!     )
//!
//!     # Execute TypeScript code with tool access
//!     result = await tools.execute(code='''
//!         async function run() {
//!             const time = await utils.getCurrentTime();
//!             const issue = await github.createIssue({
//!                 owner: 'myorg',
//!                 repo: 'myrepo',
//!                 title: `Issue created at ${time}`
//!             });
//!             return { time, issue };
//!         }
//!     ''')
//!     print(result['output'])
//!
//! asyncio.run(main())
//! ```

use pctx_code_execution_runtime::{LocalToolCallback, LocalToolMetadata, LocalToolRegistry};
use pctx_config::server::ServerConfig;
use pctx_core::{PctxTools, model::{ExecuteInput, GetFunctionDetailsInput, FunctionId}};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::sync::Arc;

// ==================== Helper Functions ====================

/// Convert Python dict/None to serde_json::Value
fn py_to_json(py: Python, obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    let json_module = py.import("json")?;
    let json_str: String = json_module
        .call_method1("dumps", (obj,))?
        .extract()?;
    serde_json::from_str(&json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON conversion failed: {}", e)))
}

/// Convert serde_json::Value to Python object
fn json_to_py(py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON serialization failed: {}", e)))?;
    let json_module = py.import("json")?;
    json_module
        .call_method1("loads", (json_str,))?
        .extract()
}

// ==================== Main PctxTools Class ====================

/// Main PCTX tools interface for Python
///
/// This is the primary entry point for using PCTX from Python.
/// It provides access to all PCTX functionality.
#[pyclass(name = "PctxTools")]
pub struct PyPctxTools {
    inner: PctxTools,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyPctxTools {
    /// Create a new PctxTools instance
    #[new]
    fn new() -> PyResult<Self> {
        tracing::debug!("Creating new PctxTools from Python");

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create async runtime: {}", e)
            ))?;

        Ok(Self {
            inner: PctxTools::default(),
            runtime,
        })
    }

    // ==================== MCP Server Methods ====================

    /// Add an MCP server to the tools collection
    ///
    /// Args:
    ///     name: Unique name for this server
    ///     command: Command to execute (e.g., "npx", "python")
    ///     args: Arguments to pass to the command (optional)
    ///     env: Environment variables to set (optional)
    ///
    /// Example:
    ///     >>> tools.add_mcp_server(
    ///     ...     name='github',
    ///     ...     command='npx',
    ///     ...     args=['-y', '@modelcontextprotocol/server-github'],
    ///     ...     env={'GITHUB_TOKEN': os.environ['GITHUB_TOKEN']}
    ///     ... )
    #[pyo3(signature = (name, command, args=None, env=None))]
    fn add_mcp_server(
        &mut self,
        name: String,
        command: String,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        tracing::debug!(name = %name, "Adding MCP server from Python");

        self.inner.servers.push(ServerConfig {
            name,
            command,
            args: args.unwrap_or_default(),
            env,
        });

        Ok(())
    }

    /// List all configured MCP servers
    ///
    /// Returns:
    ///     List of server configurations
    fn list_mcp_servers(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);

        for server in &self.inner.servers {
            let dict = PyDict::new(py);
            dict.set_item("name", &server.name)?;
            dict.set_item("command", &server.command)?;
            dict.set_item("args", &server.args)?;
            list.append(dict)?;
        }

        Ok(list.into())
    }

    // ==================== Local Tool Methods ====================

    /// Register a local tool with a Python callback
    ///
    /// Args:
    ///     name: Name of the tool (must be unique within namespace)
    ///     handler: Python callable to execute when tool is called
    ///     namespace: Namespace to organize tools (e.g., "math", "api")
    ///     description: Human-readable description (optional)
    ///     input_schema: JSON Schema for input validation (optional)
    ///
    /// Example:
    ///     >>> def get_time(args):
    ///     ...     from datetime import datetime
    ///     ...     return datetime.now().isoformat()
    ///     >>> tools.register_local_tool(
    ///     ...     name='getCurrentTime',
    ///     ...     handler=get_time,
    ///     ...     namespace='utils',
    ///     ...     description='Gets current time'
    ///     ... )
    #[pyo3(signature = (name, handler, namespace, description=None, input_schema=None))]
    fn register_local_tool(
        &mut self,
        py: Python,
        name: String,
        handler: PyObject,
        namespace: String,
        description: Option<String>,
        input_schema: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        tracing::debug!(name = %name, namespace = %namespace, "Registering Python local tool");

        // Ensure we have a local registry
        if self.inner.local_registry.is_none() {
            self.inner.local_registry = Some(LocalToolRegistry::new());
        }

        let registry = self.inner.local_registry.as_ref().unwrap();

        // Convert input schema if provided
        let schema = input_schema
            .map(|s| py_to_json(py, s))
            .transpose()?;

        // Wrap the Python callback
        let callback: LocalToolCallback = Arc::new(move |args: Option<serde_json::Value>| {
            Python::with_gil(|py| {
                let func = handler.bind(py);

                // Convert args to Python
                let py_args = match args {
                    Some(json_val) => json_to_py(py, &json_val)
                        .map_err(|e| format!("Failed to convert args: {}", e))?,
                    None => py.None(),
                };

                // Call the Python function
                let result = func
                    .call1((py_args,))
                    .map_err(|e| format!("Python callback failed: {}", e))?;

                // Convert result back to JSON
                py_to_json(py, result.as_any())
                    .map_err(|e| format!("Failed to convert result: {}", e))
            })
        });

        // Register the callback
        registry
            .register_callback(
                LocalToolMetadata {
                    name: name.clone(),
                    description,
                    input_schema: schema,
                    namespace,
                },
                callback,
            )
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Failed to register tool '{}': {}", name, e)
            ))
    }

    /// Check if a local tool is registered
    fn has_local_tool(&self, name: String) -> bool {
        self.inner
            .local_registry
            .as_ref()
            .map_or(false, |reg| reg.has(&name))
    }

    /// Delete a local tool
    fn delete_local_tool(&mut self, name: String) -> bool {
        self.inner
            .local_registry
            .as_mut()
            .map_or(false, |reg| reg.delete(&name))
    }

    /// Clear all local tools
    fn clear_local_tools(&mut self) {
        if let Some(reg) = &self.inner.local_registry {
            reg.clear();
        }
    }

    // ==================== Function Discovery Methods ====================

    /// List all available functions from MCP servers and local tools
    ///
    /// Returns:
    ///     Dict with 'functions' (list of function info) and 'code' (TypeScript imports)
    ///
    /// Example:
    ///     >>> result = await tools.list_functions()
    ///     >>> print([f"{f['namespace']}.{f['name']}" for f in result['functions']])
    fn list_functions(&self, py: Python) -> PyResult<PyObject> {
        tracing::debug!("Listing functions from Python");

        let output = self.inner.list_functions();

        let dict = PyDict::new(py);

        // Convert functions array
        let functions = PyList::empty(py);
        for func in output.functions {
            let func_dict = PyDict::new(py);
            func_dict.set_item("namespace", func.namespace)?;
            func_dict.set_item("name", func.name)?;
            func_dict.set_item("description", func.description)?;
            functions.append(func_dict)?;
        }

        dict.set_item("functions", functions)?;
        dict.set_item("code", output.code)?;

        Ok(dict.into())
    }

    /// Get detailed information about specific functions
    ///
    /// Args:
    ///     functions: List of function IDs in format "namespace.name"
    ///
    /// Returns:
    ///     Dict with 'functions' (detailed info) and 'code' (TypeScript definitions)
    ///
    /// Example:
    ///     >>> details = await tools.get_function_details(
    ///     ...     functions=['github.createIssue', 'utils.getCurrentTime']
    ///     ... )
    ///     >>> print(details['functions'][0]['inputType'])
    fn get_function_details(&self, py: Python, functions: Vec<String>) -> PyResult<PyObject> {
        tracing::debug!("Getting function details from Python");

        let function_ids: Vec<FunctionId> = functions
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.splitn(2, '.').collect();
                if parts.len() != 2 {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        format!("Invalid function ID format: '{}'. Expected 'namespace.name'", s)
                    ));
                }
                Ok(FunctionId {
                    mod_name: parts[0].to_string(),
                    fn_name: parts[1].to_string(),
                })
            })
            .collect::<PyResult<Vec<_>>>()?;

        let input_data = GetFunctionDetailsInput {
            functions: function_ids,
        };

        let output = self.runtime.block_on(async {
            self.inner
                .get_function_details(input_data)
                .await
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;

        let dict = PyDict::new(py);

        // Convert functions array
        let functions_list = PyList::empty(py);
        for func in output.functions {
            let func_dict = PyDict::new(py);
            func_dict.set_item("namespace", func.listed.namespace)?;
            func_dict.set_item("name", func.listed.name)?;
            func_dict.set_item("description", func.listed.description)?;
            func_dict.set_item("inputType", func.input_type)?;
            func_dict.set_item("outputType", func.output_type)?;
            func_dict.set_item("types", func.types)?;
            functions_list.append(func_dict)?;
        }

        dict.set_item("functions", functions_list)?;
        dict.set_item("code", output.code)?;

        Ok(dict.into())
    }

    // ==================== Code Execution ====================

    /// Execute TypeScript code with full access to tools
    ///
    /// The code must define an async `run()` function that returns a value.
    /// All registered MCP servers and local tools will be available.
    ///
    /// Args:
    ///     code: TypeScript code to execute
    ///
    /// Returns:
    ///     Dict with execution result:
    ///     - success: Whether execution succeeded
    ///     - output: The return value from run()
    ///     - stdout: Standard output
    ///     - stderr: Standard error
    ///
    /// Example:
    ///     >>> result = await tools.execute(code='''
    ///     ...     async function run() {
    ///     ...         const time = await utils.getCurrentTime();
    ///     ...         return { message: 'Current time is ' + time };
    ///     ...     }
    ///     ... ''')
    ///     >>> print(result['output'])
    fn execute(&self, py: Python, code: String) -> PyResult<PyObject> {
        tracing::debug!("Executing code from Python");

        let input_data = ExecuteInput { code };

        let output = self.runtime.block_on(async {
            self.inner.execute(input_data).await
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))?;

        let dict = PyDict::new(py);
        dict.set_item("success", output.success)?;
        dict.set_item("stdout", output.stdout)?;
        dict.set_item("stderr", output.stderr)?;

        if let Some(output_value) = output.output {
            dict.set_item("output", json_to_py(py, &output_value)?)?;
        } else {
            dict.set_item("output", py.None())?;
        }

        Ok(dict.into())
    }
}

// ==================== Python Module ====================

/// PCTX Python SDK
///
/// Complete toolkit for working with MCP servers and local tools.
#[pymodule]
fn pctx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPctxTools>()?;
    Ok(())
}
