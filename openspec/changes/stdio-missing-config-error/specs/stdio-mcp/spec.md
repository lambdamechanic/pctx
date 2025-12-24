## ADDED Requirements

### Requirement: Report missing config over stdio
When running in stdio mode, the system SHALL report a missing or unreadable config file via a JSON-RPC error response over stdout instead of silently exiting.

#### Scenario: Missing config file in stdio mode
- **WHEN** `pctx mcp start --stdio` is invoked and the configured `pctx.json` file does not exist or cannot be read
- **THEN** the server emits a JSON-RPC error response describing the config failure and exits
