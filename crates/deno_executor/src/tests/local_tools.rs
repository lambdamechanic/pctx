use super::serial;
use crate::{ExecuteOptions, LocalToolDefinition, LocalToolMetadata, execute};

#[tokio::test]
#[serial]
async fn test_execute_with_pre_registered_local_tool() {
    let code = r#"
        // Call a tool that was pre-registered from Rust
        export default (async () => {
            const result = await callJsLocalTool("add", { a: 5, b: 3 });
            return result;
        })();
    "#;

    let local_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "add".to_string(),
            description: Some("Adds two numbers".to_string()),
            input_schema: None,
        },
        callback_data: "(args) => args.a + args.b".to_string(),
    }];

    let result = execute(code, ExecuteOptions::new().with_local_tools(local_tools))
        .await
        .expect("Execution should succeed");

    if !result.success {
        eprintln!("Execution failed:");
        eprintln!("  stdout: {}", result.stdout);
        eprintln!("  stderr: {}", result.stderr);
        if let Some(err) = &result.runtime_error {
            eprintln!("  error: {}", err.message);
            if let Some(stack) = &err.stack {
                eprintln!("  stack: {stack}");
            }
        }
    }

    assert!(result.success, "Code should execute successfully");
    assert_eq!(result.output, Some(serde_json::json!(8)));
}

#[tokio::test]
#[serial]
async fn test_execute_with_multiple_local_tools() {
    let code = r#"
        export default (async () => {
            const sum = await callJsLocalTool("add", { a: 10, b: 5 });
            const product = await callJsLocalTool("multiply", { a: 4, b: 7 });
            const greeting = await callJsLocalTool("greet", { name: "World" });

            return { sum, product, greeting };
        })();
    "#;

    let local_tools = vec![
        LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "add".to_string(),
                description: Some("Adds two numbers".to_string()),
                input_schema: None,
            },
            callback_data: "(args) => args.a + args.b".to_string(),
        },
        LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers".to_string()),
                input_schema: None,
            },
            callback_data: "(args) => args.a * args.b".to_string(),
        },
        LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "greet".to_string(),
                description: Some("Greets someone".to_string()),
                input_schema: None,
            },
            callback_data: r"(args) => `Hello, ${args.name}!`".to_string(),
        },
    ];

    let result = execute(code, ExecuteOptions::new().with_local_tools(local_tools))
        .await
        .expect("Execution should succeed");

    assert!(result.success, "Code should execute successfully");
    let output = result.output.unwrap();
    assert_eq!(output["sum"], 15);
    assert_eq!(output["product"], 28);
    assert_eq!(output["greeting"], "Hello, World!");
}

#[tokio::test]
#[serial]
async fn test_local_tool_with_async_callback() {
    let code = r#"
        export default (async () => {
            const result = await callJsLocalTool("delayed", { value: 42 });
            return result;
        })();
    "#;

    let local_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "delayed".to_string(),
            description: Some("Returns value after async operation".to_string()),
            input_schema: None,
        },
        callback_data: r"
            async (args) => {
                await Promise.resolve();
                return args.value * 2;
            }
        "
        .to_string(),
    }];

    let result = execute(code, ExecuteOptions::new().with_local_tools(local_tools))
        .await
        .expect("Execution should succeed");

    assert!(result.success, "Code should execute successfully");
    assert_eq!(result.output, Some(serde_json::json!(84)));
}

#[tokio::test]
#[serial]
async fn test_local_tool_registration_error() {
    let code = r#"
        export default "should not run";
    "#;

    // Register two tools with the same name - should fail
    let local_tools = vec![
        LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "duplicate".to_string(),
                description: None,
                input_schema: None,
            },
            callback_data: "() => 1".to_string(),
        },
        LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "duplicate".to_string(),
                description: None,
                input_schema: None,
            },
            callback_data: "() => 2".to_string(),
        },
    ];

    let result = execute(code, ExecuteOptions::new().with_local_tools(local_tools))
        .await
        .expect("Execution should succeed");

    assert!(!result.success, "Should fail due to duplicate registration");
    assert!(result.stderr.contains("Local tool registration failed"));
}

// ============================================================================
// PYTHON CALLBACK TESTS (via unified local tool system)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_python_callback_via_local_tools() {
    use pctx_python_runtime::PythonCallbackRegistry;

    let registry = PythonCallbackRegistry::new();

    // Register a simple Python callback
    registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "add".to_string(),
                description: Some("Adds two numbers".to_string()),
                input_schema: None,
            },
            callback_data: "lambda args: args['a'] + args['b']".to_string(),
        })
        .expect("Failed to register Python callback");

    let code = r#"
        // Python callbacks are called via callPythonCallback
        export default (async () => {
            const result = await callPythonCallback("add", { a: 5, b: 3 });
            return result;
        })();
    "#;

    let result = execute(
        code,
        ExecuteOptions::new().with_python_callback_registry(registry),
    )
    .await
    .expect("Execution should succeed");

    if !result.success {
        eprintln!("Execution failed:");
        eprintln!("  stdout: {}", result.stdout);
        eprintln!("  stderr: {}", result.stderr);
        if let Some(err) = &result.runtime_error {
            eprintln!("  error: {}", err.message);
        }
    }

    assert!(result.success, "Code should execute successfully");
    assert_eq!(result.output, Some(serde_json::json!(8)));
}

#[tokio::test]
#[serial]
async fn test_mixed_js_and_python_tools() {
    use pctx_python_runtime::PythonCallbackRegistry;

    let registry = PythonCallbackRegistry::new();

    // Register Python callback
    registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers".to_string()),
                input_schema: None,
            },
            callback_data: "lambda args: args['a'] * args['b']".to_string(),
        })
        .unwrap();

    // Register JS local tool
    let js_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "subtract".to_string(),
            description: Some("Subtracts two numbers".to_string()),
            input_schema: None,
        },
        callback_data: "(args) => args.a - args.b".to_string(),
    }];

    let code = r#"
        export default (async () => {
            // Python callbacks use callPythonCallback, JS local tools use callJsLocalTool
            const product = await callPythonCallback("multiply", { a: 6, b: 7 });
            const difference = await callJsLocalTool("subtract", { a: 10, b: 3 });

            return { product, difference };
        })();
    "#;

    let result = execute(
        code,
        ExecuteOptions::new()
            .with_python_callback_registry(registry)
            .with_local_tools(js_tools),
    )
    .await
    .expect("Execution should succeed");

    assert!(result.success, "Code should execute successfully");
    let output = result.output.unwrap();
    assert_eq!(output["product"], 42);
    assert_eq!(output["difference"], 7);
}

#[tokio::test]
#[serial]
async fn test_python_callback_with_stdlib() {
    use pctx_python_runtime::PythonCallbackRegistry;

    let registry = PythonCallbackRegistry::new();

    registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "calculate_stats".to_string(),
                description: Some("Calculates statistics".to_string()),
                input_schema: None,
            },
            callback_data: r#"
import statistics

def calculate_stats(args):
    numbers = args['numbers']
    return {
        'mean': statistics.mean(numbers),
        'median': statistics.median(numbers),
        'sum': sum(numbers)
    }
"#
            .to_string(),
        })
        .unwrap();

    let code = r#"
        export default (async () => {
            const stats = await callPythonCallback("calculate_stats", {
                numbers: [1, 2, 3, 4, 5]
            });
            return stats;
        })();
    "#;

    let result = execute(
        code,
        ExecuteOptions::new().with_python_callback_registry(registry),
    )
    .await
    .expect("Execution should succeed");

    assert!(result.success, "Code should execute successfully");
    let output = result.output.unwrap();
    assert_eq!(output["mean"], 3.0);
    assert_eq!(output["median"], 3.0);
    assert_eq!(output["sum"], 15);
}
