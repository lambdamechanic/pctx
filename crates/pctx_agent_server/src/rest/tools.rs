use std::sync::Arc;
use std::time::Instant;

use crate::session::{OutgoingMessage, ToolCallRecord};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use pctx_code_mode::model::{FunctionId, GetFunctionDetailsInput, GetFunctionDetailsOutput};
use tracing::{error, info};
use utoipa;
use uuid::Uuid;

use crate::AppState;
use crate::types::*;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// List all available tools from both local and MCP registrations
#[utoipa::path(
    post,
    path = "/tools/list",
    tag = "tools",
    request_body = ListToolsRequest,
    responses(
        (status = 200, description = "List of all registered tools", body = ListToolsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn list_tools(
    State(state): State<AppState>,
    Json(_request): Json<ListToolsRequest>,
) -> Result<Json<ListToolsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Listing tools");

    // Convert MCP tools to ToolInfo
    let mut tools: Vec<ToolInfo> = {
        let code_mode = state.code_mode.lock().await;
        let output = code_mode.list_functions();

        output
            .functions
            .iter()
            .map(|f| ToolInfo {
                namespace: f.namespace.clone(),
                name: f.name.clone(),
                description: f.description.clone().unwrap_or_default(),
                source: ToolSource::Mcp,
            })
            .collect()
    }; // MutexGuard dropped here

    // Add local tools registered via WebSocket/REST
    let local_tools = state.session_manager.list_tools().await;
    for tool_name in local_tools {
        if let Some((namespace, name)) = tool_name.split_once('.') {
            tools.push(ToolInfo {
                namespace: namespace.to_string(),
                name: name.to_string(),
                description: String::new(), // We don't store descriptions in SessionManager yet
                source: ToolSource::Local,
            });
        }
    }

    Ok(Json(ListToolsResponse { tools }))
}

/// Get detailed information about a specific function
#[utoipa::path(
    post,
    path = "/tools/details",
    tag = "tools",
    request_body = GetFunctionDetailsRequest,
    responses(
        (status = 200, description = "Function details", body = GetFunctionDetailsOutput),
        (status = 404, description = "Function not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_function_details(
    State(state): State<AppState>,
    Json(request): Json<GetFunctionDetailsRequest>,
) -> Result<Json<GetFunctionDetailsOutput>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Getting function details for {}.{}",
        request.namespace, request.name
    );

    let code_mode = state.code_mode.lock().await;
    let input = GetFunctionDetailsInput {
        functions: vec![FunctionId {
            mod_name: request.namespace.clone(),
            fn_name: request.name.clone(),
        }],
    };
    let output = code_mode.get_function_details(input);

    // Check if we got the function
    if output.functions.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorInfo {
                    code: "NOT_FOUND".to_string(),
                    message: format!("Function not found: {}.{}", request.namespace, request.name),
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
    path = "/tools/execute",
    tag = "tools",
    request_body = ExecuteCodeRequest,
    responses(
        (status = 200, description = "Code executed successfully", body = ExecuteCodeResponse),
        (status = 400, description = "Code execution failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn execute_code(
    State(state): State<AppState>,
    Json(request): Json<ExecuteCodeRequest>,
) -> Result<Json<ExecuteCodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Executing code (timeout: {}ms)", request.timeout_ms);

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

    // Return full execution output including stdout/stderr
    Ok(Json(ExecuteCodeResponse {
        success: output.success,
        stdout: output.stdout,
        stderr: output.stderr,
        output: output.output,
        execution_time_ms,
    }))
}

/// Register local tools that will be called via WebSocket callbacks
#[utoipa::path(
    post,
    path = "/tools/local/register",
    tag = "tools",
    request_body = RegisterLocalToolsRequest,
    responses(
        (status = 200, description = "Tools registered successfully", body = RegisterLocalToolsResponse),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn register_local_tools(
    State(state): State<AppState>,
    Json(request): Json<RegisterLocalToolsRequest>,
) -> Result<Json<RegisterLocalToolsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Registering {} local tools for session {}",
        request.tools.len(),
        request.session_id
    );

    let mut registered = 0;
    for tool in &request.tools {
        let tool_name = format!("{}.{}", tool.namespace, tool.name);

        // Create callback closure that captures session state
        let session_manager_clone = Arc::clone(&state.session_manager);
        let session_storage_clone = state.session_storage.clone();
        let session_id_clone = request.session_id.clone();
        let tool_name_clone = tool_name.clone();

        use pctx_code_execution_runtime::CallbackFn;

        let callback: CallbackFn = Arc::new(move |args: Option<serde_json::Value>| {
            let session_manager_clone = session_manager_clone.clone();
            let session_storage_clone = session_storage_clone.clone();
            let session_id_clone = session_id_clone.clone();
            let tool_name_clone = tool_name_clone.clone();

            Box::pin(async move {
                let start_time = chrono::Utc::now().timestamp_millis();
                let request_id = Uuid::new_v4().to_string();

                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "execute_tool",
                    "params": {
                        "name": tool_name_clone,
                        "arguments": args
                    },
                    "id": request_id.clone()
                });

                let result = session_manager_clone
                    .execute_tool_raw(
                        &tool_name_clone,
                        OutgoingMessage::Response(request),
                        serde_json::Value::String(request_id),
                    )
                    .await
                    .map_err(|e| e.to_string());

                // Record tool call in session storage
                if let Some(storage) = &session_storage_clone {
                    let (namespace, tool_name_part) = tool_name_clone
                        .split_once('.')
                        .unwrap_or(("", &tool_name_clone));
                    let tool_call_record = ToolCallRecord {
                        session_id: session_id_clone.clone(),
                        timestamp: start_time,
                        tool_name: tool_name_part.to_string(),
                        namespace: namespace.to_string(),
                        arguments: args.clone().unwrap_or(serde_json::Value::Null),
                        result: result.as_ref().ok().cloned(),
                        error: result.as_ref().err().cloned(),
                        code_snippet: None,
                    };

                    if let Ok(mut history) = storage.load_session(&session_id_clone) {
                        history.add_tool_call(tool_call_record);
                        let _ = storage.save_session(&history);
                    }
                }

                result
            })
        });

        // Register callback in CallbackRegistry
        state
            .callback_registry
            .add(&tool_name, callback)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: ErrorInfo {
                            code: "INTERNAL_ERROR".to_string(),
                            message: format!("Failed to register callback: {}", e),
                            details: None,
                        },
                    }),
                )
            })?;

        // Register with session_manager for tracking
        state
            .session_manager
            .register_tool(
                &request.session_id,
                tool_name.clone(),
                Some(tool.description.clone()),
            )
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: ErrorInfo {
                            code: "INTERNAL_ERROR".to_string(),
                            message: format!("Failed to register tool with session: {}", e),
                            details: None,
                        },
                    }),
                )
            })?;

        // Track tool registration in session history
        if let Some(storage) = &state.session_storage {
            if let Ok(mut history) = storage.load_session(&request.session_id) {
                history.add_tool(tool_name);
                let _ = storage.save_session(&history);
            }
        }

        registered += 1;
    }

    Ok(Json(RegisterLocalToolsResponse { registered }))
}

/// Register MCP servers dynamically at runtime
#[utoipa::path(
    post,
    path = "/tools/mcp/register",
    tag = "tools",
    request_body = RegisterMcpServersRequest,
    responses(
        (status = 200, description = "MCP servers registration result", body = RegisterMcpServersResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn register_mcp_servers(
    State(state): State<AppState>,
    Json(request): Json<RegisterMcpServersRequest>,
) -> Json<RegisterMcpServersResponse> {
    info!("Registering {} MCP servers", request.servers.len());

    let mut registered = 0;
    let mut failed = Vec::new();

    for server in &request.servers {
        match register_mcp_server(&state, server).await {
            Ok(_) => {
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
    let url = url::Url::parse(&server.url).map_err(|e| format!("Invalid URL: {}", e))?;

    // Create ServerConfig
    let mut server_config = pctx_config::server::ServerConfig::new(server.name.clone(), url);

    // Add auth if provided
    if let Some(auth) = &server.auth {
        server_config.auth = serde_json::from_value(auth.clone())
            .map_err(|e| format!("Invalid auth config: {}", e))?;
    }

    // Connect to MCP server and register tools
    let mut code_mode = state.code_mode.lock().await;

    code_mode
        .add_server(&server_config)
        .await
        .map_err(|e| format!("Failed to add MCP server: {}", e))?;

    info!(
        "Successfully registered MCP server '{}' with {} tools",
        server.name,
        code_mode
            .tool_sets
            .iter()
            .find(|ts| ts.name == server.name)
            .map(|ts| ts.tools.len())
            .unwrap_or(0)
    );

    Ok(())
}

/// API error type
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Internal(String),
    ExecutionError(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
            ApiError::ExecutionError(msg) => (StatusCode::BAD_REQUEST, "EXECUTION_ERROR", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
        };

        let body = Json(ErrorResponse {
            error: ErrorInfo {
                code: code.to_string(),
                message,
                details: None,
            },
        });

        (status, body).into_response()
    }
}
