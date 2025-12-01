"""
End-to-end integration tests for PCTX Unified Client.

These tests require a running PCTX server with both MCP and WebSocket endpoints.
"""

import asyncio
import pytest  # type: ignore

from pctx.client import PctxClient


# Test configuration
SERVER_URL = "http://localhost:8080"


@pytest.mark.asyncio
async def test_unified_connect_disconnect():
    """Test unified client connection and disconnection."""
    client = PctxClient(SERVER_URL)
    await client.connect()

    assert len(client.mcp_clients) > 0
    assert client.ws_client is not None
    assert client.ws_client.session_id is not None

    await client.disconnect()


@pytest.mark.asyncio
async def test_unified_context_manager():
    """Test unified client as async context manager."""
    async with PctxClient(SERVER_URL) as client:
        assert len(client.mcp_clients) > 0
        assert client.ws_client is not None


@pytest.mark.asyncio
async def test_list_functions_via_unified():
    """Test listing MCP functions via unified client."""
    async with PctxClient(SERVER_URL) as client:
        functions = await client.list_mcp_functions()

        # Should return a list (may be empty if no MCP servers configured)
        assert isinstance(functions, list)


@pytest.mark.asyncio
async def test_register_and_use_local_tool():
    """Test registering a local tool and calling it from TypeScript."""
    async with PctxClient(SERVER_URL) as client:
        # Register a local Python tool
        callback_called = []

        def test_tool(params):
            callback_called.append(params)
            return {"result": params.get("value", 0) * 10}

        await client.register_local_tool(
            namespace="TestTools",
            name="multiply",
            callback=test_tool,
            description="Multiplies value by 10",
        )

        # Execute TypeScript code that calls the registered tool
        code = """
        async function run() {
            const result = await TestTools.multiply({value: 5});
            return result;
        }
        """
        result = await client.execute(code)

        # Verify callback was called
        assert len(callback_called) == 1
        assert callback_called[0]["value"] == 5

        # Verify result
        assert result["value"]["result"] == 50


@pytest.mark.asyncio
async def test_list_all_tools():
    """Test listing both MCP functions and local tools."""
    async with PctxClient(SERVER_URL) as client:
        # Register a local tool
        await client.register_local_tool(
            namespace="LocalTools", name="test", callback=lambda params: {"ok": True}
        )

        # List MCP tools (may be empty)
        mcp_tools = await client.list_mcp_functions()
        assert isinstance(mcp_tools, list)

        # List local tools - should include our registered tool
        local_tools = client.list_local_tools()
        assert isinstance(local_tools, list)
        assert len(local_tools) >= 1
        assert "LocalTools.test" in local_tools


@pytest.mark.asyncio
async def test_list_local_tools():
    """Test listing registered local tools and calling them from TypeScript."""
    async with PctxClient(SERVER_URL) as client:
        # Register multiple tools
        await client.register_local_tool(
            namespace="Math", name="add", callback=lambda p: p["a"] + p["b"]
        )

        await client.register_local_tool(
            namespace="Math", name="multiply", callback=lambda p: p["a"] * p["b"]
        )

        await client.register_local_tool(
            namespace="String", name="upper", callback=lambda p: p["text"].upper()
        )

        local_tools = client.list_local_tools()

        assert len(local_tools) == 3
        assert "Math.add" in local_tools
        assert "Math.multiply" in local_tools
        assert "String.upper" in local_tools

        # Test using the tools from TypeScript
        code = """
        async function run() {
            const addResult = await Math.add({a: 5, b: 3});
            const multiplyResult = await Math.multiply({a: 4, b: 7});
            const upperResult = await String.upper({text: "hello"});
            return {addResult, multiplyResult, upperResult};
        }
        """
        result = await client.execute(code)

        assert result["value"]["addResult"] == 8
        assert result["value"]["multiplyResult"] == 28
        assert result["value"]["upperResult"] == "HELLO"


@pytest.mark.asyncio
async def test_mixed_mcp_and_local_tools():
    """Test using both MCP functions and local tools from TypeScript."""
    async with PctxClient(SERVER_URL) as client:
        # Register a local tool
        await client.register_local_tool(
            namespace="DataProcessor",
            name="process",
            callback=lambda p: {"processed": True, "data": p.get("data", [])},
        )

        # Call local tool from TypeScript
        code = """
        async function run() {
            const result = await DataProcessor.process({data: [1, 2, 3]});
            return result;
        }
        """
        result = await client.execute(code)

        assert result["value"]["processed"] is True
        assert result["value"]["data"] == [1, 2, 3]


@pytest.mark.asyncio
async def test_async_local_tool():
    """Test registering and using an async local tool from TypeScript."""
    async with PctxClient(SERVER_URL) as client:

        async def async_tool(params):
            await asyncio.sleep(0.01)  # Simulate async work
            return {"result": params["value"] * 3}

        await client.register_local_tool(
            namespace="AsyncTools", name="triple", callback=async_tool
        )

        # Call async tool from TypeScript
        code = """
        async function run() {
            const result = await AsyncTools.triple({value: 7});
            return result;
        }
        """
        result = await client.execute(code)

        assert result["value"]["result"] == 21


@pytest.mark.asyncio
async def test_local_tool_with_error():
    """Test local tool that raises an error."""
    async with PctxClient(SERVER_URL) as client:

        def failing_tool(params):
            raise ValueError("Tool failed intentionally")

        await client.register_local_tool(
            namespace="ErrorTools", name="fail", callback=failing_tool
        )

        # Call failing tool from TypeScript - should propagate error
        code = """
        async function run() {
            try {
                await ErrorTools.fail({});
                return {success: false, error: "Should have thrown"};
            } catch (e) {
                return {success: true, error: e.message};
            }
        }
        """
        result = await client.execute(code)

        assert result["value"]["success"] is True
        assert "Tool failed intentionally" in result["value"]["error"]


@pytest.mark.asyncio
async def test_concurrent_local_tool_calls():
    """Test concurrent calls to local tools from TypeScript."""
    async with PctxClient(SERVER_URL) as client:
        call_count = []

        def counter_tool(params):
            call_count.append(params)
            return {"count": len(call_count), "id": params.get("id")}

        await client.register_local_tool(
            namespace="Counter", name="increment", callback=counter_tool
        )

        # Make concurrent calls from TypeScript
        code = """
        async function run() {
            const results = await Promise.all([
                Counter.increment({id: 0}),
                Counter.increment({id: 1}),
                Counter.increment({id: 2}),
                Counter.increment({id: 3}),
                Counter.increment({id: 4})
            ]);
            return results;
        }
        """
        result = await client.execute(code)

        # All calls should have completed
        assert len(call_count) == 5
        assert isinstance(result["value"], list)
        assert len(result["value"]) == 5
        # Each result should have a count
        for item in result["value"]:
            assert "count" in item


@pytest.mark.asyncio
async def test_local_tool_with_complex_data():
    """Test local tool with complex nested data structures."""
    async with PctxClient(SERVER_URL) as client:

        def complex_tool(params):
            return {
                "input": params,
                "nested": {
                    "arrays": [[1, 2], [3, 4]],
                    "objects": {"key1": "value1", "key2": "value2"},
                },
                "processed": True,
            }

        await client.register_local_tool(
            namespace="DataTools", name="complex", callback=complex_tool
        )

        # Call with complex data from TypeScript
        code = """
        async function run() {
            const result = await DataTools.complex({id: 123, data: {nested: "value"}});
            return result;
        }
        """
        result = await client.execute(code)

        assert result["value"]["processed"] is True
        assert result["value"]["input"]["id"] == 123
        assert result["value"]["nested"]["arrays"] == [[1, 2], [3, 4]]


@pytest.mark.asyncio
async def test_unified_client_basic_execution():
    """Test basic code execution without using any tools."""
    async with PctxClient(SERVER_URL) as client:
        # Should be able to execute basic code
        code = """
        async function run() {
            return 42;
        }
        """
        result = await client.execute(code)
        assert result["value"] == 42

        # MCP client should be automatically connected to main server
        assert len(client.mcp_clients) == 1


@pytest.mark.asyncio
async def test_multiple_namespaces_local_tools():
    """Test registering tools in multiple namespaces."""
    async with PctxClient(SERVER_URL) as client:
        # Register tools in different namespaces
        await client.register_local_tool(
            namespace="Math", name="add", callback=lambda p: p["a"] + p["b"]
        )

        await client.register_local_tool(
            namespace="String", name="concat", callback=lambda p: p["a"] + p["b"]
        )

        await client.register_local_tool(
            namespace="Array", name="merge", callback=lambda p: p["a"] + p["b"]
        )

        # Call tools from different namespaces in TypeScript
        code = """
        async function run() {
            const mathResult = await Math.add({a: 10, b: 20});
            const stringResult = await String.concat({a: "hello", b: "world"});
            const arrayResult = await Array.merge({a: [1, 2], b: [3, 4]});
            return {mathResult, stringResult, arrayResult};
        }
        """
        result = await client.execute(code)

        assert result["value"]["mathResult"] == 30
        assert result["value"]["stringResult"] == "helloworld"
        assert result["value"]["arrayResult"] == [1, 2, 3, 4]
