## ADDED Requirements
### Requirement: Disable failing MCP during session initialization
When an upstream MCP fails to initialize, the system SHALL log a warning and remove that MCP from the current session registry, allowing other MCPs to continue.

#### Scenario: Initialization failure disables MCP
- **WHEN** an MCP initialization fails during a session
- **THEN** the system logs a warning describing the failure
- **AND** the MCP is removed from the session registry
- **AND** other MCPs remain available
