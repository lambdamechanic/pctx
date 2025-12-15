use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use pctx_code_mode::CodeMode;
use tokio::sync::RwLock;
use uuid::Uuid;

#[async_trait]
pub trait PctxSessionBackend: Clone + Send + Sync + 'static {
    /// Retrieve a `CodeMode` struct by it's session ID from the backend
    async fn get(&self, session_id: Uuid) -> Option<CodeMode>;

    /// Add a new `CodeMode` struct to the backend
    async fn insert(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()>;

    /// Update a `CodeMode` struct as a full replacement (PUT not PATCH)
    /// in the backend
    async fn update(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()>;

    /// Deletes a `CodeMode` struct from the backend, returning the deleted
    /// instance if it exists.
    async fn delete(&self, session_id: Uuid) -> bool;

    /// Checks if a `CodeMode` struct exists for the given ID
    async fn exists(&self, session_id: Uuid) -> bool;

    /// Returns the number of active `CodeMode` sessions in the backend
    async fn count(&self) -> usize;

    /// Returns a full list of active `CodeMode` sessions in the backend.
    async fn list_sessions(&self) -> Vec<Uuid>;
}

/// Manages `CodeMode` sessions locally using thread-safe
/// smart references and read/write locks
#[derive(Clone, Default)]
pub struct LocalBackend {
    /// Map of `session_id` -> `Arc<RwLock<CodeMode>>`
    /// Each `CodeMode` has its own lock for better concurrency
    sessions: Arc<RwLock<HashMap<Uuid, Arc<RwLock<CodeMode>>>>>,
}

#[async_trait]
impl PctxSessionBackend for LocalBackend {
    async fn get(&self, session_id: Uuid) -> Option<CodeMode> {
        let sessions = self.sessions.read().await;
        let code_mode_lock = sessions.get(&session_id)?;
        Some(code_mode_lock.read().await.clone())
    }

    async fn insert(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()> {
        let code_mode_lock = Arc::new(RwLock::new(code_mode));
        self.sessions
            .write()
            .await
            .insert(session_id, code_mode_lock);

        Ok(())
    }

    async fn update(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()> {
        let sessions = self.sessions.read().await;
        let to_update = sessions
            .get(&session_id)
            .context(format!("CodeMode session {session_id} does not exist"))?;

        *to_update.write().await = code_mode;

        Ok(())
    }

    async fn delete(&self, session_id: Uuid) -> bool {
        let deleted = self.sessions.write().await.remove(&session_id);
        deleted.is_some()
    }

    async fn exists(&self, session_id: Uuid) -> bool {
        self.sessions.read().await.contains_key(&session_id)
    }

    async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }

    async fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.read().await.keys().copied().collect()
    }
}
