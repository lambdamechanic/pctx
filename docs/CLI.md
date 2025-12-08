# Command-Line Help for `pctx`

This document contains the help content for the `pctx` command-line program.

**Command Overview:**

* [`pctx`↴](#pctx)
* [`pctx mcp`↴](#pctx-mcp)
* [`pctx mcp init`↴](#pctx-mcp-init)
* [`pctx mcp list`↴](#pctx-mcp-list)
* [`pctx mcp add`↴](#pctx-mcp-add)
* [`pctx mcp remove`↴](#pctx-mcp-remove)
* [`pctx mcp start`↴](#pctx-mcp-start)
* [`pctx mcp dev`↴](#pctx-mcp-dev)
* [`pctx agent`↴](#pctx-agent)
* [`pctx agent dev`↴](#pctx-agent-dev)

## `pctx`

PCTX aggregates multiple MCP servers into a single endpoint, exposing them as a TypeScript API for AI agents to call via code execution.

**Usage:** `pctx [OPTIONS] <COMMAND>`

EXAMPLES:
  # MCP mode (with pctx.json configuration)
  pctx mcp init 
  pctx mcp add my-server https://mcp.example.com
  pctx mcp dev

  # Agent mode (REST API + WebSocket, no config)
  pctx agent dev


###### **Subcommands:**

* `mcp` — MCP server commands (with pctx.json configuration)
* `agent` — Agent mode commands (REST API + WebSocket, no config file)

###### **Options:**

* `-c`, `--config <CONFIG>` — Config file path, defaults to ./pctx.json

  Default value: `pctx.json`
* `-q`, `--quiet` — No logging except for errors
* `-v`, `--verbose` — Verbose logging (-v) or trace logging (-vv)



## `pctx mcp`

MCP server commands (with pctx.json configuration)

**Usage:** `pctx mcp <COMMAND>`

###### **Subcommands:**

* `init` — Initialize pctx.json configuration file
* `list` — List MCP servers and test connections
* `add` — Add an MCP server to configuration
* `remove` — Remove an MCP server from configuration
* `start` — Start the PCTX MCP server
* `dev` — Start the PCTX MCP server with terminal UI



## `pctx mcp init`

Initialize pctx.json configuration file.

**Usage:** `pctx mcp init [OPTIONS]`

###### **Options:**

* `-y`, `--yes` — Use default values and skip interactive adding of upstream MCPs



## `pctx mcp list`

Lists configured MCP servers and tests the connection to each.

**Usage:** `pctx mcp list`



## `pctx mcp add`

Add a new MCP server to the configuration.

**Usage:** `pctx mcp add [OPTIONS] <NAME> <URL>`

###### **Arguments:**

* `<NAME>` — Unique name for this server
* `<URL>` — HTTP(S) URL of the MCP server endpoint

###### **Options:**

* `-b`, `--bearer <BEARER>` — use bearer authentication to connect to MCP server using PCTX's secret string syntax.

   e.g. `--bearer '${env:BEARER_TOKEN}'`
* `-H`, `--header <HEADER>` — use custom headers to connect to MCP server using PCTX's secret string syntax. Many headers can be defined.

   e.g. `--headers 'x-api-key: ${keychain:API_KEY}'`
* `-f`, `--force` — Overrides any existing server under the same name & skips testing connection to the MCP server



## `pctx mcp remove`

Remove an MCP server from the configuration.

**Usage:** `pctx mcp remove <NAME>`

###### **Arguments:**

* `<NAME>` — Name of the server to remove



## `pctx mcp start`

Start the PCTX MCP server (exposes /mcp endpoint).

**Usage:** `pctx mcp start [OPTIONS]`

###### **Options:**

* `-p`, `--port <PORT>` — Port to listen on

  Default value: `8080`
* `--host <HOST>` — Host address to bind to (use 0.0.0.0 for external access)

  Default value: `127.0.0.1`
* `--no-banner` — Don't show the server banner



## `pctx mcp dev`

Start the PCTX MCP server in development mode with an interactive terminal UI with data and logging.

**Usage:** `pctx mcp dev [OPTIONS]`

###### **Options:**

* `-p`, `--port <PORT>` — Port to listen on

  Default value: `8080`
* `--host <HOST>` — Host address to bind to (use 0.0.0.0 for external access)

  Default value: `127.0.0.1`
* `--log-file <LOG_FILE>` — Path to JSONL log file

  Default value: `pctx-dev.jsonl`



## `pctx agent`

Agent mode commands (REST API + WebSocket, no config file)

**Usage:** `pctx agent <COMMAND>`

###### **Subcommands:**

* `dev` — Start agent mode (REST API + WebSocket)



## `pctx agent dev`

Start agent mode with REST API and WebSocket server. No tools preloaded - use REST API to register tools and MCP servers dynamically.

**Usage:** `pctx agent dev [OPTIONS]`

###### **Options:**

* `-p`, `--port <PORT>` — Port to run the server on

  Default value: `8080`
* `--host <HOST>` — Host to bind to

  Default value: `127.0.0.1`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

