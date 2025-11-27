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
    use pyo3::{ffi::c_str, Python};

    let mut tools = PctxTools::default();

    // Register a Python tool using the new API (PyObject directly)
    let python_registry = pctx_python_runtime::PythonCallbackRegistry::new();
    Python::with_gil(|py| {
        let func = py
            .eval(c_str!("lambda args: args['a'] * args['b']"), None, None)
            .expect("Failed to create lambda");

        python_registry
            .register_callable(
                LocalToolMetadata {
                    name: "multiply".to_string(),
                    description: Some("Multiplies two numbers".to_string()),
                    input_schema: None,
                    namespace: "PythonTools".to_string(),
                },
                func.unbind(),
            )
            .expect("Failed to register Python tool");
    });

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
    use pyo3::Python;

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

    // Register Python tool using the new API
    let python_registry = pctx_python_runtime::PythonCallbackRegistry::new();
    Python::with_gil(|py| {
        use pyo3::ffi::c_str;

        let func = py
            .eval(c_str!("lambda args: args['a'] * args['b']"), None, None)
            .expect("Failed to create lambda");

        python_registry
            .register_callable(
                LocalToolMetadata {
                    name: "multiply".to_string(),
                    description: Some("Multiplies two numbers".to_string()),
                    input_schema: None,
                    namespace: "PythonTools".to_string(),
                },
                func.unbind(),
            )
            .expect("Failed to register Python tool");
    });

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
    assert!(output["pi"].as_f64().unwrap() > 3.14);
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
    let python_registry = pctx_python_runtime::PythonCallbackRegistry::new();

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

        python_registry
            .register_callable(
                LocalToolMetadata {
                    name: "jsonParser".to_string(),
                    description: Some("Parse JSON using the json module".to_string()),
                    input_schema: None,
                    namespace: "PythonTools".to_string(),
                },
                func.unbind(),
            )
            .expect("Failed to register Python callback");
    });

    tools.python_registry = Some(python_registry);

    // Use the HIGH-LEVEL API - the generated TypeScript namespace function
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
