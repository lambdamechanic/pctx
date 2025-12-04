use std::sync::Arc;

use crate::state::{code_mode_manager::CodeModeManager, ws_manager::WsManager};

pub mod code_mode_manager;
pub mod ws_manager;

/// Shared application state
#[derive(Clone, Default)]
pub struct AppState {
    pub ws_manager: Arc<WsManager>,
    pub code_mode_manager: Arc<CodeModeManager>,
}
