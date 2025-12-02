pub mod rest;
pub mod server;
pub mod session;
pub mod types;
pub mod websocket;

use std::sync::Arc;

use pctx_code_execution_runtime::CallableToolRegistry;
use pctx_code_mode::CodeMode;
use session::{SessionManager, SessionStorage};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub code_mode: Arc<tokio::sync::Mutex<CodeMode>>,
    pub session_storage: Option<Arc<SessionStorage>>,
    pub callable_registry: CallableToolRegistry,
}

impl AppState {
    pub fn new(code_mode: CodeMode) -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            code_mode: Arc::new(tokio::sync::Mutex::new(code_mode)),
            session_storage: None,
            callable_registry: CallableToolRegistry::new(),
        }
    }

    pub fn with_session_storage(mut self, storage: SessionStorage) -> Self {
        self.session_storage = Some(Arc::new(storage));
        self
    }
}

pub use server::start_server;
