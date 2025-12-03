use std::sync::Arc;
use std::time::Instant;

use crate::session::OutgoingMessage;
use axum::{Json, extract::State, http::StatusCode};
use pctx_code_execution_runtime::CallbackFn;
use pctx_code_mode::model::{
    ExecuteInput, ExecuteOutput, GetFunctionDetailsInput, GetFunctionDetailsOutput,
    ListFunctionsOutput,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::AppState;
use crate::model::{
    ErrorInfo, ErrorResponse, HealthResponse, McpServerConfig, RegisterLocalToolsRequest,
    RegisterLocalToolsResponse, RegisterMcpServersRequest, RegisterMcpServersResponse,
};

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

/// List all available code mode functions from both server and tool registrations
#[utoipa::path(
    post,
    path = "/code-mode/functions/list",
    tag = "CodeMode",
    responses(
        (status = 200, description = "List of all code mode functions as source code & structured output", body = ListFunctionsOutput),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(crate) async fn list_functions(
    State(state): State<AppState>,
) -> Result<Json<ListFunctionsOutput>, (StatusCode, Json<ErrorResponse>)> {
    info!("Listing tools");

    let code_mode = state.code_mode.lock().await;
    let functions = code_mode.list_functions();

    Ok(Json(functions))
}

/// Get detailed information about a specific function
#[utoipa::path(
    post,
    path = "/code-mode/functions/details",
    tag = "CodeMode",
    request_body = GetFunctionDetailsInput,
    responses(
        (status = 200, description = "Function details", body = GetFunctionDetailsOutput),
        (status = 404, description = "Function not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(crate) async fn get_function_details(
    State(state): State<AppState>,
    Json(request): Json<GetFunctionDetailsInput>,
) -> Result<Json<GetFunctionDetailsOutput>, (StatusCode, Json<ErrorResponse>)> {
    let requested_functions = request
        .functions
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(", ");
    info!("Getting function details for {requested_functions}",);

    let code_mode = state.code_mode.lock().await;
    let output = code_mode.get_function_details(request);

    // Check if we got the function
    if output.functions.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorInfo {
                    code: "NOT_FOUND".to_string(),
                    message: format!("Functions not found: {requested_functions}"),
                    details: None,
                },
            }),
        ));
    }

    Ok(Json(output))
}

/// Execute TypeScript code with access to registered tools
#[utoipa::path(
    post,
    path = "/code-mode/execute",
    tag = "CodeMode",
    request_body = ExecuteInput,
    responses(
        (status = 200, description = "Code executed successfully", body = ExecuteOutput),
        (status = 400, description = "Code execution failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(crate) async fn execute_code(
    State(state): State<AppState>,
    Json(request): Json<ExecuteInput>,
) -> Result<Json<ExecuteOutput>, (StatusCode, Json<ErrorResponse>)> {
    info!("Executing code");

    let start = Instant::now();
    let current_span = tracing::Span::current();

    // Clone the CodeMode Arc to move into spawn_blocking
    let code_mode = Arc::clone(&state.code_mode);
    let callback_registry = state.callback_registry.clone();
    let code = request.code;

    // Use spawn_blocking with current-thread runtime for Deno's unsync operations
    let output = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
        let _guard = current_span.enter();

        // Create new current-thread runtime for Deno ops
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;

        rt.block_on(async {
            let code_mode_guard = code_mode.lock().await;
            code_mode_guard
                .execute(&code, callback_registry)
                .await
                .map_err(|e| anyhow::anyhow!("Execution error: {e}"))
        })
    })
    .await
    .map_err(|e| {
        error!("Task join failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorInfo {
                    code: "INTERNAL_ERROR".to_string(),
                    message: format!("Task join failed: {e}"),
                    details: None,
                },
            }),
        )
    })?
    .map_err(|e| {
        error!("Sandbox execution error: {e}");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorInfo {
                    code: "EXECUTION_ERROR".to_string(),
                    message: format!("Execution failed: {e}"),
                    details: None,
                },
            }),
        )
    })?;

    let execution_time_ms = start.elapsed().as_millis() as u64;
    info!("Completed execution in {execution_time_ms}ms");

    Ok(axum::Json(output))
}
/// Register tools that will be called via WebSocket callbacks
#[utoipa::path(
    post,
    path = "/register/tools",
    tag = "registration",
    request_body = RegisterLocalToolsRequest,
    responses(
        (status = 200, description = "Tools registered successfully", body = RegisterLocalToolsResponse),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(crate) async fn register_tools(
    State(state): State<AppState>,
    Json(request): Json<RegisterLocalToolsRequest>,
) -> Result<Json<RegisterLocalToolsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Registering {} tools for session {}",
        request.tools.len(),
        request.session_id
    );

    let mut registered = 0;
    for tool in &request.tools {
        // Create callback closure that captures session state
        let session_manager_clone = Arc::clone(&state.session_manager);
        let tool_id = tool.id();

        let callback: CallbackFn = Arc::new(move |args: Option<serde_json::Value>| {
            let session_manager_clone = session_manager_clone.clone();
            let tool_id_clone = tool_id.clone();

            Box::pin(async move {
                let request_id = Uuid::new_v4().to_string();

                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "execute_tool",
                    "params": {
                        "name": tool_id_clone,
                        "arguments": args
                    },
                    "id": request_id.clone()
                });

                session_manager_clone
                    .execute_callback_raw(
                        &tool_id_clone,
                        OutgoingMessage::Response(request),
                        serde_json::Value::String(request_id),
                    )
                    .await
                    .map_err(|e| e.to_string())
            })
        });

        // Register callback in CallbackRegistry
        state
            .callback_registry
            .add(&tool.id(), callback)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: ErrorInfo {
                            code: "INTERNAL_ERROR".to_string(),
                            message: format!("Failed to register callback: {e}"),
                            details: None,
                        },
                    }),
                )
            })?;

        let mut code_mode = state.code_mode.lock().await;
        code_mode.add_callback(tool).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: ErrorInfo {
                        code: "INTERNAL_ERROR".to_string(),
                        message: format!("Failed to register callback in CodeMode: {e}"),
                        details: None,
                    },
                }),
            )
        })?;

        // Register with session_manager for tracking
        state
            .session_manager
            .register_callback(&request.session_id, tool.id(), tool.description.clone())
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: ErrorInfo {
                            code: "INTERNAL_ERROR".to_string(),
                            message: format!("Failed to register tool with session: {e}"),
                            details: None,
                        },
                    }),
                )
            })?;

        registered += 1;
    }

    Ok(Json(RegisterLocalToolsResponse { registered }))
}

/// Register MCP servers dynamically at runtime
#[utoipa::path(
    post,
    path = "/register/servers",
    tag = "registration",
    request_body = RegisterMcpServersRequest,
    responses(
        (status = 200, description = "MCP servers registration result", body = RegisterMcpServersResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(crate) async fn register_servers(
    State(state): State<AppState>,
    Json(request): Json<RegisterMcpServersRequest>,
) -> Json<RegisterMcpServersResponse> {
    info!("Registering {} MCP servers", request.servers.len());

    let mut registered = 0;
    let mut failed = Vec::new();

    for server in &request.servers {
        match register_mcp_server(&state, server).await {
            Ok(()) => {
                registered += 1;
                info!("Successfully registered MCP server: {}", server.name);
            }
            Err(e) => {
                error!("Failed to register MCP server {}: {}", server.name, e);
                failed.push(server.name.clone());
            }
        }
    }

    Json(RegisterMcpServersResponse { registered, failed })
}

async fn register_mcp_server(state: &AppState, server: &McpServerConfig) -> Result<(), String> {
    // Parse and validate URL
    let url = url::Url::parse(&server.url).map_err(|e| format!("Invalid URL: {e}"))?;

    // Create ServerConfig
    let mut server_config = pctx_config::server::ServerConfig::new(server.name.clone(), url);

    // Add auth if provided
    if let Some(auth) = &server.auth {
        server_config.auth = serde_json::from_value(auth.clone())
            .map_err(|e| format!("Invalid auth config: {e}"))?;
    }

    // Connect to MCP server and register tools
    let mut code_mode = state.code_mode.lock().await;

    code_mode
        .add_server(&server_config)
        .await
        .map_err(|e| format!("Failed to add MCP server: {e}"))?;

    info!(
        "Successfully registered MCP server '{}' with {} tools",
        server.name,
        code_mode
            .tool_sets
            .iter()
            .find(|ts| ts.name == server.name)
            .map_or(0, |ts| ts.tools.len())
    );

    Ok(())
}
