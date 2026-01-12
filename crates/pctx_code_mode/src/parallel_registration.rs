//! Parallel MCP server registration
//!
//! This module provides functionality to connect to and initialize multiple MCP servers
//! in parallel, significantly reducing startup time compared to sequential initialization.

use pctx_config::server::ServerConfig;
use tracing::{debug, error, info, warn};

/// Result of successfully connecting and initializing an MCP server
pub struct ServerRegistrationResult {
    pub server_config: ServerConfig,
    pub tool_set: pctx_codegen::ToolSet,
}

/// Error result from attempting to register a server
pub struct ServerRegistrationError {
    pub server_name: String,
    pub error_message: String,
}

/// Results from parallel server registration
pub struct ParallelRegistrationResults {
    pub successful: Vec<ServerRegistrationResult>,
    pub failed: Vec<ServerRegistrationError>,
}

impl ParallelRegistrationResults {
    /// Add successful registrations to a CodeMode instance, checking for duplicates
    ///
    /// Returns the number of servers successfully added
    pub fn add_to_code_mode(&mut self, code_mode: &mut crate::CodeMode) -> usize {
        let mut added = 0;

        // Drain successful results to avoid cloning
        for server_result in self.successful.drain(..) {
            // Check for duplicate names
            if code_mode
                .tool_sets
                .iter()
                .any(|t| t.name == server_result.server_config.name)
            {
                warn!(
                    "MCP server '{}' conflicts with existing ToolSet name, skipping",
                    server_result.server_config.name
                );
                self.failed.push(ServerRegistrationError {
                    server_name: server_result.server_config.name,
                    error_message: "Conflicts with existing ToolSet name".to_string(),
                });
                continue;
            }

            code_mode.tool_sets.push(server_result.tool_set);
            code_mode.servers.push(server_result.server_config);
            added += 1;
        }

        added
    }
}

/// Register multiple MCP servers in parallel with a timeout
///
/// This function spawns parallel tasks to connect to, list tools from, and initialize
/// multiple MCP servers concurrently. This is much faster than sequential registration,
/// especially for stdio-based MCP servers which can be slow to start.
///
/// # Arguments
/// * `servers` - Slice of ServerConfig to register
/// * `timeout_secs` - Timeout in seconds for each server registration (default: 30)
///
/// # Returns
/// ParallelRegistrationResults containing successful registrations and failures
pub async fn register_servers_parallel(
    servers: &[ServerConfig],
    timeout_secs: u64,
) -> ParallelRegistrationResults {
    let registration_timeout = std::time::Duration::from_secs(timeout_secs);
    let mut tasks = Vec::new();

    for server in servers {
        let server = server.clone();
        let task = tokio::spawn(async move {
            let server_name = server.name.clone();
            let result =
                tokio::time::timeout(registration_timeout, register_single_server(&server)).await;

            match result {
                Ok(Ok(server_result)) => Ok((server_name, server_result)),
                Ok(Err(e)) => Err((server_name, e)),
                Err(_) => Err((
                    server_name,
                    format!(
                        "Registration timed out after {}s",
                        registration_timeout.as_secs()
                    ),
                )),
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;

    let mut successful = Vec::new();
    let mut failed = Vec::new();

    // Process results
    for result in results {
        match result {
            Ok(Ok((server_name, server_result))) => {
                successful.push(server_result);
                debug!("Successfully registered MCP server: {}", server_name);
            }
            Ok(Err((server_name, error_msg))) => {
                error!(
                    "Failed to register MCP server {}: {}",
                    server_name, error_msg
                );
                failed.push(ServerRegistrationError {
                    server_name,
                    error_message: error_msg,
                });
            }
            Err(e) => {
                error!("Task panicked during server registration: {}", e);
                failed.push(ServerRegistrationError {
                    server_name: "unknown".to_string(),
                    error_message: format!("Task panicked: {}", e),
                });
            }
        }
    }

    ParallelRegistrationResults { successful, failed }
}

/// Register servers from a custom conversion function
///
/// This is useful when you have a different server config type (like HTTP API models)
/// that need to be converted to ServerConfig. The conversion function handles
/// creating ServerConfig instances and extracting server names for error reporting.
pub async fn register_servers_parallel_with_conversion<T, F>(
    servers: &[T],
    timeout_secs: u64,
    convert_fn: F,
) -> ParallelRegistrationResults
where
    T: Clone + Send + 'static,
    F: Fn(&T) -> Result<(String, ServerConfig), (String, String)> + Send + Sync + 'static,
{
    let registration_timeout = std::time::Duration::from_secs(timeout_secs);
    let mut tasks = Vec::new();
    let convert_fn = std::sync::Arc::new(convert_fn);

    for server in servers {
        let server = server.clone();
        let convert_fn = convert_fn.clone();

        let task = tokio::spawn(async move {
            // First convert to ServerConfig
            let (server_name, server_config) = match convert_fn(&server) {
                Ok(result) => result,
                Err((name, err)) => return Err((name, err)),
            };

            // Then register
            let result =
                tokio::time::timeout(registration_timeout, register_single_server(&server_config))
                    .await;

            match result {
                Ok(Ok(server_result)) => Ok((server_name, server_result)),
                Ok(Err(e)) => Err((server_name, e)),
                Err(_) => Err((
                    server_name,
                    format!(
                        "Registration timed out after {}s",
                        registration_timeout.as_secs()
                    ),
                )),
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;

    let mut successful = Vec::new();
    let mut failed = Vec::new();

    // Process results
    for result in results {
        match result {
            Ok(Ok((server_name, server_result))) => {
                successful.push(server_result);
                debug!("Successfully registered MCP server: {}", server_name);
            }
            Ok(Err((server_name, error_msg))) => {
                error!(
                    "Failed to register MCP server {}: {}",
                    server_name, error_msg
                );
                failed.push(ServerRegistrationError {
                    server_name,
                    error_message: error_msg,
                });
            }
            Err(e) => {
                error!("Task panicked during server registration: {}", e);
                failed.push(ServerRegistrationError {
                    server_name: "unknown".to_string(),
                    error_message: format!("Task panicked: {}", e),
                });
            }
        }
    }

    ParallelRegistrationResults { successful, failed }
}

/// Connect to and initialize a single MCP server
///
/// This performs the following slow I/O operations:
/// 1. Connect to the MCP server (especially slow for stdio servers)
/// 2. List all available tools
/// 3. Convert MCP tool schemas to codegen tool schemas
/// 4. Create a ToolSet for the server
///
/// This function is designed to run in parallel via tokio::spawn.
async fn register_single_server(server: &ServerConfig) -> Result<ServerRegistrationResult, String> {
    // Connect to the MCP server (this is the slow operation)
    debug!(
        "Connecting to MCP server '{}'({})...",
        &server.name,
        server.display_target()
    );
    let mcp_client = server
        .connect()
        .await
        .map_err(|e| format!("Failed to connect: {e}"))?;

    debug!(
        "Successfully connected to '{}', listing tools...",
        server.name
    );

    // List all tools (another potentially slow operation)
    let listed_tools = mcp_client
        .list_all_tools()
        .await
        .map_err(|e| format!("Failed to list tools: {e}"))?;

    debug!("Found {} tools from '{}'", listed_tools.len(), server.name);

    // Convert MCP tools to codegen tools
    let mut codegen_tools = vec![];
    for mcp_tool in listed_tools {
        let input_schema: pctx_codegen::RootSchema =
            serde_json::from_value(serde_json::json!(mcp_tool.input_schema)).map_err(|e| {
                format!(
                    "Failed parsing inputSchema for tool `{}`: {e}",
                    &mcp_tool.name
                )
            })?;

        let output_schema = if let Some(o) = mcp_tool.output_schema {
            Some(
                serde_json::from_value::<pctx_codegen::RootSchema>(serde_json::json!(o)).map_err(
                    |e| {
                        format!(
                            "Failed parsing outputSchema for tool `{}`: {e}",
                            &mcp_tool.name
                        )
                    },
                )?,
            )
        } else {
            None
        };

        codegen_tools.push(
            pctx_codegen::Tool::new_mcp(
                &mcp_tool.name,
                mcp_tool.description.map(String::from),
                input_schema,
                output_schema,
            )
            .map_err(|e| format!("Failed to create tool `{}`: {e}", &mcp_tool.name))?,
        );
    }

    let description = mcp_client
        .peer_info()
        .and_then(|p| p.server_info.title.clone())
        .unwrap_or(format!("MCP server at {}", server.display_target()));

    let tool_set = pctx_codegen::ToolSet::new(&server.name, &description, codegen_tools);

    info!(
        "Successfully initialized MCP server '{}' with {} tools",
        server.name,
        tool_set.tools.len()
    );

    Ok(ServerRegistrationResult {
        server_config: server.clone(),
        tool_set,
    })
}
