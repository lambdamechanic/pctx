use pyo3::prelude::*;
use serde_json::Value;

/// A listed function with namespace, name, and description
#[pyclass(name = "ListedFunction")]
#[derive(Clone)]
pub struct PyListedFunction {
    #[pyo3(get)]
    pub namespace: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: Option<String>,
}

#[pymethods]
impl PyListedFunction {
    fn __repr__(&self) -> String {
        format!(
            "ListedFunction(namespace='{}', name='{}', description={})",
            self.namespace,
            self.name,
            self.description
                .as_ref()
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "None".to_string())
        )
    }
}

impl From<pctx_code_mode::model::ListedFunction> for PyListedFunction {
    fn from(f: pctx_code_mode::model::ListedFunction) -> Self {
        Self {
            namespace: f.namespace,
            name: f.name,
            description: f.description,
        }
    }
}

/// Output from list_functions
#[pyclass(name = "ListFunctionsOutput")]
pub struct PyListFunctionsOutput {
    #[pyo3(get)]
    pub functions: Vec<PyListedFunction>,
    #[pyo3(get)]
    pub code: String,
}

#[pymethods]
impl PyListFunctionsOutput {
    fn __repr__(&self) -> String {
        format!(
            "ListFunctionsOutput(functions=[{} items], code='{}')",
            self.functions.len(),
            if self.code.len() > 50 {
                format!("{}...", &self.code[..50])
            } else {
                self.code.clone()
            }
        )
    }
}

impl From<pctx_code_mode::model::ListFunctionsOutput> for PyListFunctionsOutput {
    fn from(output: pctx_code_mode::model::ListFunctionsOutput) -> Self {
        Self {
            functions: output.functions.into_iter().map(Into::into).collect(),
            code: output.code,
        }
    }
}

/// Detailed function information including types
#[pyclass(name = "FunctionDetails")]
#[derive(Clone)]
pub struct PyFunctionDetails {
    #[pyo3(get)]
    pub namespace: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: Option<String>,
    #[pyo3(get)]
    pub input_type: String,
    #[pyo3(get)]
    pub output_type: String,
    #[pyo3(get)]
    pub types: String,
}

#[pymethods]
impl PyFunctionDetails {
    fn __repr__(&self) -> String {
        format!(
            "FunctionDetails(namespace='{}', name='{}')",
            self.namespace, self.name
        )
    }
}

impl From<pctx_code_mode::model::FunctionDetails> for PyFunctionDetails {
    fn from(f: pctx_code_mode::model::FunctionDetails) -> Self {
        Self {
            namespace: f.listed.namespace,
            name: f.listed.name,
            description: f.listed.description,
            input_type: f.input_type,
            output_type: f.output_type,
            types: f.types,
        }
    }
}

/// Output from get_function_details
#[pyclass(name = "GetFunctionDetailsOutput")]
pub struct PyGetFunctionDetailsOutput {
    #[pyo3(get)]
    pub functions: Vec<PyFunctionDetails>,
    #[pyo3(get)]
    pub code: String,
}

#[pymethods]
impl PyGetFunctionDetailsOutput {
    fn __repr__(&self) -> String {
        format!(
            "GetFunctionDetailsOutput(functions=[{} items])",
            self.functions.len()
        )
    }
}

impl From<pctx_code_mode::model::GetFunctionDetailsOutput> for PyGetFunctionDetailsOutput {
    fn from(output: pctx_code_mode::model::GetFunctionDetailsOutput) -> Self {
        Self {
            functions: output.functions.into_iter().map(Into::into).collect(),
            code: output.code,
        }
    }
}

/// Output from execute
#[pyclass(name = "ExecuteOutput")]
pub struct PyExecuteOutput {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub stdout: String,
    #[pyo3(get)]
    pub stderr: String,
    pub output: Option<Value>,
}

#[pymethods]
impl PyExecuteOutput {
    #[getter]
    fn output<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        match &self.output {
            Some(value) => {
                let py_value = pythonize::pythonize(py, value)?;
                Ok(Some(py_value))
            }
            None => Ok(None),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecuteOutput(success={}, stdout='{}', stderr='{}')",
            self.success,
            if self.stdout.len() > 30 {
                format!("{}...", &self.stdout[..30])
            } else {
                self.stdout.clone()
            },
            if self.stderr.len() > 30 {
                format!("{}...", &self.stderr[..30])
            } else {
                self.stderr.clone()
            }
        )
    }
}

impl From<pctx_code_mode::model::ExecuteOutput> for PyExecuteOutput {
    fn from(output: pctx_code_mode::model::ExecuteOutput) -> Self {
        Self {
            success: output.success,
            stdout: output.stdout,
            stderr: output.stderr,
            output: output.output,
        }
    }
}
