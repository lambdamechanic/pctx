"""
Pytest configuration for PCTX client tests.

Provides fixtures and utilities for testing.
"""

import asyncio
import os

import pytest


@pytest.fixture(scope="session")
def event_loop():
    """Create an instance of the default event loop for the test session."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(scope="session", autouse=True)
def check_pctx_server():
    """
    Check if PCTX server is running before running tests.

    Set PCTX_SKIP_SERVER_CHECK=1 to skip this check.
    """
    if os.environ.get("PCTX_SKIP_SERVER_CHECK"):
        return

    import httpx

    mcp_url = os.environ.get("PCTX_MCP_URL", "http://localhost:8080/mcp")

    print(f"\nChecking if PCTX server is running at {mcp_url}...")

    try:
        response = httpx.get(mcp_url.replace("/mcp", "/"), timeout=5.0)
        print(f"✓ PCTX server is running")
    except Exception as e:
        pytest.skip(
            f"PCTX server not running at {mcp_url}. "
            f"Start it with 'cargo run --bin pctx -- start' or set PCTX_SKIP_SERVER_CHECK=1 to skip this check. "
            f"Error: {e}"
        )


@pytest.fixture
def mcp_url():
    """Get MCP URL from environment or use default."""
    return os.environ.get("PCTX_MCP_URL", "http://localhost:8080/mcp")


@pytest.fixture
def ws_url():
    """Get WebSocket URL from environment or use default."""
    return os.environ.get("PCTX_WS_URL", "ws://localhost:8080/local-tools")
