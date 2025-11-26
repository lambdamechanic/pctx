use super::serial;
use crate::{JsLocalToolDefinition, JsLocalToolMetadata, execute};

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

    let local_tools = vec![JsLocalToolDefinition {
        metadata: JsLocalToolMetadata {
            name: "add".to_string(),
            description: Some("Adds two numbers".to_string()),
            input_schema: None,
        },
        callback_code: "(args) => args.a + args.b".to_string(),
    }];

    let result = execute(code, None, None, Some(local_tools))
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
        JsLocalToolDefinition {
            metadata: JsLocalToolMetadata {
                name: "add".to_string(),
                description: Some("Adds two numbers".to_string()),
                input_schema: None,
            },
            callback_code: "(args) => args.a + args.b".to_string(),
        },
        JsLocalToolDefinition {
            metadata: JsLocalToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers".to_string()),
                input_schema: None,
            },
            callback_code: "(args) => args.a * args.b".to_string(),
        },
        JsLocalToolDefinition {
            metadata: JsLocalToolMetadata {
                name: "greet".to_string(),
                description: Some("Greets someone".to_string()),
                input_schema: None,
            },
            callback_code: r#"(args) => `Hello, ${args.name}!`"#.to_string(),
        },
    ];

    let result = execute(code, None, None, Some(local_tools))
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

    let local_tools = vec![JsLocalToolDefinition {
        metadata: JsLocalToolMetadata {
            name: "delayed".to_string(),
            description: Some("Returns value after async operation".to_string()),
            input_schema: None,
        },
        callback_code: r"
            async (args) => {
                await Promise.resolve();
                return args.value * 2;
            }
        "
        .to_string(),
    }];

    let result = execute(code, None, None, Some(local_tools))
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
        JsLocalToolDefinition {
            metadata: JsLocalToolMetadata {
                name: "duplicate".to_string(),
                description: None,
                input_schema: None,
            },
            callback_code: "() => 1".to_string(),
        },
        JsLocalToolDefinition {
            metadata: JsLocalToolMetadata {
                name: "duplicate".to_string(),
                description: None,
                input_schema: None,
            },
            callback_code: "() => 2".to_string(),
        },
    ];

    let result = execute(code, None, None, Some(local_tools))
        .await
        .expect("Execution should succeed");

    assert!(!result.success, "Should fail due to duplicate registration");
    assert!(result.stderr.contains("Local tool registration failed"));
}
