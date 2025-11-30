"""
Unified PCTX Client

Combines MCP HTTP client and WebSocket client for complete PCTX functionality.
"""

from typing import Any, Callable, Dict, List, Optional

from .client import PctxClient as WebSocketClient
from .mcp_client import McpClient
from .namespace_proxy import NamespaceProxy
from .exceptions import ConnectionError


class PctxUnifiedClient:
    """
    Unified PCTX Client

    Provides a single interface for:
    - MCP operations (list_functions, get_function_details, execute)
    - Local tool registration via WebSocket
    - Code execution with both MCP and local tools

    Example:
        ```python
        async with PctxUnifiedClient(
            mcp_url="http://localhost:8080/mcp",
            ws_url="ws://localhost:8080/local-tools"
        ) as client:
            # List MCP functions
            functions = await client.list_functions()

            # Register a local Python tool
            await client.register_local_tool(
                namespace="MyTools",
                name="getData",
                callback=lambda params: {"data": [...]}
            )

            # Execute code that uses both MCP and local tools
            result = await client.execute('''
                async function run() {
                    // Use MCP tool
                    const notionResults = await Notion.apiPostSearch({query: "test"});

                    // Use local tool
                    const localData = await MyTools.getData({});

                    return {notionResults, localData};
                }
            ''')
        ```
    """

    def __init__(
        self,
        mcp_url: str,
        ws_url: Optional[str] = None,
        timeout: float = 30.0
    ):
        """
        Initialize the unified PCTX client.

        Args:
            mcp_url: MCP endpoint URL (e.g., "http://localhost:8080/mcp")
            ws_url: Optional WebSocket URL for local tools (e.g., "ws://localhost:8080/local-tools")
            timeout: Request timeout in seconds
        """
        self.mcp_url = mcp_url
        self.ws_url = ws_url
        self.timeout = timeout

        self.mcp_client = McpClient(mcp_url, timeout)
        self.ws_client: Optional[WebSocketClient] = None
        self._namespaces: Dict[str, NamespaceProxy] = {}

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.disconnect()

    async def connect(self):
        """Connect to both MCP and WebSocket endpoints."""
        await self.mcp_client.connect()

        if self.ws_url:
            self.ws_client = WebSocketClient(self.ws_url)
            await self.ws_client.connect()

        # Initialize namespace proxies for available functions
        await self._initialize_namespaces()

    async def disconnect(self):
        """Disconnect from both endpoints."""
        await self.mcp_client.close()

        if self.ws_client:
            await self.ws_client.disconnect()
            self.ws_client = None

    async def _initialize_namespaces(self):
        """Initialize namespace proxies from available MCP functions."""
        try:
            functions = await self.list_functions()
            for func in functions:
                # Function names are in format "Namespace.functionName"
                if "." in func["name"]:
                    namespace, tool_name = func["name"].split(".", 1)
                    self._get_or_create_namespace(namespace)._add_tool(tool_name)
        except Exception:
            # If we can't list functions, skip namespace initialization
            pass

    def _get_or_create_namespace(self, namespace: str) -> NamespaceProxy:
        """Get existing namespace proxy or create a new one."""
        if namespace not in self._namespaces:
            # Create namespace proxy with execution function
            self._namespaces[namespace] = NamespaceProxy(
                namespace=namespace,
                execute_fn=self._execute_tool
            )
            # Make it accessible as attribute
            setattr(self, namespace, self._namespaces[namespace])
        return self._namespaces[namespace]

    async def _execute_tool(self, full_name: str, params: Dict[str, Any]) -> Any:
        """
        Execute a tool by its full name.

        This is the internal execution function used by namespace proxies.
        It determines whether to use MCP or WebSocket based on the tool.
        """
        # Check if it's a local tool
        if self.ws_client and full_name in self.ws_client.tools:
            # Execute via local tools
            code = f"""
            async function run() {{
                const result = await CALLABLE_TOOLS.execute('{full_name}', {self._params_to_js(params)});
                return result;
            }}
            """
            result = await self.ws_client.execute_code(code)
            if result.get("success"):
                return result.get("value", {}).get("result")
            else:
                raise Exception(f"Tool execution failed: {result.get('stderr', 'Unknown error')}")
        else:
            # Execute via MCP
            code = f"""
            async function run() {{
                const result = await {full_name}({self._params_to_js(params)});
                return result;
            }}
            """
            result = await self.mcp_client.execute(code)
            if result.get("success"):
                return result.get("value")
            else:
                raise Exception(f"Tool execution failed: {result.get('stderr', 'Unknown error')}")

    def _params_to_js(self, params: Dict[str, Any]) -> str:
        """Convert Python params dict to JavaScript object literal."""
        import json
        return json.dumps(params)

    def __getattr__(self, name: str) -> NamespaceProxy:
        """Get a namespace by attribute access."""
        if name.startswith('_'):
            raise AttributeError(f"'{type(self).__name__}' object has no attribute '{name}'")

        if name in self._namespaces:
            return self._namespaces[name]

        raise AttributeError(
            f"Namespace '{name}' not found. "
            f"Available namespaces: {', '.join(self._namespaces.keys())}"
        )

    def __dir__(self):
        """List available namespaces for tab completion."""
        base_attrs = [attr for attr in object.__dir__(self) if not attr.startswith('_')]
        return base_attrs + list(self._namespaces.keys())

    # ========== MCP Operations ==========

    async def list_functions(self) -> List[Dict[str, Any]]:
        """
        List all available functions from registered MCP servers.

        Returns:
            List of function metadata dicts

        Raises:
            ConnectionError: If request fails
        """
        return await self.mcp_client.list_functions()

    async def get_function_details(
        self,
        functions: List[str]
    ) -> Dict[str, Dict[str, Any]]:
        """
        Get detailed schemas for specific functions.

        Args:
            functions: List of function names (e.g., ["Notion.apiPostSearch"])

        Returns:
            Dict mapping function names to their details

        Raises:
            ConnectionError: If request fails
        """
        return await self.mcp_client.get_function_details(functions)

    async def execute(self, code: str, use_ws: bool = False) -> Dict[str, Any]:
        """
        Execute TypeScript/JavaScript code.

        Args:
            code: TypeScript code to execute (must contain `async function run()`)
            use_ws: If True, execute via WebSocket (required for local tools),
                   otherwise use MCP HTTP endpoint

        Returns:
            Execution result containing success, output, stdout, stderr

        Raises:
            ExecutionError: If execution fails
            ConnectionError: If WebSocket is requested but not connected
        """
        if use_ws:
            if not self.ws_client:
                raise ConnectionError("WebSocket client not connected. Pass ws_url to constructor.")
            return await self.ws_client.execute_code(code)
        else:
            return await self.mcp_client.execute(code)

    # ========== Local Tools Operations ==========

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
        Register a Python tool callback via WebSocket.

        Args:
            namespace: Tool namespace (e.g., "MyTools")
            name: Tool name (e.g., "getData")
            callback: Python function to call when tool is invoked
            description: Optional tool description
            input_schema: Optional JSON schema for input validation
            output_schema: Optional JSON schema for output validation

        Raises:
            ConnectionError: If WebSocket client not connected
            ToolError: If registration fails
        """
        if not self.ws_client:
            raise ConnectionError("WebSocket client not connected. Pass ws_url to constructor.")

        await self.ws_client.register_tool(
            namespace=namespace,
            name=name,
            callback=callback,
            description=description,
            input_schema=input_schema,
            output_schema=output_schema
        )

        # Add tool to namespace proxy for clean API access
        self._get_or_create_namespace(namespace)._add_tool(name)

    def list_local_tools(self) -> List[str]:
        """
        List all registered local tools.

        Returns:
            List of tool names in format "namespace.name"

        Raises:
            ConnectionError: If WebSocket client not connected
        """
        if not self.ws_client:
            raise ConnectionError("WebSocket client not connected")

        return list(self.ws_client.tools.keys())

    # ========== Convenience Methods ==========

    async def list_all_tools(self) -> Dict[str, List[Dict[str, Any]]]:
        """
        List both MCP functions and local tools.

        Returns:
            Dict with keys "mcp" and "local":
            {
                "mcp": [{name, description, ...}, ...],
                "local": [{"name": "namespace.name"}, ...]
            }
        """
        mcp_functions = await self.list_functions()

        local_tools = []
        if self.ws_client:
            local_tools = [{"name": name} for name in self.ws_client.tools.keys()]

        return {
            "mcp": mcp_functions,
            "local": local_tools
        }

    async def execute_with_local_tools(self, code: str) -> Dict[str, Any]:
        """
        Execute code via WebSocket (required when using local tools).

        This is a convenience method for execute(code, use_ws=True).

        Args:
            code: TypeScript code to execute

        Returns:
            Execution result

        Raises:
            ExecutionError: If execution fails
            ConnectionError: If WebSocket not connected
        """
        return await self.execute(code, use_ws=True)
