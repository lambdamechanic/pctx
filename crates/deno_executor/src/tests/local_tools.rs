use super::serial;
use crate::{CallbackRuntime, ExecuteOptions, LocalToolDefinition, LocalToolMetadata, execute};

#[tokio::test]
#[serial]
async fn test_js_local_tool_with_dependencies() {
    // Test that JavaScript local tools can use built-in dependencies
    // Using Math which is a lightweight built-in JavaScript global object
    let local_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "calculate_circle".to_string(),
            description: Some("Calculate circle properties using Math API".to_string()),
            input_schema: None,
            namespace: "TestTools".to_string(),
        },
        runtime: CallbackRuntime::JavaScript,
        callback_data: r#"(args) => {
    // Use the built-in Math API to calculate circle properties
    const radius = args.radius;
    const area = Math.PI * Math.pow(radius, 2);
    const circumference = 2 * Math.PI * radius;
    return {
        radius: radius,
        area: Math.round(area * 100) / 100,  // Round to 2 decimal places
        circumference: Math.round(circumference * 100) / 100,
        pi: Math.PI
    };
}"#
        .to_string(),
    }];

    let code = r#"
async function test() {
    const result = await callJsLocalTool("calculate_circle", {
        radius: 5
    });
    return result;
}

export default await test();
"#;

    let options = ExecuteOptions::new().with_local_tools(local_tools);
    let result = execute(code, options)
        .await
        .expect("execution should succeed");

    if !result.success {
        eprintln!("Runtime error: {:?}", result.runtime_error);
        eprintln!("Stdout: {}", result.stdout);
        eprintln!("Stderr: {}", result.stderr);
        eprintln!("Diagnostics: {:?}", result.diagnostics);
    }

    assert!(result.success, "Code should execute successfully");
    assert!(
        result.runtime_error.is_none(),
        "Should have no runtime errors"
    );

    let output = result.output.expect("Should have output");
    assert_eq!(output["radius"], 5);
    // Area = π * 5^2 ≈ 78.54
    assert_eq!(output["area"], 78.54);
    // Circumference = 2 * π * 5 ≈ 31.42
    assert_eq!(output["circumference"], 31.42);
    assert!(output["pi"].as_f64().unwrap() > 3.14);
}

#[tokio::test]
#[serial]
async fn test_js_local_tool_with_date() {
    // Test that JavaScript local tools can use the Date API
    // This verifies access to another built-in JavaScript global object
    let local_tools = vec![LocalToolDefinition {
        metadata: LocalToolMetadata {
            name: "format_date".to_string(),
            description: Some("Format date using Date API".to_string()),
            input_schema: None,
            namespace: "TestTools".to_string(),
        },
        runtime: CallbackRuntime::JavaScript,
        callback_data: r#"(args) => {
    // Use the built-in Date API to work with dates
    const date = new Date(args.timestamp);
    return {
        year: date.getFullYear(),
        month: date.getMonth() + 1,  // getMonth() returns 0-11
        day: date.getDate(),
        hours: date.getHours(),
        isValidDate: !isNaN(date.getTime())
    };
}"#
        .to_string(),
    }];

    let code = r#"
async function test() {
    // Use a fixed timestamp for reproducible testing
    // 2024-01-15 10:30:00 UTC
    const result = await callJsLocalTool("format_date", {
        timestamp: "2024-01-15T10:30:00Z"
    });
    return result;
}

export default await test();
"#;

    let options = ExecuteOptions::new().with_local_tools(local_tools);
    let result = execute(code, options)
        .await
        .expect("execution should succeed");

    if !result.success {
        eprintln!("Runtime error: {:?}", result.runtime_error);
        eprintln!("Stdout: {}", result.stdout);
        eprintln!("Stderr: {}", result.stderr);
        eprintln!("Diagnostics: {:?}", result.diagnostics);
    }

    assert!(result.success, "Code should execute successfully");
    assert!(
        result.runtime_error.is_none(),
        "Should have no runtime errors"
    );

    let output = result.output.expect("Should have output");
    assert_eq!(output["year"], 2024);
    assert_eq!(output["month"], 1);
    assert_eq!(output["day"], 15);
    assert_eq!(output["isValidDate"], true);
}
