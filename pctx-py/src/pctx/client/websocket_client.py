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

from .exceptions import ConnectionError, ExecutionError


class WebSocketClient:
    """
    PCTX WebSocket Client

    Connects to a PCTX WebSocket server, allowing you to
    receive and handle tool execution requests from the server
    """

    def __init__(self, url: str):
        """
        Initialize the WebSocket client.

        Args:
            url: WebSocket server URL (e.g., "ws://localhost:8080/ws")
        """
        self.url = url
        self.ws: Optional[ClientConnection] = None
        self.session_id: Optional[str] = None
        self.tools: Dict[str, Callable] = {}  # tool_name -> callback
        self.pending_requests: Dict[Any, asyncio.Future] = {}  # request_id -> Future
        self._message_handler_task: Optional[asyncio.Task] = None
        self._request_counter = 0

    async def connect(self, code_mode_session: str):
        """
        Connect to the WebSocket server.

        Raises:
            ConnectionError: If connection fails
        """
        try:
            headers = {"x-code-mode-session": code_mode_session}
            self.ws = await websockets.connect(self.url, additional_headers=headers)
        except Exception as e:
            raise ConnectionError(f"Failed to connect to {self.url}: {e}") from e

        # Start message handler after receiving session_created
        self._message_handler_task = asyncio.create_task(self._handle_messages())

    async def disconnect(self):
        """Disconnect from the WebSocket server."""
        if self._message_handler_task:
            self._message_handler_task.cancel()

        if self.ws:
            await self.ws.close()
            self.ws = None

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
