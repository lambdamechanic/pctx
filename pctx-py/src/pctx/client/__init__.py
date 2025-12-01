"""
PCTX Python Client - Execute code with MCP and local Python tools

This package provides the main PctxClient for executing TypeScript/JavaScript
code with access to both MCP tools and local Python callbacks.

Main client:
- PctxClient: Execute code with both MCP and local tools (from unified_client.py)

Low-level clients (for advanced use):
- WebSocketClient: Direct WebSocket client for local tool registration
- McpClient: Direct HTTP client for MCP operations
"""

from .pctx_client import PctxClient
from .websocket_client import WebSocketClient
from .mcp_client import McpClient
from .exceptions import PctxError, ConnectionError, ExecutionError, ToolError

__all__ = [
    "PctxClient",
    "WebSocketClient",
    "McpClient",
    "PctxError",
    "ConnectionError",
    "ExecutionError",
    "ToolError",
]

__version__ = "0.1.0"
