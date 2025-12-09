# PCTX Python Client

Python client for using Code Mode via PCTX - execute JavaScript code with access to your Python functions.

## Installation

```bash
pip install pctx-client
```

## Quick Start

1. Install PCTX server

```bash
# Homebrew
brew install portofcontext/tap/pctx

# cURL
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/portofcontext/pctx/main/install.sh | sh

# npm
npm i -g @portofcontext/pctx
```

2. Install Python pctx with the langchain extra

```
pip install pctx-client[langchain] langchain
```

3. Start the Code Mode server for agents

```bash
pctx agent start
```

4. Define and run `main.py`

```python
import asyncio

from pctx_client import Pctx, tool
from langchain.agents import create_agent

# Define your tools
@tool
def get_weather(city: str) -> str:
    """Get weather for a given city."""
    return f"It's always sunny in {city}!"


@tool
def get_time(city: str) -> str:
    """Get time for a given city."""
    return f"It is midnight in {city}!"


async def main():
    # Initialize client with your tools
    p = Pctx(tools=[get_weather, get_time])

    # Define your agent
    agent = create_agent(
        model="anthropic:claude-sonnet-4-5-20250929", # choose any model supported by langchain!
        tools=p.langchain_tools(),
        system_prompt="You are a helpful assistant",
    )

    # Connect to PCTX
    await p.connect()

    result = await agent.ainvoke(
        {
            "messages": [
                {"role": "user", "content": "what is the weather and time in nyc"}
            ]
        }
    )

    print(result)

    # Disconnect when done
    await p.disconnect()

if __name__ == "__main__":
    asyncio.run(main())
```

## Features

- **Tool Decorator**: Easily expose Python functions as callable tools
- **Async Support**: Full async/await support for non-blocking operations
- **MCP Server Integration**: Connect to MCP servers for extended functionality

## Usage

### Defining Tools

Use the `@tool` decorator to expose Python functions:

```python
@tool("function_name", namespace="namespace")
def my_function(arg1: str, arg2: int) -> str:
    """Function description"""
    return f"{arg1}: {arg2}"
```

### List Available Functions

```python
functions = await p.list_functions()
print(functions.code)
```

### Get Function Details

```python
details = await p.get_function_details(["Namespace.functionName"])
print(details.code)
```

## Optional Integrations

### LangChain

```bash
pip install pctx[langchain]
```

### CrewAI

```bash
pip install pctx[crewai]
```
