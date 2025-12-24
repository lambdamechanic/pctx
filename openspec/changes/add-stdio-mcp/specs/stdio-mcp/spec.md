## ADDED Requirements

### Requirement: Configure stdio upstream MCP servers
The system SHALL allow configuring upstream MCP servers that are started over stdio using a command, optional arguments, and optional environment variables.

#### Scenario: Load stdio server config
- **WHEN** a config includes an upstream server with a stdio transport definition
- **THEN** PCTX accepts the configuration and can attempt to connect using the provided command, args, and env values

### Requirement: Register stdio upstream servers via CLI
The system SHALL provide a CLI path to add stdio upstream servers comparable to the existing HTTP add flow.

#### Scenario: Add a stdio server
- **WHEN** a user runs `pctx mcp add-stdio` with a name, command, and optional args/env
- **THEN** the stdio server is saved into the config using the provided values

### Requirement: Serve MCP over stdio
The system SHALL support running the PCTX MCP server over stdio via an option on `pctx mcp start` and `pctx mcp dev`.

#### Scenario: Start PCTX in stdio mode
- **WHEN** `pctx mcp start --stdio` or `pctx mcp dev --stdio` is invoked
- **THEN** PCTX serves MCP traffic over stdio instead of HTTP

### Requirement: Warn about stdio upstream servers in HTTP mode
The system SHALL warn when starting in HTTP server mode if the config contains any stdio upstream servers.

#### Scenario: Start HTTP mode with stdio upstream
- **WHEN** `pctx mcp start` or `pctx mcp dev` runs in HTTP mode and stdio upstream servers exist in the config
- **THEN** a warning is emitted indicating stdio servers will not be reachable in HTTP mode

### Requirement: Pass stdio env values as literals
The system SHALL pass stdio environment variables as literal strings without secret resolution.

#### Scenario: Provide env values for stdio server
- **WHEN** a stdio upstream server is configured with environment values
- **THEN** those values are passed unchanged to the process environment
