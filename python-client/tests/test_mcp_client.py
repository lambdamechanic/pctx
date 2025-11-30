"""
Integration tests for PCTX MCP HTTP Client.

These tests require a running PCTX server with MCP endpoint.
"""

import pytest

from pctx_client import McpClient
from pctx_client.exceptions import ConnectionError, ExecutionError


# Test configuration
MCP_URL = "http://localhost:8080/mcp"


@pytest.mark.asyncio
async def test_mcp_connect_disconnect():
    """Test basic connection and disconnection."""
    client = McpClient(MCP_URL)
    await client.connect()

    assert client.client is not None

    await client.close()
    assert client.client is None


@pytest.mark.asyncio
async def test_mcp_context_manager():
    """Test client as async context manager."""
    async with McpClient(MCP_URL) as client:
        assert client.client is not None


@pytest.mark.asyncio
async def test_list_functions():
    """Test listing available MCP functions."""
    async with McpClient(MCP_URL) as client:
        functions = await client.list_functions()

        assert isinstance(functions, list)
        # Should have at least the built-in tools
        assert len(functions) >= 3

        # Check for built-in tools
        function_names = [f.get("name") for f in functions]
        assert "list_functions" in function_names
        assert "get_function_details" in function_names
        assert "execute" in function_names


@pytest.mark.asyncio
async def test_get_function_details():
    """Test getting function details."""
    async with McpClient(MCP_URL) as client:
        # Get details for the execute function
        details = await client.get_function_details(["execute"])

        assert isinstance(details, dict)
        assert "execute" in details

        execute_details = details["execute"]
        assert "name" in execute_details
        assert execute_details["name"] == "execute"
        assert "input_schema" in execute_details
        assert "output_schema" in execute_details


@pytest.mark.asyncio
async def test_get_multiple_function_details():
    """Test getting details for multiple functions."""
    async with McpClient(MCP_URL) as client:
        details = await client.get_function_details([
            "list_functions",
            "get_function_details",
            "execute"
        ])

        assert isinstance(details, dict)
        assert len(details) == 3
        assert "list_functions" in details
        assert "get_function_details" in details
        assert "execute" in details


@pytest.mark.asyncio
async def test_execute_simple_code():
    """Test executing simple TypeScript code via MCP."""
    async with McpClient(MCP_URL) as client:
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
    async with McpClient(MCP_URL) as client:
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
async def test_execute_with_mcp_tools():
    """Test executing code that uses MCP tools."""
    async with McpClient(MCP_URL) as client:
        # First, list functions to ensure MCP servers are available
        functions = await client.list_functions()

        # Find an MCP namespace (not the built-in tools)
        mcp_functions = [f for f in functions if "." in f.get("name", "")]

        if mcp_functions:
            # Test executing code that uses an MCP function
            mcp_func = mcp_functions[0]
            namespace = mcp_func["name"].split(".")[0]

            code = f"""
            async function run() {{
                // List available functions in the namespace
                const functions = await {namespace}.constructor.name;
                return {{namespace: "{namespace}", available: true}};
            }}
            """

            result = await client.execute(code)
            assert result["success"] is True
        else:
            pytest.skip("No MCP servers configured for testing")


@pytest.mark.asyncio
async def test_execute_code_with_error():
    """Test code execution that throws an error."""
    async with McpClient(MCP_URL) as client:
        code = """
        async function run() {
            throw new Error("Intentional test error");
        }
        """

        with pytest.raises(ExecutionError) as exc_info:
            await client.execute(code)

        assert "Intentional test error" in str(exc_info.value)


@pytest.mark.asyncio
async def test_execute_code_with_typescript_types():
    """Test executing TypeScript code with type annotations."""
    async with McpClient(MCP_URL) as client:
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
    async with McpClient(MCP_URL) as client:
        code = """
        async function run() {
            // Simulate async operations
            const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

            const results: number[] = [];
            for (let i = 0; i < 3; i++) {
                await delay(10);
                results.push(i * 2);
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


@pytest.mark.asyncio
async def test_concurrent_requests():
    """Test multiple concurrent requests."""
    async with McpClient(MCP_URL) as client:
        import asyncio

        # Execute multiple requests concurrently
        codes = [
            """async function run() { return {id: 1, value: 10}; }""",
            """async function run() { return {id: 2, value: 20}; }""",
            """async function run() { return {id: 3, value: 30}; }""",
        ]

        results = await asyncio.gather(*[
            client.execute(code) for code in codes
        ])

        assert len(results) == 3
        for result in results:
            assert result["success"] is True

        outputs = [r["output"] for r in results]
        assert {"id": 1, "value": 10} in outputs
        assert {"id": 2, "value": 20} in outputs
        assert {"id": 3, "value": 30} in outputs
