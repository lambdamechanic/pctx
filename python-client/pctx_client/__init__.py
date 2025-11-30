"""
PCTX Python Client - Complete client for Port of Context

This package provides Python clients for PCTX:
- PctxClient: WebSocket client for local tool registration
- McpClient: HTTP client for MCP operations
- PctxUnifiedClient: Combined client for both MCP and local tools
"""

from .client import PctxClient
from .mcp_client import McpClient
from .unified_client import PctxUnifiedClient
from .exceptions import PctxError, ConnectionError, ExecutionError, ToolError

__all__ = [
    "PctxClient",
    "McpClient",
    "PctxUnifiedClient",
    "PctxError",
    "ConnectionError",
    "ExecutionError",
    "ToolError",
]

__version__ = "0.1.0"
