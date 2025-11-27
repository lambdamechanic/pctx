//! Generic local tool registry for storing tool metadata and callback definitions
//!
//! This module provides a runtime-agnostic registry for user-defined callback tools.
//! The actual callback execution is delegated to the runtime (JavaScript, Python, etc.),
//! while this registry tracks metadata and callback specifications.

use crate::error::McpError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Metadata for a local tool registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolMetadata {
    pub name: String,
    pub description: Option<String>,
    /// JSON Schema for tool input parameters
    pub input_schema: Option<serde_json::Value>,
}

/// A complete local tool definition with callback specification
///
/// This is generic over different runtimes. The callback data can be:
/// - JavaScript: A string containing JS code like "(args) => args.a + args.b"
/// - Python: A string containing Python code or a pickled function
/// - Any other runtime: Whatever format that runtime needs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolDefinition {
    pub metadata: LocalToolMetadata,
    /// Runtime-specific callback data
    /// For JS: JavaScript callback code (e.g., "(args) => args.a + args.b")
    /// For Python: Python callback code or reference
    pub callback_data: String,
}

/// Arguments for calling a local tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallLocalToolArgs {
    pub name: String,
    /// Tool arguments as JSON object
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// Registry for local tool metadata and definitions
///
/// This registry stores:
/// 1. Pre-registered tools (from Rust) with their callback data
/// 2. Runtime-registered tools (from the runtime) - just metadata
pub struct LocalToolRegistry {
    /// Tools registered from Rust (before runtime creation)
    pre_registered: Arc<RwLock<HashMap<String, LocalToolDefinition>>>,
    /// Metadata for all tools (both pre-registered and runtime-registered)
    tools: Arc<RwLock<HashMap<String, LocalToolMetadata>>>,
}

impl std::fmt::Debug for LocalToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalToolRegistry")
            .field(
                "pre_registered_count",
                &self.pre_registered.read().unwrap().len(),
            )
            .field("tools_count", &self.tools.read().unwrap().len())
            .finish()
    }
}

impl LocalToolRegistry {
    pub fn new() -> Self {
        Self {
            pre_registered: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a local tool from Rust (before runtime creation)
    ///
    /// This allows you to pre-register tools with their callback data.
    /// The tools will be automatically registered when the runtime starts.
    ///
    /// # Errors
    ///
    /// Returns an error if a tool with the same name is already registered
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    ///
    /// # Example
    ///
    /// ```rust
    /// use pctx_code_execution_runtime::{LocalToolRegistry, LocalToolDefinition, LocalToolMetadata};
    ///
    /// let registry = LocalToolRegistry::new();
    /// registry.register(LocalToolDefinition {
    ///     metadata: LocalToolMetadata {
    ///         name: "add".to_string(),
    ///         description: Some("Adds two numbers".to_string()),
    ///         input_schema: None,
    ///     },
    ///     callback_data: "(args) => args.a + args.b".to_string(),
    /// }).unwrap();
    /// ```
    pub fn register(&self, definition: LocalToolDefinition) -> Result<(), McpError> {
        let mut pre_registered = self.pre_registered.write().unwrap();
        let mut tools = self.tools.write().unwrap();

        if tools.contains_key(&definition.metadata.name) {
            return Err(McpError::Config(format!(
                "Local tool with name \"{}\" is already registered",
                definition.metadata.name
            )));
        }

        let name = definition.metadata.name.clone();
        tools.insert(name.clone(), definition.metadata.clone());
        pre_registered.insert(name, definition);
        Ok(())
    }

    /// Get all pre-registered tools (for runtime initialization)
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn get_pre_registered(&self) -> Vec<LocalToolDefinition> {
        let pre_registered = self.pre_registered.read().unwrap();
        pre_registered.values().cloned().collect()
    }

    /// Register local tool metadata only (called from runtime)
    ///
    /// # Errors
    ///
    /// Returns an error if a tool with the same name is already registered
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn register_metadata_only(&self, metadata: LocalToolMetadata) -> Result<(), McpError> {
        let mut tools = self.tools.write().unwrap();

        if tools.contains_key(&metadata.name) {
            return Err(McpError::Config(format!(
                "Local tool with name \"{}\" is already registered",
                metadata.name
            )));
        }

        tools.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// Check if a local tool is registered
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn has(&self, name: &str) -> bool {
        let tools = self.tools.read().unwrap();
        tools.contains_key(name)
    }

    /// Get local tool metadata
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn get_metadata(&self, name: &str) -> Option<LocalToolMetadata> {
        let tools = self.tools.read().unwrap();
        tools.get(name).cloned()
    }

    /// List all registered local tools
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn list(&self) -> Vec<LocalToolMetadata> {
        let tools = self.tools.read().unwrap();
        tools.values().cloned().collect()
    }

    /// Delete a local tool
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn delete(&self, name: &str) -> bool {
        let mut tools = self.tools.write().unwrap();
        tools.remove(name).is_some()
    }

    /// Clear all local tools
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn clear(&self) {
        let mut tools = self.tools.write().unwrap();
        tools.clear();
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
            pre_registered: Arc::clone(&self.pre_registered),
            tools: Arc::clone(&self.tools),
        }
    }
}
