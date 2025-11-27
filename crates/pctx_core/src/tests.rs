use crate::{PctxTools, model::ExecuteInput};
use deno_executor::{CallbackRuntime, LocalToolDefinition, LocalToolMetadata};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_javascript_local_tool() {
    let mut tools = PctxTools::default();

    // Register a JavaScript tool
    tools.local_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "add".to_string(),
            description: Some("Adds two numbers".to_string()),
            input_schema: None,
            namespace: "LocalTools".to_string(),
        },
        runtime: CallbackRuntime::JavaScript,
        callback_data: "(args) => args.a + args.b".to_string(),
    }];

    // Now use the HIGH-LEVEL API - the generated TypeScript namespace function
    let code = r#"
        async function run() {
            return await LocalTools.add({ a: 5, b: 3 });
        }
    "#;

    let result = tools
        .execute(ExecuteInput {
            code: code.to_string(),
        })
        .await
        .expect("Execution should succeed");

    assert!(result.success);
    assert_eq!(result.output, Some(serde_json::json!(8)));
}

#[tokio::test]
#[serial]
async fn test_python_local_tool() {
    let mut tools = PctxTools::default();

    // Register a Python tool
    let python_registry = pctx_python_runtime::PythonCallbackRegistry::new();
    python_registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers".to_string()),
                input_schema: None,
                namespace: "PythonTools".to_string(),
            },
            runtime: CallbackRuntime::Python,
            callback_data: "lambda args: args['a'] * args['b']".to_string(),
        })
        .expect("Failed to register Python tool");

    tools.python_registry = Some(python_registry);

    // Use the HIGH-LEVEL API - the generated TypeScript namespace function
    let code = r#"
        async function run() {
            return await PythonTools.multiply({ a: 6, b: 7 });
        }
    "#;

    let result = tools
        .execute(ExecuteInput {
            code: code.to_string(),
        })
        .await
        .expect("Execution should succeed");

    if !result.success {
        eprintln!("Execution failed:");
        eprintln!("stderr: {}", result.stderr);
        eprintln!("stdout: {}", result.stdout);
    }

    assert!(result.success);
    assert_eq!(result.output, Some(serde_json::json!(42)));
}

#[tokio::test]
#[serial]
async fn test_mixed_js_and_python_tools() {
    let mut tools = PctxTools::default();

    // Register JavaScript tool
    tools.local_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "add".to_string(),
            description: Some("Adds two numbers".to_string()),
            input_schema: None,
            namespace: "LocalTools".to_string(),
        },
        runtime: CallbackRuntime::JavaScript,
        callback_data: "(args) => args.a + args.b".to_string(),
    }];

    // Register Python tool
    let python_registry = pctx_python_runtime::PythonCallbackRegistry::new();
    python_registry
        .register(LocalToolDefinition {
            metadata: LocalToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers".to_string()),
                input_schema: None,
                namespace: "PythonTools".to_string(),
            },
            runtime: CallbackRuntime::Python,
            callback_data: "lambda args: args['a'] * args['b']".to_string(),
        })
        .expect("Failed to register Python tool");

    tools.python_registry = Some(python_registry);

    // Use the HIGH-LEVEL API - generated TypeScript namespace functions
    let code = r#"
        async function run() {
            const sum = await LocalTools.add({ a: 10, b: 5 });
            const product = await PythonTools.multiply({ a: 6, b: 7 });
            return { sum, product };
        }
    "#;

    let result = tools
        .execute(ExecuteInput {
            code: code.to_string(),
        })
        .await
        .expect("Execution should succeed");

    assert!(result.success);
    let output = result.output.unwrap();
    assert_eq!(output["sum"], 15);
    assert_eq!(output["product"], 42);
}
