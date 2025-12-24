# Change: Remove stdio mode from `pctx mcp dev`

## Why
Stdio transport expects clean JSON-RPC on stdout. The interactive dev UI emits extra output, which makes stdio handshakes fragile and hard to debug.

## What Changes
- **BREAKING**: Remove `--stdio` support from `pctx mcp dev`.
- Preserve stdio support via `pctx mcp start --stdio`.
- Update CLI help/docs to direct users to `pctx mcp start --stdio` for stdio transport.

## Impact
- Affected specs: `stdio-mcp`
- Affected code: CLI command definitions for `pctx mcp dev` and documentation in `docs/CLI.md` / README as needed
