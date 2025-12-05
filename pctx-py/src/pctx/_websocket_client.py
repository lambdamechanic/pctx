"""
PCTX WebSocket Client

Connects to a PCTX WebSocket server to register Python tool callbacks
and execute TypeScript code.
"""

import asyncio
import json
from typing import Any

import pydantic
import websockets
from pctx.models import (
    ErrorCode,
    ErrorData,
    ExecuteToolResult,
    JsonRpcError,
    JsonRpcExecuteToolRequest,
    JsonRpcExecuteToolResponse,
)
from pctx._tool import Tool
from websockets.asyncio.client import ClientConnection

from .exceptions import ConnectionError


class WebSocketClient:
    """
    PCTX WebSocket Client

    Connects to a PCTX WebSocket server, allowing you to
    receive and handle tool execution requests from the server
    """

    def __init__(self, url: str, tools: list[Tool] | None = None):
        """
        Initialize the WebSocket client.

        Args:
            url: WebSocket server URL (e.g., "ws://localhost:8080/ws")
        """
        self.url = url
        self.ws: ClientConnection | None = None
        self.tools = tools or []

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

    async def _send(self, message: dict[str, Any]):
        """
        Send a message via the websocket
        """
        if self.ws is None:
            raise ConnectionError(
                "Cannot send messages when websocket is not connected"
            )

        await self.ws.send(json.dumps(message))

    async def _handle_messages(self):
        """Background task to handle incoming WebSocket messages."""
        if self.ws is None:
            raise ConnectionError(
                "Cannot handle messages when websocket is not connected"
            )

        try:
            async for message in self.ws:
                try:
                    exec_req = JsonRpcExecuteToolRequest.model_validate_json(message)
                    res = await self._handle_execute(exec_req)
                    await self._send(res.model_dump(mode="json"))
                except pydantic.ValidationError as e:
                    print(
                        f"Failed to decode execution request message: {message} - {e}"
                    )
                except Exception as e:
                    print(f"Error processing message: {e}")
        except asyncio.CancelledError:
            pass
        except Exception as e:
            print(f"Message handler error: {e}")

    async def _handle_execute(
        self, req: JsonRpcExecuteToolRequest
    ) -> JsonRpcExecuteToolResponse | JsonRpcError:
        # Find tool to execute
        tool = next(
            (t for t in self.tools if t.name == req.params.name and t.namespace), None
        )
        if tool is None:
            return JsonRpcError(
                id=req.id,
                error=ErrorData(
                    code=ErrorCode.METHOD_NOT_FOUND,
                    message=f"No tool `{req.params.name}` exists in namespace `{req.params.namespace}`",
                ),
            )

        args = req.params.args or {}
        try:
            if tool.func is not None:
                output = tool.invoke(**args)
            else:
                output = await tool.ainvoke(**args)

            return JsonRpcExecuteToolResponse(
                id=req.id, result=ExecuteToolResult(output=output)
            )
        except pydantic.ValidationError as e:
            return JsonRpcError(
                id=req.id,
                error=ErrorData(
                    code=ErrorCode.INVALID_PARAMS,
                    message=f"Failed validating tool params: {e}",
                ),
            )
        except Exception as e:
            return JsonRpcError(
                id=req.id,
                error=ErrorData(
                    code=ErrorCode.INTERNAL_ERROR,
                    message=f"Failed executing tool: {e}",
                ),
            )
