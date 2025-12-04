use std::sync::Arc;
use std::time::Instant;

use crate::state::ws_manager::WsExecuteTool;
use axum::{Json, extract::State, http::StatusCode};
use pctx_code_execution_runtime::CallbackFn;
use pctx_code_mode::{
    CodeMode,
    model::{
        ExecuteInput, ExecuteOutput, GetFunctionDetailsInput, GetFunctionDetailsOutput,
        ListFunctionsOutput,
    },
};
use serde_json::json;
use tracing::{error, info};
use uuid::Uuid;

use crate::AppState;
use crate::extractors::CodeModeSession;
use crate::model::{
    CloseSessionRequest, CloseSessionResponse, CreateSessionResponse, ErrorCode, ErrorData,
    HealthResponse, McpServerConfig, RegisterMcpServersRequest, RegisterMcpServersResponse,
    RegisterToolsRequest, RegisterToolsResponse,
};

pub(crate) type ApiResult<T> = Result<T, (StatusCode, Json<ErrorData>)>;

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
pub(crate) async fn create_session(
    State(state): State<AppState>,
) -> ApiResult<Json<CreateSessionResponse>> {
    let session_id = Uuid::new_v4();
    info!("Creating new CodeMode session: {session_id}");

    let code_mode = CodeMode::default();
    state.code_mode_manager.add(session_id, code_mode).await;

    info!("Created CodeMode session: {session_id}");

    Ok(Json(CreateSessionResponse { session_id }))
}

/// Close a `CodeMode` session
#[utoipa::path(
    post,
    path = "/code-mode/session/close",
    tag = "CodeMode",
    request_body = CloseSessionRequest,
    responses(
        (status = 200, description = "Session closed successfully", body = CloseSessionResponse),
        (status = 404, description = "Session not found", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn close_session(
    State(state): State<AppState>,
    Json(request): Json<CloseSessionRequest>,
) -> ApiResult<Json<CloseSessionResponse>> {
    info!("Closing CodeMode session: {}", request.session_id);

    let existed = state.code_mode_manager.delete(request.session_id).await;

    if existed.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorData {
                code: ErrorCode::InvalidSession,
                message: format!("Code mode session {} does not exist", request.session_id),
                details: None,
            }),
        ));
    }

    info!("Closed CodeMode session: {}", request.session_id);

    Ok(Json(CloseSessionResponse { success: true }))
}

/// List all available code mode functions from both server and tool registrations
#[utoipa::path(
    post,
    path = "/code-mode/functions/list",
    tag = "CodeMode",
    responses(
        (status = 200, description = "List of all code mode functions as source code & structured output", body = ListFunctionsOutput),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn list_functions(
    State(state): State<AppState>,
    CodeModeSession(session_id): CodeModeSession,
) -> ApiResult<Json<ListFunctionsOutput>> {
    info!(session_id =? session_id, "Listing tools");

    let code_mode_lock = state.code_mode_manager.get(session_id).await.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorData {
            code: ErrorCode::InvalidSession,
            message: format!("Code mode session {session_id} does not exist"),
            details: None,
        }),
    ))?;

    let functions = code_mode_lock.read().await.list_functions();

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
        (status = 404, description = "Function not found", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn get_function_details(
    State(state): State<AppState>,
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

    let code_mode_lock = state.code_mode_manager.get(session_id).await.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorData {
            code: ErrorCode::InvalidSession,
            message: format!("Code mode session {session_id} does not exist"),
            details: None,
        }),
    ))?;

    let details = code_mode_lock.read().await.get_function_details(request);

    Ok(Json(details))
}

/// Execute TypeScript code with access to registered tools
#[utoipa::path(
    post,
    path = "/code-mode/execute",
    tag = "CodeMode",
    request_body = ExecuteInput,
    responses(
        (status = 200, description = "Code executed successfully", body = ExecuteOutput),
        (status = 400, description = "Code execution failed", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn execute_code(
    State(state): State<AppState>,
    CodeModeSession(session_id): CodeModeSession,
    Json(request): Json<ExecuteInput>,
) -> ApiResult<Json<ExecuteOutput>> {
    info!("Executing code");

    let start = Instant::now();
    let current_span = tracing::Span::current();

    // Clone the CodeMode Arc to move into spawn_blocking
    let code_mode_lock = state.code_mode_manager.get(session_id).await.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorData {
            code: ErrorCode::InvalidSession,
            message: format!("Code mode session {session_id} does not exist"),
            details: None,
        }),
    ))?;

    // Use spawn_blocking with current-thread runtime for Deno's unsync operations
    let output = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
        let _guard = current_span.enter();

        // Create new current-thread runtime for Deno ops
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;

        rt.block_on(async {
            code_mode_lock
                .read()
                .await
                .execute(&request.code)
                .await
                .map_err(|e| anyhow::anyhow!("Execution error: {e}"))
        })
    })
    .await
    .map_err(|e| {
        error!("Task join failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorData {
                code: ErrorCode::Internal,
                message: format!("Execute task join failed: {e}"),
                details: None,
            }),
        )
    })?
    .map_err(|e| {
        error!("Sandbox execution error: {e}");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorData {
                code: ErrorCode::Execution,
                message: format!("Execution failed: {e}"),
                details: None,
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
    request_body = RegisterToolsRequest,
    responses(
        (status = 200, description = "Tools registered successfully", body = RegisterToolsResponse),
        (status = 404, description = "Session not found", body = ErrorData),
        (status = 500, description = "Internal server error", body = ErrorData)
    )
)]
pub(crate) async fn register_tools(
    State(state): State<AppState>,
    CodeModeSession(session_id): CodeModeSession,
    Json(request): Json<RegisterToolsRequest>,
) -> ApiResult<Json<RegisterToolsResponse>> {
    info!(
        "Registering {} tools for session {session_id}",
        request.tools.len(),
    );

    let code_mode_lock = state.code_mode_manager.get(session_id).await.unwrap();
    let ws_session_lock = state
        .ws_manager
        .get_for_code_mode_session(session_id)
        .await
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorData {
                code: ErrorCode::InvalidSession,
                message: format!("Failed to find websocket for session {session_id}"),
                details: None,
            }),
        ))?;

    let mut registered = 0;
    for tool in &request.tools {
        // Create callback closure that captures session state
        let ws_session_lock_clone = ws_session_lock.clone();
        let tool_cfg = tool.clone();

        let callback: CallbackFn = Arc::new(move |args: Option<serde_json::Value>| {
            let tool_cfg_clone = tool_cfg.clone();
            let ws_session_lock_clone = ws_session_lock_clone.clone();

            Box::pin(async move {
                let ws_session = ws_session_lock_clone.read().await;

                let callback_res = ws_session
                    .execute_callback(WsExecuteTool {
                        id: Uuid::new_v4(),
                        namespace: tool_cfg_clone.namespace,
                        name: tool_cfg_clone.name,
                        args,
                    })
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(json!(callback_res.output))
            })
        });

        let mut code_mode_write = code_mode_lock.write().await;
        code_mode_write.add_callback(tool, callback).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorData {
                    code: ErrorCode::Internal,
                    message: format!("Failed to register callback in CodeMode: {e}"),
                    details: None,
                }),
            )
        })?;

        registered += 1;
    }

    Ok(Json(RegisterToolsResponse { registered }))
}

/// Register MCP servers dynamically at runtime
#[utoipa::path(
    post,
    path = "/register/servers",
    tag = "registration",
    request_body = RegisterMcpServersRequest,
    responses(
        (status = 200, description = "MCP servers registration result", body = RegisterMcpServersResponse),
        (status = 500, description = "Internal server error", body = ErrorData)
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
    todo!();

    // // Parse and validate URL
    // let url = url::Url::parse(&server.url).map_err(|e| format!("Invalid URL: {e}"))?;

    // // Create ServerConfig
    // let mut server_config = pctx_config::server::ServerConfig::new(server.name.clone(), url);

    // // Add auth if provided
    // if let Some(auth) = &server.auth {
    //     server_config.auth = serde_json::from_value(auth.clone())
    //         .map_err(|e| format!("Invalid auth config: {e}"))?;
    // }

    // // Connect to MCP server and register tools
    // let mut code_mode = state.code_mode.lock().await;

    // code_mode
    //     .add_server(&server_config)
    //     .await
    //     .map_err(|e| format!("Failed to add MCP server: {e}"))?;

    // info!(
    //     "Successfully registered MCP server '{}' with {} tools",
    //     server.name,
    //     code_mode
    //         .tool_sets
    //         .iter()
    //         .find(|ts| ts.name == server.name)
    //         .map_or(0, |ts| ts.tools.len())
    // );

    Ok(())
}
