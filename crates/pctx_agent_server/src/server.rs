use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{AppState, rest, types::*, websocket};

#[derive(OpenApi)]
#[openapi(
    paths(
        rest::health,
        rest::list_tools,
        rest::get_function_details,
        rest::execute_code,
        rest::register_local_tools,
        rest::register_mcp_servers,
    ),
    components(
        schemas(
            HealthResponse,
            ListToolsRequest,
            ListToolsResponse,
            ToolInfo,
            ToolSource,
            GetFunctionDetailsRequest,
            GetFunctionDetailsResponse,
            ExecuteCodeRequest,
            ExecuteCodeResponse,
            RegisterLocalToolsRequest,
            LocalToolDefinition,
            RegisterLocalToolsResponse,
            RegisterMcpServersRequest,
            McpServerConfig,
            RegisterMcpServersResponse,
            ErrorResponse,
            ErrorInfo,
        )
    ),
    tags(
        (name = "tools", description = "Tool management and execution endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    info(
        title = "PCTX Agent Server API",
        version = "0.1.0",
        description = "REST API for PCTX agent mode - dynamic tool and MCP server registration with code execution",
    )
)]
pub struct ApiDoc;

/// Start the agent server
pub async fn start_server(host: &str, port: u16, state: AppState) -> Result<()> {
    let app = create_router(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🚀 PCTX Agent Server listening on http://{}", addr);
    info!("   OpenAPI documentation: http://{}/swagger-ui/", addr);
    info!("");
    info!("Use REST API to register tools and MCP servers dynamically.");
    info!(
        "WebSocket endpoint at ws://{}/ws for local tool callbacks.",
        addr
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Create the Axum router with all routes
fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(rest::health))
        // Tools endpoints
        .route("/tools/list", post(rest::list_tools))
        .route("/tools/details", post(rest::get_function_details))
        .route("/tools/execute", post(rest::execute_code))
        .route("/tools/local/register", post(rest::register_local_tools))
        .route("/tools/mcp/register", post(rest::register_mcp_servers))
        // WebSocket endpoint
        .route("/ws", get(websocket::ws_handler))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add state
        .with_state(state)
        // Add middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, cleaning up...");
}
