use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use crate::error::McpError;

pub type CallbackFn = Arc<
    dyn Fn(
            Option<serde_json::Value>,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;

/// Singleton registry for callbacks
#[derive(Clone, Default)]
pub struct CallbackRegistry {
    callbacks: Arc<RwLock<HashMap<String, CallbackFn>>>,
}

impl CallbackRegistry {
    /// Returns the ids of this [`CallbackRegistry`].
    ///
    /// # Panics
    ///
    /// Panics if it fails acquiring the lock
    pub fn ids(&self) -> Vec<String> {
        self.callbacks
            .read()
            .unwrap()
            .keys()
            .map(String::from)
            .collect()
    }

    /// Adds callback to registry
    ///
    /// # Panics
    ///
    /// Panics if cannot obtain lock
    ///
    /// # Errors
    ///
    /// This function will return an error if a callback already exists with the same ID
    pub fn add(
        &self,
        id: &str, // namespace.name
        callback: CallbackFn,
    ) -> Result<(), McpError> {
        let mut callbacks = self.callbacks.write().unwrap();

        if callbacks.contains_key(id) {
            return Err(McpError::Config(format!(
                "Callback with id \"{id}\" is already registered"
            )));
        }

        callbacks.insert(id.into(), callback);

        Ok(())
    }

    /// Remove a callback from the registry by id
    ///
    /// # Panics
    ///
    /// Panics if cannot obtain lock
    pub fn remove(&self, id: &str) -> Option<CallbackFn> {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.remove(id)
    }

    /// Get a Callback from the registry by id
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (i.e., a thread panicked while holding the lock)
    pub fn get(&self, id: &str) -> Option<CallbackFn> {
        let callbacks = self.callbacks.read().unwrap();
        callbacks.get(id).cloned()
    }

    /// Confirms the callback registry contains a given id
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (i.e., a thread panicked while holding the lock)
    pub fn has(&self, id: &str) -> bool {
        let callbacks = self.callbacks.read().unwrap();
        callbacks.contains_key(id)
    }

    /// invokes the callback with the provided args
    ///
    /// # Errors
    ///
    /// This function will return an error if a callback by the provided id doesn't exist
    /// or if the callback itself fails
    pub async fn invoke(
        &self,
        id: &str,
        args: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        let callback = self.get(id).ok_or_else(|| {
            McpError::ToolCall(format!("Callback with id \"{id}\" does not exist"))
        })?;

        callback(args).await.map_err(|e| {
            McpError::ExecutionError(format!("Failed calling callback with id \"{id}\": {e}",))
        })
    }
}
