mod handler;
/// WebSocket server for PCTX local tools
///
/// Provides a WebSocket endpoint for clients to connect and register local tools
/// that can be called from Deno sandbox code execution.
pub mod protocol;
pub mod session;

pub use handler::WebSocketHandler;
pub use session::{
    CodeExecutorFn, ExecuteCodeError, ExecuteCodeResult, OutgoingMessage, Session, SessionManager,
    SessionManagerExt,
};

use axum::{
    Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// WebSocket server for local tools
pub struct LocalToolsServer {
    session_manager: Arc<SessionManager>,
}

impl LocalToolsServer {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
        }
    }

    /// Create a new server with a code executor
    pub fn with_code_executor(code_executor: CodeExecutorFn) -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new().with_code_executor(code_executor)),
        }
    }

    /// Create a new server with a pre-created session manager
    ///
    /// This is useful when you need to wire up the session manager manually,
    /// such as when integrating with code execution that needs to call back
    /// to WebSocket-registered tools.
    pub fn with_session_manager(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }

    /// Create an Axum router with the WebSocket endpoint
    pub fn router(&self) -> Router {
        Router::new()
            .route("/local-tools", get(websocket_handler))
            .with_state(self.session_manager.clone())
    }

    /// Get the session manager
    pub fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }

    /// Start the server on the given address
    pub async fn serve(self, addr: impl Into<String>) -> Result<(), std::io::Error> {
        let addr = addr.into();
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on {}", addr);

        let app = self.router();
        axum::serve(listener, app).await
    }

    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        self.session_manager.session_count().await
    }

    /// Get list of registered tools
    pub async fn list_tools(&self) -> Vec<String> {
        self.session_manager.list_tools().await
    }
}

impl Default for LocalToolsServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Axum handler for WebSocket upgrade
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(session_manager): State<Arc<SessionManager>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, session_manager))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, session_manager: Arc<SessionManager>) {
    let handler = WebSocketHandler::new(socket, session_manager);

    match handler.run().await {
        Ok(_) => info!("WebSocket connection closed normally"),
        Err(e) => error!("WebSocket connection error: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let server = LocalToolsServer::new();
        assert_eq!(server.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_manager_shared() {
        let server = LocalToolsServer::new();
        let manager1 = server.session_manager();
        let manager2 = server.session_manager();

        // Both should point to the same manager
        assert_eq!(
            manager1.session_count().await,
            manager2.session_count().await
        );
    }
}
