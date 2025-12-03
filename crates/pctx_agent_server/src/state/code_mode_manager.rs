//! Session-based CodeMode management

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use pctx_code_mode::CodeMode;

/// Manages CodeMode instances per session
#[derive(Clone)]
pub struct CodeModeManager {
    /// Map of session_id -> CodeMode
    sessions: Arc<RwLock<HashMap<Uuid, CodeMode>>>,
}

impl CodeModeManager {
    /// Create a new CodeModeManager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a CodeMode for a session (clones it)
    pub async fn get(&self, session_id: Uuid) -> Option<CodeMode> {
        self.sessions.read().await.get(&session_id).cloned()
    }

    /// Get a reference to a CodeMode for a session with read lock
    /// Useful for read-only operations without cloning
    pub async fn with_read<F, R>(&self, session_id: Uuid, f: F) -> Option<R>
    where
        F: FnOnce(&CodeMode) -> R,
    {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).map(f)
    }

    /// Get a mutable reference to a CodeMode for a session with write lock
    /// Useful for mutations
    pub async fn with_write<F, R>(&self, session_id: Uuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut CodeMode) -> R,
    {
        let mut sessions = self.sessions.write().await;
        sessions.get_mut(&session_id).map(f)
    }

    /// Set/Insert a CodeMode for a session
    pub async fn set(&self, session_id: Uuid, code_mode: CodeMode) {
        self.sessions.write().await.insert(session_id, code_mode);
    }

    /// Update a CodeMode for a session using a closure
    /// Returns true if the session existed and was updated
    pub async fn update<F>(&self, session_id: Uuid, f: F) -> bool
    where
        F: FnOnce(&mut CodeMode),
    {
        let mut sessions = self.sessions.write().await;
        if let Some(code_mode) = sessions.get_mut(&session_id) {
            f(code_mode);
            true
        } else {
            false
        }
    }

    /// Delete a CodeMode for a session
    /// Returns the CodeMode if it existed
    pub async fn delete(&self, session_id: Uuid) -> Option<CodeMode> {
        self.sessions.write().await.remove(&session_id)
    }

    /// Check if a session has a CodeMode
    pub async fn exists(&self, session_id: Uuid) -> bool {
        self.sessions.read().await.contains_key(&session_id)
    }

    /// Get the number of active sessions
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// List all session IDs
    pub async fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.read().await.keys().cloned().collect()
    }
}

impl Default for CodeModeManager {
    fn default() -> Self {
        Self::new()
    }
}
