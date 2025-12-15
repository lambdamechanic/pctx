use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use pctx_code_mode::CodeMode;
use tokio::sync::RwLock;
use uuid::Uuid;

pub trait PctxSessionBackend {
    /// Retrieve a `CodeMode` struct by it's session ID from the backend
    async fn get(&self, session_id: Uuid) -> Option<CodeMode>;

    /// Add a new `CodeMode` struct to the backend
    async fn insert(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()>;

    /// Update a `CodeMode` struct as a full replacement (PUT not PATCH)
    /// in the backend
    async fn update(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()>;

    /// Deletes a `CodeMode` struct from the backend, returning the deleted
    /// instance if it exists.
    async fn delete(&self, session_id: Uuid) -> Option<CodeMode>;

    /// Checks if a `CodeMode` struct exists for the given ID
    async fn exists(&self, session_id: Uuid) -> bool;

    /// Returns the number of active `CodeMode` sessions in the backend
    async fn count(&self, session_id: Uuid) -> usize;

    /// Returns a full list of active `CodeMode` sessions in the backend.
    async fn list_sessions(&self) -> Vec<Uuid>;
}

/// Manages `CodeMode` sessions locally using thread-safe
/// smart references and read/write locks
pub struct LocalCodeModeBackend {
    /// Map of `session_id` -> `Arc<RwLock<CodeMode>>`
    /// Each `CodeMode` has its own lock for better concurrency
    sessions: Arc<RwLock<HashMap<Uuid, Arc<RwLock<CodeMode>>>>>,
}

impl PctxSessionBackend for LocalCodeModeBackend {
    async fn get(&self, session_id: Uuid) -> Option<CodeMode> {
        todo!()
    }

    async fn insert(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()> {
        todo!()
    }

    async fn update(&self, session_id: Uuid, code_mode: CodeMode) -> Result<()> {
        todo!()
    }

    async fn delete(&self, session_id: Uuid) -> Option<CodeMode> {
        todo!()
    }

    async fn exists(&self, session_id: Uuid) -> bool {
        todo!()
    }

    async fn count(&self, session_id: Uuid) -> usize {
        todo!()
    }

    async fn list_sessions(&self) -> Vec<Uuid> {
        todo!()
    }
}
