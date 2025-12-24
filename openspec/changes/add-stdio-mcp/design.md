# Design: Add stdio MCP transport support

## Background
PCTX aggregates upstream MCP servers via HTTP and exposes its own MCP server over HTTP. Some MCP servers are only available via stdio (child process), and some clients expect to communicate with a stdio server directly.

## Goals
- Support upstream MCP servers over stdio with minimal configuration.
- Allow PCTX to run as a stdio MCP server via an option on `start`/`dev`.
- Preserve existing HTTP behavior by default.
- Warn when HTTP mode is used while stdio upstream servers are configured.

## Non-Goals
- Secret resolution for stdio env values.
- Advanced process management (restarts, health checks, supervised lifecycles).
- Authentication mechanisms for stdio transports.

## Configuration Shape
Extend upstream server configuration to support a transport kind:
- HTTP: existing `url` + `auth` fields unchanged.
- Stdio: `command`, `args`, and `env`.

Env values are passed as literal strings without secret expansion.

## CLI Shape
Add a stdio variant of `pctx mcp add` mirroring the HTTP flow:
- Introduce a `pctx mcp add-stdio` subcommand that accepts `name`, `command`, optional `--arg` values, and optional `--env KEY=VALUE` entries.
- Preserve existing `pctx mcp add <name> <url>` behavior.

## Server Mode Selection
- `pctx mcp start` and `pctx mcp dev` gain a `--stdio` flag.
- HTTP remains the default.
- When HTTP mode is chosen and the config includes any stdio upstream servers, emit a warning explaining they will not be reachable in HTTP mode.

## Implementation Notes
- Use the rmcp stdio transport for upstream clients and for PCTX server mode.
- Reuse existing code-mode tooling by adding stdio connection support in the MCP registry.
- Ensure config validation and serialization are forward compatible with existing `pctx.json` files.
