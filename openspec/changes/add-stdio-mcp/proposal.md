# Change: Add stdio MCP transport support

## Why
PCTX currently supports MCP over HTTP only. Supporting stdio lets it connect to local MCP servers and run as a stdio server itself, which is common for editor and agent integrations.

## What Changes
- Add stdio transport configuration for upstream MCP servers (command, args, env).
- Add CLI support to register stdio servers alongside existing HTTP servers.
- Add stdio server mode for `pctx mcp start` and `pctx mcp dev`.
- Warn when starting in HTTP mode while stdio upstream servers exist in config.
- Update documentation for config and CLI usage.

## Impact
- Affected specs: `stdio-mcp`
- Affected code: `crates/pctx_config`, `crates/pctx`, `crates/pctx_mcp_server`, `docs/config.md`, `docs/CLI.md`
