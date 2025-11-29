use crate::{CodeMode, model::ExecuteInput};
use pctx_code_execution_runtime::CallableToolRegistry;
use pctx_executor::CallableToolMetadata;
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_nodejs_callback_simulation() {
    let mut tools = CodeMode::default();

    // Register a simulated Node.js callback using the test helper
    let registry = CallableToolRegistry::new();

    // This simulates what wrap_nodejs_callback() would create - a Rust closure
    // that wraps a JavaScript function from the host Node.js environment
    let callback = Arc::new(|args: Option<serde_json::Value>| {
        let args = args.ok_or("Missing arguments")?;
        let a = args["a"].as_i64().ok_or("Missing 'a'")?;
        let b = args["b"].as_i64().ok_or("Missing 'b'")?;
        // Simulates: (args) => args.a * args.b
        Ok(serde_json::json!(a * b))
    });

    registry
        .register_callback(
            CallableToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers (Node.js)".to_string()),
                input_schema: None,
                output_schema: Some(serde_json::json!({
                    "type": "number",
                    "description": "The product of the two numbers"
                })),
                namespace: "NodeTools".to_string(),
            },
            callback,
        )
        .expect("Failed to register Node.js callback");

    // Use the unified registry directly
    tools.callable_registry = Some(registry);

    // Execute code that calls the simulated Node.js callback
    let code = r#"
        async function run() {
            return await NodeTools.multiply({ a: 6, b: 7 });
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

    assert!(result.success, "Node.js callback simulation should work");
    assert_eq!(result.output, Some(serde_json::json!(42)));
}

#[tokio::test]
#[serial]
async fn test_js_code_can_use_dependencies() {
    let tools = CodeMode::default();

    // Test that user code can access built-in JavaScript dependencies
    // Using Math which is a lightweight built-in JavaScript global
    let code = r#"
        async function run() {
            // User code that uses Math API directly
            const radius = 5;
            const area = Math.PI * Math.pow(radius, 2);
            const circumference = 2 * Math.PI * radius;

            return {
                radius: radius,
                area: Math.round(area * 100) / 100,
                circumference: Math.round(circumference * 100) / 100,
                pi: Math.PI
            };
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
    assert_eq!(output["radius"], 5);
    // Area = π * 5^2 ≈ 78.54
    assert_eq!(output["area"], 78.54);
    // Circumference = 2 * π * 5 ≈ 31.42
    assert_eq!(output["circumference"], 31.42);
    assert!(output["pi"].as_f64().unwrap() > 3.0);
}

#[tokio::test]
#[serial]
async fn test_js_code_can_use_date_api() {
    let tools = CodeMode::default();

    // Test that user code can access the Date API
    let code = r#"
        async function run() {
            // Use a fixed timestamp for reproducible testing: 2024-01-15 10:30:00 UTC
            const date = new Date("2024-01-15T10:30:00Z");

            return {
                year: date.getFullYear(),
                month: date.getMonth() + 1,
                day: date.getDate(),
                hours: date.getHours(),
                isValidDate: !isNaN(date.getTime())
            };
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
    assert_eq!(output["year"], 2024);
    assert_eq!(output["month"], 1);
    assert_eq!(output["day"], 15);
    assert_eq!(output["isValidDate"], true);
}
