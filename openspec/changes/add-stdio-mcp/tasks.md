# Tasks

## 1. Implementation
- [ ] Extend config schema to represent HTTP vs stdio upstream MCP servers.
- [ ] Add CLI support to register stdio servers (command, args, env) alongside HTTP servers.
- [ ] Implement stdio upstream connections in code-mode MCP registry.
- [ ] Add `--stdio` server mode for `pctx mcp start` and `pctx mcp dev`.
- [ ] Emit a warning when running in HTTP mode with stdio upstream configs present.
- [ ] Update documentation for config and CLI usage.

## 2. Validation
- [ ] Add/update tests for config parsing and stdio registration.
- [ ] Add/update tests for server mode selection and warnings.
- [ ] Run relevant test suite (e.g., `cargo test -p pctx_config -p pctx`).
