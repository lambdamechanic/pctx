# pctx

Python SDK for PCTX - A complete toolkit for working with MCP servers and local tools.

## Installation

```bash
pip install pctx
```

## Features

- **MCP Server Integration**: Connect to any Model Context Protocol server
- **Local Tools**: Register Python functions as tools
- **Function Discovery**: List and inspect all available functions
- **Code Execution**: Execute TypeScript code with full tool access
- **Pythonic API**: Idiomatic Python interfaces with type hints
- **High Performance**: Native Rust implementation via PyO3

## Quick Start

```python
from pctx import PctxTools
import os

tools = PctxTools()

# Register an MCP server
tools.add_mcp_server(
    name='github',
    command='npx',
    args=['-y', '@modelcontextprotocol/server-github'],
    env={'GITHUB_TOKEN': os.environ['GITHUB_TOKEN']}
)

# Register a local tool
def get_current_time(args):
    from datetime import datetime
    return datetime.now().isoformat()

tools.register_local_tool(
    name='getCurrentTime',
    handler=get_current_time,
    namespace='utils',
    description='Gets the current ISO timestamp'
)

# List all available functions
result = tools.list_functions()
print([f"{f['namespace']}.{f['name']}" for f in result['functions']])

# Execute TypeScript code with tool access
result = tools.execute(code='''
    async function run() {
        const time = await utils.getCurrentTime();
        return { message: 'Current time is ' + time };
    }
''')
print(result['output'])
```

## API Reference

### `PctxTools`

Main class for interacting with PCTX.

#### Methods

##### `add_mcp_server(name: str, command: str, args: List[str] = None, env: Dict[str, str] = None) -> None`

Register an MCP server.

```python
tools.add_mcp_server(
    name='github',
    command='npx',
    args=['-y', '@modelcontextprotocol/server-github'],
    env={'GITHUB_TOKEN': os.environ['GITHUB_TOKEN']}
)
```

##### `register_local_tool(name: str, handler: Callable, namespace: str, description: str = None, input_schema: dict = None) -> None`

Register a local Python function as a tool.

```python
def add_numbers(args):
    return args['a'] + args['b']

tools.register_local_tool(
    name='add',
    handler=add_numbers,
    namespace='math',
    description='Adds two numbers',
    input_schema={
        'type': 'object',
        'properties': {
            'a': {'type': 'number'},
            'b': {'type': 'number'}
        }
    }
)
```

##### `list_functions() -> dict`

List all available functions from MCP servers and local tools.

```python
result = tools.list_functions()
# Returns dict with:
# - functions: List of {namespace, name, description}
# - code: TypeScript import code
```

##### `get_function_details(functions: List[str]) -> dict`

Get detailed type information about specific functions.

```python
details = tools.get_function_details(
    functions=['github.createIssue', 'math.add']
)
# Returns full TypeScript type definitions
```

##### `execute(code: str) -> dict`

Execute TypeScript code with access to all registered tools.

```python
result = tools.execute(code='''
    async function run() {
        const sum = await math.add({ a: 5, b: 3 });
        return { result: sum };
    }
''')
print(result['output'])  # {'result': 8}
```

## Async Support

The SDK is compatible with Python's asyncio. While the methods are synchronous,
they internally use a Tokio runtime to handle async operations.

## License

MIT
