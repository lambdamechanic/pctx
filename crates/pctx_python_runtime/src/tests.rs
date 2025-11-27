//! Tests for Python callback functionality
//!
//! Python callbacks are tested end-to-end via the execute() API in deno_executor.
//! See deno_executor/src/tests/integration.rs for integration tests.

use super::*;
use pctx_code_execution_runtime::CallbackRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_register_and_execute_simple_lambda() {
    let registry = PythonCallbackRegistry::new();

    registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "add".to_string(),
                description: Some("Adds two numbers".to_string()),
                input_schema: None,
                namespace: "TestTools".to_string(),
            },
            runtime: CallbackRuntime::Python,
            callback_data: "lambda args: args['a'] + args['b']".to_string(),
        })
        .expect("Failed to register callback");

    assert!(registry.has("add"));

    let result = execute_python_tool(&registry, "add", Some(serde_json::json!({"a": 5, "b": 3})))
        .expect("Failed to execute callback");

    assert_eq!(result, serde_json::json!(8));
}

#[test]
#[serial]
fn test_register_duplicate_tool() {
    let registry = PythonCallbackRegistry::new();

    registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "duplicate".to_string(),
                description: None,
                input_schema: None,
                namespace: "TestTools".to_string(),
            },
            runtime: CallbackRuntime::Python,
            callback_data: "lambda args: 1".to_string(),
        })
        .unwrap();

    let result = registry.register(LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "duplicate".to_string(),
            description: None,
            input_schema: None,
            namespace: "TestTools".to_string(),
        },
        runtime: CallbackRuntime::Python,
        callback_data: "lambda args: 2".to_string(),
    });

    assert!(result.is_err());
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
