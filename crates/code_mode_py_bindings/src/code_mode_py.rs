use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;

use pctx_code_execution_runtime::{CallableToolMetadata, CallableToolRegistry, LocalToolCallback};
use pctx_code_mode::CodeMode;
use pctx_config::server::ServerConfig;

use crate::types::*;

/// Python wrapper for CodeMode
#[pyclass(name = "CodeMode")]
pub struct PyCodeMode {
    inner: Arc<tokio::sync::RwLock<CodeMode>>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyCodeMode {
    #[new]
    #[pyo3(signature = (mcp_servers=None, local_tools=None))]
    fn new(
        _py: Python<'_>,
        mcp_servers: Option<Bound<'_, PyList>>,
        local_tools: Option<Bound<'_, PyList>>,
    ) -> PyResult<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let mut code_mode = CodeMode::default();

        // Register local tools if provided
        if let Some(tools_list) = local_tools {
            let registry = code_mode
                .callable_registry
                .get_or_insert_with(CallableToolRegistry::new);

            for item in tools_list.iter() {
                let tool_dict = item.downcast::<PyDict>()?;
                Self::register_tool_from_dict(registry, tool_dict)?;
            }
        }

        let code_mode = Arc::new(tokio::sync::RwLock::new(code_mode));

        // Add MCP servers if provided
        if let Some(servers_list) = mcp_servers {
            for item in servers_list.iter() {
                let server_dict = item.downcast::<PyDict>()?;
                let name = server_dict
                    .get_item("name")?
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("Server missing 'name'")
                    })?
                    .extract::<String>()?;
                let url_str = server_dict
                    .get_item("url")?
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("Server missing 'url'")
                    })?
                    .extract::<String>()?;

                let url = url_str.parse::<url::Url>().map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {}", e))
                })?;

                let server_config = ServerConfig::new(name.clone(), url);

                let code_mode_clone = Arc::clone(&code_mode);
                runtime.block_on(async move {
                    let mut cm = code_mode_clone.write().await;
                    cm.add_server(&server_config).await.map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to add server: {}",
                            e
                        ))
                    })
                })?;
            }
        }

        Ok(Self {
            inner: code_mode,
            runtime: Arc::new(runtime),
        })
    }

    /// Register a local tool
    #[pyo3(signature = (namespace, name, callback, description=None, input_schema=None))]
    fn register_local_tool(
        &self,
        namespace: String,
        name: String,
        callback: PyObject,
        description: Option<String>,
        input_schema: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let schema_value = if let Some(schema_dict) = input_schema {
            Some(pythonize::depythonize(&schema_dict).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to convert input_schema: {}",
                    e
                ))
            })?)
        } else {
            None
        };

        let metadata = CallableToolMetadata {
            name: name.clone(),
            description,
            input_schema: schema_value,
            namespace,
        };

        // Create a Rust closure that calls the Python callback
        let py_callback: LocalToolCallback = {
            Arc::new(move |args: Option<serde_json::Value>| {
                Python::with_gil(|py| {
                    let py_args = if let Some(args_value) = args {
                        pythonize::pythonize(py, &args_value)
                            .map_err(|e| format!("Failed to convert args to Python: {}", e))?
                    } else {
                        py.None().into_bound(py)
                    };

                    let result = callback
                        .call1(py, (py_args,))
                        .map_err(|e| format!("Python callback failed: {}", e))?;

                    let json_result: serde_json::Value = pythonize::depythonize(result.bind(py))
                        .map_err(|e| format!("Failed to convert Python result to JSON: {}", e))?;

                    Ok(json_result)
                })
            })
        };

        // Register the callback
        self.runtime.block_on(async {
            let mut cm = self.inner.write().await;
            let registry = cm
                .callable_registry
                .get_or_insert_with(CallableToolRegistry::new);

            registry
                .register_callback(metadata, py_callback)
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to register tool: {}",
                        e
                    ))
                })
        })
    }

    /// Add an MCP server
    fn add_mcp_server<'py>(
        &self,
        py: Python<'py>,
        name: String,
        url_str: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let url = url_str.parse::<url::Url>().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {}", e))
            })?;

            let server_config = ServerConfig::new(name, url);

            let mut cm = inner.write().await;
            cm.add_server(&server_config).await.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to add server: {}",
                    e
                ))
            })?;

            Ok(())
        })
    }

    /// List all available functions
    fn list_functions(&self, _py: Python<'_>) -> PyResult<PyListFunctionsOutput> {
        self.runtime.block_on(async {
            let cm = self.inner.read().await;
            let output = cm.list_functions();
            Ok(output.into())
        })
    }

    /// Get detailed information about specific functions
    fn get_function_details(
        &self,
        _py: Python<'_>,
        functions: Vec<String>,
    ) -> PyResult<PyGetFunctionDetailsOutput> {
        self.runtime.block_on(async {
            let cm = self.inner.read().await;

            // Parse function IDs (namespace.name format)
            let function_ids: Vec<pctx_code_mode::model::FunctionId> = functions
                .iter()
                .map(|s| {
                    let parts: Vec<&str> = s.splitn(2, '.').collect();
                    if parts.len() != 2 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Invalid function format '{}'. Expected 'namespace.name'",
                            s
                        )));
                    }
                    Ok(pctx_code_mode::model::FunctionId {
                        mod_name: parts[0].to_string(),
                        fn_name: parts[1].to_string(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let input = pctx_code_mode::model::GetFunctionDetailsInput {
                functions: function_ids,
            };

            let output = cm.get_function_details(input);
            Ok(output.into())
        })
    }

    /// Execute TypeScript code - returns a coroutine
    fn execute<'py>(&self, py: Python<'py>, code: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            // We need to spawn_blocking because Deno execution is not Send
            let output = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    let cm = inner.read().await;
                    let input = pctx_code_mode::model::ExecuteInput { code };
                    cm.execute(input).await
                })
            })
            .await
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Task join failed: {}",
                    e
                ))
            })?
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Execution failed: {}",
                    e
                ))
            })?;

            Python::with_gil(|py| {
                let py_output: PyExecuteOutput = output.into();
                Ok(Py::new(py, py_output)?.into_any())
            })
        })
    }
}

impl PyCodeMode {
    /// Helper to register a tool from a Python dict
    fn register_tool_from_dict(
        registry: &CallableToolRegistry,
        tool_dict: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let namespace = tool_dict
            .get_item("namespace")?
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Tool missing 'namespace'")
            })?
            .extract::<String>()?;

        let name = tool_dict
            .get_item("name")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Tool missing 'name'"))?
            .extract::<String>()?;

        let callback = tool_dict
            .get_item("callback")?
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Tool missing 'callback'")
            })?
            .unbind();

        let description = tool_dict
            .get_item("description")?
            .map(|v| v.extract::<String>())
            .transpose()?;

        let input_schema = tool_dict
            .get_item("input_schema")?
            .map(|schema| pythonize::depythonize(&schema))
            .transpose()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to convert input_schema: {}",
                    e
                ))
            })?;

        let metadata = CallableToolMetadata {
            name: name.clone(),
            description,
            input_schema,
            namespace,
        };

        // Create a Rust closure that calls the Python callback
        let py_callback: LocalToolCallback = {
            Arc::new(move |args: Option<serde_json::Value>| {
                Python::with_gil(|py| {
                    let py_args = if let Some(args_value) = args {
                        pythonize::pythonize(py, &args_value)
                            .map_err(|e| format!("Failed to convert args to Python: {}", e))?
                    } else {
                        py.None().into_bound(py)
                    };

                    let result = callback
                        .call1(py, (py_args,))
                        .map_err(|e| format!("Python callback failed: {}", e))?;

                    let json_result: serde_json::Value = pythonize::depythonize(result.bind(py))
                        .map_err(|e| format!("Failed to convert Python result to JSON: {}", e))?;

                    Ok(json_result)
                })
            })
        };

        registry
            .register_callback(metadata, py_callback)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to register tool: {}",
                    e
                ))
            })?;

        Ok(())
    }
}
