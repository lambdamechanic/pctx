//! Generic local tool registry for storing tool metadata and callback definitions
//!
//! This module provides a runtime-agnostic registry for user-defined callback tools.
//! Callbacks are stored as Rust closures, regardless of their source language (Python, Node.js, etc.).
//!
//! ## Architecture: Unified Host Callbacks
//!
//! PCTX supports local tool callbacks from multiple host environments:
//!
//! ### Host Python Callbacks (via `PyO3`)
//! - **Execution**: Host Python interpreter
//! - **Storage**: `callbacks` `HashMap` (as Rust closures)
//! - **Registration**: `pctx_python_runtime::PythonCallbackRegistry::register_callable()`
//! - **Use case**: Python-based local tools from Python SDK
//!
//! ### Host Node.js Callbacks (via napi-rs - future)
//! - **Execution**: Host Node.js environment
//! - **Storage**: `callbacks` `HashMap` (as Rust closures)
//! - **Registration**: `pctx_nodejs_bindings::wrap_nodejs_callback()`
//! - **Use case**: JavaScript-based local tools from TypeScript SDK
//!
//! ## Unified Callback Storage
//!
//! All callbacks are stored as the same Rust closure type:
//! ```rust
//! use std::sync::Arc;
//! pub type LocalToolCallback = Arc<
//!     dyn Fn(Option<serde_json::Value>) -> Result<serde_json::Value, String>
//!     + Send + Sync
//! >;
//! ```
//!
//! This unified type allows tools from different languages (Python, Node.js, Rust)
//! to be stored together and called through the same interface.

use crate::error::McpError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A callable function that executes a local tool
///
/// This is a Rust closure that wraps any kind of callback (Python, Node.js, Rust native, etc.).
/// All language-specific FFI complexity is hidden inside the closure.
///
/// # Arguments
/// * Input: `Option<serde_json::Value>` - Tool arguments as JSON
/// * Output: `Result<serde_json::Value, String>` - Tool result or error message
pub type LocalToolCallback =
    Arc<dyn Fn(Option<serde_json::Value>) -> Result<serde_json::Value, String> + Send + Sync>;

/// Metadata for a local tool registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolMetadata {
    pub name: String,
    pub description: Option<String>,
    /// JSON Schema for tool input parameters
    pub input_schema: Option<serde_json::Value>,
    /// Namespace for this tool (e.g., "math", "`db_connections`")
    pub namespace: String,
}

/// Arguments for calling a local tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallLocalToolArgs {
    pub name: String,
    /// Tool arguments as JSON object
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// Registry for local tool metadata and callbacks
///
/// This registry stores all local tool callbacks as Rust closures, regardless of their
/// source language (Python, Node.js, Rust, etc.). Each callback is wrapped using
/// language-specific FFI (`PyO3` for Python, napi-rs for Node.js) into a unified
/// `LocalToolCallback` type.
///
/// ## Storage
/// - **metadata**: Tool metadata (name, description, schema, namespace)
/// - **callbacks**: Rust closures that execute the tools (unified interface for all languages)
pub struct LocalToolRegistry {
    /// Metadata for all registered tools
    metadata: Arc<RwLock<HashMap<String, LocalToolMetadata>>>,
    /// Unified callback storage (Python, Node.js, Rust, etc. all stored as Rust closures)
    callbacks: Arc<RwLock<HashMap<String, LocalToolCallback>>>,
}

impl std::fmt::Debug for LocalToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalToolRegistry")
            .field("tool_count", &self.metadata.read().unwrap().len())
            .field("callback_count", &self.callbacks.read().unwrap().len())
            .finish()
    }
}

impl LocalToolRegistry {
    pub fn new() -> Self {
        Self {
            metadata: Arc::new(RwLock::new(HashMap::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a local tool with a Rust callback (NEW UNIFIED API)
    ///
    /// This is the primary registration method. The callback can wrap:
    /// - Python functions (via `PyO3`)
    /// - JavaScript functions (via Deno ops)
    /// - Native Rust code
    /// - Anything else that can be called from Rust
    ///
    /// # Arguments
    /// * `metadata` - Tool metadata (name, description, schema, namespace)
    /// * `callback` - Rust closure that executes the tool
    ///
    /// # Errors
    /// Returns an error if a tool with the same name is already registered
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned
    ///
    /// # Example
    /// ```rust,no_run
    /// use pctx_code_execution_runtime::{LocalToolRegistry, LocalToolMetadata, LocalToolCallback};
    /// use std::sync::Arc;
    ///
    /// let registry = LocalToolRegistry::new();
    ///
    /// // Register a simple Rust callback
    /// let callback: LocalToolCallback = Arc::new(|args| {
    ///     let a = args.as_ref().and_then(|v| v["a"].as_i64()).unwrap_or(0);
    ///     let b = args.as_ref().and_then(|v| v["b"].as_i64()).unwrap_or(0);
    ///     Ok(serde_json::json!(a + b))
    /// });
    ///
    /// registry.register_callback(
    ///     LocalToolMetadata {
    ///         name: "add".to_string(),
    ///         description: Some("Adds two numbers".to_string()),
    ///         input_schema: None,
    ///         namespace: "math".to_string(),
    ///     },
    ///     callback,
    /// ).unwrap();
    /// ```
    pub fn register_callback(
        &self,
        metadata: LocalToolMetadata,
        callback: LocalToolCallback,
    ) -> Result<(), McpError> {
        let mut metadata_map = self.metadata.write().unwrap();
        let mut callbacks = self.callbacks.write().unwrap();

        if metadata_map.contains_key(&metadata.name) {
            return Err(McpError::Config(format!(
                "Local tool with name \"{}\" is already registered",
                metadata.name
            )));
        }

        let name = metadata.name.clone();
        metadata_map.insert(name.clone(), metadata);
        callbacks.insert(name, callback);

        Ok(())
    }

    /// Execute a registered tool by name
    ///
    /// # Arguments
    /// * `name` - Name of the tool to execute
    /// * `args` - Optional JSON arguments
    ///
    /// # Returns
    /// The tool's result as JSON, or an error message
    ///
    /// # Errors
    /// Returns an error if the tool is not found or the callback fails
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned
    pub fn execute(
        &self,
        name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let callbacks = self.callbacks.read().unwrap();

        let callback = callbacks
            .get(name)
            .ok_or_else(|| format!("Tool '{name}' not found"))?;

        callback(args)
    }

    /// Check if a local tool is registered
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn has(&self, name: &str) -> bool {
        let metadata_map = self.metadata.read().unwrap();
        metadata_map.contains_key(name)
    }

    /// Get local tool metadata
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn get_metadata(&self, name: &str) -> Option<LocalToolMetadata> {
        let metadata_map = self.metadata.read().unwrap();
        metadata_map.get(name).cloned()
    }

    /// List all registered local tools
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn list(&self) -> Vec<LocalToolMetadata> {
        let metadata_map = self.metadata.read().unwrap();
        metadata_map.values().cloned().collect()
    }

    /// Delete a local tool
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn delete(&self, name: &str) -> bool {
        let mut metadata_map = self.metadata.write().unwrap();
        let mut callbacks = self.callbacks.write().unwrap();

        let removed_metadata = metadata_map.remove(name).is_some();
        callbacks.remove(name);
        removed_metadata
    }

    /// Clear all local tools
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn clear(&self) {
        let mut metadata_map = self.metadata.write().unwrap();
        let mut callbacks = self.callbacks.write().unwrap();

        metadata_map.clear();
        callbacks.clear();
    }
}

impl Default for LocalToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LocalToolRegistry {
    fn clone(&self) -> Self {
        Self {
            metadata: Arc::clone(&self.metadata),
            callbacks: Arc::clone(&self.callbacks),
        }
    }
}
