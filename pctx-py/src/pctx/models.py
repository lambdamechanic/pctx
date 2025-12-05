from typing import Any, TypedDict, NotRequired, Literal
from pydantic import BaseModel
from enum import IntEnum


# ------------- Tool Callback Config ------------


class ToolConfig(TypedDict):
    name: str
    namespace: str
    description: NotRequired[str]
    input_schema: NotRequired[dict[str, Any] | None]
    output_schema: NotRequired[dict[str, Any] | None]


# -------------- MCP Server Config --------------


class BearerAuth(TypedDict):
    """Bearer token authentication"""

    type: Literal["bearer"]
    token: str


class HeadersAuth(TypedDict):
    """Custom headers authentication"""

    type: Literal["headers"]
    headers: dict[str, str]


class ServerConfig(TypedDict):
    """Configuration for an MCP server connection"""

    name: str
    url: str
    auth: NotRequired[BearerAuth | HeadersAuth]


# -------------- Websocket jsonrpc Messages --------------


class ErrorCode(IntEnum):
    RESOURCE_NOT_FOUND = -32002
    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_PARAMS = -32602
    INTERNAL_ERROR = -32603
    PARSE_ERROR = -32700


class ErrorData(BaseModel):
    code: ErrorCode
    message: str
    data: dict[str, Any] | None = None


class JsonRpcError(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    id: str | int
    error: ErrorData


class ExecuteToolParams(BaseModel):
    id: str | int
    namespace: str
    name: str
    args: dict[str, Any] | None


class JsonRpcExecuteToolRequest(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    id: str | int
    method: Literal["execute_tool"]
    params: ExecuteToolParams


class ExecuteToolResult(BaseModel):
    output: Any | None


class JsonRpcExecuteToolResponse(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    id: str | int
    result: ExecuteToolResult


# -------------- Code Mode Outputs --------------


class ListedFunction(BaseModel):
    """Represents a listed function with basic metadata"""

    namespace: str
    name: str
    description: str | None = None


class ListFunctionsOutput(BaseModel):
    """Output from listing available functions"""

    functions: list[ListedFunction]
    code: str


class FunctionDetails(BaseModel):
    """Detailed information about a function including types"""

    namespace: str
    name: str
    description: str | None = None
    input_type: str
    output_type: str
    types: str


class GetFunctionDetailsOutput(BaseModel):
    """Output from getting detailed function information"""

    functions: list[FunctionDetails]
    code: str


class ExecuteOutput(BaseModel):
    """Output from executing TypeScript code"""

    success: bool
    stdout: str
    stderr: str
    output: Any | None = None
