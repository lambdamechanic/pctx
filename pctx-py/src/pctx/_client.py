"""
PCTX Client

Main client for executing code with both MCP tools and local Python tools.
"""

from typing import TYPE_CHECKING
from urllib.parse import urlparse

from httpx import AsyncClient
from pytest import Session

from pctx._tool import Tool
from pctx._websocket_client import WebSocketClient
from pctx.models import (
    ExecuteOutput,
    GetFunctionDetailsOutput,
    ListFunctionsOutput,
    ServerConfig,
    ToolConfig,
)

if TYPE_CHECKING:
    try:
        from langchain_core.tools import BaseTool as LangchainBaseTool

    except ImportError:
        pass


class Pctx:
    """
    PCTX Client

    Execute TypeScript/JavaScript code with access to both MCP tools and local Python tools.
    """

    def __init__(
        self,
        tools: list[Tool] | None = None,
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

        self._ws_client = WebSocketClient(url=f"{ws_scheme}://{host}/ws", tools=tools)
        self._client = AsyncClient(base_url=f"{http_scheme}://{host}")
        self._session_id: str | None = None

        self._tools = tools or []
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
        configs: list[ToolConfig] = [
            {
                "name": t.name,
                "namespace": t.namespace,
                "description": t.description,
                "input_schema": t.input_schema.model_json_schema()
                if t.input_schema
                else None,
                "output_schema": t.output_schema.model_json_schema()
                if t.output_schema
                else None,
            }
            for t in self._tools
        ]

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

    # ========== Main code mode methods method ==========

    async def list_functions(self) -> ListFunctionsOutput:
        if self._session_id is None:
            raise Session(
                "No code mode session exists, run Pctx(...).connect() before calling"
            )
        list_res = await self._client.post("/code-mode/functions/list")
        list_res.raise_for_status()

        return ListFunctionsOutput.model_validate(list_res.json())

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

        return GetFunctionDetailsOutput.model_validate(list_res.json())

    async def execute(self, code: str, timeout: float = 30.0) -> ExecuteOutput:
        if self._session_id is None:
            raise Session(
                "No code mode session exists, run Pctx(...).connect() before calling"
            )
        return await self._ws_client.execute_code(code, timeout=timeout)

    # ========== Registrations ==========

    async def _register_tools(self, configs: list[ToolConfig]):
        res = await self._client.post("/register/tools", json={"tools": configs})
        res.raise_for_status()

    async def _register_servers(self, configs: list[ServerConfig]):
        res = await self._client.post("/register/servers", json={"servers": configs})
        res.raise_for_status()

    def langchain_tools(self) -> "list[LangchainBaseTool]":
        """
        Expose PCTX code mode tools as langchain tools

        Requires the 'langchain' extra to be installed:
            pip install pctx[langchain]

        Raises:
            ImportError: If langchain is not installed.
        """
        try:
            from langchain_core.tools import tool as langchain_tool
        except ImportError as e:
            raise ImportError(
                "LangChain is not installed. Install it with: pip install pctx[langchain]"
            ) from e

        @langchain_tool
        async def list_functions() -> ListFunctionsOutput:
            """
            ALWAYS USE THIS TOOL FIRST to list all available functions organized by namespace.

            WORKFLOW:
            1. Start here - Call this tool to see what functions are available
            2. Then call get_function_details() for specific functions you need to understand
            3. Finally call execute() to run your TypeScript code

            This returns function signatures without full details.
            """
            return await self.list_functions()

        @langchain_tool
        async def get_function_details(
            functions: list[str],
        ) -> GetFunctionDetailsOutput:
            """
            Get detailed information about specific functions you want to use.

            WHEN TO USE: After calling list_functions(), use this to learn about parameter types, return values, and usage for specific functions.

            REQUIRED FORMAT: Functions must be specified as 'namespace.functionName' (e.g., 'Namespace.apiPostSearch')

            This tool is lightweight and only returns details for the functions you request, avoiding unnecessary token usage.
            Only request details for functions you actually plan to use in your code.

            NOTE ON RETURN TYPES:
            - If a function returns Promise<any>, the MCP server didn't provide an output schema
            - The actual value is a parsed object (not a string) - access properties directly
            - Don't use JSON.parse() on the results - they're already JavaScript objects
            """
            return await self.get_function_details(
                functions,
            )

        @langchain_tool
        async def execute(code: str, timeout: float = 30) -> ExecuteOutput:
            """
            Execute TypeScript code that calls namespaced functions. USE THIS LAST after list_functions() and get_function_details().

            TOKEN USAGE WARNING: This tool could return LARGE responses if your code returns big objects.
            To minimize tokens:
            - Filter/map/reduce data IN YOUR CODE before returning
            - Only return specific fields you need (e.g., return {id: result.id, count: items.length})
            - Use console.log() for intermediate results instead of returning everything
            - Avoid returning full API responses - extract just what you need

            REQUIRED CODE STRUCTURE:
            async function run() {
                // Your code here
                // Call namespace.functionName() - MUST include namespace prefix
                // Process data here to minimize return size
                return onlyWhatYouNeed; // Keep this small!
            }

            IMPORTANT RULES:
            - Functions MUST be called as 'Namespace.functionName' (e.g., 'Notion.apiPostSearch')
            - Only functions from list_functions() are available - no fetch(), fs, or other Node/Deno APIs
            - Variables don't persist between execute() calls - return or log anything you need later
            - Add console.log() statements between API calls to track progress if errors occur
            - Code runs in an isolated Deno sandbox with restricted network access

            RETURN TYPE NOTE:
            - Functions without output schemas show Promise<any> as return type
            - The actual runtime value is already a parsed JavaScript object, NOT a JSON string
            - Do NOT call JSON.parse() on results - they're already objects
            - Access properties directly (e.g., result.data) or inspect with console.log() first
            - If you see 'Promise<any>', the structure is unknown - log it to see what's returned
            """
            return await self.execute(code, timeout=timeout)

        return [list_functions, get_function_details, execute]
