"""
End-to-end integration tests for PCTX Unified Client.

These tests require a running PCTX server with both MCP and WebSocket endpoints.
"""

import asyncio
import pytest

from pctx_client import PctxUnifiedClient
from pctx_client.exceptions import ConnectionError, ExecutionError


# Test configuration
MCP_URL = "http://localhost:8080/mcp"
WS_URL = "ws://localhost:8080/local-tools"


@pytest.mark.asyncio
async def test_unified_connect_disconnect():
    """Test unified client connection and disconnection."""
    client = PctxUnifiedClient(MCP_URL, WS_URL)
    await client.connect()

    assert client.mcp_client.client is not None
    assert client.ws_client is not None
    assert client.ws_client.session_id is not None

    await client.disconnect()


@pytest.mark.asyncio
async def test_unified_context_manager():
    """Test unified client as async context manager."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        assert client.mcp_client.client is not None
        assert client.ws_client is not None


@pytest.mark.asyncio
async def test_list_functions_via_unified():
    """Test listing MCP functions via unified client."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        functions = await client.list_functions()

        assert isinstance(functions, list)
        assert len(functions) >= 3

        function_names = [f.get("name") for f in functions]
        assert "list_functions" in function_names
        assert "execute" in function_names


@pytest.mark.asyncio
async def test_get_function_details_via_unified():
    """Test getting function details via unified client."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        details = await client.get_function_details(["execute"])

        assert isinstance(details, dict)
        assert "execute" in details


@pytest.mark.asyncio
async def test_execute_via_mcp():
    """Test executing code via MCP (without local tools)."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        code = """
        async function run() {
            return {source: "mcp", value: 100};
        }
        """

        result = await client.execute(code, use_ws=False)

        assert result["success"] is True
        assert result["output"]["source"] == "mcp"
        assert result["output"]["value"] == 100


@pytest.mark.asyncio
async def test_execute_via_websocket():
    """Test executing code via WebSocket."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        code = """
        async function run() {
            return {source: "websocket", value: 200};
        }
        """

        result = await client.execute(code, use_ws=True)

        assert result is not None
        assert "value" in result
        assert result["value"]["source"] == "websocket"
        assert result["value"]["value"] == 200


@pytest.mark.asyncio
async def test_register_and_use_local_tool():
    """Test registering a local tool and using it with clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        # Register a local Python tool
        callback_called = []

        def test_tool(params):
            callback_called.append(params)
            return {"result": params.get("value", 0) * 10}

        await client.register_local_tool(
            namespace="TestTools",
            name="multiply",
            callback=test_tool,
            description="Multiplies value by 10"
        )

        # Use clean Python API: client.TestTools.multiply(5)
        result = await client.TestTools.multiply(5)

        # Verify callback was called
        assert len(callback_called) == 1
        assert callback_called[0]["value"] == 5

        # Verify result
        assert result["result"] == 50


@pytest.mark.asyncio
async def test_list_all_tools():
    """Test listing both MCP functions and local tools."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        # Register a local tool
        await client.register_local_tool(
            namespace="LocalTools",
            name="test",
            callback=lambda params: {"ok": True}
        )

        # List all tools
        all_tools = await client.list_all_tools()

        assert "mcp" in all_tools
        assert "local" in all_tools

        # Check MCP tools
        assert isinstance(all_tools["mcp"], list)
        assert len(all_tools["mcp"]) >= 3

        # Check local tools
        assert isinstance(all_tools["local"], list)
        assert len(all_tools["local"]) >= 1
        assert any(t["name"] == "LocalTools.test" for t in all_tools["local"])


@pytest.mark.asyncio
async def test_list_local_tools():
    """Test listing registered local tools and using them with clean API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        # Register multiple tools
        await client.register_local_tool(
            namespace="Math",
            name="add",
            callback=lambda p: p["a"] + p["b"]
        )

        await client.register_local_tool(
            namespace="Math",
            name="multiply",
            callback=lambda p: p["a"] * p["b"]
        )

        await client.register_local_tool(
            namespace="String",
            name="upper",
            callback=lambda p: p["text"].upper()
        )

        local_tools = client.list_local_tools()

        assert len(local_tools) == 3
        assert "Math.add" in local_tools
        assert "Math.multiply" in local_tools
        assert "String.upper" in local_tools

        # Test using the tools with clean API
        add_result = await client.Math.add(a=5, b=3)
        assert add_result == 8

        multiply_result = await client.Math.multiply(a=4, b=7)
        assert multiply_result == 28

        upper_result = await client.String.upper(text="hello")
        assert upper_result == "HELLO"


@pytest.mark.asyncio
async def test_mixed_mcp_and_local_tools():
    """Test using both MCP functions and local tools with clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        # Register a local tool
        await client.register_local_tool(
            namespace="DataProcessor",
            name="process",
            callback=lambda p: {"processed": True, "data": p.get("data", [])}
        )

        # Use clean Python API
        result = await client.DataProcessor.process(data=[1, 2, 3])

        assert result["processed"] is True
        assert result["data"] == [1, 2, 3]


@pytest.mark.asyncio
async def test_async_local_tool():
    """Test registering and using an async local tool with clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        async def async_tool(params):
            await asyncio.sleep(0.01)  # Simulate async work
            return {"result": params["value"] * 3}

        await client.register_local_tool(
            namespace="AsyncTools",
            name="triple",
            callback=async_tool
        )

        # Use clean Python API: client.AsyncTools.triple(7)
        result = await client.AsyncTools.triple(7)

        assert result["result"] == 21


@pytest.mark.asyncio
async def test_local_tool_with_error():
    """Test local tool that raises an error with clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        def failing_tool(params):
            raise ValueError("Tool failed intentionally")

        await client.register_local_tool(
            namespace="ErrorTools",
            name="fail",
            callback=failing_tool
        )

        # Use clean Python API - should raise an exception
        try:
            await client.ErrorTools.fail()
            assert False, "Should have raised an exception"
        except Exception as e:
            assert "Tool failed intentionally" in str(e) or "execution failed" in str(e).lower()


@pytest.mark.asyncio
async def test_concurrent_local_tool_calls():
    """Test concurrent calls to local tools with clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        call_count = []

        def counter_tool(params):
            call_count.append(params)
            return {"count": len(call_count)}

        await client.register_local_tool(
            namespace="Counter",
            name="increment",
            callback=counter_tool
        )

        # Make concurrent calls using clean Python API
        import asyncio
        results = await asyncio.gather(*[
            client.Counter.increment(id=i) for i in range(5)
        ])

        # All calls should have completed
        assert len(call_count) == 5
        assert isinstance(results, list)
        assert len(results) == 5
        # Each result should have a count
        for result in results:
            assert "count" in result


@pytest.mark.asyncio
async def test_local_tool_with_complex_data():
    """Test local tool with complex nested data structures using clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        def complex_tool(params):
            return {
                "input": params,
                "nested": {
                    "arrays": [[1, 2], [3, 4]],
                    "objects": {"key1": "value1", "key2": "value2"}
                },
                "processed": True
            }

        await client.register_local_tool(
            namespace="DataTools",
            name="complex",
            callback=complex_tool
        )

        # Use clean Python API with keyword arguments
        result = await client.DataTools.complex(id=123, data={"nested": "value"})

        assert result["processed"] is True
        assert result["input"]["id"] == 123
        assert result["nested"]["arrays"] == [[1, 2], [3, 4]]


@pytest.mark.asyncio
async def test_unified_client_without_websocket():
    """Test unified client with only MCP (no WebSocket)."""
    async with PctxUnifiedClient(MCP_URL) as client:
        # Should still be able to use MCP functions
        functions = await client.list_functions()
        assert len(functions) >= 3

        # Execute via MCP should work
        result = await client.execute("async function run() { return 42; }")
        assert result["success"] is True
        assert result["output"] == 42

        # Trying to use WebSocket should fail
        with pytest.raises(ConnectionError):
            await client.execute("async function run() { return 1; }", use_ws=True)


@pytest.mark.asyncio
async def test_multiple_namespaces_local_tools():
    """Test registering tools in multiple namespaces with clean Python API."""
    async with PctxUnifiedClient(MCP_URL, WS_URL) as client:
        # Register tools in different namespaces
        await client.register_local_tool(
            namespace="Math",
            name="add",
            callback=lambda p: p["a"] + p["b"]
        )

        await client.register_local_tool(
            namespace="String",
            name="concat",
            callback=lambda p: p["a"] + p["b"]
        )

        await client.register_local_tool(
            namespace="Array",
            name="merge",
            callback=lambda p: p["a"] + p["b"]
        )

        # Use clean Python API
        math_result = await client.Math.add(a=10, b=20)
        string_result = await client.String.concat(a="hello", b="world")
        array_result = await client.Array.merge(a=[1, 2], b=[3, 4])

        assert math_result == 30
        assert string_result == "helloworld"
        assert array_result == [1, 2, 3, 4]
