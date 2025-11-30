# PCTX Python Client

Complete Python client for [Port of Context (PCTX)](https://github.com/portofcontext/pctx) with support for both MCP operations and local tool registration.

## Features

- **MCP Client**: HTTP client for MCP operations (list_functions, get_function_details, execute)
- **WebSocket Client**: Register local Python tools and execute code with access to them
- **Unified Client**: Combined interface for both MCP and local tools
- **Full async/await support**
- **Type-safe with proper error handling**

## Installation

```bash
pip install pctx-client
```

Or install from source:

```bash
cd python-client
pip install -e .
```

For development:

```bash
pip install -e ".[dev]"
```

## Quick Start

### Using MCP Client Only

```python
import asyncio
from pctx_client import McpClient

async def main():
    async with McpClient("http://localhost:8080/mcp") as client:
        # List all available MCP functions
        functions = await client.list_functions()
        print(f"Available functions: {len(functions)}")

        # Get details for specific functions
        details = await client.get_function_details(["Notion.apiPostSearch"])
        print(details)

        # Execute TypeScript code
        result = await client.execute('''
            async function run() {
                const results = await Notion.apiPostSearch({
                    query: "test"
                });
                return results;
            }
        ''')
        print(result)

asyncio.run(main())
```

### Using WebSocket Client for Local Tools

```python
import asyncio
from pctx_client import PctxClient

async def main():
    async with PctxClient("ws://localhost:8080/local-tools") as client:
        # Register a Python tool
        def get_user_data(params):
            user_id = params.get("user_id")
            return {
                "id": user_id,
                "name": "John Doe",
                "email": "john@example.com"
            }

        await client.register_tool(
            namespace="UserService",
            name="getUserData",
            callback=get_user_data,
            description="Fetches user data by ID",
            input_schema={
                "type": "object",
                "properties": {
                    "user_id": {"type": "number"}
                },
                "required": ["user_id"]
            }
        )

        # Execute code that uses the local tool (low-level API)
        result = await client.execute_code('''
            async function run() {
                const user = await CALLABLE_TOOLS.execute('UserService.getUserData', {
                    user_id: 123
                });
                return user;
            }
        ''')

        print(result)

asyncio.run(main())
```

**Note**: For a cleaner Python API, use the `PctxUnifiedClient` instead (see below).

### Using Unified Client (Recommended)

The unified client provides the best of both worlds with a clean Pythonic API:

```python
import asyncio
from pctx_client import PctxUnifiedClient

async def main():
    async with PctxUnifiedClient(
        mcp_url="http://localhost:8080/mcp",
        ws_url="ws://localhost:8080/local-tools"
    ) as client:
        # List all MCP functions
        mcp_functions = await client.list_functions()
        print(f"MCP functions: {len(mcp_functions)}")

        # Register local Python tools
        def process_data(params):
            data = params.get("data", [])
            return {"processed": [x * 2 for x in data]}

        await client.register_local_tool(
            namespace="DataProcessor",
            name="processData",
            callback=process_data
        )

        # Use clean Python API - call tools like regular Python functions!
        result = await client.DataProcessor.processData(data=[1, 2, 3, 4, 5])

        print(f"Processed: {result['processed']}")
        # Output: Processed: [2, 4, 6, 8, 10]

asyncio.run(main())
```

The unified client automatically creates namespace proxies, so you can call tools like:
- `await client.AsyncTools.triple(7)` instead of writing TypeScript code
- `await client.Math.add(a=5, b=3)` for natural Python syntax
- `await client.UserService.getUserData(user_id=123)` with keyword arguments

This completely abstracts away the internal `CALLABLE_TOOLS.execute()` mechanism!

## Running Tests

```bash
./run_tests.sh
```

See full documentation in the repository.
