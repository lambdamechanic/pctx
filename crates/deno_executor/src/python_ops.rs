//! Deno ops for executing Python callbacks
//!
//! These ops allow TypeScript code running in Deno to execute Python callbacks
//! registered in the PythonCallbackRegistry.

use deno_core::{OpState, op2};
use pctx_python_runtime::{LocalToolMetadata, PythonCallbackRegistry};

/// Error type for Python callback operations
#[derive(Debug, thiserror::Error)]
pub(crate) enum PythonCallbackError {
    #[error("Python callback registry not available")]
    RegistryNotAvailable,

    #[error("Python callback execution failed: {0}")]
    ExecutionFailed(String),
}

impl deno_error::JsErrorClass for PythonCallbackError {
    fn get_class(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Error")
    }

    fn get_message(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
    }

    fn get_additional_properties(
        &self,
    ) -> Box<
        dyn Iterator<Item = (std::borrow::Cow<'static, str>, deno_error::PropertyValue)> + 'static,
    > {
        Box::new(std::iter::empty())
    }

    fn get_ref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }
}

/// Execute a Python callback by name
///
/// This op bridges from TypeScript to Python, allowing the Deno runtime
/// to invoke Python callbacks that were registered externally.
#[op2]
#[serde]
pub(crate) fn op_python_callback_execute(
    state: &mut OpState,
    #[string] name: String,
    #[serde] args: Option<serde_json::Value>,
) -> Result<serde_json::Value, PythonCallbackError> {
    let registry = state
        .try_borrow::<PythonCallbackRegistry>()
        .ok_or(PythonCallbackError::RegistryNotAvailable)?;

    registry
        .execute(&name, args)
        .map_err(|e| PythonCallbackError::ExecutionFailed(e.to_string()))
}

/// Check if a Python callback is registered
#[op2(fast)]
pub(crate) fn op_python_callback_has(state: &mut OpState, #[string] name: String) -> bool {
    state
        .try_borrow::<PythonCallbackRegistry>()
        .map(|registry| registry.has(&name))
        .unwrap_or(false)
}

/// List all registered Python callbacks
#[op2]
#[serde]
pub(crate) fn op_python_callback_list(state: &mut OpState) -> Vec<LocalToolMetadata> {
    state
        .try_borrow::<PythonCallbackRegistry>()
        .map(|registry| registry.list())
        .unwrap_or_default()
}

// Define the Python callback extension
deno_core::extension!(
    pctx_python_callbacks,
    ops = [
        op_python_callback_execute,
        op_python_callback_has,
        op_python_callback_list,
    ],
);
