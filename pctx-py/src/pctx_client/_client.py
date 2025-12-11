"""
PCTX Client

Main client for executing code with both MCP tools and local Python tools.
"""

from pathlib import Path
from typing import TYPE_CHECKING
from urllib.parse import urlparse

from httpx import AsyncClient

from pctx_client._tool import AsyncTool, Tool
from pctx_client._websocket_client import WebSocketClient
from pctx_client.exceptions import ConnectionError, SessionError
from pctx_client.models import (
    ExecuteInput,
    ExecuteOutput,
    GetFunctionDetailsInput,
    GetFunctionDetailsOutput,
    ListFunctionsOutput,
    ServerConfig,
    ToolConfig,
)
from pydantic import BaseModel

if TYPE_CHECKING:
    try:
        from langchain_core.tools import BaseTool as LangchainBaseTool
        from crewai.tools import BaseTool as CrewAiBaseTool
        from openai import BaseModel as OpenAIBaseModel
        from pydantic_ai.tools import Tool as PydanticAITool
    except ImportError:
        pass


class Pctx:
    """
    PCTX Client

    Execute TypeScript/JavaScript code with access to both MCP tools and local Python tools.
    """

    def __init__(
        self,
        tools: list[Tool | AsyncTool] | None = None,
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

        try:
            connect_res = await self._client.post("/code-mode/session/create")
            connect_res.raise_for_status()
        except Exception as e:
            # Check if this is a connection error (server not running)
            error_message = str(e).lower()
            if any(
                msg in error_message
                for msg in ["connection", "refused", "failed to connect", "unreachable"]
            ):
                raise ConnectionError(
                    f"Failed to connect to PCTX server at {self._client.base_url}. "
                    "Please ensure the server is running.\n"
                    "Start the server with: pctx server start"
                ) from e
            # Re-raise other errors as-is
            raise

        # Parse the session ID from the response
        try:
            self._session_id = connect_res.json()["session_id"]
        except (KeyError, ValueError) as e:
            raise ConnectionError(
                f"Received invalid response from PCTX server at {self._client.base_url}. "
                "The server may be running but not responding correctly."
            ) from e

        self._client.headers = {"x-code-mode-session": self._session_id or ""}

        # Connect WebSocket client
        await self._ws_client.connect(self._session_id or "")

        # Register all local tools
        configs: list[ToolConfig] = [
            {
                "name": t.name,
                "namespace": t.namespace,
                "description": t.description,
                "input_schema": t.input_json_schema(),
                "output_schema": t.output_json_schema(),
            }
            for t in self._tools
        ]

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
            raise SessionError(
                "No code mode session exists, run Pctx(...).connect() before calling"
            )
        list_res = await self._client.post("/code-mode/functions/list")
        list_res.raise_for_status()

        return ListFunctionsOutput.model_validate(list_res.json())

    async def get_function_details(
        self, functions: list[str]
    ) -> GetFunctionDetailsOutput:
        if self._session_id is None:
            raise SessionError(
                "No code mode session exists, run Pctx(...).connect() before calling"
            )
        list_res = await self._client.post(
            "/code-mode/functions/details", json={"functions": functions}
        )
        list_res.raise_for_status()

        return GetFunctionDetailsOutput.model_validate(list_res.json())

    async def execute(self, code: str, timeout: float = 30.0) -> ExecuteOutput:
        if self._session_id is None:
            raise SessionError(
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

        @langchain_tool(description=DEFAULT_LIST_FUNCTIONS_DESCRIPTION)
        async def list_functions() -> str:
            return (await self.list_functions()).code

        @langchain_tool(description=DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION)
        async def get_function_details(functions: list[str]) -> str:
            return (
                await self.get_function_details(
                    functions,
                )
            ).code

        @langchain_tool(description=DEFAULT_EXECUTE_DESCRIPTION)
        async def execute(code: str, timeout: float = 30) -> str:
            return (await self.execute(code, timeout=timeout)).markdown()

        return [list_functions, get_function_details, execute]

    def crewai_tools(self) -> "list[CrewAiBaseTool]":
        """
        Expose PCTX code mode tools as crewai tools

        Requires the 'crewai' extra to be installed:
            pip install pctx[crewai]

        Raises:
            ImportError: If crewai is not installed.
        """
        try:
            from crewai.tools import BaseTool as CrewAiBaseTool
        except ImportError as e:
            raise ImportError(
                "CrewAI is not installed. Install it with: pip install pctx[crewai]"
            ) from e

        import asyncio

        # Capture the current event loop for later use from threads
        try:
            main_loop = asyncio.get_running_loop()
        except RuntimeError:
            main_loop = None

        class ListFunctionsTool(CrewAiBaseTool):
            name: str = "list_functions"
            description: str = DEFAULT_LIST_FUNCTIONS_DESCRIPTION

            def _run(_self) -> str:
                # When called from CrewAI's thread pool, use the main event loop
                if main_loop is not None:
                    future = asyncio.run_coroutine_threadsafe(
                        self.list_functions(), main_loop
                    )
                    return future.result(timeout=30).code
                else:
                    # No event loop captured, create a new one
                    return asyncio.run(self.list_functions()).code

        class GetFunctionDetailsTool(CrewAiBaseTool):
            name: str = "get_function_details"
            description: str = DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION
            args_schema: type[BaseModel] = GetFunctionDetailsInput

            def _run(_self, functions: list[str]) -> str:
                # When called from CrewAI's thread pool, use the main event loop
                if main_loop is not None:
                    future = asyncio.run_coroutine_threadsafe(
                        self.get_function_details(functions=functions), main_loop
                    )
                    return future.result(timeout=30).code
                else:
                    # No event loop captured, create a new one
                    return asyncio.run(
                        self.get_function_details(functions=functions)
                    ).code

        class ExecuteTool(CrewAiBaseTool):
            name: str = "execute"
            description: str = DEFAULT_EXECUTE_DESCRIPTION
            args_schema: type[BaseModel] = ExecuteInput

            def _run(_self, code: str) -> str:
                # When called from CrewAI's thread pool, use the main event loop
                if main_loop is not None:
                    future = asyncio.run_coroutine_threadsafe(
                        self.execute(code=code), main_loop
                    )
                    return future.result(timeout=30).markdown()
                else:
                    # No event loop captured, create a new one
                    return asyncio.run(self.execute(code=code)).markdown()

        return [ListFunctionsTool(), GetFunctionDetailsTool(), ExecuteTool()]

    def openai_agents_tools(self) -> "list":
        """
        Expose PCTX code mode tools as OpenAI Agents SDK function tools

        Requires the 'openai' extra to be installed:
            pip install pctx[openai]

        Returns:
            List of function tools compatible with OpenAI Agents SDK

        Raises:
            ImportError: If openai is not installed.
        """
        try:
            from agents import function_tool
        except ImportError as e:
            raise ImportError(
                "OpenAI Agents SDK is not installed. Install it with: pip install pctx[openai]"
            ) from e

        # OpenAI Agents SDK uses function decorators to create tools
        # We need to create wrapper functions that call our async methods


        async def list_functions_wrapper() -> str:
            return (await self.list_functions()).code

        async def get_function_details_wrapper(functions: list[str]) -> str:
            return (await self.get_function_details(functions)).code

        async def execute_wrapper(code: str, timeout: float = 30.0) -> str:
            return (await self.execute(code, timeout=timeout)).markdown()

        # Set docstrings and apply decorator
        list_functions_wrapper.__doc__ = DEFAULT_LIST_FUNCTIONS_DESCRIPTION
        get_function_details_wrapper.__doc__ = f"""{DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION}

Args:
    functions: List of function names in 'namespace.functionName' format"""
        execute_wrapper.__doc__ = f"""{DEFAULT_EXECUTE_DESCRIPTION}

Args:
    code: TypeScript code to execute
    timeout: Timeout in seconds (default: 30)"""

        # Apply the function_tool decorator
        list_functions_tool = function_tool(name_override="list_functions")(list_functions_wrapper)
        get_function_details_tool = function_tool(name_override="get_function_details")(get_function_details_wrapper)
        execute_tool = function_tool(name_override="execute")(execute_wrapper)

        return [list_functions_tool, get_function_details_tool, execute_tool]

    def pydantic_ai_tools(self) -> "list[PydanticAITool]":
        """
        Expose PCTX code mode tools as Pydantic AI tools

        Requires the 'pydantic-ai' extra to be installed:
            pip install pctx[pydantic-ai]

        Raises:
            ImportError: If pydantic-ai is not installed.
        """
        try:
            from pydantic_ai.tools import Tool as PydanticAITool
        except ImportError as e:
            raise ImportError(
                "Pydantic AI is not installed. Install it with: pip install pctx[pydantic-ai]"
            ) from e

        # Pydantic AI uses function decorators to create tools
        # We need to create wrapper functions that call our async methods

        async def list_functions_wrapper() -> str:
            return (await self.list_functions()).code

        async def get_function_details_wrapper(functions: list[str]) -> str:
            return (await self.get_function_details(functions)).code

        async def execute_wrapper(code: str, timeout: float = 30.0) -> str:
            return (await self.execute(code, timeout=timeout)).markdown()

        # Create Pydantic AI tools using the Tool class with explicit descriptions
        tools = [
            PydanticAITool(
                list_functions_wrapper,
                name="list_functions",
                description=DEFAULT_LIST_FUNCTIONS_DESCRIPTION,
            ),
            PydanticAITool(
                get_function_details_wrapper,
                name="get_function_details",
                description=DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION,
            ),
            PydanticAITool(
                execute_wrapper,
                name="execute",
                description=DEFAULT_EXECUTE_DESCRIPTION,
            ),
        ]

        return tools


def _load_tool_description(name: str) -> str:
    """Load tool description from markdown file."""
    # Get the repository root (3 levels up from this file)
    client_file = Path(__file__)
    repo_root = client_file.parent.parent.parent.parent
    tool_descriptions_dir = repo_root / "tool_descriptions"
    description_file = tool_descriptions_dir / f"{name}.md"

    if description_file.exists():
        return description_file.read_text().strip()
    else:
        # Fallback to hardcoded descriptions if file not found
        return _FALLBACK_DESCRIPTIONS.get(name, "")


# Load descriptions from markdown files
DEFAULT_LIST_FUNCTIONS_DESCRIPTION = _load_tool_description("list_functions")
DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION = _load_tool_description(
    "get_function_details"
)
DEFAULT_EXECUTE_DESCRIPTION = _load_tool_description("execute")

# Fallback descriptions if markdown files are not found
# These should match the markdown files in tool_descriptions/
_FALLBACK_DESCRIPTIONS = {
    "list_functions": """ALWAYS USE THIS TOOL FIRST to list all available functions organized by namespace.

WORKFLOW:
1. Start here - Call this tool to see what functions are available
2. Then call get_function_details() for specific functions you need to understand
3. Finally call execute() to run your TypeScript code

This returns function signatures without full details.""",
    "get_function_details": """Get detailed information about specific functions you want to use.

WHEN TO USE: After calling list_functions(), use this to learn about parameter types, return values, and usage for specific functions.

REQUIRED FORMAT: Functions must be specified as 'namespace.functionName' (e.g., 'Namespace.apiPostSearch')

This tool is lightweight and only returns details for the functions you request, avoiding unnecessary token usage.
Only request details for functions you actually plan to use in your code.

NOTE ON RETURN TYPES:
- If a function returns Promise<any>, the MCP server didn't provide an output schema
- The actual value is a parsed object (not a string) - access properties directly
- Don't use JSON.parse() on the results - they're already JavaScript objects""",
    "execute": """Execute TypeScript code that calls namespaced functions. USE THIS LAST after list_functions() and get_function_details().

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
- If you see 'Promise<any>', the structure is unknown - log it to see what's returned""",
}
