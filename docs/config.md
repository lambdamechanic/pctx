# Configuration Guide

The `pctx.json` file defines your MCP server aggregation, authentication, and runtime configuration.

## File Location

By default, `pctx` looks for `./pctx.json` in the current working directory.

Override with the `--config` flag:

```bash
pctx --config /path/to/config.json start
```

## Quick Start

Initialize a new configuration:

```bash
pctx init
```

This creates a basic `pctx.json` and prompts you to add upstream MCP servers.

## Fields

### Root Fields

| Field         | Type                  | Required | Description                                            |
| ------------- | --------------------- | -------- | ------------------------------------------------------ |
| `name`        | `string`              | Yes      | Name of your MCP server instance                       |
| `version`     | `string`              | Yes      | Version of your MCP server                             |
| `description` | `string`              | No       | Optional description of your MCP server                |
| `servers`     | `array[ServerConfig]` | Yes      | List of upstream MCP server configurations (see below) |
| `logger`      | `LoggerConfig`        | No       | Logger configuration (see below)                       |

### Server Configuration

Each server in the `servers` array has the following fields:

| Field  | Type         | Required | Description                                    |
| ------ | ------------ | -------- | ---------------------------------------------- |
| `name` | `string`     | Yes      | Unique identifier used as TypeScript namespace |
| `url`  | `string`     | Yes      | HTTP(S) URL of the MCP server endpoint         |
| `auth` | `AuthConfig` | No       | Authentication configuration (see below)       |

#### Server Names as Namespaces

The `name` will be case converted to `camelCase` and used as the TypeScript namespace for accessing that server's tools:

```typescript
// Server name: "g_drive"
await gDrive.getSheet({ sheetId: "abc" });

// Server name: "slack"
await slack.sendMessage({ channel: "#general", text: "hi" });
```

**Requirements:**

- Must be unique within the configuration
- Should be a valid TypeScript identifier (alphanumeric, underscores, no spaces) to avoid clashes after case conversion
- Keep it short and descriptive

## Authentication

The `auth` field supports two types of authentication `BearerToken | Custom`:

### Bearer Token Authentication

| Field   | Type           | Required | Description                                                                                             |
| ------- | -------------- | -------- | ------------------------------------------------------------------------------------------------------- |
| `type`  | `"bearer"`     | Yes      | Constant designating this object as a bearer token config                                               |
| `token` | `SecretString` | Yes      | Secret string value (see below for syntax) of the bearer token. `Bearer ` prefix is added automatically |

**Example:**

```json
{
  "type": "bearer",
  "token": "${env:API_TOKEN}"
}
```

This adds an `Authorization: Bearer <token>` header to all requests.

### Custom Header Authentication

| Field     | Type                      | Required | Description                                                      |
| --------- | ------------------------- | -------- | ---------------------------------------------------------------- |
| `type`    | `"custom"`                | Yes      | Constant designating this object as a custom headers config      |
| `headers` | `map[string]SecretString` | Yes      | Map of header name to Secret string value (see below for syntax) |

**Example:**

```json
{
  "type": "custom",
  "headers": {
    "x-api-key": "${env:API_KEY}",
    "x-custom-header": "static-value"
  }
}
```

Use this for API key authentication or any custom header requirements.

## Secret String Syntax

Both `token` and header values support a secret string syntax for secure credential management.

### Environment Variables

**Format:** `${env:VARIABLE_NAME}`

```json
{
  "token": "${env:MCP_API_TOKEN}"
}
```

The value is read from the environment variable at runtime.

**Example:**

```bash
export MCP_API_TOKEN="sk_test_123"
pctx start
```

### System Keychain

**Format:** `${keychain:KEY_NAME}`

```json
{
  "token": "${keychain:mcp-api-key}"
}
```

Reads from your OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service).

### External Commands

**Format:** `${command:shell command}`

```json
{
  "token": "${command:aws secretsmanager get-secret-value --secret-id my-token --query SecretString --output text}"
}
```

Executes the command and uses its stdout as the value (whitespace is trimmed).

**Use cases:**

- AWS Secrets Manager
- Azure Key Vault
- HashiCorp Vault
- 1Password CLI
- Pass (password store)
- Custom secret management scripts

**Examples:**

AWS Secrets Manager:

```json
{
  "type": "bearer",
  "token": "${command:aws secretsmanager get-secret-value --secret-id mcp-token --query SecretString --output text}"
}
```

1Password CLI:

```json
{
  "type": "bearer",
  "token": "${command:op read op://vault/item/field}"
}
```

### Combining Plain Text and Secrets

Secret strings support interpolation with multiple parts:

```json
{
  "headers": {
    "authorization": "ApiKey ${keychain:api-key}",
    "x-custom": "prefix-${env:SUFFIX}"
  }
}
```

### Plain Text (Not Recommended)

You can use plain text values, but this is not recommended for production:

```json
{
  "token": "sk_test_hardcoded_token"
}
```

**Warning:** Never commit credentials to version control. Use secret strings instead.

## Logger Configuration

The optional `logger` field controls logging behavior for the pctx server MPC server. This configuration only applies
to `pctx start`, other commands like `pctx add` use the CLI verbosity controls (`-v/-vv/-q`).

| Field     | Type           | Required | Default     | Description                                        |
| --------- | -------------- | -------- | ----------- | -------------------------------------------------- |
| `enabled` | `boolean`      | No       | `true`      | Enable or disable logging                          |
| `level`   | `LogLevel`     | No       | `"info"`    | Minimum log level to display (see levels below)    |
| `format`  | `LoggerFormat` | No       | `"compact"` | Output format for log messages (see formats below) |
| `colors`  | `boolean`      | No       | `true`      | Enable or disable colorized output                 |

### Log Levels

Valid values for `level` (in order of increasing severity):

- `"trace"` - Most verbose, shows all logs including detailed execution traces
- `"debug"` - Detailed debugging information
- `"info"` - General informational messages (default)
- `"warn"` - Warning messages for potentially problematic situations
- `"error"` - Error messages only

### Log Formats

Valid values for `format`:

- `"compact"` - Condensed single-line format (default)
- `"pretty"` - Human-readable multi-line format with indentation
- `"json"` - Structured JSON format for log aggregation tools

### Examples

**Minimal logging (errors only):**

```json
{
  "logger": {
    "level": "error"
  }
}
```

**Debug mode with pretty formatting:**

```json
{
  "logger": {
    "enabled": true,
    "level": "debug",
    "format": "pretty",
    "colors": true
  }
}
```

**JSON logging for production (no colors):**

```json
{
  "logger": {
    "level": "info",
    "format": "json",
    "colors": false
  }
}
```

**Disable logging completely:**

```json
{
  "logger": {
    "enabled": false
  }
}
```

## Complete Example

```json
{
  "name": "my-ai-agent",
  "description": "MCP server aggregation for my AI agent",
  "logger": {
    "enabled": true,
    "level": "info",
    "format": "compact",
    "colors": true
  },
  "servers": [
    {
      "name": "stripe",
      "url": "https://mcp.stripe.com",
      "auth": {
        "type": "bearer",
        "token": "${env:STRIPE_MCP_KEY}"
      }
    },
    {
      "name": "gdrive",
      "url": "https://mcp.gdrive.example.com",
      "auth": {
        "type": "custom",
        "headers": {
          "x-api-key": "${keychain:gdrive-api-key}"
        }
      }
    },
    {
      "name": "internal",
      "url": "https://internal-mcp.company.com",
      "auth": {
        "type": "bearer",
        "token": "${command:vault kv get -field=token secret/mcp}"
      }
    },
    {
      "name": "public",
      "url": "https://public-mcp.example.com"
    }
  ]
}
```

## Managing Configuration

### Add a Server

```bash
pctx add my-server https://mcp.example.com
```

With authentication:

```bash
pctx add stripe https://mcp.stripe.com --bearer '${env:STRIPE_KEY}'
```

With custom headers:

```bash
pctx add gdrive https://mcp.example.com \
  --header 'x-api-key: ${keychain:api-key}' \
  --header 'x-custom: value'
```

### Remove a Server

```bash
pctx remove my-server
```

### List Servers

```bash
pctx list
```

This shows all configured servers and tests their connections.

## Troubleshooting

### "Failed to connect" Error

**Check:**

1. URL is correct and accessible
2. Server is running
3. Network/firewall allows the connection

### "Server requires authentication" Error

The server returned 401/403. Add authentication:

```bash
pctx add my-server https://mcp.example.com \
  --bearer '${env:TOKEN}'
```

### "Environment variable not found" Error

The specified environment variable isn't set:

```bash
# Check what's needed
cat pctx.json | grep env:

# Set the variable
export API_TOKEN="your-token"

# Or add to .env and source it
echo "API_TOKEN=your-token" >> .env
source .env
```

### "Failed to retrieve password from keychain" Error

The keychain entry doesn't exist. Create it:

```bash
# macOS
security add-generic-password -s pctx -a my-key -w "my-value"
```

Or use a different secret method.

### "Auth command failed" Error

The external command returned non-zero exit or empty output:

```bash
# Test the command directly
aws secretsmanager get-secret-value --secret-id my-token

# Check authentication for the tool
aws sts get-caller-identity
```
