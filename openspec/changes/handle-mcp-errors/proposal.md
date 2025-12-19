# Change: Handle MCP initialization errors without crashing

## Why
MCP upstreams can fail to initialize for valid environmental reasons. The runtime currently panics on these errors, crashing the session. We should degrade gracefully by disabling the failing MCP for the current session.

## What Changes
- On MCP initialization failure, log a warning and remove the MCP from the in-memory registry for the current session only.
- Continue serving other MCPs without terminating the process.
- Tool call failures do not disable the MCP; they surface as tool call errors.

## Impact
- Affected specs: stdio-mcp
- Affected code: `crates/pctx_code_execution_runtime/src/mcp_registry.rs`, possibly related runtime logging
