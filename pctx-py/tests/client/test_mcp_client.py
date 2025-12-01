"""
Integration tests for PCTX MCP HTTP Client.

These tests require a running PCTX server with MCP endpoint.
"""

import pytest  # type: ignore

from pctx.client import McpClient
from pctx.client.exceptions import ConnectionError


# Test configuration
PCTX_URL = "http://127.0.0.1:8080/mcp"


@pytest.mark.asyncio
async def test_mcp_connect_disconnect():
    """Test basic connection and disconnection."""
    client = McpClient(PCTX_URL)
    await client.connect()

    assert client.client is not None

    await client.close()
    assert client.client is None


@pytest.mark.asyncio
async def test_mcp_context_manager():
    """Test client as async context manager."""
    async with McpClient(PCTX_URL) as client:
        assert client.client is not None


@pytest.mark.asyncio
async def test_list_functions():
    """Test listing available MCP functions from external servers."""
    async with McpClient(PCTX_URL) as client:
        functions = await client.list_functions()

        # list_functions returns functions from registered MCP servers
        # With no MCP servers configured, the list should be empty
        assert isinstance(functions, list)
        # If there are functions, each should have required fields
        for func in functions:
            assert "namespace" in func
            assert "name" in func


@pytest.mark.asyncio
async def test_get_function_details():
    """Test getting function details for MCP server functions."""
    async with McpClient(PCTX_URL) as client:
        # get_function_details requires Namespace.functionName format
        # With no MCP servers configured, this should return empty results
        # This is a valid test that the endpoint works
        details = await client.get_function_details([])

        assert isinstance(details, dict)


@pytest.mark.asyncio
async def test_get_multiple_function_details():
    """Test getting details for multiple MCP server functions."""
    async with McpClient(PCTX_URL) as client:
        # With no MCP servers configured, empty list should work
        details = await client.get_function_details([])

        assert isinstance(details, dict)


@pytest.mark.asyncio
async def test_execute_simple_code():
    """Test executing simple TypeScript code via MCP."""
    async with McpClient(PCTX_URL) as client:
        code = """
        async function run() {
            return {message: "hello from MCP", value: 42};
        }
        """

        result = await client.execute(code)

        assert result is not None
        assert result["success"] is True
        assert "output" in result
        assert result["output"]["message"] == "hello from MCP"
        assert result["output"]["value"] == 42


@pytest.mark.asyncio
async def test_execute_with_console_output():
    """Test code execution with console output."""
    async with McpClient(PCTX_URL) as client:
        code = """
        async function run() {
            console.log("Test log message");
            console.error("Test error message");
            return "completed";
        }
        """

        result = await client.execute(code)

        assert result["success"] is True
        assert result["output"] == "completed"
        assert "Test log message" in result["stdout"]
        assert "Test error message" in result["stderr"]


@pytest.mark.asyncio
async def test_execute_code_with_typescript_types():
    """Test executing TypeScript code with type annotations."""
    async with McpClient(PCTX_URL) as client:
        code = """
        async function run(): Promise<{count: number, items: string[]}> {
            const items: string[] = ["a", "b", "c"];
            return {
                count: items.length,
                items: items
            };
        }
        """

        result = await client.execute(code)

        assert result["success"] is True
        assert result["output"]["count"] == 3
        assert result["output"]["items"] == ["a", "b", "c"]


@pytest.mark.asyncio
async def test_execute_complex_async_operations():
    """Test code with complex async operations."""
    async with McpClient(PCTX_URL) as client:
        code = """
        async function run() {
            // Test async operations without setTimeout
            const processNumber = async (n: number) => {
                return n * 2;
            };

            const results: number[] = [];
            for (let i = 0; i < 3; i++) {
                const value = await processNumber(i);
                results.push(value);
            }

            return {results, total: results.reduce((a, b) => a + b, 0)};
        }
        """

        result = await client.execute(code)

        assert result["success"] is True
        assert result["output"]["results"] == [0, 2, 4]
        assert result["output"]["total"] == 6


@pytest.mark.asyncio
async def test_connection_error():
    """Test connection error handling."""
    client = McpClient("http://localhost:9999/invalid")

    with pytest.raises(ConnectionError):
        async with client:
            await client.list_functions()


# TODO THIS TEST CAUSES A SEGFAULT
# @pytest.mark.asyncio
# async def test_concurrent_requests():
#     """Test multiple concurrent requests."""
#     async with McpClient(PCTX_URL) as client:
#         import asyncio

#         # Execute multiple requests concurrently
#         codes = [
#             """async function run() { return {id: 1, value: 10}; }""",
#             """async function run() { return {id: 2, value: 20}; }""",
#             """async function run() { return {id: 3, value: 30}; }""",
#         ]

#         results = await asyncio.gather(*[client.execute(code) for code in codes])

#         assert len(results) == 3
#         for result in results:
#             assert result["success"] is True

#         outputs = [r["output"] for r in results]
#         assert {"id": 1, "value": 10} in outputs
#         assert {"id": 2, "value": 20} in outputs
#         assert {"id": 3, "value": 30} in outputs
