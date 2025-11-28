# code_mode_py_bindings

Python bindings for the `pctx_code_mode` crate

## Overview

This crate provides Python bindings that allow Python developers to:
- Load and connect to MCP servers with custom namespaces
- Register local Python functions as tools with namespaces
- List available functions across all registered tools
- Get detailed function information including TypeScript type definitions
- Execute TypeScript code that calls registered tools

## API

```python
from pctx_code_mode import CodeMode

# Initialize with optional configuration
cm = CodeMode(
    mcp_servers=[
        {"name": "github", "url": "http://localhost:3000"}
    ],
    local_tools=[
        {
            "namespace": "math",
            "name": "add",
            "description": "Adds two numbers",
            "callback": lambda params: {"result": params["a"] + params["b"]},
            "input_schema": {
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                }
            }
        }
    ]
)

# Or register tools after initialization
cm.register_local_tool(
    namespace="math",
    name="multiply",
    callback=lambda params: {"result": params["a"] * params["b"]},
    description="Multiplies two numbers",  # optional
    input_schema={...}  # optional
)

# Add MCP server (async)
await cm.add_mcp_server(name="github", url="http://localhost:3000")

result = cm.list_functions()
print(result.code)
for func in result.functions:
    print(f"{func.namespace}.{func.name}: {func.description}")

details = cm.get_function_details(["Math.add", "Github.list_repos"])
print(details.code)  # Full TypeScript type definitions

code = """
async function run() {
    const sum = await Math.add({a: 5, b: 3});
    const repos = await Github.list_repos({owner: "anthropics"});
    return {sum, repos};
}
"""
output = await cm.execute(code)
```

## Key Features

### Namespace Merging
Tools from MCP servers and local tools can share the same namespace:

```python
cm = CodeMode(
    mcp_servers=[{"name": "github", "url": "..."}],  # namespace: "github"
    local_tools=[
        {
            "namespace": "github",  # Same namespace!
            "name": "custom_action",
            "callback": lambda params: {...}
        }
    ]
)

# Results in merged namespace (note: namespaces are PascalCase in TypeScript):
# - Github.list_repos (from MCP)
# - Github.get_issues (from MCP)
# - Github.custom_action (local tool)
```

## Building

This crate uses [maturin](https://www.maturin.rs/) for building Python wheels:

```bash
cd crates/code_mode_py_bindings
maturin develop

# Release build
maturin build --release

# Build and install wheel
maturin build --release
pip install target/wheels/pctx_code_mode-*.whl
```

## Testing

```bash
maturin develop
uv run pytest tests/
```


The implementation uses:
- `pyo3` for Python bindings
- `pyo3-async-runtimes` for async Python support
- `pythonize` for JSON ↔ Python object conversion
- `tokio::spawn_blocking` to handle Deno's non-Send execution

## License

MIT
