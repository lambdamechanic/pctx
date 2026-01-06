use anyhow::Context;
use axum::{Json, extract::State, http::StatusCode};

use pctx_code_mode::{
    CodeMode,
    model::{GetFunctionDetailsInput, GetFunctionDetailsOutput, ListFunctionsOutput},
};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::extractors::CodeModeSession;
use crate::model::{
    ApiError, ApiResult, CloseSessionResponse, CreateSessionResponse, ErrorCode, ErrorData,
    HealthResponse, McpServerConfig, RegisterMcpServersRequest, RegisterMcpServersResponse,
    RegisterToolsRequest, RegisterToolsResponse,
};
use crate::state::{AppState, backend::PctxSessionBackend};

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create a new `CodeMode` session
#[utoipa::path(
    post,
    path = "/code-mode/session/create",
    tag = "CodeMode",
    responses(
        (status = 200, description = "Session created successfully", body = CreateSessionResponse),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn create_session<B: PctxSessionBackend>(
    State(state): State<AppState<B>>,
) -> ApiResult<Json<CreateSessionResponse>> {
    let session_id = Uuid::new_v4();
    info!("Creating new CodeMode session: {session_id}");

    let code_mode = CodeMode::default();
    state
        .backend
        .insert(session_id, code_mode)
        .await
        .context("Failed inserting code mode session into backend")?;

    info!("Created CodeMode session: {session_id}");

    Ok(Json(CreateSessionResponse { session_id }))
}

/// Close a `CodeMode` session
#[utoipa::path(
    post,
    path = "/code-mode/session/close",
    tag = "CodeMode",
    params(
        ("x-code-mode-session" = String, Header, description = "Current code mode session")
    ),
    responses(
        (status = 200, description = "Session closed successfully", body = CloseSessionResponse),
        (status = 404, description = "Session not found", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn close_session<B: PctxSessionBackend>(
    State(state): State<AppState<B>>,
    CodeModeSession(session_id): CodeModeSession,
) -> ApiResult<Json<CloseSessionResponse>> {
    info!("Closing CodeMode session: {session_id}");

    let existed = state
        .backend
        .delete(session_id)
        .await
        .context("Failed deleting code mode session from backend")?;

    if !existed {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            ErrorData {
                code: ErrorCode::InvalidSession,
                message: format!("Code Mode session {session_id} does not exist"),
                details: None,
            },
        ));
    }

    info!("Closed CodeMode session: {session_id}");

    Ok(Json(CloseSessionResponse { success: true }))
}

/// List all available code mode functions from both server and tool registrations
#[utoipa::path(
    post,
    path = "/code-mode/functions/list",
    tag = "CodeMode",
    params(
        ("x-code-mode-session" = String, Header, description = "Current code mode session")
    ),
    responses(
        (status = 200, description = "List of all code mode functions as source code & structured output", body = ListFunctionsOutput),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn list_functions<B: PctxSessionBackend>(
    State(state): State<AppState<B>>,
    CodeModeSession(session_id): CodeModeSession,
) -> ApiResult<Json<ListFunctionsOutput>> {
    info!(session_id =? session_id, "Listing tools");

    let code_mode = state
        .backend
        .get(session_id)
        .await
        .context("Failed getting code mode session")?
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            ErrorData {
                code: ErrorCode::InvalidSession,
                message: format!("Code Mode session {session_id} does not exist"),
                details: None,
            },
        ))?;

    let functions = code_mode.list_functions();

    Ok(Json(functions))
}

/// Get detailed information about a specific function
#[utoipa::path(
    post,
    path = "/code-mode/functions/details",
    tag = "CodeMode",
    params(
        ("x-code-mode-session" = String, Header, description = "Current code mode session")
    ),
    request_body = GetFunctionDetailsInput,
    responses(
        (status = 200, description = "Function details", body = GetFunctionDetailsOutput),
        (status = 404, description = "Function not found", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn get_function_details<B: PctxSessionBackend>(
    State(state): State<AppState<B>>,
    CodeModeSession(session_id): CodeModeSession,
    Json(request): Json<GetFunctionDetailsInput>,
) -> ApiResult<Json<GetFunctionDetailsOutput>> {
    let requested_functions = request
        .functions
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(", ");
    info!(
        session_id =? session_id,
        "Getting function details for {requested_functions}"
    );

    let code_mode = state.backend.get(session_id).await?.ok_or(ApiError::new(
        StatusCode::NOT_FOUND,
        ErrorData {
            code: ErrorCode::InvalidSession,
            message: format!("Code mode session {session_id} does not exist"),
            details: None,
        },
    ))?;

    let details = code_mode.get_function_details(request);

    Ok(Json(details))
}

/// Register tools that will be called via WebSocket callbacks
#[utoipa::path(
    post,
    path = "/register/tools",
    tag = "registration",
    params(
        ("x-code-mode-session" = String, Header, description = "Current code mode session")
    ),
    request_body = RegisterToolsRequest,
    responses(
        (status = 200, description = "Tools registered successfully", body = RegisterToolsResponse),
        (status = 404, description = "Session not found", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn register_tools<B: PctxSessionBackend>(
    State(state): State<AppState<B>>,
    CodeModeSession(session_id): CodeModeSession,
    Json(request): Json<RegisterToolsRequest>,
) -> ApiResult<Json<RegisterToolsResponse>> {
    info!(
        "Registering {} tools for session {session_id}",
        request.tools.len(),
    );

    let mut code_mode = state
        .backend
        .get(session_id)
        .await
        .context("Failed getting codemode session from backend")?
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            ErrorData {
                code: ErrorCode::InvalidSession,
                message: format!("Code mode session {session_id} does not exist"),
                details: None,
            },
        ))?;

    let mut registered = 0;
    for tool in &request.tools {
        code_mode
            .add_callback(tool)
            .context("Failed adding callback")?;

        registered += 1;
    }

    // Update the backend with the modified CodeMode
    state.backend.update(session_id, code_mode).await?;

    Ok(Json(RegisterToolsResponse { registered }))
}

/// Register MCP servers dynamically at runtime
#[utoipa::path(
    post,
    path = "/register/servers",
    tag = "registration",
    params(
        ("x-code-mode-session" = String, Header, description = "Current code mode session")
    ),
    request_body = RegisterMcpServersRequest,
    responses(
        (status = 200, description = "MCP servers registration result", body = RegisterMcpServersResponse),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn register_servers<B: PctxSessionBackend>(
    State(state): State<AppState<B>>,
    CodeModeSession(session_id): CodeModeSession,
    Json(request): Json<RegisterMcpServersRequest>,
) -> ApiResult<Json<RegisterMcpServersResponse>> {
    info!(
        "Registering {} MCP servers in session {session_id}",
        request.servers.len()
    );

    let mut code_mode = state
        .backend
        .get(session_id)
        .await
        .context("Failed getting code mode session from backend")?
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            ErrorData {
                code: ErrorCode::InvalidSession,
                message: format!("Code mode session {session_id} does not exist"),
                details: None,
            },
        ))?;

    // Spawn parallel registration tasks with timeout
    let registration_timeout = std::time::Duration::from_secs(30);
    let mut tasks = Vec::new();

    for server in request.servers {
        let server_name = match &server {
            McpServerConfig::Http { name, .. } => name.clone(),
            McpServerConfig::Stdio { name, .. } => name.clone(),
        };

        let task = tokio::spawn(async move {
            let result =
                tokio::time::timeout(registration_timeout, register_mcp_server_config(&server))
                    .await;

            match result {
                Ok(Ok(server_result)) => Ok((server_name, server_result)),
                Ok(Err(e)) => Err((server_name, format!("Failed to register: {e}"))),
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

    let mut registered = 0;
    let mut failed = Vec::new();

    // Process results and add successful servers to code_mode
    for result in results {
        match result {
            Ok(Ok((server_name, server_result))) => {
                // Check for duplicate names
                if code_mode
                    .tool_sets
                    .iter()
                    .any(|t| t.name == server_result.server_config.name)
                {
                    error!(
                        "MCP server '{}' conflicts with existing ToolSet name",
                        server_result.server_config.name
                    );
                    failed.push(server_name);
                    continue;
                }

                // Add the pre-initialized tool_set and server directly to code_mode
                code_mode.tool_sets.push(server_result.tool_set);
                code_mode.servers.push(server_result.server_config);
                registered += 1;
            }
            Ok(Err((server_name, error_msg))) => {
                error!(
                    "Failed to register MCP server {}: {}",
                    server_name, error_msg
                );
                failed.push(server_name);
            }
            Err(e) => {
                error!("Task panicked during server registration: {}", e);
                failed.push("unknown".to_string());
            }
        }
    }

    // Update the backend with the modified CodeMode
    state
        .backend
        .update(session_id, code_mode)
        .await
        .context("Failed updating code mode session in backend")?;

    Ok(Json(RegisterMcpServersResponse { registered, failed }))
}

/// Holds the result of successfully connecting and initializing an MCP server
struct ServerRegistrationResult {
    server_config: pctx_config::server::ServerConfig,
    tool_set: codegen::ToolSet,
}

/// Fully connects to and initializes an MCP server, including listing tools
/// Potentially slow operation (stdio) so parallelize
async fn register_mcp_server_config(
    server: &McpServerConfig,
) -> Result<ServerRegistrationResult, String> {
    let server_config = match server {
        McpServerConfig::Http { name, url, auth } => {
            // Parse and validate URL
            let parsed_url = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;

            // Create HTTP ServerConfig
            let mut server_config =
                pctx_config::server::ServerConfig::new(name.clone(), parsed_url);

            // Add auth if provided
            if let Some(auth_value) = auth {
                let auth = serde_json::from_value(auth_value.clone())
                    .map_err(|e| format!("Invalid auth config: {e}"))?;
                server_config.set_auth(Some(auth));
            }

            server_config
        }
        McpServerConfig::Stdio {
            name,
            command,
            args,
            env,
        } => {
            // Create stdio ServerConfig
            pctx_config::server::ServerConfig::new_stdio(
                name.clone(),
                command.clone(),
                args.clone(),
                env.clone(),
            )
        }
    };

    let server_name = match server {
        McpServerConfig::Http { name, .. } => name.clone(),
        McpServerConfig::Stdio { name, .. } => name.clone(),
    };

    // Connect to the MCP server (this is the slow operation)
    debug!(
        "Connecting to MCP server '{}'({})...",
        &server_config.name,
        server_config.display_target()
    );
    let mcp_client = server_config
        .connect()
        .await
        .map_err(|e| format!("Failed to connect: {e}"))?;

    debug!(
        "Successfully connected to '{}', listing tools...",
        server_config.name
    );

    // List all tools (another potentially slow operation)
    let listed_tools = mcp_client
        .list_all_tools()
        .await
        .map_err(|e| format!("Failed to list tools: {e}"))?;

    debug!("Found {} tools from '{}'", listed_tools.len(), server_name);

    // Convert MCP tools to codegen tools
    let mut codegen_tools = vec![];
    for mcp_tool in listed_tools {
        let input_schema: codegen::RootSchema =
            serde_json::from_value(serde_json::json!(mcp_tool.input_schema)).map_err(|e| {
                format!(
                    "Failed parsing inputSchema for tool `{}`: {e}",
                    &mcp_tool.name
                )
            })?;

        let output_schema = if let Some(o) = mcp_tool.output_schema {
            Some(
                serde_json::from_value::<codegen::RootSchema>(serde_json::json!(o)).map_err(
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
            codegen::Tool::new_mcp(
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
        .unwrap_or(format!("MCP server at {}", server_config.display_target()));

    let tool_set = codegen::ToolSet::new(&server_config.name, &description, codegen_tools);

    info!(
        "Successfully initialized MCP server '{}' with {} tools",
        server_name,
        tool_set.tools.len()
    );

    Ok(ServerRegistrationResult {
        server_config,
        tool_set,
    })
}
