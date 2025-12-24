# Change: Return MCP error on missing config in stdio mode

## Why
When `pctx mcp start --stdio` is invoked without a readable config file, it exits immediately. For stdio clients, this looks like a generic handshake failure. Returning a JSON-RPC error on stdout makes the failure actionable and avoids silent connection drops.

## What Changes
- **Behavior**: In stdio mode, when config loading fails (e.g., missing `pctx.json`), the server emits a JSON-RPC error response instead of exiting without a protocol message.
- **Transport**: The error MUST be emitted as a single MCP message on stdout (newline-delimited JSON), with any logging kept on stderr.

## Impact
- Affected specs: `stdio-mcp`
- Affected code: CLI/config load path for `pctx mcp start --stdio`
