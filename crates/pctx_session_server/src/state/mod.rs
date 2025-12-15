use std::sync::Arc;

use crate::{
    LocalBackend,
    state::{backend::PctxSessionBackend, ws_manager::WsManager},
};

pub(crate) mod backend;
pub(crate) mod ws_manager;

/// Shared application state
#[derive(Clone)]
pub struct AppState<B: PctxSessionBackend> {
    pub ws_manager: Arc<WsManager>,
    pub backend: Arc<B>,
}

impl<B: PctxSessionBackend> AppState<B> {
    pub fn new(backend: B) -> Self {
        Self {
            ws_manager: Arc::default(),
            backend: Arc::new(backend),
        }
    }
}

impl AppState<LocalBackend> {
    pub fn new_local() -> Self {
        Self {
            ws_manager: Arc::default(),
            backend: Arc::new(LocalBackend::default()),
        }
    }
}
