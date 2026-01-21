# PCTX Code Mode

A TypeScript code execution engine that enables AI agents to dynamically call tools through generated code. Code Mode converts tool schemas (like MCP tools) into TypeScript interfaces, executes LLM-generated code in a sandboxed Deno runtime, and bridges function calls back to your Rust callbacks.

## Quick Start

```rust
use pctx_code_mode::{CodeMode, Tool, ToolSet, RootSchema, CallbackRegistry};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct GreetInput {
    name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Define your tools with JSON schemas
    let tool = Tool::new_callback(
        "greet",
        Some("Greets a person by name".to_string()),
        serde_json::from_value(serde_json::to_value(schema_for!(GreetInput))?)?,
        None,
    )?;

    let toolset = ToolSet::new("Greeter", "Greeting functions", vec![tool]);

    // 2. Create CodeMode instance with your tools
    let mut code_mode = CodeMode::default();
    code_mode.tool_sets = vec![toolset];

    // 3. Register callbacks that execute when tools are called
    let registry = CallbackRegistry::default();
    registry.add("Greeter.greet", Arc::new(|args| {
        Box::pin(async move {
            let name = args
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            Ok(serde_json::json!({ "message": format!("Hello, {name}!") }))
        })
    }))?;

    // 4. Execute LLM-generated TypeScript code
    let code = r#"
        async function run() {
            const result = await Greeter.greet({ name: "Alice" });
            return result;
        }
    "#;

    let output = code_mode.execute(code, Some(registry)).await?;

    if output.success {
        println!("Result: {}", serde_json::to_string_pretty(&output.output)?);
    } else {
        eprintln!("Error: {}", output.stderr);
    }

    Ok(())
}
```

## Core Concepts

### 1. Tools and ToolSets

Tools represent individual functions that can be called from TypeScript code. They are organized into ToolSets (namespaces).

```rust
use pctx_code_mode::{Tool, ToolSet};

// Create a tool with input/output schemas
let tool = Tool::new_callback(
    "fetchData",                           // Function name
    Some("Fetches data from API"),         // Description
    input_schema,                          // JSON Schema for input
    Some(output_schema),                   // Optional output schema
)?;

// Organize tools into a namespace
let toolset = ToolSet::new(
    "DataApi",                             // Namespace
    "Data fetching functions",             // Description
    vec![tool],                            // Tools
);
```

### 2. CodeMode

The main execution engine that manages tools and executes TypeScript code.

```rust
let mut code_mode = CodeMode::default();
code_mode.tool_sets = vec![toolset1, toolset2];

// List available functions
let list = code_mode.list_functions();
for func in list.functions {
    println!("{}.{}: {:?}", func.namespace, func.name, func.description);
}

// Get detailed type information
let details = code_mode.get_function_details(GetFunctionDetailsInput {
    functions: vec![
        FunctionId { mod_name: "DataApi".into(), fn_name: "fetchData".into() }
    ],
});
println!("TypeScript definitions:\n{}", details.code);
```

### 3. Callbacks

Callbacks are Rust functions that execute when TypeScript code calls your tools.

```rust
use pctx_code_mode::{CallbackRegistry, CallbackFn};
use std::sync::Arc;

let registry = CallbackRegistry::default();

let callback: CallbackFn = Arc::new(|args| {
    Box::pin(async move {
        // Extract arguments
        let id = args
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_i64())
            .ok_or("Missing id")?;

        // Do async work
        let data = fetch_from_database(id).await?;

        // Return JSON result
        Ok(serde_json::to_value(data)?)
    })
});

// Register with namespace.function format
registry.add("DataApi.fetchData", callback)?;
```

### 4. Code Execution

Execute LLM-generated TypeScript code that calls your registered tools.

```rust
let code = r#"
    async function run() {
        // Call your registered tools
        const user = await DataApi.fetchData({ id: 123 });
        const greeting = await Greeter.greet({ name: user.name });

        // Chain multiple calls
        const result = await DataApi.saveData({
            id: user.id,
            message: greeting.message
        });

        // Return the final result
        return result;
    }
"#;

let output = code_mode.execute(code, Some(registry)).await?;

match output.success {
    true => println!("Success: {:?}", output.output),
    false => eprintln!("Error: {}", output.stderr),
}
```

## API Reference

### CodeMode

The main execution engine.

#### `new()` / `default()`

```rust
let code_mode = CodeMode::default();
```

#### `list_functions() -> ListFunctionsOutput`

Lists all available functions with their TypeScript interface declarations.

```rust
let list = code_mode.list_functions();
println!("Available functions:\n{}", list.code);
for func in list.functions {
    println!("  {}.{}", func.namespace, func.name);
}
```

#### `get_function_details(input: GetFunctionDetailsInput) -> GetFunctionDetailsOutput`

Gets detailed TypeScript type definitions for specific functions.

```rust
use pctx_code_mode::model::{GetFunctionDetailsInput, FunctionId};

let details = code_mode.get_function_details(GetFunctionDetailsInput {
    functions: vec![
        FunctionId {
            mod_name: "DataApi".to_string(),
            fn_name: "fetchData".to_string(),
        }
    ],
});

println!("TypeScript code:\n{}", details.code);
```

#### `execute(code: &str, callbacks: Option<CallbackRegistry>) -> Result<ExecuteOutput>`

Executes TypeScript code in a sandboxed Deno runtime.

```rust
let output = code_mode.execute(typescript_code, Some(callback_registry)).await?;

if output.success {
    println!("Return value: {:?}", output.output);
    println!("Stdout: {}", output.stdout);
} else {
    eprintln!("Stderr: {}", output.stderr);
}
```

#### `add_callback(config: &CallbackConfig) -> Result<()>`

Dynamically adds a callback-based tool to the code mode.

```rust
use pctx_code_mode::model::CallbackConfig;

code_mode.add_callback(&CallbackConfig {
    name: "logMessage".to_string(),
    namespace: "Logger".to_string(),
    description: Some("Logs a message".to_string()),
    input_schema: Some(serde_json::json!({
        "type": "object",
        "properties": {
            "message": { "type": "string" }
        },
        "required": ["message"]
    })),
    output_schema: None,
})?;
```

### CallbackRegistry

Thread-safe registry for managing callback functions.

#### `default() -> CallbackRegistry`

```rust
let registry = CallbackRegistry::default();
```

#### `add(id: &str, callback: CallbackFn) -> Result<()>`

Registers a callback with a specific ID (format: `Namespace.functionName`).

```rust
registry.add("DataApi.fetchData", Arc::new(|args| {
    Box::pin(async move {
        // Your implementation
        Ok(serde_json::json!({"result": "data"}))
    })
}))?;
```

#### `has(id: &str) -> bool`

Checks if a callback is registered.

```rust
if registry.has("DataApi.fetchData") {
    println!("Callback is registered");
}
```

### Types

#### `Tool`

```rust
pub struct Tool {
    pub name: String,
    pub fn_name: String,
    pub description: Option<String>,
    pub input_signature: String,
    pub output_signature: String,
    pub types: String,
    // ... internal fields
}
```

Create tools for MCP-style tools or callbacks:

```rust
// MCP-style tool
let tool = Tool::new_mcp(
    "toolName",
    Some("Description"),
    input_schema,
    output_schema,
)?;

// Callback-based tool
let tool = Tool::new_callback(
    "toolName",
    Some("Description"),
    input_schema,
    output_schema,
)?;
```

#### `ToolSet`

```rust
pub struct ToolSet {
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub tools: Vec<Tool>,
}
```

```rust
let toolset = ToolSet::new("MyNamespace", "Description", vec![tool1, tool2]);
```

#### `ExecuteOutput`

```rust
pub struct ExecuteOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub output: Option<serde_json::Value>,
}
```

#### `CallbackFn`

Type alias for callback functions:

```rust
pub type CallbackFn = Arc<
    dyn Fn(Option<serde_json::Value>) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
    + Send
    + Sync
>;
```

## Advanced Usage

### Converting MCP Tools

Convert MCP (Model Context Protocol) tools into Code Mode tools:

```rust
use rmcp::model::Tool as McpTool;

fn convert_mcp_tool(mcp_tool: &McpTool) -> Result<Tool> {
    let mut schema_value = serde_json::to_value(&mcp_tool.input_schema)?;

    // Dereference JSON Schema $refs
    unbinder::dereference_schema(&mut schema_value, unbinder::Options::default());

    let input_schema: RootSchema = serde_json::from_value(schema_value)?;

    Tool::new_mcp(
        &mcp_tool.name,
        mcp_tool.description.as_ref().map(|s| s.to_string()),
        input_schema,
        None,
    )
}
```

### Dynamic Tool Registration

Register tools at runtime based on configuration:

```rust
for config in tool_configs {
    code_mode.add_callback(&CallbackConfig {
        name: config.name,
        namespace: config.namespace,
        description: Some(config.description),
        input_schema: Some(config.input_schema),
        output_schema: config.output_schema,
    })?;

    // Register the corresponding callback
    let callback_id = format!("{}.{}", config.namespace, config.name);
    registry.add(&callback_id, create_callback_for_config(&config))?;
}
```

### Async Tool Execution

Callbacks support full async operations:

```rust
registry.add("Database.query", Arc::new(|args| {
    Box::pin(async move {
        let query = args
            .and_then(|v| v.get("sql"))
            .and_then(|v| v.as_str())
            .ok_or("Missing sql parameter")?;

        // Perform async database query
        let pool = get_db_pool().await;
        let rows = sqlx::query(query)
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_value(rows)?)
    })
}))?;
```

### Error Handling

```rust
let output = code_mode.execute(code, Some(registry)).await?;

if !output.success {
    // Check stderr for execution errors
    if output.stderr.contains("TypeError") {
        eprintln!("Type error in generated code: {}", output.stderr);
    } else if output.stderr.contains("not found") {
        eprintln!("Tool not found: {}", output.stderr);
    } else {
        eprintln!("Execution failed: {}", output.stderr);
    }
}
```

### TypeScript Code Requirements

LLM-generated code must follow this pattern:

```typescript
async function run() {
    // Your code that calls registered tools
    const result = await Namespace.toolName({ param: value });

    // MUST return a value
    return result;
}
```

The code execution engine:
- Wraps your code with namespace implementations
- Automatically calls `run()` and captures its return value
- Provides the return value in `ExecuteOutput.output`

## Architecture

1. **Tool Definition**: Tools are defined with JSON Schemas for inputs/outputs
2. **Code Generation**: TypeScript interface definitions are generated from schemas
3. **Code Execution**: User code is wrapped with namespace implementations and executed in Deno
4. **Callback Routing**: Function calls in TypeScript are routed to Rust callbacks
5. **Result Marshaling**: JSON values are passed between TypeScript and Rust

### Sandbox Security

Code is executed in a Deno runtime with:
- Network access restricted to allowed hosts
- No file system access
- No subprocess spawning
- Isolated V8 context per execution

Configure allowed hosts:

```rust
code_mode.servers = vec![
    ServerConfig {
        // Your server configuration
        // Only hosts in server configs are allowed network access
    }
];

let allowed = code_mode.allowed_hosts();
println!("Allowed hosts: {:?}", allowed);
```

## Examples

### Multi-Tool Workflow

```rust
let code = r#"
    async function run() {
        // Fetch user data
        const user = await UserApi.getUser({ id: 123 });

        // Process the data
        const processed = await DataProcessor.transform({
            input: user.data,
            format: "normalized"
        });

        // Save results
        const saved = await Storage.save({
            key: `user_${user.id}`,
            value: processed
        });

        return {
            userId: user.id,
            saved: saved.success,
            location: saved.url
        };
    }
"#;

let output = code_mode.execute(code, Some(registry)).await?;
```

### Error Recovery

```rust
let code = r#"
    async function run() {
        try {
            return await RiskyApi.operation({ id: 1 });
        } catch (error) {
            console.error("Operation failed:", error);
            // Fall back to safe default
            return await SafeApi.getDefault();
        }
    }
"#;

let output = code_mode.execute(code, Some(registry)).await?;

// Check console output
if !output.stdout.is_empty() {
    println!("Console output: {}", output.stdout);
}
```

### Parallel Execution

```rust
let code = r#"
    async function run() {
        // Execute multiple operations in parallel
        const [users, posts, comments] = await Promise.all([
            UserApi.listUsers(),
            PostApi.listPosts(),
            CommentApi.listComments()
        ]);

        return { users, posts, comments };
    }
"#;
```

## Related Crates

- `pctx_codegen`: TypeScript code generation from JSON schemas
- `pctx_executor`: Deno runtime execution engine
- `pctx_code_execution_runtime`: Runtime environment and callback system

## License

MIT
