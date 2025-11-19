# Command-Line Help for `pctx`

This document contains the help content for the `pctx` command-line program.

**Command Overview:**

* [`pctx`↴](#pctx)
* [`pctx list`↴](#pctx-list)
* [`pctx add`↴](#pctx-add)
* [`pctx remove`↴](#pctx-remove)
* [`pctx start`↴](#pctx-start)
* [`pctx dev`↴](#pctx-dev)
* [`pctx init`↴](#pctx-init)

## `pctx`

PCTX aggregates multiple MCP servers into a single endpoint, exposing them as a TypeScript API for AI agents to call via code execution.

**Usage:** `pctx [OPTIONS] <COMMAND>`

EXAMPLES:
  pctx init 
  pctx add my-server https://mcp.example.com
  pctx dev


###### **Subcommands:**

* `list` — List MCP servers and test connections
* `add` — Add an MCP server to configuration
* `remove` — Remove an MCP server from configuration
* `start` — Start the PCTX server
* `dev` — Start the PCTX server with terminal UI
* `init` — Initialize configuration file

###### **Options:**

* `-c`, `--config <CONFIG>` — Config file path, defaults to ./pctx.json

  Default value: `pctx.json`
* `-q`, `--quiet` — No logging except for errors
* `-v`, `--verbose` — Verbose logging (-v) or trace logging (-vv)



## `pctx list`

Lists configured MCP servers and tests the connection to each.

**Usage:** `pctx list`



## `pctx add`

Add a new MCP server to the configuration.

**Usage:** `pctx add [OPTIONS] <NAME> <URL>`

###### **Arguments:**

* `<NAME>` — Unique name for this server
* `<URL>` — HTTP(S) URL of the MCP server endpoint

###### **Options:**

* `-b`, `--bearer <BEARER>` — use bearer authentication to connect to MCP server using PCTX's secret string syntax.

   e.g. `--bearer '${env:BEARER_TOKEN}'`
* `-H`, `--header <HEADER>` — use custom headers to connect to MCP server using PCTX's secret string syntax. Many headers can be defined.

   e.g. `--headers 'x-api-key: ${keychain:API_KEY}'`
* `-f`, `--force` — Overrides any existing server under the same name & skips testing connection to the MCP server



## `pctx remove`

Remove an MCP server from the configuration.

**Usage:** `pctx remove <NAME>`

###### **Arguments:**

* `<NAME>` — Name of the server to remove



## `pctx start`

Start the PCTX server (exposes /mcp endpoint).

**Usage:** `pctx start [OPTIONS]`

###### **Options:**

* `-p`, `--port <PORT>` — Port to listen on

  Default value: `8080`
* `--host <HOST>` — Host address to bind to (use 0.0.0.0 for external access)

  Default value: `127.0.0.1`
* `--no-banner` — Don't show the server banner



## `pctx dev`

Start the PCTX server in development mode with an interactive terminal UI with data and logging.

**Usage:** `pctx dev [OPTIONS]`

###### **Options:**

* `-p`, `--port <PORT>` — Port to listen on

  Default value: `8080`
* `--host <HOST>` — Host address to bind to (use 0.0.0.0 for external access)

  Default value: `127.0.0.1`
* `--log-file <LOG_FILE>` — Path to JSONL log file

  Default value: `pctx-dev.jsonl`



## `pctx init`

Initialize pctx.json configuration file.

**Usage:** `pctx init [OPTIONS]`

###### **Options:**

* `-y`, `--yes` — Use default values and skip interactive adding of upstream MCPs



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

