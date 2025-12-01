"""
PCTX WebSocket Client

Connects to a PCTX WebSocket server to register Python tool callbacks
and execute TypeScript code.
"""

import asyncio
import json
from typing import Any, Callable, Dict, Optional

import websockets
from websockets.asyncio.client import ClientConnection

from .exceptions import ConnectionError, ExecutionError, ToolError


class WebSocketClient:
    """
    PCTX WebSocket Client

    Connects to a PCTX WebSocket server, allowing you to:
    - Register local Python tool callbacks
    - Register MCP servers for code execution
    - Receive and handle tool execution requests from the server

    Note: Code execution is done via MCP, not through WebSocket.
    Use the PctxClient (unified client) for code execution.

    Example:
        ```python
        async with WebSocketClient("ws://localhost:8080/local-tools") as client:
            # Register a tool
            await client.register_tool(
                namespace="math",
                name="add",
                callback=lambda params: {"result": params["a"] + params["b"]},
                description="Adds two numbers"
            )

            # Register an MCP server
            await client.register_mcp(
                name="my_mcp",
                url="http://localhost:8080/mcp"
            )
        ```
    """

    def __init__(self, url: str):
        """
        Initialize the PCTX client.

        Args:
            url: WebSocket server URL (e.g., "ws://localhost:8080/local-tools")
        """
        self.url = url
        self.ws: Optional[ClientConnection] = None
        self.session_id: Optional[str] = None
        self.tools: Dict[str, Callable] = {}  # tool_name -> callback
        self.pending_requests: Dict[Any, asyncio.Future] = {}  # request_id -> Future
        self._message_handler_task: Optional[asyncio.Task] = None
        self._request_counter = 0

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.disconnect()

    async def connect(self):
        """
        Connect to the WebSocket server.

        Raises:
            ConnectionError: If connection fails
        """
        try:
            self.ws = await websockets.connect(self.url)
        except Exception as e:
            raise ConnectionError(f"Failed to connect to {self.url}: {e}") from e

        # Wait for session_created notification before starting message handler
        try:
            message = await asyncio.wait_for(self.ws.recv(), timeout=5.0)
            data = json.loads(message)
            if data.get("method") == "session_created":
                self.session_id = data.get("params", {}).get("session_id")
            else:
                raise ConnectionError(f"Expected session_created, got: {data}")
        except asyncio.TimeoutError:
            raise ConnectionError("Timeout waiting for session_created")
        except Exception as e:
            raise ConnectionError(f"Error receiving session_created: {e}") from e

        # Start message handler after receiving session_created
        self._message_handler_task = asyncio.create_task(self._handle_messages())

    async def disconnect(self):
        """Disconnect from the WebSocket server."""
        if self._message_handler_task:
            self._message_handler_task.cancel()
            try:
                await self._message_handler_task
            except asyncio.CancelledError:
                pass

        if self.ws:
            await self.ws.close()
            self.ws = None

    async def register_tool(
        self,
        namespace: str,
        name: str,
        callback: Callable,
        description: Optional[str] = None,
        input_schema: Optional[Dict[str, Any]] = None,
        output_schema: Optional[Dict[str, Any]] = None,
    ):
        """
        Register a Python tool callback.

        Args:
            namespace: Tool namespace (e.g., "math")
            name: Tool name (e.g., "add")
            callback: Python function to call when tool is invoked
            description: Optional tool description
            input_schema: Optional JSON schema for input validation
            output_schema: Optional JSON schema for output validation

        Raises:
            ToolError: If registration fails
        """
        if not self.ws:
            raise ToolError("Not connected to server")

        tool_name = f"{namespace}.{name}"
        self.tools[tool_name] = callback

        # Send registration request
        request_id = self._next_request_id()
        request = {
            "jsonrpc": "2.0",
            "method": "register_tool",
            "params": {
                "namespace": namespace,
                "name": name,
            },
            "id": request_id,
        }

        if description:
            request["params"]["description"] = description
        if input_schema:
            request["params"]["input_schema"] = input_schema
        if output_schema:
            request["params"]["output_schema"] = output_schema

        response = await self._send_request(request)

        if "error" in response:
            raise ToolError(f"Failed to register tool: {response['error']['message']}")

    async def register_mcp(
        self,
        name: str,
        url: str,
        auth: Optional[Dict[str, Any]] = None,
    ):
        """
        Register an MCP (Model Context Protocol) server.

        The registered MCP server will be available in the Deno sandbox for
        code execution via registerMCP() and callMCPTool().

        Args:
            name: Unique name for the MCP server
            url: URL of the MCP server (e.g., "http://localhost:3000")
            auth: Optional authentication configuration (e.g., {"bearer": {"token": "..."}})

        Raises:
            ToolError: If registration fails

        Example:
            ```python
            # Register a public MCP server
            await client.register_mcp(
                name="weather",
                url="https://weather-mcp-server.example.com"
            )

            # Register with authentication
            await client.register_mcp(
                name="private-api",
                url="https://api.example.com",
                auth={"bearer": {"token": "secret-token"}}
            )
            ```
        """
        if not self.ws:
            raise ToolError("Not connected to server")

        request_id = self._next_request_id()
        request = {
            "jsonrpc": "2.0",
            "method": "register_mcp",
            "params": {
                "name": name,
                "url": url,
            },
            "id": request_id,
        }

        if auth:
            request["params"]["auth"] = auth

        response = await self._send_request(request)

        if "error" in response:
            raise ToolError(
                f"Failed to register MCP server: {response['error']['message']}"
            )

    async def _send_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """
        Send a JSON-RPC request and wait for response.

        Args:
            request: JSON-RPC request dict

        Returns:
            JSON-RPC response dict
        """
        request_id = request["id"]
        future = asyncio.Future()
        self.pending_requests[request_id] = future

        try:
            await self.ws.send(json.dumps(request))
            response = await asyncio.wait_for(future, timeout=30.0)
            return response
        except asyncio.TimeoutError:
            raise ExecutionError("Request timeout")
        finally:
            self.pending_requests.pop(request_id, None)

    async def _handle_messages(self):
        """Background task to handle incoming WebSocket messages."""
        try:
            async for message in self.ws:
                try:
                    data = json.loads(message)
                    await self._process_message(data)
                except json.JSONDecodeError:
                    print(f"Failed to decode message: {message}")
                except Exception as e:
                    print(f"Error processing message: {e}")
        except asyncio.CancelledError:
            pass
        except Exception as e:
            print(f"Message handler error: {e}")

    async def _process_message(self, data: Dict[str, Any]):
        """
        Process an incoming WebSocket message.

        Handles:
        - Responses to our requests
        - execute_tool requests from the server
        """
        # Check if this is a response to one of our requests
        if "id" in data and data["id"] in self.pending_requests:
            future = self.pending_requests[data["id"]]
            if not future.done():
                future.set_result(data)
            return

        # Check if this is an execute_tool request from the server
        if data.get("method") == "execute_tool":
            await self._handle_tool_execution(data)
            return

    async def _handle_tool_execution(self, request: Dict[str, Any]):
        """
        Handle an execute_tool request from the server.

        This is called when the server wants to execute one of our registered tools.
        """
        request_id = request.get("id")
        params = request.get("params", {})
        tool_name = params.get("name")
        arguments = params.get("arguments", {})

        # Execute the tool callback
        if tool_name in self.tools:
            try:
                callback = self.tools[tool_name]
                # Call the callback (support both sync and async)
                if asyncio.iscoroutinefunction(callback):
                    result = await callback(arguments)
                else:
                    result = callback(arguments)

                # Send success response
                response = {
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": request_id,
                }
                await self.ws.send(json.dumps(response))
            except Exception as e:
                # Send error response
                response = {
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": str(e),
                    },
                    "id": request_id,
                }
                await self.ws.send(json.dumps(response))
        else:
            # Tool not found
            response = {
                "jsonrpc": "2.0",
                "error": {
                    "code": -32001,
                    "message": f"Tool not found: {tool_name}",
                },
                "id": request_id,
            }
            await self.ws.send(json.dumps(response))

    def _next_request_id(self) -> int:
        """Generate next request ID."""
        self._request_counter += 1
        return self._request_counter
