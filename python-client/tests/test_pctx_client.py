"""
Integration tests for the new PctxClient API.

These tests demonstrate the correct usage pattern:
- Register local tools and MCPs at initialization
- Use execute() to run code that calls both MCP and local tools
"""

import asyncio
import pytest

from pctx_client import PctxClient
from pctx_client.exceptions import ConnectionError, ExecutionError


# Test configuration
MCP_URL = "http://localhost:8080/mcp"
WS_URL = "ws://localhost:8080/local-tools"


@pytest.mark.asyncio
async def test_basic_code_execution():
    """Test basic code execution without any tools."""
    async with PctxClient(ws_url=WS_URL) as client:
        result = await client.execute("""
            async function run() {
                return {message: "hello", value: 42};
            }
        """)

        assert result["success"] is True
        assert result["value"]["message"] == "hello"
        assert result["value"]["value"] == 42


@pytest.mark.asyncio
async def test_execute_with_local_tools():
    """Test executing code that calls local Python tools."""

    # Define local tools
    def get_data(params):
        return {"data": [1, 2, 3, 4, 5]}

    def multiply(params):
        return params["a"] * params["b"]

    async def async_fetch(params):
        await asyncio.sleep(0.01)
        return {"user_id": params.get("id"), "name": "John Doe"}

    local_tools = [
        {
            "namespace": "DataTools",
            "name": "getData",
            "callback": get_data,
            "description": "Get sample data array"
        },
        {
            "namespace": "Math",
            "name": "multiply",
            "callback": multiply
        },
        {
            "namespace": "UserTools",
            "name": "fetchUser",
            "callback": async_fetch
        }
    ]

    async with PctxClient(ws_url=WS_URL, local_tools=local_tools) as client:
        # Execute code that uses the local tools
        result = await client.execute("""
            async function run() {
                const data = await DataTools.getData({});
                const product = await Math.multiply({a: 10, b: 20});
                const user = await UserTools.fetchUser({id: 123});

                return {
                    dataLength: data.data.length,
                    product: product,
                    userName: user.name
                };
            }
        """)

        assert result["success"] is True
        assert result["value"]["dataLength"] == 5
        assert result["value"]["product"] == 200
        assert result["value"]["userName"] == "John Doe"


@pytest.mark.asyncio
async def test_execute_with_mcp_tools():
    """Test executing code that calls MCP tools."""
    async with PctxClient(ws_url=WS_URL, mcps=[MCP_URL]) as client:
        # Execute code that uses MCP tools
        result = await client.execute("""
            async function run() {
                // List available MCP functions
                const functions = await list_functions();
                return {
                    functionCount: functions.length,
                    hasList: functions.some(f => f.name === "list_functions"),
                    hasExecute: functions.some(f => f.name === "execute")
                };
            }
        """)

        assert result["success"] is True
        assert result["value"]["functionCount"] >= 2
        assert result["value"]["hasList"] is True
        assert result["value"]["hasExecute"] is True


@pytest.mark.asyncio
async def test_execute_with_both_mcp_and_local_tools():
    """Test executing code that uses both MCP and local Python tools."""

    # Define local tools
    def process_results(params):
        data = params.get("data", [])
        return {
            "processed": True,
            "count": len(data),
            "summary": f"Processed {len(data)} items"
        }

    local_tools = [
        {
            "namespace": "Processor",
            "name": "processResults",
            "callback": process_results
        }
    ]

    async with PctxClient(ws_url=WS_URL, local_tools=local_tools, mcps=[MCP_URL]) as client:
        # Execute code that uses both types of tools
        result = await client.execute("""
            async function run() {
                // Get MCP functions
                const mcpFunctions = await list_functions();

                // Process them with local tool
                const processed = await Processor.processResults({
                    data: mcpFunctions
                });

                return {
                    mcpToolCount: mcpFunctions.length,
                    processingResult: processed
                };
            }
        """)

        assert result["success"] is True
        assert result["value"]["mcpToolCount"] >= 2
        assert result["value"]["processingResult"]["processed"] is True
        assert result["value"]["processingResult"]["count"] >= 2


@pytest.mark.asyncio
async def test_list_tools():
    """Test listing registered tools."""
    def tool1(params):
        return {"ok": True}

    def tool2(params):
        return {"ok": True}

    local_tools = [
        {"namespace": "Tools", "name": "tool1", "callback": tool1},
        {"namespace": "Tools", "name": "tool2", "callback": tool2}
    ]

    async with PctxClient(ws_url=WS_URL, local_tools=local_tools) as client:
        tools = client.list_local_tools()
        assert "Tools.tool1" in tools
        assert "Tools.tool2" in tools
        assert len(tools) == 2


@pytest.mark.asyncio
async def test_dynamic_tool_registration():
    """Test registering tools after initialization."""
    async with PctxClient(ws_url=WS_URL) as client:
        # Register tool dynamically
        def dynamic_tool(params):
            return {"dynamic": True, "value": params.get("value", 0) * 2}

        await client.register_local_tool(
            namespace="DynamicTools",
            name="double",
            callback=dynamic_tool
        )

        # Use the dynamically registered tool
        result = await client.execute("""
            async function run() {
                const result = await DynamicTools.double({value: 21});
                return result;
            }
        """)

        assert result["success"] is True
        assert result["value"]["dynamic"] is True
        assert result["value"]["value"] == 42


@pytest.mark.asyncio
async def test_error_handling():
    """Test error handling when tool execution fails."""
    def failing_tool(params):
        raise ValueError("Intentional failure")

    local_tools = [
        {"namespace": "ErrorTools", "name": "fail", "callback": failing_tool}
    ]

    async with PctxClient(ws_url=WS_URL, local_tools=local_tools) as client:
        result = await client.execute("""
            async function run() {
                try {
                    await ErrorTools.fail({});
                    return {failed: false};
                } catch (e) {
                    return {failed: true, error: e.message};
                }
            }
        """)

        assert result["success"] is True
        assert result["value"]["failed"] is True


@pytest.mark.asyncio
async def test_complex_data_structures():
    """Test passing complex data structures between Python and JS."""
    def complex_processor(params):
        return {
            "input": params,
            "nested": {
                "arrays": [[1, 2], [3, 4]],
                "objects": {"key1": "value1", "key2": {"nested": "value"}}
            },
            "processed": True
        }

    local_tools = [
        {"namespace": "Complex", "name": "process", "callback": complex_processor}
    ]

    async with PctxClient(ws_url=WS_URL, local_tools=local_tools) as client:
        result = await client.execute("""
            async function run() {
                const result = await Complex.process({
                    id: 123,
                    data: {nested: "structure", array: [1, 2, 3]}
                });

                return {
                    receivedInput: result.input,
                    nestedData: result.nested,
                    wasProcessed: result.processed
                };
            }
        """)

        assert result["success"] is True
        assert result["value"]["receivedInput"]["id"] == 123
        assert result["value"]["nestedData"]["arrays"] == [[1, 2], [3, 4]]
        assert result["value"]["wasProcessed"] is True


@pytest.mark.asyncio
async def test_concurrent_tool_calls():
    """Test concurrent execution of multiple tool calls."""
    call_log = []

    def log_call(params):
        call_log.append(params.get("id"))
        return {"id": params.get("id"), "timestamp": len(call_log)}

    local_tools = [
        {"namespace": "Logger", "name": "log", "callback": log_call}
    ]

    async with PctxClient(ws_url=WS_URL, local_tools=local_tools) as client:
        result = await client.execute("""
            async function run() {
                const results = await Promise.all([
                    Logger.log({id: 1}),
                    Logger.log({id: 2}),
                    Logger.log({id: 3}),
                    Logger.log({id: 4}),
                    Logger.log({id: 5})
                ]);

                return {
                    count: results.length,
                    results: results
                };
            }
        """)

        assert result["success"] is True
        assert result["value"]["count"] == 5
        assert len(call_log) == 5
