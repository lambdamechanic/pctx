//! JavaScript local tool registry for storing tool metadata
//!
//! This module provides a registry for user-defined JavaScript callback tools.
//! The actual JavaScript callbacks are stored on the JS side, while this registry only tracks metadata.

use crate::error::McpError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Metadata for a JS local tool registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsLocalToolMetadata {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// JSON Schema for tool input parameters
    pub input_schema: Option<serde_json::Value>,
}

/// A complete local tool definition with JavaScript callback code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsLocalToolDefinition {
    /// Tool metadata
    pub metadata: JsLocalToolMetadata,
    /// JavaScript callback code (e.g., "(args) => args.a + args.b")
    pub callback_code: String,
}

/// Arguments for calling a JS local tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallJsLocalToolArgs {
    /// Tool name
    pub name: String,
    /// Tool arguments as JSON object
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// Registry for JS local tool metadata and definitions
///
/// This registry stores:
/// 1. Pre-registered tools (from Rust) with their JavaScript callback code
/// 2. Runtime-registered tools (from JS) - just metadata
pub struct JsLocalToolRegistry {
    /// Tools registered from Rust (before runtime creation)
    pre_registered: Arc<RwLock<HashMap<String, JsLocalToolDefinition>>>,
    /// Metadata for all tools (both pre-registered and runtime-registered)
    tools: Arc<RwLock<HashMap<String, JsLocalToolMetadata>>>,
}

impl JsLocalToolRegistry {
    pub fn new() -> Self {
        Self {
            pre_registered: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a local tool from Rust (before runtime creation)
    ///
    /// This allows you to pre-register tools with their JavaScript callback code.
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
    /// use pctx_code_execution_runtime::{JsLocalToolRegistry, JsLocalToolDefinition, JsLocalToolMetadata};
    ///
    /// let registry = JsLocalToolRegistry::new();
    /// registry.register(JsLocalToolDefinition {
    ///     metadata: JsLocalToolMetadata {
    ///         name: "add".to_string(),
    ///         description: Some("Adds two numbers".to_string()),
    ///         input_schema: None,
    ///     },
    ///     callback_code: "(args) => args.a + args.b".to_string(),
    /// }).unwrap();
    /// ```
    pub fn register(&self, definition: JsLocalToolDefinition) -> Result<(), McpError> {
        let mut pre_registered = self.pre_registered.write().unwrap();
        let mut tools = self.tools.write().unwrap();

        if tools.contains_key(&definition.metadata.name) {
            return Err(McpError::Config(format!(
                "JS local tool with name \"{}\" is already registered",
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
    pub fn get_pre_registered(&self) -> Vec<JsLocalToolDefinition> {
        let pre_registered = self.pre_registered.read().unwrap();
        pre_registered.values().cloned().collect()
    }

    /// Register JS local tool metadata
    ///
    /// # Errors
    ///
    /// Returns an error if a tool with the same name is already registered
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn register_metadata_only(&self, metadata: JsLocalToolMetadata) -> Result<(), McpError> {
        let mut tools = self.tools.write().unwrap();

        if tools.contains_key(&metadata.name) {
            return Err(McpError::Config(format!(
                "JS local tool with name \"{}\" is already registered",
                metadata.name
            )));
        }

        tools.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// Check if a JS local tool is registered
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn has(&self, name: &str) -> bool {
        let tools = self.tools.read().unwrap();
        tools.contains_key(name)
    }

    /// Get JS local tool metadata
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn get_metadata(&self, name: &str) -> Option<JsLocalToolMetadata> {
        let tools = self.tools.read().unwrap();
        tools.get(name).cloned()
    }

    /// List all registered JS local tools
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn list(&self) -> Vec<JsLocalToolMetadata> {
        let tools = self.tools.read().unwrap();
        tools.values().cloned().collect()
    }

    /// Delete a JS local tool
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn delete(&self, name: &str) -> bool {
        let mut tools = self.tools.write().unwrap();
        tools.remove(name).is_some()
    }

    /// Clear all JS local tools
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned
    pub fn clear(&self) {
        let mut tools = self.tools.write().unwrap();
        tools.clear();
    }
}

impl Default for JsLocalToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for JsLocalToolRegistry {
    fn clone(&self) -> Self {
        Self {
            pre_registered: Arc::clone(&self.pre_registered),
            tools: Arc::clone(&self.tools),
        }
    }
}
