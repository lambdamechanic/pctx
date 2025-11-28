use pyo3::prelude::*;

mod code_mode_py;
mod types;

use code_mode_py::PyCodeMode;

/// PCTX Code Mode - Python bindings for code execution with MCP servers and local tools
#[pymodule]
fn pctx_code_mode(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCodeMode>()?;
    m.add_class::<types::PyListedFunction>()?;
    m.add_class::<types::PyListFunctionsOutput>()?;
    m.add_class::<types::PyFunctionDetails>()?;
    m.add_class::<types::PyGetFunctionDetailsOutput>()?;
    m.add_class::<types::PyExecuteOutput>()?;
    Ok(())
}
