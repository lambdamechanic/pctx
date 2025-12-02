#!/usr/bin/env python3
"""
Test script for PCTX Agent Dev Mode
Connects via WebSocket, registers tools, and makes some tool calls
"""

import asyncio
import json
import sys
from datetime import datetime

try:
    import websockets
    import requests
except ImportError:
    print("Missing dependencies. Install with:")
    print("  pip install websockets requests")
    sys.exit(1)


class PctxAgentClient:
    def __init__(self, host="127.0.0.1", port=8080):
        self.host = host
        self.port = port
        self.ws_url = f"ws://{host}:{port}/ws"
        self.rest_url = f"http://{host}:{port}"
        self.session_id = None
        self.request_id = 0

    def next_request_id(self):
        """Generate next request ID"""
        self.request_id += 1
        return self.request_id

    async def handle_tool_execution(self, websocket, request):
        """Handle tool execution request from server"""
        request_id = request.get("id")
        params = request.get("params", {})
        tool_name = params.get("name")
        arguments = params.get("arguments", {})

        print(f"    → Server requested: {tool_name}({arguments})")

        # Simulate tool execution
        result = None
        if tool_name == "Calculator.add":
            result = arguments.get("a", 0) + arguments.get("b", 0)
        elif tool_name == "Calculator.multiply":
            result = arguments.get("a", 0) * arguments.get("b", 0)
        elif tool_name == "Database.query":
            result = [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
            ]
        elif tool_name == "FileSystem.readFile":
            result = {"app": "pctx", "version": "0.2.1"}
        else:
            result = f"Mock result for {tool_name}"

        # Send response
        response = {"jsonrpc": "2.0", "id": request_id, "result": result}
        await websocket.send(json.dumps(response))
        print(f"    ← Sent result: {result}")

    async def connect_and_run(self):
        """Connect to agent server and run test scenario"""
        print(f"Connecting to {self.ws_url}...")

        async with websockets.connect(self.ws_url) as websocket:
            # Wait for session_created notification
            message = await websocket.recv()
            data = json.loads(message)
            print(f"✓ Connected! Session: {data}")

            if data.get("method") == "session_created":
                self.session_id = data.get("params", {}).get("session_id")
                print(f"✓ Session ID: {self.session_id}")

            if not self.session_id:
                print("✗ Failed to get session ID")
                return

            # Register some local tools
            await self.register_tools()

            # Start background task to handle tool execution requests
            async def handle_messages():
                try:
                    async for message in websocket:
                        data = json.loads(message)
                        if data.get("method") == "execute_tool":
                            await self.handle_tool_execution(websocket, data)
                except Exception as e:
                    print(f"Message handler error: {e}")

            message_task = asyncio.create_task(handle_messages())

            # Execute some code to generate logs
            await self.execute_code_with_tools()

            print("\n✓ Session active - check the TUI now!")
            print("  - Sessions panel should show 1 active session")
            print(
                "  - Tools panel should show 4 registered tools (Calculator, FileSystem, Database)"
            )
            print("  - Logs panel should show execution logs and tool calls!")
            print("  - Use arrow keys to navigate between panels")
            print("\nPress Ctrl+C to disconnect and end session...")

            try:
                # Keep connection alive
                await asyncio.sleep(300)  # 5 minutes
            except KeyboardInterrupt:
                print("\n✓ Disconnecting...")
                message_task.cancel()

    async def register_tools(self):
        """Register tools via REST API"""
        if not self.session_id:
            print("✗ No session ID")
            return

        tools = [
            {
                "namespace": "Calculator",
                "name": "add",
                "description": "Add two numbers together",
                "parameters": {
                    "type": "object",
                    "properties": {"a": {"type": "number"}, "b": {"type": "number"}},
                    "required": ["a", "b"],
                },
            },
            {
                "namespace": "Calculator",
                "name": "multiply",
                "description": "Multiply two numbers",
                "parameters": {
                    "type": "object",
                    "properties": {"a": {"type": "number"}, "b": {"type": "number"}},
                    "required": ["a", "b"],
                },
            },
            {
                "namespace": "FileSystem",
                "name": "readFile",
                "description": "Read a file from disk",
                "parameters": {
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"],
                },
            },
            {
                "namespace": "Database",
                "name": "query",
                "description": "Execute a database query",
                "parameters": {
                    "type": "object",
                    "properties": {"sql": {"type": "string"}},
                    "required": ["sql"],
                },
            },
        ]

        payload = {"session_id": self.session_id, "tools": tools}

        print(f"\n→ Registering {len(tools)} tools...")
        response = requests.post(
            f"{self.rest_url}/tools/local/register",
            json=payload,
            headers={"Content-Type": "application/json"},
        )

        if response.status_code == 200:
            result = response.json()
            print(f"✓ Registered {result.get('registered', 0)} tools")
        else:
            print(f"✗ Failed to register tools: {response.status_code}")
            print(f"  {response.text}")

    async def execute_code_with_tools(self):
        """Execute some TypeScript code that uses the tools"""
        print("\n→ Executing code with tool calls...")

        # Execute code that actually calls the registered tools
        code_snippets = [
            {
                "name": "Simple calculation",
                "code": """
                async function run() {
                    console.log("Calling Calculator.add(10, 20)");
                    const result = await Calculator.add({ a: 10, b: 20 });
                    console.log("Result:", result);
                    return { operation: "add", result };
                }
                """,
            },
            {
                "name": "Another calculation",
                "code": """
                async function run() {
                    console.log("Calling Calculator.multiply(5, 7)");
                    const result = await Calculator.multiply({ a: 5, b: 7 });
                    console.log("Result:", result);
                    return { operation: "multiply", result };
                }
                """,
            },
            {
                "name": "Multiple tool calls",
                "code": """
                async function run() {
                    console.log("Calling Calculator.add and Calculator.multiply");
                    const sum = await Calculator.add({ a: 100, b: 200 });
                    const product = await Calculator.multiply({ a: sum, b: 2 });
                    console.log("Sum:", sum, "Product:", product);
                    return { sum, product };
                }
                """,
            },
            {
                "name": "Database query",
                "code": """
                async function run() {
                    console.log("Calling Database.query('SELECT * FROM users')");
                    const users = await Database.query({ sql: "SELECT * FROM users" });
                    console.log("Found users:", users);
                    return users;
                }
                """,
            },
            {
                "name": "File system operation",
                "code": """
                async function run() {
                    console.log("Calling FileSystem.readFile('config.json')");
                    const config = await FileSystem.readFile({ path: "config.json" });
                    console.log("Config loaded:", config);
                    return config;
                }
                """,
            },
        ]

        for snippet in code_snippets:
            print(f"  • {snippet['name']}...")
            payload = {"code": snippet["code"], "timeout_ms": 5000}

            try:
                response = requests.post(
                    f"{self.rest_url}/tools/execute",
                    json=payload,
                    headers={"Content-Type": "application/json"},
                    timeout=10,
                )

                if response.status_code == 200:
                    print(f"    ✓ Success")
                else:
                    print(f"    ⚠ Status: {response.status_code}")
                    try:
                        error_data = response.json()
                        print(
                            f"    Error: {error_data.get('error', {}).get('message', 'Unknown error')}"
                        )
                    except:
                        pass
            except Exception as e:
                print(f"    ⚠ Error: {e}")

            # Small delay between calls
            await asyncio.sleep(0.5)

        print("✓ Code execution complete")


async def main():
    """Main entry point"""
    print("=" * 60)
    print("PCTX Agent Dev Mode Test Script")
    print("=" * 60)
    print()
    print("This script will:")
    print("  1. Connect to the agent server via WebSocket")
    print("  2. Register 4 tools (Calculator, FileSystem, Database)")
    print("  3. Execute code that calls these tools")
    print("  4. Keep the session active so you can see it in the TUI")
    print()
    print("Make sure the agent dev server is running:")
    print("  cd crates/pctx")
    print("  cargo run -- agent dev --port 8080")
    print()

    # Wait a moment for user to read
    await asyncio.sleep(2)

    client = PctxAgentClient(host="127.0.0.1", port=8080)

    try:
        await client.connect_and_run()
    except ConnectionRefusedError:
        print("\n✗ Connection refused!")
        print("  Make sure the agent dev server is running:")
        print("    cargo run -- agent dev --port 8080")
    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback

        traceback.print_exc()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n✓ Stopped")
