//! Tests for Python callback functionality
//!
//! Python callbacks are tested end-to-end via the execute() API in deno_executor.
//! See deno_executor/src/tests/integration.rs for integration tests.

use super::*;
use pyo3::ffi::c_str;
use serial_test::serial;

#[test]
#[serial]
fn test_register_and_execute_simple_lambda() {
    let registry = PythonCallbackRegistry::new();

    // Register a Python callable directly (no code compilation)
    Python::with_gil(|py| {
        let func = py
            .eval(c_str!("lambda args: args['a'] + args['b']"), None, None)
            .expect("Failed to create lambda");

        registry
            .register_callable(
                LocalToolMetadata {
                    name: "add".to_string(),
                    description: Some("Adds two numbers".to_string()),
                    input_schema: None,
                    namespace: "TestTools".to_string(),
                },
                func.unbind(),
            )
            .expect("Failed to register callback");
    });

    assert!(registry.has("add"));

    let result = execute_python_tool(&registry, "add", Some(serde_json::json!({"a": 5, "b": 3})))
        .expect("Failed to execute callback");

    assert_eq!(result, serde_json::json!(8));
}

#[test]
#[serial]
fn test_register_duplicate_tool() {
    let registry = PythonCallbackRegistry::new();

    Python::with_gil(|py| {
        let func1 = py.eval(c_str!("lambda args: 1"), None, None).unwrap();

        registry
            .register_callable(
                LocalToolMetadata {
                    name: "duplicate".to_string(),
                    description: None,
                    input_schema: None,
                    namespace: "TestTools".to_string(),
                },
                func1.unbind(),
            )
            .unwrap();

        let func2 = py.eval(c_str!("lambda args: 2"), None, None).unwrap();

        let result = registry.register_callable(
            LocalToolMetadata {
                name: "duplicate".to_string(),
                description: None,
                input_schema: None,
                namespace: "TestTools".to_string(),
            },
            func2.unbind(),
        );

        assert!(result.is_err());
    });
}

#[test]
#[serial]
fn test_execute_nonexistent_tool() {
    let registry = PythonCallbackRegistry::new();
    let result = execute_python_tool(&registry, "nonexistent", None);

    assert!(result.is_err());
    match result {
        Err(PythonRuntimeError::ToolNotFound(name)) => {
            assert_eq!(name, "nonexistent");
        }
        _ => panic!("Expected ToolNotFound error"),
    }
}
