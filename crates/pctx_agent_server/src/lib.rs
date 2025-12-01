pub mod rest;
pub mod server;
pub mod types;
pub mod websocket;

use std::sync::Arc;

use pctx_code_mode::CodeMode;
use pctx_session_types::SessionManager;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub code_mode: Arc<tokio::sync::Mutex<CodeMode>>,
}

impl AppState {
    pub fn new(code_mode: CodeMode) -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            code_mode: Arc::new(tokio::sync::Mutex::new(code_mode)),
        }
    }
}

pub use server::start_server;
