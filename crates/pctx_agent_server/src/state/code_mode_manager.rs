//! Session-based `CodeMode` management

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use pctx_code_mode::CodeMode;

/// Manages `CodeMode` instances per session
#[derive(Clone, Default)]
pub struct CodeModeManager {
    /// Map of `session_id` -> `Arc<RwLock<CodeMode>>`
    /// Each `CodeMode` has its own lock for better concurrency
    sessions: Arc<RwLock<HashMap<Uuid, Arc<RwLock<CodeMode>>>>>,
}

impl CodeModeManager {
    /// Get an Arc to the `CodeMode` for a session
    /// This allows you to keep a reference and lock it independently
    pub async fn get(&self, session_id: Uuid) -> Option<Arc<RwLock<CodeMode>>> {
        self.sessions.read().await.get(&session_id).cloned()
    }

    /// Get a cloned copy of the `CodeMode` for a session
    pub async fn get_cloned(&self, session_id: Uuid) -> Option<CodeMode> {
        let sessions = self.sessions.read().await;
        let code_mode_lock = sessions.get(&session_id)?;
        Some(code_mode_lock.read().await.clone())
    }

    /// Set/Insert a `CodeMode` for a session
    pub async fn add(&self, session_id: Uuid, code_mode: CodeMode) {
        let code_mode_lock = Arc::new(RwLock::new(code_mode));
        self.sessions
            .write()
            .await
            .insert(session_id, code_mode_lock);
    }

    /// Delete a `CodeMode` for a session
    /// Returns the `Arc<RwLock<CodeMode>>` if it existed
    pub async fn delete(&self, session_id: Uuid) -> Option<Arc<RwLock<CodeMode>>> {
        self.sessions.write().await.remove(&session_id)
    }

    /// Check if a session has a `CodeMode`
    pub async fn exists(&self, session_id: Uuid) -> bool {
        self.sessions.read().await.contains_key(&session_id)
    }

    /// Get the number of active sessions
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// List all session IDs
    pub async fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.read().await.keys().copied().collect()
    }
}
