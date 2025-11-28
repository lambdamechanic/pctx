use crate::{PctxTools, model::ExecuteInput, test_utils};
use deno_executor::LocalToolMetadata;
use pctx_code_execution_runtime::LocalToolRegistry;
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_nodejs_callback_simulation() {
    let mut tools = PctxTools::default();

    // Register a simulated Node.js callback using the test helper
    let registry = LocalToolRegistry::new();

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
            LocalToolMetadata {
                name: "multiply".to_string(),
                description: Some("Multiplies two numbers (Node.js)".to_string()),
                input_schema: None,
                namespace: "NodeTools".to_string(),
            },
            callback,
        )
        .expect("Failed to register Node.js callback");

    // Use the unified registry directly
    tools.local_registry = Some(registry);

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
async fn test_python_local_tool() {
    use pyo3::{Python, ffi::c_str};

    let mut tools = PctxTools::default();

    // Register a Python tool directly in the local registry
    let registry = pctx_code_execution_runtime::LocalToolRegistry::new();
    Python::with_gil(|py| {
        let func = py
            .eval(c_str!("lambda args: args['a'] * args['b']"), None, None)
            .expect("Failed to create lambda");

        let callback = test_utils::wrap_python_callback(func.unbind());

        registry
            .register_callback(
                LocalToolMetadata {
                    name: "multiply".to_string(),
                    description: Some("Multiplies two numbers".to_string()),
                    input_schema: None,
                    namespace: "PythonTools".to_string(),
                },
                callback,
            )
            .expect("Failed to register Python tool");
    });

    tools.local_registry = Some(registry);

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
async fn test_js_code_can_use_dependencies() {
    let tools = PctxTools::default();

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
    let tools = PctxTools::default();

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

#[tokio::test]
#[serial]
async fn test_python_callback_can_use_dependencies() {
    use pyo3::Python;

    let mut tools = PctxTools::default();

    // Test that Python callbacks can use standard library imports
    // Using json module which is a lightweight built-in dependency
    let registry = pctx_code_execution_runtime::LocalToolRegistry::new();

    // For complex code with imports, we compile it in the test and pass the PyObject
    Python::with_gil(|py| {
        use pyo3::types::{PyDict, PyDictMethods, PyModuleMethods};
        use std::ffi::CString;

        let code = r#"
import json

def tool(args):
    # Use json module to parse and re-serialize
    data = {"name": args["name"], "count": args["count"] * 2}
    # Verify json module works by dumping and loading
    json_str = json.dumps(data)
    return json.loads(json_str)
"#;

        // Create globals dict with builtins
        let globals = PyDict::new(py);
        let main_module = py.import("__main__").unwrap();
        let main_dict = main_module.dict();
        if let Ok(Some(builtins)) = main_dict.get_item("__builtins__") {
            globals.set_item("__builtins__", builtins).unwrap();
        }

        // Execute the code to define the function
        let code_cstr = CString::new(code).unwrap();
        py.run(code_cstr.as_c_str(), Some(&globals), None).unwrap();

        // Extract the 'tool' function
        let func = globals.get_item("tool").unwrap().unwrap();

        let callback = test_utils::wrap_python_callback(func.unbind());

        registry
            .register_callback(
                LocalToolMetadata {
                    name: "jsonParser".to_string(),
                    description: Some("Parse JSON using the json module".to_string()),
                    input_schema: None,
                    namespace: "PythonTools".to_string(),
                },
                callback,
            )
            .expect("Failed to register Python callback");
    });

    tools.local_registry = Some(registry);

    let code = r#"
        async function run() {
            return await PythonTools.jsonParser({ name: "test", count: 5 });
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
    assert_eq!(
        result.output,
        Some(serde_json::json!({"name": "test", "count": 10}))
    );
}

#[tokio::test]
#[serial]
async fn test_mixed_rust_and_python_callbacks() {
    use pyo3::{Python, ffi::c_str};

    let mut tools = PctxTools::default();

    // First, register a simulated Node.js callback
    let registry = LocalToolRegistry::new();

    let add_callback = Arc::new(|args: Option<serde_json::Value>| {
        let args = args.ok_or("Missing arguments")?;
        let a = args["a"].as_i64().ok_or("Missing 'a'")?;
        let b = args["b"].as_i64().ok_or("Missing 'b'")?;
        // Simulates: (args) => args.a + args.b
        Ok(serde_json::json!(a + b))
    });

    registry
        .register_callback(
            LocalToolMetadata {
                name: "add".to_string(),
                description: Some("Adds two numbers (Node.js)".to_string()),
                input_schema: None,
                namespace: "NodeTools".to_string(), // Simulating Node.js tools
            },
            add_callback,
        )
        .expect("Failed to register Node.js callback");

    // Now add Python callbacks to the same registry
    Python::with_gil(|py| {
        let multiply_func = py
            .eval(c_str!("lambda args: args['x'] * args['y']"), None, None)
            .expect("Failed to create lambda");

        let callback = test_utils::wrap_python_callback(multiply_func.unbind());

        registry
            .register_callback(
                LocalToolMetadata {
                    name: "multiply".to_string(),
                    description: Some("Multiplies two numbers".to_string()),
                    input_schema: None,
                    namespace: "PythonTools".to_string(),
                },
                callback,
            )
            .expect("Failed to register Python tool");
    });

    tools.local_registry = Some(registry);

    // Use BOTH tools - one from Node.js (simulated), one from Python
    let code = r#"
        async function run() {
            // Call simulated Node.js callback
            const sum = await NodeTools.add({ a: 10, b: 5 });

            // Call Python callback
            const product = await PythonTools.multiply({ x: 6, y: 7 });

            return { sum, product };
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

    assert!(
        result.success,
        "Mixed Node.js and Python callbacks should work"
    );
    let output = result.output.unwrap();
    assert_eq!(
        output["sum"], 15,
        "Node.js callback should compute 10 + 5 = 15"
    );
    assert_eq!(
        output["product"], 42,
        "Python callback should compute 6 * 7 = 42"
    );
}
