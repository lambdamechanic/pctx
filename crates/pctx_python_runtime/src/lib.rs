//! # PCTX Python Runtime
//!
//! Python callback execution runtime for PCTX local tools.
//!
//! This crate provides a Python runtime that integrates with the generic local tool system,
//! allowing users to register Python callbacks that can be invoked from any AI agent framework.
//!
//! ## Features
//!
//! - **Python Callback Storage**: Store and execute Python functions via pyo3
//! - **Generic Integration**: Uses the generic `LocalToolRegistry` from `pctx_code_execution_runtime`
//! - **Type Safety**: JSON serialization for arguments and return values
//! - **Error Handling**: Comprehensive error handling for Python exceptions
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pctx_python_runtime::{PythonCallbackRegistry, execute_python_tool};
//! use pctx_code_execution_runtime::{LocalToolDefinition, LocalToolMetadata};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create registry
//! let registry = PythonCallbackRegistry::new();
//!
//! // Register a Python callback
//! registry.register(LocalToolDefinition {
//!     metadata: LocalToolMetadata {
//!         name: "add".to_string(),
//!         description: Some("Adds two numbers".to_string()),
//!         input_schema: None,
//!     },
//!     callback_data: "lambda args: args['a'] + args['b']".to_string(),
//! })?;
//!
//! // Execute the callback
//! let result = execute_python_tool(
//!     &registry,
//!     "add",
//!     Some(serde_json::json!({"a": 5, "b": 3}))
//! )?;
//!
//! assert_eq!(result, serde_json::json!(8));
//! # Ok(())
//! # }
//! ```

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::debug;

pub use pctx_code_execution_runtime::{LocalToolDefinition, LocalToolMetadata, LocalToolRegistry};

#[derive(Debug, Error)]
pub enum PythonRuntimeError {
    #[error("Python error: {0}")]
    PythonError(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Registration error: {0}")]
    RegistrationError(String),
}

impl From<PyErr> for PythonRuntimeError {
    fn from(err: PyErr) -> Self {
        Python::with_gil(|_py| PythonRuntimeError::PythonError(err.to_string()))
    }
}

impl From<serde_json::Error> for PythonRuntimeError {
    fn from(err: serde_json::Error) -> Self {
        PythonRuntimeError::SerializationError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PythonRuntimeError>;

/// Stored callback with its execution environment
struct StoredCallback {
    /// The Python callable
    callback: PyObject,
    /// The globals dict containing imports and other context
    #[allow(dead_code)]
    globals: PyObject,
}

/// Registry for Python callbacks
///
/// This registry stores Python callable objects that can be invoked as local tools.
/// It integrates with the generic `LocalToolRegistry` for metadata management.
#[derive(Clone)]
pub struct PythonCallbackRegistry {
    /// Generic tool registry for metadata
    tool_registry: LocalToolRegistry,
    /// Python callbacks stored with their execution environment
    /// We use Arc<RwLock> to allow sharing across threads
    callbacks: Arc<RwLock<HashMap<String, StoredCallback>>>,
}

impl PythonCallbackRegistry {
    /// Create a new Python callback registry
    pub fn new() -> Self {
        Self {
            tool_registry: LocalToolRegistry::new(),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a Python callback from a tool definition
    ///
    /// The `callback_data` field should contain Python code that evaluates to a callable.
    /// This can be:
    /// - A lambda: `"lambda args: args['a'] + args['b']"`
    /// - A function definition: `"def add(args): return args['a'] + args['b']"`
    /// - An async function: `"async def fetch(args): ..."`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The tool is already registered
    /// - The Python code fails to compile or evaluate
    /// - The evaluated result is not callable
    pub fn register(&self, definition: LocalToolDefinition) -> Result<()> {
        debug!(
            name = %definition.metadata.name,
            "Registering Python callback"
        );

        // Compile and store the Python callback
        Python::with_gil(|py| {
            // Try to evaluate the callback_data as Python code
            let code_cstring = CString::new(definition.callback_data.as_bytes())
                .map_err(|e| PythonRuntimeError::RegistrationError(e.to_string()))?;

            // Try eval first (for simple lambdas)
            let (callback, globals) = match py.eval(code_cstring.as_c_str(), None, None) {
                Ok(obj) => {
                    // For simple eval, use empty dict as globals (no imports needed)
                    (obj.unbind(), PyDict::new(py).unbind())
                }
                Err(_) => {
                    // If eval fails, try exec (for multi-line functions with imports)
                    // Create a fresh globals dict for this callback
                    let globals = PyDict::new(py);

                    // Copy __builtins__ from main module to support imports
                    let main_module = py.import("__main__")?;
                    let main_dict = main_module.dict();
                    if let Ok(Some(builtins)) = main_dict.get_item("__builtins__") {
                        globals.set_item("__builtins__", builtins)?;
                    }

                    // Execute the code - imports will be added to globals
                    py.run(code_cstring.as_c_str(), Some(&globals), None)?;

                    // Try to extract the function by common names from globals
                    // (functions defined at module level end up in globals)
                    let func = if let Ok(Some(func)) = globals.get_item("tool") {
                        func
                    } else if let Ok(Some(func)) =
                        globals.get_item(definition.metadata.name.clone())
                    {
                        func
                    } else {
                        // Get the last defined callable from globals
                        let mut found_callable = None;
                        for (key, value) in globals.iter() {
                            let key_str = key.to_string();
                            if value.is_callable()
                                && !key_str.starts_with("__")
                                && key_str != "__builtins__"
                            {
                                found_callable = Some(value);
                            }
                        }
                        found_callable.ok_or_else(|| {
                            PythonRuntimeError::RegistrationError(
                                "callback_data did not produce a callable".to_string(),
                            )
                        })?
                    };

                    (func.unbind(), globals.unbind())
                }
            };

            // Verify it's callable
            if !callback.bind(py).is_callable() {
                return Err(PythonRuntimeError::RegistrationError(
                    "callback_data did not produce a callable".to_string(),
                ));
            }

            // Store in callbacks map with its globals
            let mut callbacks = self.callbacks.write().unwrap();
            if callbacks.contains_key(&definition.metadata.name) {
                return Err(PythonRuntimeError::RegistrationError(format!(
                    "Tool '{}' is already registered",
                    definition.metadata.name
                )));
            }
            callbacks.insert(
                definition.metadata.name.clone(),
                StoredCallback {
                    callback,
                    globals: globals.into(),
                },
            );

            // Register metadata with the generic registry
            self.tool_registry
                .register(definition)
                .map_err(|e| PythonRuntimeError::RegistrationError(e.to_string()))?;

            Ok(())
        })
    }

    /// Get the generic tool registry for metadata access
    pub fn tool_registry(&self) -> &LocalToolRegistry {
        &self.tool_registry
    }

    /// Check if a tool is registered
    pub fn has(&self, name: &str) -> bool {
        self.tool_registry.has(name)
    }

    /// List all registered tools (metadata only)
    pub fn list(&self) -> Vec<LocalToolMetadata> {
        self.tool_registry.list()
    }

    /// Get all registered tools as LocalToolDefinitions
    ///
    /// This allows extracting the tools from the Python registry
    /// to pass them to a unified local tools API
    pub fn list_tools(&self) -> Vec<LocalToolDefinition> {
        self.tool_registry.get_pre_registered()
    }

    /// Execute a Python callback by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to execute
    /// * `args` - Optional JSON arguments to pass to the callback
    ///
    /// # Returns
    ///
    /// The result of the callback execution as a JSON value
    pub fn execute(&self, name: &str, args: Option<Value>) -> Result<Value> {
        debug!(name = %name, "Executing Python callback");

        Python::with_gil(|py| {
            // Get the callback with its globals
            let callbacks = self.callbacks.read().unwrap();
            let stored = callbacks
                .get(name)
                .ok_or_else(|| PythonRuntimeError::ToolNotFound(name.to_string()))?;

            // Convert args to Python dict
            let py_args = if let Some(args) = args {
                let args_str = serde_json::to_string(&args)?;
                let json_module = PyModule::import(py, "json")?;
                json_module.call_method1("loads", (args_str,))?
            } else {
                PyDict::new(py).into_any()
            };

            // Call the Python function
            let result = stored.callback.call1(py, (py_args,))?;

            // Convert result back to JSON
            let json_module = PyModule::import(py, "json")?;
            let json_str: String = json_module.call_method1("dumps", (result,))?.extract()?;
            let value = serde_json::from_str(&json_str)?;

            Ok(value)
        })
    }
}

impl Default for PythonCallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PythonCallbackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythonCallbackRegistry")
            .field("tool_registry", &self.tool_registry)
            .field(
                "callbacks",
                &format!("<{} callbacks>", self.callbacks.read().unwrap().len()),
            )
            .finish()
    }
}

/// Execute a Python tool by name using the registry
///
/// This is a convenience function for executing a registered Python callback.
pub fn execute_python_tool(
    registry: &PythonCallbackRegistry,
    name: &str,
    args: Option<Value>,
) -> Result<Value> {
    registry.execute(name, args)
}

#[cfg(test)]
mod tests;
