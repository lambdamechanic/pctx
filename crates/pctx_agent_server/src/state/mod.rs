use std::sync::Arc;

use crate::state::{code_mode_manager::CodeModeManager, ws_manager::WsManager};

pub(crate) mod code_mode_manager;
pub(crate) mod ws_manager;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub ws_manager: Arc<WsManager>,
    pub code_mode_manager: Arc<CodeModeManager>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ws_manager: Arc::new(WsManager::new()),
            code_mode_manager: Arc::new(CodeModeManager::new()),
        }
    }
}
