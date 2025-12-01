"""
Integration tests for PCTX Python client.

These tests require a running PCTX WebSocket server with code execution enabled.
"""

import asyncio
import pytest

from pctx_client import PctxClient
from pctx_client.exceptions import ConnectionError, ExecutionError, ToolError


# Test configuration
WEBSOCKET_URL = "ws://localhost:8765/local-tools"


@pytest.mark.asyncio
async def test_connect_disconnect():
    """Test basic connection and disconnection."""
    client = PctxClient(WEBSOCKET_URL)
    await client.connect()

    assert client.session_id is not None
    assert client.ws is not None

    await client.disconnect()
    assert client.ws is None


@pytest.mark.asyncio
async def test_context_manager():
    """Test client as async context manager."""
    async with PctxClient(WEBSOCKET_URL) as client:
        assert client.session_id is not None
        assert client.ws is not None


@pytest.mark.asyncio
async def test_register_tool():
    """Test registering a Python tool."""
    async with PctxClient(WEBSOCKET_URL) as client:
        def add_callback(params):
            return {"result": params["a"] + params["b"]}

        await client.register_tool(
            namespace="math",
            name="add",
            callback=add_callback,
            description="Adds two numbers",
            input_schema={
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                },
                "required": ["a", "b"]
            }
        )

        assert "math.add" in client.tools


@pytest.mark.asyncio
async def test_execute_simple_code():
    """Test executing simple TypeScript code."""
    async with PctxClient(WEBSOCKET_URL) as client:
        code = """
        async function run() {
            return {message: "hello", value: 42};
        }
        """

        result = await client.execute_code(code)

        assert result is not None
        assert "value" in result
        assert result["value"]["message"] == "hello"
        assert result["value"]["value"] == 42


@pytest.mark.asyncio
async def test_execute_with_console_output():
    """Test code execution with console output."""
    async with PctxClient(WEBSOCKET_URL) as client:
        code = """
        async function run() {
            console.log("Test output");
            console.error("Test error");
            return "done";
        }
        """

        result = await client.execute_code(code)

        assert result["value"] == "done"
        assert "Test output" in result["stdout"]
        assert "Test error" in result["stderr"]


@pytest.mark.asyncio
async def test_execute_code_with_error():
    """Test code execution that throws an error."""
    async with PctxClient(WEBSOCKET_URL) as client:
        code = """
        async function run(): Promise<void> {
            throw new Error("Test error");
        }
        """

        with pytest.raises(ExecutionError) as exc_info:
            await client.execute_code(code)

        assert "Test error" in str(exc_info.value)


@pytest.mark.asyncio
async def test_tool_callback_execution():
    """Test that tool callbacks are executed when called from code."""
    async with PctxClient(WEBSOCKET_URL) as client:
        # Track callback invocations
        callback_called = []

        def multiply_callback(params):
            callback_called.append(params)
            return {"result": params["a"] * params["b"]}

        await client.register_tool(
            namespace="math",
            name="multiply",
            callback=multiply_callback,
            description="Multiplies two numbers"
        )

        code = """
        async function run() {
            const result = await math.multiply({a: 5, b: 3});
            return result;
        }
        """

        result = await client.execute_code(code)

        # Verify callback was called with correct params
        assert len(callback_called) == 1
        assert callback_called[0] == {"a": 5, "b": 3}

        # Verify result
        assert result["value"]["result"] == 15


@pytest.mark.asyncio
async def test_async_tool_callback():
    """Test async tool callbacks."""
    async with PctxClient(WEBSOCKET_URL) as client:
        async def async_callback(params):
            await asyncio.sleep(0.01)  # Simulate async work
            return {"result": params["value"] * 2}

        await client.register_tool(
            namespace="async",
            name="double",
            callback=async_callback
        )

        code = """
        async function run() {
            const result = await async.double({value: 21});
            return result;
        }
        """

        result = await client.execute_code(code)
        assert result["value"]["result"] == 42


@pytest.mark.asyncio
async def test_multiple_tools_same_namespace():
    """Test registering multiple tools in the same namespace."""
    async with PctxClient(WEBSOCKET_URL) as client:
        await client.register_tool(
            namespace="math",
            name="add",
            callback=lambda params: {"result": params["a"] + params["b"]}
        )

        await client.register_tool(
            namespace="math",
            name="subtract",
            callback=lambda params: {"result": params["a"] - params["b"]}
        )

        assert "math.add" in client.tools
        assert "math.subtract" in client.tools


@pytest.mark.asyncio
async def test_tool_with_complex_return():
    """Test tool callback with complex return value."""
    async with PctxClient(WEBSOCKET_URL) as client:
        def complex_callback(params):
            return {
                "numbers": [1, 2, 3],
                "nested": {"key": "value"},
                "count": 42
            }

        await client.register_tool(
            namespace="test",
            name="complex",
            callback=complex_callback
        )

        code = """
        async function run() {
            const result = await test.complex({});
            return result;
        }
        """

        result = await client.execute_code(code)
        value = result["value"]

        assert value["numbers"] == [1, 2, 3]
        assert value["nested"]["key"] == "value"
        assert value["count"] == 42


@pytest.mark.asyncio
async def test_tool_callback_exception():
    """Test that exceptions in tool callbacks are handled properly."""
    async with PctxClient(WEBSOCKET_URL) as client:
        def failing_callback(params):
            raise ValueError("Callback failed!")

        await client.register_tool(
            namespace="test",
            name="failing",
            callback=failing_callback
        )

        code = """
        async function run() {
            try {
                const result = await test.failing({});
                return {success: true, result};
            } catch (error) {
                return {success: false, error: error.message};
            }
        }
        """

        result = await client.execute_code(code)
        value = result["value"]

        assert value["success"] is False
        assert "Callback failed" in value["error"]


@pytest.mark.asyncio
async def test_concurrent_code_execution():
    """Test executing multiple code requests concurrently."""
    async with PctxClient(WEBSOCKET_URL) as client:
        codes = [
            """async function run() { return {id: 1, value: 10 + 5}; }""",
            """async function run() { return {id: 2, value: 20 * 2}; }""",
            """async function run() { return {id: 3, value: 100 / 4}; }""",
        ]

        results = await asyncio.gather(*[
            client.execute_code(code) for code in codes
        ])

        assert len(results) == 3
        values = [r["value"] for r in results]

        # Results should match (order may vary)
        assert {"id": 1, "value": 15} in values
        assert {"id": 2, "value": 40} in values
        assert {"id": 3, "value": 25} in values


@pytest.mark.asyncio
async def test_tool_with_optional_schemas():
    """Test registering tool with optional schemas."""
    async with PctxClient(WEBSOCKET_URL) as client:
        # No schemas
        await client.register_tool(
            namespace="test",
            name="no_schema",
            callback=lambda params: {"ok": True}
        )

        # Only input schema
        await client.register_tool(
            namespace="test",
            name="input_only",
            callback=lambda params: params.get("value", 0) * 2,
            input_schema={"type": "object"}
        )

        # Both schemas
        await client.register_tool(
            namespace="test",
            name="both_schemas",
            callback=lambda params: {"doubled": params["n"] * 2},
            input_schema={
                "type": "object",
                "properties": {"n": {"type": "number"}}
            },
            output_schema={
                "type": "object",
                "properties": {"doubled": {"type": "number"}}
            }
        )

        assert len(client.tools) == 3


@pytest.mark.asyncio
async def test_connection_error():
    """Test connection error handling."""
    client = PctxClient("ws://localhost:9999/invalid")

    with pytest.raises(ConnectionError):
        await client.connect()
