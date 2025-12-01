# PCTX Python Client

Python client for [Port of Context (PCTX)](https://github.com/portofcontext/pctx) - execute TypeScript/JavaScript code with access to both MCP tools and local Python callbacks.

## Features

- **Execute code with mixed tooling**: Write TypeScript/JS code that seamlessly calls both MCP tools and Python functions
- **Simple API**: Register local tools and MCPs at initialization, then just call `execute()`
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

### Basic Usage

```python
import asyncio
from pctx_client import PctxClient

# Define local Python tools
def get_user_data(params):
    user_id = params.get("user_id")
    return {
        "id": user_id,
        "name": "John Doe",
        "email": "john@example.com"
    }

def process_data(params):
    data = params.get("data", [])
    return {"processed": [x * 2 for x in data]}

# Configure local tools
local_tools = [
    {
        "namespace": "UserService",
        "name": "getUserData",
        "callback": get_user_data,
        "description": "Fetches user data by ID"
    },
    {
        "namespace": "DataProcessor",
        "name": "processData",
        "callback": process_data
    }
]

async def main():
    # Initialize client with local tools and MCP servers
    async with PctxClient(
        ws_url="ws://localhost:8080/local-tools",
        local_tools=local_tools,
        mcps=["http://localhost:8080/mcp"]  # Optional
    ) as client:
        # Execute code that uses both MCP and local tools
        result = await client.execute('''
            async function run() {
                // Use MCP tool
                const notionResults = await Notion.apiPostSearch({
                    query: "test"
                });

                // Use local Python tools
                const user = await UserService.getUserData({user_id: 123});
                const processed = await DataProcessor.processData({
                    data: [1, 2, 3, 4, 5]
                });

                return {
                    notion: notionResults,
                    user: user,
                    processed: processed.processed
                };
            }
        ''')

        print(result["value"])

asyncio.run(main())
```

### Simple Example Without MCP

```python
import asyncio
from pctx_client import PctxClient

async def main():
    # Define a simple local tool
    def add(params):
        return params["a"] + params["b"]

    local_tools = [
        {"namespace": "Math", "name": "add", "callback": add}
    ]

    async with PctxClient(
        ws_url="ws://localhost:8080/local-tools",
        local_tools=local_tools
    ) as client:
        result = await client.execute('''
            async function run() {
                const sum = await Math.add({a: 10, b: 20});
                return {sum};
            }
        ''')

        print(result["value"]["sum"])  # Output: 30

asyncio.run(main())
```

### Dynamic Tool Registration

You can also register tools after initialization:

```python
async with PctxClient(ws_url="ws://localhost:8080/local-tools") as client:
    # Register tool dynamically
    await client.register_local_tool(
        namespace="MyTools",
        name="getData",
        callback=lambda params: {"data": [1, 2, 3]}
    )

    result = await client.execute('''
        async function run() {
            return await MyTools.getData({});
        }
    ''')
```

## Why This Design?

The whole point of PCTX is to **execute code** that uses tools, not to call tools directly from Python. You write Python scripts that:

1. Define local Python callbacks (tools)
2. Execute TypeScript/JavaScript code via `client.execute()`
3. That code can call both MCP tools and your local Python tools seamlessly

This is much more powerful than just calling `client.Math.add(a=10, b=20)` from Python!

## Running Tests

```bash
./run_tests.sh
```

See full documentation in the repository.
