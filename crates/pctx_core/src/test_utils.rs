//! Test utilities for pctx_core
//!
//! This module provides helper functions for testing, including Python callback wrappers.

use pctx_code_execution_runtime::LocalToolCallback;
use pyo3::{prelude::*, types::PyDict};
use std::sync::Arc;

/// Wrap a Python callable into a Rust closure for testing
///
/// This function creates a `LocalToolCallback` (Rust closure) that wraps a Python callable.
/// All `PyO3` complexity is hidden inside the closure - from the caller's perspective,
/// it's just a regular Rust function.
///
/// # Arguments
/// * `py_func` - A Python callable (function, lambda, method, etc.)
///
/// # Returns
/// A `LocalToolCallback` that can be registered in `CallableToolRegistry`
pub fn wrap_python_callback(py_func: PyObject) -> LocalToolCallback {
    Arc::new(move |args: Option<serde_json::Value>| {
        Python::with_gil(|py| {
            // Bind the PyObject to this GIL context
            let func = py_func.bind(py);

            // Convert JSON args to Python dict
            let py_args = match args {
                Some(json_val) => {
                    // Convert JSON to Python object
                    let py_str = serde_json::to_string(&json_val)
                        .map_err(|e| format!("Failed to serialize args: {e}"))?;
                    let json_module = py
                        .import("json")
                        .map_err(|e| format!("Failed to import json module: {e}"))?;
                    json_module
                        .call_method1("loads", (py_str,))
                        .map_err(|e| format!("Failed to deserialize args in Python: {e}"))?
                }
                None => PyDict::new(py).into_any(),
            };

            // Call the Python function
            let result = func
                .call1((py_args,))
                .map_err(|e| format!("Python callback failed: {e}"))?;

            // Convert Python result back to JSON
            let json_module = py
                .import("json")
                .map_err(|e| format!("Failed to import json module: {e}"))?;
            let json_str: String = json_module
                .call_method1("dumps", (result,))
                .and_then(|s| s.extract())
                .map_err(|e| format!("Failed to serialize Python result: {e}"))?;

            serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON result: {e}"))
        })
    })
}
