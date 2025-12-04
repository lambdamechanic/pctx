"""
PCTX Client

Main client for executing code with both MCP tools and local Python tools.
"""

from typing import Any
from urllib.parse import urlparse

from httpx import AsyncClient
from pctx.client.models import (
    GetFunctionDetailsOutput,
    ListFunctionsOutput,
    ServerConfig,
    ToolConfig,
)
from pctx.tools.tool import Tool
from pytest import Session

from .websocket_client import WebSocketClient


class Pctx:
    """
    PCTX Client

    Execute TypeScript/JavaScript code with access to both MCP tools and local Python tools.
    """

    def __init__(
        self,
        tools: dict[str, list[Tool]] | None = None,
        servers: list[ServerConfig] | None = None,
        url: str = "http://localhost:8080",
    ):
        """
        Initialize the PCTX client.
        """

        # Parse and normalize the URL
        parsed = urlparse(url)

        # Determine the base host and port
        if parsed.scheme in ["ws", "wss"]:
            # WebSocket URL provided - derive HTTP from it
            http_scheme = "https" if parsed.scheme == "wss" else "http"
            host = parsed.netloc
        elif parsed.scheme in ["http", "https"]:
            # HTTP URL provided - derive WebSocket from it
            http_scheme = parsed.scheme
            host = parsed.netloc
        else:
            raise ValueError(
                f"Invalid URL scheme: {parsed.scheme}. Expected http, https, ws, or wss"
            )

        ws_scheme = "wss" if http_scheme == "https" else "ws"

        self._ws_client = WebSocketClient(url=f"{ws_scheme}://{host}/ws")
        self._client = AsyncClient(base_url=f"{http_scheme}://{host}")
        self._session_id: str | None = None

        self._tools = tools or {}
        self._servers = servers or []

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.disconnect()

    async def connect(self):
        """Connect to WebSocket, register local tools, and register MCP servers."""
        if self._session_id is not None:
            await self.disconnect()

        connect_res = await self._client.post("/code-mode/session/create")
        connect_res.raise_for_status()
        self._session_id = connect_res.json()["session_id"]
        self._client.headers = {"x-code-mode-session": self._session_id or ""}

        # Connect WebSocket client
        await self._ws_client.connect(self._session_id or "")

        # Register all local tools
        configs: list[ToolConfig] = []
        for namespace, tools in self._tools.items():
            if len(tools) == 0:
                continue

            configs.extend(
                [
                    {
                        "name": t.name,
                        "namespace": namespace,
                        "description": t.description,
                        "input_schema": t.input_schema.model_json_schema()
                        if t.input_schema
                        else None,
                        "output_schema": t.output_schema.model_json_schema()
                        if t.output_schema
                        else None,
                    }
                    for t in tools
                ]
            )
        print("registering...")
        await self._register_tools(configs)
        await self._register_servers(self._servers)

        # Register additional MCP servers

    async def disconnect(self):
        """Disconnect from all endpoints."""
        await self._ws_client.disconnect()
        close_res = await self._client.post("/code-mode/session/close")
        close_res.raise_for_status()
        self._session_id = None

    # ========== Main execution method ==========

    async def list_functions(self) -> ListFunctionsOutput:
        if self._session_id is None:
            raise Session(
                "No code mode session exists, run Pctx(...).connect() before calling"
            )
        list_res = await self._client.post("/code-mode/functions/list")
        list_res.raise_for_status()

        return list_res.json()

    async def get_function_details(
        self, functions: list[str]
    ) -> GetFunctionDetailsOutput:
        if self._session_id is None:
            raise Session(
                "No code mode session exists, run Pctx(...).connect() before calling"
            )
        list_res = await self._client.post(
            "/code-mode/functions/details", json={"functions": functions}
        )
        list_res.raise_for_status()

        return list_res.json()

    # ========== Registrations ==========

    async def _register_tools(self, configs: list[ToolConfig]):
        res = await self._client.post("/register/tools", json={"tools": configs})
        res.raise_for_status()

    async def _register_servers(self, configs: list[ServerConfig]):
        res = await self._client.post("/register/servers", json={"servers": configs})
        res.raise_for_status()
