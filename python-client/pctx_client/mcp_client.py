"""
MCP HTTP Client

Connects to a PCTX server's MCP endpoint to list functions,
get function details, and execute code.
"""

import json
from typing import Any, Dict, List, Optional

import httpx # type: ignore

from .exceptions import ConnectionError, ExecutionError


class McpClient:
    """
    MCP HTTP Client

    Connects to a PCTX server's MCP endpoint for:
    - Listing available functions
    - Getting function details and schemas
    - Executing TypeScript code

    Example:
        ```python
        async with McpClient("http://localhost:8080/mcp") as client:
            # List all functions
            functions = await client.list_functions()

            # Get function details
            details = await client.get_function_details(["Notion.apiPostSearch"])

            # Execute code
            result = await client.execute('''
                async function run() {
                    const results = await Notion.apiPostSearch({
                        query: "test"
                    });
                    return results;
                }
            ''')
        ```
    """

    def __init__(self, base_url: str, timeout: float = 30.0):
        """
        Initialize the MCP client.

        Args:
            base_url: MCP endpoint URL (e.g., "http://localhost:8080/mcp")
            timeout: Request timeout in seconds
        """
        self.base_url = base_url.rstrip('/')
        self.timeout = timeout
        self.client: Optional[httpx.AsyncClient] = None

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.close()

    async def connect(self):
        """Initialize HTTP client."""
        self.client = httpx.AsyncClient(timeout=self.timeout)

    async def close(self):
        """Close HTTP client."""
        if self.client:
            await self.client.aclose()
            self.client = None

    async def list_functions(self) -> List[Dict[str, Any]]:
        """
        List all available functions from registered MCP servers.

        Returns:
            List of function metadata dicts with keys:
            - name: Full function name (e.g., "Notion.apiPostSearch")
            - description: Function description
            - input_schema: JSON schema for inputs
            - output_schema: JSON schema for outputs

        Raises:
            ConnectionError: If request fails
        """
        if not self.client:
            raise ConnectionError("Client not connected")

        try:
            response = await self.client.post(
                f"{self.base_url}",
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "list_functions",
                        "arguments": {}
                    },
                    "id": 1
                },
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json, text/event-stream"
                }
            )
            response.raise_for_status()

            # Parse SSE response (format: "data: {json}\n\n")
            response_text = response.text.strip()
            if response_text.startswith("data: "):
                data = json.loads(response_text[6:])  # Remove "data: " prefix
            else:
                data = response.json()

            if "error" in data:
                raise ConnectionError(f"MCP error: {data['error']}")

            # Extract functions from structuredContent if available
            result = data.get("result", {})
            structured_content = result.get("structuredContent", {})
            if "functions" in structured_content:
                return structured_content["functions"]

            # Fallback to parsing from text content
            content = result.get("content", [])
            if content and len(content) > 0:
                text = content[0].get("text", "{}")
                return json.loads(text).get("functions", [])

            return []

        except httpx.HTTPError as e:
            raise ConnectionError(f"HTTP error: {e}") from e
        except json.JSONDecodeError as e:
            raise ConnectionError(f"Invalid JSON response: {e}") from e

    async def get_function_details(
        self,
        functions: List[str]
    ) -> Dict[str, Dict[str, Any]]:
        """
        Get detailed schemas for specific functions.

        Args:
            functions: List of function names (e.g., ["Notion.apiPostSearch"])

        Returns:
            Dict mapping function names to their details:
            {
                "Notion.apiPostSearch": {
                    "name": "apiPostSearch",
                    "namespace": "Notion",
                    "description": "...",
                    "input_schema": {...},
                    "output_schema": {...}
                }
            }

        Raises:
            ConnectionError: If request fails
        """
        if not self.client:
            raise ConnectionError("Client not connected")

        try:
            response = await self.client.post(
                f"{self.base_url}",
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "get_function_details",
                        "arguments": {
                            "functions": functions
                        }
                    },
                    "id": 2
                },
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json, text/event-stream"
                }
            )
            response.raise_for_status()

            # Parse SSE response (format: "data: {json}\n\n")
            response_text = response.text.strip()
            if response_text.startswith("data: "):
                data = json.loads(response_text[6:])  # Remove "data: " prefix
            else:
                data = response.json()

            if "error" in data:
                raise ConnectionError(f"MCP error: {data['error']}")

            # Extract function details from structuredContent or text content
            result = data.get("result", {})
            structured_content = result.get("structuredContent", {})
            if structured_content:
                return structured_content

            content = result.get("content", [])
            if content and len(content) > 0:
                text = content[0].get("text", "{}")
                return json.loads(text)

            return {}

        except httpx.HTTPError as e:
            raise ConnectionError(f"HTTP error: {e}") from e
        except json.JSONDecodeError as e:
            raise ConnectionError(f"Invalid JSON response: {e}") from e

    async def execute(self, code: str) -> Dict[str, Any]:
        """
        Execute TypeScript/JavaScript code.

        Args:
            code: TypeScript code to execute (must contain `async function run()`)

        Returns:
            Execution result containing:
            - success: Whether execution succeeded
            - output: Return value from run() function
            - stdout: Captured console output
            - stderr: Captured error output

        Raises:
            ExecutionError: If execution fails
        """
        if not self.client:
            raise ExecutionError("Client not connected")

        try:
            response = await self.client.post(
                f"{self.base_url}",
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "execute",
                        "arguments": {
                            "code": code
                        }
                    },
                    "id": 3
                },
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json, text/event-stream"
                }
            )
            response.raise_for_status()

            # Parse SSE response (format: "data: {json}\n\n")
            response_text = response.text.strip()
            if response_text.startswith("data: "):
                data = json.loads(response_text[6:])  # Remove "data: " prefix
            else:
                data = response.json()

            if "error" in data:
                raise ExecutionError(f"Execution error: {data['error']}")

            # Extract execution result from structuredContent or text content
            result = data.get("result", {})
            structured_content = result.get("structuredContent", {})
            if structured_content:
                exec_result = structured_content
            else:
                content = result.get("content", [])
                if content and len(content) > 0:
                    text = content[0].get("text", "{}")
                    exec_result = json.loads(text)
                else:
                    raise ExecutionError("Empty response from server")

            # Check if execution succeeded
            if not exec_result.get("success", False):
                stderr = exec_result.get("stderr", "Unknown error")
                raise ExecutionError(f"Code execution failed: {stderr}")

            return exec_result

        except httpx.HTTPError as e:
            raise ExecutionError(f"HTTP error: {e}") from e
        except json.JSONDecodeError as e:
            raise ExecutionError(f"Invalid JSON response: {e}") from e
