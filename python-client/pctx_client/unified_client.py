"""
PCTX Client

Main client for executing code with both MCP tools and local Python tools.
"""

from typing import Any, Callable, Dict, List, Optional

from .client import PctxClient as WebSocketClient
from .mcp_client import McpClient
from .exceptions import ConnectionError


class PctxClient:
    """
    PCTX Client

    Execute TypeScript/JavaScript code with access to both MCP tools and local Python tools.

    Example:
        ```python
        # Define local tools
        def get_data(params):
            return {"data": [1, 2, 3]}

        async def fetch_user(params):
            user_id = params.get("user_id")
            return {"id": user_id, "name": "John"}

        local_tools = [
            {
                "namespace": "MyTools",
                "name": "getData",
                "callback": get_data,
                "description": "Get sample data"
            },
            {
                "namespace": "MyTools",
                "name": "fetchUser",
                "callback": fetch_user
            }
        ]

        # Initialize client with local tools
        async with PctxClient(
            ws_url="ws://localhost:8080/local-tools",
            local_tools=local_tools,
            mcps=["http://localhost:8080/mcp"]  # Optional MCP servers
        ) as client:
            # Execute code that uses both MCP and local tools
            result = await client.execute('''
                async function run() {
                    // Use MCP tool
                    const notionResults = await Notion.apiPostSearch({query: "test"});

                    // Use local Python tool
                    const localData = await MyTools.getData({});
                    const user = await MyTools.fetchUser({user_id: 123});

                    return {notionResults, localData, user};
                }
            ''')
        ```
    """

    def __init__(
        self,
        ws_url: str,
        local_tools: Optional[List[Dict[str, Any]]] = None,
        mcps: Optional[List[str]] = None,
        timeout: float = 30.0
    ):
        """
        Initialize the PCTX client.

        Args:
            ws_url: WebSocket URL (e.g., "ws://localhost:8080/local-tools")
            local_tools: Optional list of local tool definitions. Each dict should have:
                - namespace: str - Tool namespace (e.g., "MyTools")
                - name: str - Tool name (e.g., "getData")
                - callback: Callable - Python function to call
                - description: Optional[str] - Tool description
                - input_schema: Optional[Dict] - JSON schema for validation
                - output_schema: Optional[Dict] - JSON schema for validation
            mcps: Optional list of MCP server URLs for accessing MCP tools
            timeout: Request timeout in seconds
        """
        self.ws_url = ws_url
        self.timeout = timeout
        self.local_tools_config = local_tools or []
        self.mcp_urls = mcps or []

        self.ws_client: Optional[WebSocketClient] = None
        self.mcp_clients: List[McpClient] = []

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.disconnect()

    async def connect(self):
        """Connect to WebSocket and register local tools."""
        # Connect WebSocket client
        self.ws_client = WebSocketClient(self.ws_url)
        await self.ws_client.connect()

        # Register all local tools
        for tool_config in self.local_tools_config:
            await self.ws_client.register_tool(
                namespace=tool_config["namespace"],
                name=tool_config["name"],
                callback=tool_config["callback"],
                description=tool_config.get("description"),
                input_schema=tool_config.get("input_schema"),
                output_schema=tool_config.get("output_schema")
            )

        # Connect to MCP servers if configured
        for mcp_url in self.mcp_urls:
            mcp_client = McpClient(mcp_url, self.timeout)
            await mcp_client.connect()
            self.mcp_clients.append(mcp_client)

    async def disconnect(self):
        """Disconnect from all endpoints."""
        if self.ws_client:
            await self.ws_client.disconnect()
            self.ws_client = None

        for mcp_client in self.mcp_clients:
            await mcp_client.close()
        self.mcp_clients = []

    # ========== Main execution method ==========

    async def execute(self, code: str) -> Dict[str, Any]:
        """
        Execute TypeScript/JavaScript code with access to both MCP tools and local tools.

        The code must contain an `async function run()` that returns a value.

        Within the code, you can:
        - Call MCP tools like: `await Notion.apiPostSearch({query: "test"})`
        - Call local Python tools like: `await MyTools.getData({})`

        Args:
            code: TypeScript/JavaScript code to execute (must contain `async function run()`)

        Returns:
            Execution result dict with structure:
            {
                "success": bool,
                "value": any,  # The return value from run()
                "stdout": str,
                "stderr": str
            }

        Raises:
            ExecutionError: If execution fails
            ConnectionError: If not connected
        """
        if not self.ws_client:
            raise ConnectionError("Client not connected. Use async with or call connect()")

        return await self.ws_client.execute_code(code)

    # ========== Optional tool registration after initialization ==========

    async def register_local_tool(
        self,
        namespace: str,
        name: str,
        callback: Callable,
        description: Optional[str] = None,
        input_schema: Optional[Dict[str, Any]] = None,
        output_schema: Optional[Dict[str, Any]] = None,
    ):
        """
        Register an additional local tool after initialization.

        Most tools should be registered via the local_tools parameter in __init__.
        Use this method only when you need to dynamically add tools.

        Args:
            namespace: Tool namespace (e.g., "MyTools")
            name: Tool name (e.g., "getData")
            callback: Python function to call when tool is invoked
            description: Optional tool description
            input_schema: Optional JSON schema for input validation
            output_schema: Optional JSON schema for output validation

        Raises:
            ConnectionError: If not connected
        """
        if not self.ws_client:
            raise ConnectionError("Client not connected")

        await self.ws_client.register_tool(
            namespace=namespace,
            name=name,
            callback=callback,
            description=description,
            input_schema=input_schema,
            output_schema=output_schema
        )

    # ========== Utility methods ==========

    def list_local_tools(self) -> List[str]:
        """
        List all registered local tools.

        Returns:
            List of tool names in format "namespace.name"
        """
        if not self.ws_client:
            return []
        return list(self.ws_client.tools.keys())

    async def list_mcp_functions(self) -> List[Dict[str, Any]]:
        """
        List all available functions from registered MCP servers.

        Returns:
            List of function metadata dicts
        """
        all_functions = []
        for mcp_client in self.mcp_clients:
            functions = await mcp_client.list_functions()
            all_functions.extend(functions)
        return all_functions
