## MODIFIED Requirements

### Requirement: Serve MCP over stdio
The system SHALL support running the PCTX MCP server over stdio via an option on `pctx mcp start` only.

#### Scenario: Start PCTX in stdio mode
- **WHEN** `pctx mcp start --stdio` is invoked
- **THEN** PCTX serves MCP traffic over stdio instead of HTTP

#### Scenario: Dev mode rejects stdio
- **WHEN** `pctx mcp dev --stdio` is invoked
- **THEN** the command fails with guidance to use `pctx mcp start --stdio`
