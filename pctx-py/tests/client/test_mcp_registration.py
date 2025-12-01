"""
Tests for MCP server registration via WebSocket client.

These tests verify that:
1. MCP servers can be registered through the WebSocket protocol
2. Registration validates the URL format
3. Duplicate registrations are handled correctly
"""

import pytest  # type: ignore

from pctx.client.websocket_client import WebSocketClient
from pctx.client.exceptions import ToolError


# Test configuration
WS_URL = "ws://localhost:8080/local-tools"

# Public MCP server for testing (Backdocket's public MCP server)
PUBLIC_MCP_URL = "https://ai.backdocket.com/mcp"


@pytest.mark.asyncio
async def test_register_mcp_basic():
    """Test basic MCP server registration with a public server."""
    async with WebSocketClient(WS_URL) as client:
        # Register a public MCP server (Backdocket)
        await client.register_mcp(name="backdocket", url=PUBLIC_MCP_URL)

        # If we get here without exception, registration succeeded
        assert True


@pytest.mark.asyncio
async def test_register_mcp_with_auth():
    """Test MCP server registration with authentication."""
    async with WebSocketClient(WS_URL) as client:
        # Register an MCP server with auth
        await client.register_mcp(
            name="auth-mcp",
            url="https://api.example.com",
            auth={"bearer": {"token": "test-token"}},
        )

        # If we get here without exception, registration succeeded
        assert True


@pytest.mark.asyncio
async def test_register_mcp_duplicate_fails():
    """Test that registering the same MCP server twice fails."""
    async with WebSocketClient(WS_URL) as client:
        # Register first time - should succeed
        await client.register_mcp(name="duplicate-test", url=PUBLIC_MCP_URL)

        # Try to register again with same name - should fail
        with pytest.raises(ToolError, match="already registered"):
            await client.register_mcp(
                name="duplicate-test",
                url="https://different-url.example.com",  # Different URL, same name
            )


@pytest.mark.asyncio
async def test_register_mcp_invalid_url():
    """Test that invalid URLs are rejected."""
    async with WebSocketClient(WS_URL) as client:
        # Try to register with invalid URL
        with pytest.raises(ToolError, match="Invalid URL"):
            await client.register_mcp(name="invalid-url", url="not-a-valid-url")


@pytest.mark.asyncio
async def test_register_multiple_mcp_servers():
    """Test registering multiple MCP servers."""
    async with WebSocketClient(WS_URL) as client:
        # Register the public Backdocket server
        await client.register_mcp(name="backdocket", url=PUBLIC_MCP_URL)

        # Register hypothetical servers (these won't be used, just registered)
        await client.register_mcp(name="server2", url="http://localhost:3001")

        await client.register_mcp(name="server3", url="https://api.example.com")

        # All registrations should succeed
        assert True


@pytest.mark.asyncio
async def test_register_mcp_and_tool():
    """Test that MCP registration works alongside tool registration."""
    async with WebSocketClient(WS_URL) as client:
        # Register a local tool
        await client.register_tool(
            namespace="test",
            name="add",
            callback=lambda params: {"result": params["a"] + params["b"]},
            description="Add two numbers",
        )

        # Register the public MCP server
        await client.register_mcp(name="backdocket", url=PUBLIC_MCP_URL)

        # Both should coexist
        assert True
