# WebSocket Architecture Refactor Plan

## Executive Summary

Refactor from PyO3/maturin native bindings to a WebSocket client/server architecture where:
- The PCTX server opens a WebSocket endpoint for clients to connect
- Clients (Python, JavaScript, etc.) connect via WebSocket to register local tools
- When Deno sandbox executes code that calls local tools, it sends WebSocket messages back to the client for execution
- Results are returned over the wire

## Goals

1. **Eliminate native bindings complexity** - Remove PyO3, maturin, and platform-specific wheel builds
2. **Language-agnostic client architecture** - Any language with WebSocket support can integrate
3. **Simplified CI/CD** - Single Rust binary, no multi-platform wheel builds
4. **Maintain existing functionality** - All current features work the same from user perspective

## Current Architecture

```
Python Client (PyO3 bindings)
    ↓ (native FFI calls)
CodeMode (Rust)
    ↓ (stores callbacks in CallableToolRegistry)
Deno Sandbox
    ↓ (executes TypeScript code)
op_execute_local_tool (Deno op)
    ↓ (calls Rust closure)
Python callback (via PyO3 GIL)
```

## Target Architecture

```
Python/JS/Any Client (WebSocket)
    ↓ (WebSocket connection)
PCTX Server (WebSocket endpoint)
    ↓ (stores tool metadata + client session mapping)
Deno Sandbox
    ↓ (executes TypeScript code)
op_execute_local_tool (Deno op)
    ↓ (sends WebSocket message to client)
Client executes callback
    ↓ (returns result over WebSocket)
Deno receives result, continues execution
```

## Technical Design

### 1. WebSocket Protocol

#### Connection
- Client connects to `ws://localhost:PORT/local-tools`
- Server assigns session ID
- Client sends authentication (optional)

#### Message Types (JSON-RPC 2.0 style)

**Client → Server: Register Tool**
```json
{
  "jsonrpc": "2.0",
  "method": "register_tool",
  "params": {
    "namespace": "MyTools",
    "name": "getData",
    "description": "Fetches data",
    "input_schema": { /* JSON Schema */ },
    "output_schema": { /* JSON Schema */ }
  },
  "id": 1
}
```

**Server → Client: Response**
```json
{
  "jsonrpc": "2.0",
  "result": { "success": true },
  "id": 1
}
```

**Server → Client: Execute Tool**
```json
{
  "jsonrpc": "2.0",
  "method": "execute_tool",
  "params": {
    "name": "MyTools.getData",
    "arguments": { "userId": 123 }
  },
  "id": 2
}
```

**Client → Server: Execution Result**
```json
{
  "jsonrpc": "2.0",
  "result": { "data": [...] },
  "id": 2
}
```

**Client → Server: Execute Code**
```json
{
  "jsonrpc": "2.0",
  "method": "execute",
  "params": {
    "code": "async function run() { ... }"
  },
  "id": 3
}
```

**Server → Client: Execution Output**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "output": { ... },
    "stdout": "...",
    "stderr": ""
  },
  "id": 3
}
```

### 2. Server Components

#### New Crate: `pctx_websocket_server`
- WebSocket server implementation using `tokio-tungstenite` or `axum::extract::ws`
- Session management (track connected clients)
- Tool registry with client session mapping
- Request/response correlation (match execution requests to responses)

#### Modified: `pctx/src/mcp/server.rs`
- Add WebSocket endpoint: `/local-tools`
- Keep existing HTTP MCP endpoint: `/mcp`

#### Modified: `pctx_code_execution_runtime/src/local_tool_ops.rs`
- `op_execute_local_tool` becomes async
- Instead of calling Rust closure, sends WebSocket message
- Waits for response with timeout
- Returns result to Deno

#### New: `pctx_code_execution_runtime/src/websocket_tool_registry.rs`
- Registry that stores:
  - Tool metadata (name, namespace, schemas)
  - Associated client session ID
  - Pending execution requests (request_id → oneshot channel)

### 3. Client Library (Python Example)

#### New Repository: `pctx-python-client`

```python
import asyncio
import websockets
import json

class PctxClient:
    def __init__(self, url="ws://localhost:3000/local-tools"):
        self.url = url
        self.ws = None
        self.tools = {}
        self.pending_requests = {}
        self.request_id = 0

    async def connect(self):
        self.ws = await websockets.connect(self.url)
        asyncio.create_task(self._handle_messages())

    async def register_tool(self, namespace, name, callback, description=None, input_schema=None, output_schema=None):
        self.tools[f"{namespace}.{name}"] = callback

        msg = {
            "jsonrpc": "2.0",
            "method": "register_tool",
            "params": {
                "namespace": namespace,
                "name": name,
                "description": description,
                "input_schema": input_schema,
                "output_schema": output_schema
            },
            "id": self._next_id()
        }
        await self.ws.send(json.dumps(msg))

    async def execute(self, code):
        request_id = self._next_id()
        future = asyncio.Future()
        self.pending_requests[request_id] = future

        msg = {
            "jsonrpc": "2.0",
            "method": "execute",
            "params": {"code": code},
            "id": request_id
        }
        await self.ws.send(json.dumps(msg))
        return await future

    async def _handle_messages(self):
        async for message in self.ws:
            data = json.loads(message)

            if "method" in data and data["method"] == "execute_tool":
                # Server requesting tool execution
                await self._execute_tool(data)
            elif "result" in data or "error" in data:
                # Response to our request
                request_id = data["id"]
                if request_id in self.pending_requests:
                    future = self.pending_requests.pop(request_id)
                    if "error" in data:
                        future.set_exception(Exception(data["error"]))
                    else:
                        future.set_result(data["result"])

    async def _execute_tool(self, request):
        tool_name = request["params"]["name"]
        arguments = request["params"].get("arguments")

        try:
            callback = self.tools[tool_name]
            result = callback(arguments)
            if asyncio.iscoroutine(result):
                result = await result

            response = {
                "jsonrpc": "2.0",
                "result": result,
                "id": request["id"]
            }
        except Exception as e:
            response = {
                "jsonrpc": "2.0",
                "error": {"code": -32000, "message": str(e)},
                "id": request["id"]
            }

        await self.ws.send(json.dumps(response))
```

### 4. Deno Runtime Changes

#### Challenge: Async Local Tool Execution
Current: `op_execute_local_tool` is synchronous - executes closure immediately
New: Must send WebSocket message and await response

**Solution: Use Deno async ops**

```rust
// Before (sync)
#[op2]
#[serde]
pub(crate) fn op_execute_local_tool(
    state: &mut OpState,
    #[string] name: String,
    #[serde] arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    let registry = state.borrow::<CallableToolRegistry>();
    registry.execute(&name, arguments)
        .map_err(McpError::ExecutionError)
}

// After (async)
#[op2(async)]
#[serde]
pub(crate) async fn op_execute_local_tool(
    state: Rc<RefCell<OpState>>,
    #[string] name: String,
    #[serde] arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    let registry = state.borrow().borrow::<WebSocketToolRegistry>().clone();
    registry.execute_remote(&name, arguments).await
        .map_err(McpError::ExecutionError)
}
```

### 5. Data Structures

#### `WebSocketSession`
```rust
struct WebSocketSession {
    id: String,
    sender: mpsc::UnboundedSender<Message>,
    registered_tools: HashSet<String>, // namespace.name
}
```

#### `WebSocketToolRegistry`
```rust
struct WebSocketToolRegistry {
    // Tool metadata
    tools: HashMap<String, CallableToolMetadata>,
    // Tool name → session ID
    tool_sessions: HashMap<String, String>,
    // Session ID → session
    sessions: HashMap<String, WebSocketSession>,
    // Request ID → response channel
    pending_executions: HashMap<String, oneshot::Sender<Result<Value, String>>>,
}
```

## Migration Steps (Test-Driven Development)

### Phase 1: Foundation (Tests First)
**Goal:** Set up WebSocket infrastructure with tests

1. **Create test for WebSocket connection**
   - Test: Client can connect to server
   - Test: Server assigns session ID
   - Test: Connection is bidirectional

2. **Create test for tool registration**
   - Test: Client registers tool, server acknowledges
   - Test: Multiple clients can register different tools
   - Test: Duplicate tool registration fails

3. **Implement WebSocket server crate**
   - Add `tokio-tungstenite` or use `axum::extract::ws`
   - Implement session management
   - Pass connection tests

4. **Implement tool registration protocol**
   - JSON-RPC message parsing
   - Tool metadata storage
   - Pass registration tests

### Phase 2: Local Tool Execution (Tests First)
**Goal:** Execute local tools via WebSocket

5. **Create test for synchronous tool execution**
   - Test: Server requests tool execution
   - Test: Client executes and returns result
   - Test: Result propagates back to caller

6. **Create test for async tool execution**
   - Test: Client async callback works
   - Test: Multiple concurrent executions don't interfere

7. **Implement WebSocketToolRegistry**
   - Create registry with session mapping
   - Implement remote execution logic
   - Request/response correlation

8. **Modify op_execute_local_tool to be async**
   - Change from closure call to WebSocket RPC
   - Add timeout handling
   - Pass execution tests

### Phase 3: Deno Integration (Tests First)
**Goal:** Wire up WebSocket tools to Deno sandbox

9. **Create test for Deno calling WebSocket tool**
   - Test: TypeScript code calls local tool
   - Test: Execution flows through WebSocket
   - Test: Result returns to TypeScript

10. **Integrate WebSocketToolRegistry into Deno runtime**
    - Add registry to OpState
    - Wire up async op
    - Pass integration tests

11. **Create test for error handling**
    - Test: Client disconnection during execution
    - Test: Timeout on slow execution
    - Test: Client error propagates to Deno

12. **Implement error handling**
    - Graceful degradation on disconnect
    - Timeout with configurable duration
    - Error propagation

### Phase 4: Code Execution API (Tests First)
**Goal:** Allow clients to execute code via WebSocket

13. **Create test for code execution request**
    - Test: Client sends code execution request
    - Test: Server executes and returns result
    - Test: Stdout/stderr captured

14. **Implement execute method in WebSocket handler**
    - Route `execute` method to CodeMode
    - Return ExecuteOutput via WebSocket
    - Pass execution tests

### Phase 5: Python Client Library (Tests First)
**Goal:** Replace PyO3 bindings with WebSocket client

15. **Create Python client tests**
    - Test: Connect to server
    - Test: Register tool
    - Test: Execute code that calls tool
    - Test: Receive result

16. **Implement Python client library**
    - WebSocket connection management
    - Tool registration
    - Message handling
    - Pass all tests

17. **Port existing Python binding tests to WebSocket client**
    - Adapt `tests/test_code_mode.py` tests
    - All 40+ tests should pass with WebSocket client

### Phase 6: Integration & Migration (Tests First)
**Goal:** Full system integration

18. **Create end-to-end integration tests**
    - Test: Full workflow with MCP servers + local tools
    - Test: Multiple concurrent clients
    - Test: Client reconnection

19. **Update pctx CLI to start WebSocket endpoint**
    - Modify `pctx start` command
    - Add WebSocket configuration options
    - Integration tests pass

20. **Create migration guide**
    - Document for Python users
    - Example code conversions
    - Breaking changes

### Phase 7: Cleanup
**Goal:** Remove old code

21. **Remove PyO3 bindings crate**
    - Delete `crates/code_mode_py_bindings`
    - Remove maturin CI workflow
    - Update documentation

22. **Update README and examples**
    - New architecture diagrams
    - WebSocket client examples
    - Updated installation instructions

## Test Strategy

### Unit Tests
- WebSocket message parsing
- Session management
- Tool registry operations
- Request/response correlation

### Integration Tests
- Client ↔ Server communication
- Tool registration and execution flow
- Code execution with local tool callbacks
- MCP + local tools together

### End-to-End Tests
- Python client full workflow
- Multiple concurrent clients
- Error scenarios and recovery
- Performance under load

### Test Files Structure
```
crates/pctx_websocket_server/
  src/
    lib.rs
    session.rs
    registry.rs
    protocol.rs
  tests/
    connection_tests.rs
    tool_registration_tests.rs
    tool_execution_tests.rs
    integration_tests.rs

crates/pctx_code_execution_runtime/
  src/
    websocket_tool_registry.rs
    local_tool_ops.rs (modified)
  tests/
    websocket_tool_execution_tests.rs

clients/python/
  pctx_client/
    __init__.py
    client.py
  tests/
    test_connection.py
    test_tool_registration.py
    test_code_execution.py
    test_integration.py
```

## Risk Mitigation

### Risk 1: Async Deno Ops Performance
**Mitigation:** Benchmark before/after. WebSocket RTT should be <5ms on localhost.

### Risk 2: Client Disconnection During Execution
**Mitigation:**
- Implement execution timeout
- Return error to Deno with clear message
- Document retry strategies for clients

### Risk 3: Breaking Changes for Users
**Mitigation:**
- Maintain backwards compatibility period
- Provide migration tool/script
- Clear documentation and examples

### Risk 4: WebSocket Connection Management
**Mitigation:**
- Implement heartbeat/ping-pong
- Auto-reconnection in client library
- Session recovery on reconnect

## Success Criteria

- [ ] All existing Python binding tests pass with WebSocket client
- [ ] CI builds single Rust binary (no wheel builds)
- [ ] Python client library published to PyPI
- [ ] Performance: <5ms overhead for local tool execution
- [ ] Documentation complete with migration guide
- [ ] End-to-end tests with 100% pass rate

## Timeline Estimate

- Phase 1: 2-3 days
- Phase 2: 2-3 days
- Phase 3: 2-3 days
- Phase 4: 1-2 days
- Phase 5: 2-3 days
- Phase 6: 2-3 days
- Phase 7: 1 day

**Total:** ~12-18 days for complete refactor

## Dependencies to Add

```toml
# pctx/Cargo.toml
tokio-tungstenite = "0.24"
# or use axum's built-in WebSocket support
futures-util = "0.3"

# pctx_code_execution_runtime/Cargo.toml
tokio = { version = "1.41", features = ["sync"] }
```

## Dependencies to Remove

- `pyo3` (from code_mode_py_bindings)
- `pythonize` (from code_mode_py_bindings)
- `maturin` (build tool)
- Entire `code_mode_py_bindings` crate

## Python Client Dependencies

```toml
# pyproject.toml (new client repo)
[project]
name = "pctx-client"
version = "0.1.0"
dependencies = [
    "websockets>=12.0",
]
```

## Open Questions

1. **Authentication:** Do we need auth for WebSocket connections? (Probably not for localhost, but consider for future)
2. **Encryption:** WSS for remote connections?
3. **Rate Limiting:** Limit tool executions per client?
4. **Session Persistence:** Save registered tools across server restarts?
5. **JavaScript Client:** Should we build an npm package too?

## Notes

- WebSocket is bidirectional, perfect for callback architecture
- JSON-RPC 2.0 provides standard request/response pattern
- Axum has built-in WebSocket support, may not need extra dependency
- Async Deno ops are well-supported, just need to use `#[op2(async)]`
- This architecture enables future language clients easily (Go, Ruby, etc.)
