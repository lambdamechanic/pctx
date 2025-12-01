//! # PCTX Session Types
//!
//! Core types for WebSocket session management and code execution.
//!
//! This crate provides the foundational types used by both the WebSocket server
//! and the code execution runtime, preventing circular dependencies.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{RwLock, mpsc as tokio_mpsc};
use uuid::Uuid;

/// Unique identifier for a WebSocket session
pub type SessionId = String;

/// Messages that can be sent to a WebSocket client
#[derive(Debug, Clone)]
pub enum OutgoingMessage {
    /// JSON-RPC response
    Response(serde_json::Value),
    /// JSON-RPC notification
    Notification(serde_json::Value),
}

/// WebSocket session representing a connected client
pub struct Session {
    pub id: SessionId,
    /// Channel to send messages to the client
    pub sender: tokio_mpsc::UnboundedSender<OutgoingMessage>,
    /// Tools registered by this session
    pub registered_tools: HashSet<String>, // namespace.name format
}

impl Session {
    pub fn new(sender: tokio_mpsc::UnboundedSender<OutgoingMessage>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender,
            registered_tools: HashSet::new(),
        }
    }

    pub fn with_id(id: String, sender: tokio_mpsc::UnboundedSender<OutgoingMessage>) -> Self {
        Self {
            id,
            sender,
            registered_tools: HashSet::new(),
        }
    }

    /// Register a tool for this session
    pub fn register_tool(&mut self, tool_name: String) {
        self.registered_tools.insert(tool_name);
    }

    /// Unregister a tool from this session
    pub fn unregister_tool(&mut self, tool_name: &str) -> bool {
        self.registered_tools.remove(tool_name)
    }

    /// Check if a tool is registered by this session
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.registered_tools.contains(tool_name)
    }
}

/// Pending execution request waiting for response from client
pub struct PendingExecution {
    pub tool_name: String,
    pub response_tx: std::sync::mpsc::Sender<Result<serde_json::Value, String>>,
}

/// Result of code execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecuteCodeResult {
    pub success: bool,
    pub value: Option<serde_json::Value>,
    pub stdout: String,
    pub stderr: String,
}

/// Error type for code execution
#[derive(Debug, thiserror::Error)]
pub enum ExecuteCodeError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("No code executor configured")]
    NoExecutor,
}

/// Callback for executing code - takes code string, returns execution result
pub type CodeExecutorFn = Arc<
    dyn Fn(
            String,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<ExecuteCodeResult, ExecuteCodeError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

/// Error types for session operations
#[derive(Debug, thiserror::Error)]
pub enum RegisterToolError {
    #[error("Tool already registered")]
    AlreadyRegistered,
    #[error("Session not found")]
    SessionNotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("Session not found")]
    SessionNotFound,
    #[error("Channel closed")]
    ChannelClosed,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteToolError {
    #[error("Tool not found")]
    ToolNotFound,
    #[error("Failed to send execution request")]
    SendFailed,
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Response channel closed")]
    ChannelClosed,
    #[error("Execution timeout")]
    Timeout,
}

/// Manager for all WebSocket sessions
///
/// This is the core session management type that coordinates between
/// WebSocket clients and code execution.
pub struct SessionManager {
    /// Active sessions by ID
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    /// Tool name → session ID mapping
    tool_sessions: Arc<RwLock<HashMap<String, SessionId>>>,
    /// Request ID → pending execution mapping
    pending_executions: Arc<RwLock<HashMap<serde_json::Value, PendingExecution>>>,
    /// Code execution callback (optional) - uses std::sync::RwLock for synchronous setup
    code_executor: Arc<StdRwLock<Option<CodeExecutorFn>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tool_sessions: Arc::new(RwLock::new(HashMap::new())),
            pending_executions: Arc::new(RwLock::new(HashMap::new())),
            code_executor: Arc::new(StdRwLock::new(None)),
        }
    }

    pub fn with_code_executor(self, executor: CodeExecutorFn) -> Self {
        *self.code_executor.write().unwrap() = Some(executor);
        self
    }

    /// Set the code executor (for setting after construction)
    pub fn set_code_executor(&self, executor: CodeExecutorFn) {
        *self.code_executor.write().unwrap() = Some(executor);
    }

    /// Add a new session
    pub async fn add_session(&self, session: Session) -> SessionId {
        let session_id = session.id.clone();
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    /// Remove a session and clean up all its registered tools
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            // Clean up tool mappings
            let mut tool_sessions = self.tool_sessions.write().await;
            for tool_name in &session.registered_tools {
                tool_sessions.remove(tool_name);
            }
        }
    }

    /// Register a tool for a session
    pub async fn register_tool(
        &self,
        session_id: &str,
        tool_name: String,
        _description: Option<String>,
    ) -> Result<(), RegisterToolError> {
        // Check if tool already exists
        let tool_sessions = self.tool_sessions.read().await;
        if tool_sessions.contains_key(&tool_name) {
            return Err(RegisterToolError::AlreadyRegistered);
        }
        drop(tool_sessions);

        // Add to session
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or(RegisterToolError::SessionNotFound)?;
        session.register_tool(tool_name.clone());
        drop(sessions);

        // Add to tool mapping
        let mut tool_sessions = self.tool_sessions.write().await;
        tool_sessions.insert(tool_name, session_id.to_string());

        Ok(())
    }

    /// Get the session ID for a tool
    pub async fn get_tool_session(&self, tool_name: &str) -> Option<SessionId> {
        self.tool_sessions.read().await.get(tool_name).cloned()
    }

    /// Send a message to a specific session
    pub async fn send_to_session(
        &self,
        session_id: &str,
        message: OutgoingMessage,
    ) -> Result<(), SendError> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id).ok_or(SendError::SessionNotFound)?;
        session
            .sender
            .send(message)
            .map_err(|_| SendError::ChannelClosed)?;
        Ok(())
    }

    /// Handle a response from a client for a pending execution
    pub async fn handle_execution_response(
        &self,
        request_id: &serde_json::Value,
        result: Result<serde_json::Value, String>,
    ) -> Result<(), ()> {
        eprintln!(
            "[SessionManager] Handling execution response for request_id: {:?}",
            request_id
        );
        let mut pending = self.pending_executions.write().await;
        eprintln!(
            "[SessionManager] Pending executions count: {}",
            pending.len()
        );
        if let Some(execution) = pending.remove(request_id) {
            eprintln!("[SessionManager] Found pending execution, sending result");
            let send_result = execution.response_tx.send(result);
            eprintln!("[SessionManager] mpsc send result: {:?}", send_result);
            Ok(())
        } else {
            eprintln!(
                "[SessionManager] No pending execution found for request_id: {:?}",
                request_id
            );
            Err(())
        }
    }

    /// Get list of all registered tools
    pub async fn list_tools(&self) -> Vec<String> {
        self.tool_sessions.read().await.keys().cloned().collect()
    }

    /// Get number of active sessions
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Get access to pending executions (for WebSocket server use)
    pub fn pending_executions(&self) -> &Arc<RwLock<HashMap<serde_json::Value, PendingExecution>>> {
        &self.pending_executions
    }

    /// Get access to code executor (for WebSocket server use)
    pub fn code_executor(&self) -> &Arc<StdRwLock<Option<CodeExecutorFn>>> {
        &self.code_executor
    }

    /// Get access to sessions (for WebSocket server use)
    pub fn sessions(&self) -> &Arc<RwLock<HashMap<SessionId, Session>>> {
        &self.sessions
    }

    /// Get access to tool sessions (for WebSocket server use)
    pub fn tool_sessions(&self) -> &Arc<RwLock<HashMap<String, SessionId>>> {
        &self.tool_sessions
    }

    /// Execute a tool and wait for response
    ///
    /// This is a low-level method that sends a raw response message and waits for the result.
    /// Higher-level wrappers in pctx_websocket_server may provide better ergonomics.
    pub async fn execute_tool_raw(
        &self,
        tool_name: &str,
        message: OutgoingMessage,
        request_id: serde_json::Value,
    ) -> Result<serde_json::Value, ExecuteToolError> {
        // Find session for tool
        let session_id = self
            .get_tool_session(tool_name)
            .await
            .ok_or(ExecuteToolError::ToolNotFound)?;

        // Create std::sync::mpsc channel for response
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        // Store pending execution
        let pending = PendingExecution {
            tool_name: tool_name.to_string(),
            response_tx,
        };
        self.pending_executions
            .write()
            .await
            .insert(request_id.clone(), pending);

        // Send message to client
        self.send_to_session(&session_id, message)
            .await
            .map_err(|_| ExecuteToolError::SendFailed)?;

        // Wait for response with timeout
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || response_rx.recv()),
        )
        .await;

        // Clean up pending execution
        self.pending_executions.write().await.remove(&request_id);

        match result {
            Ok(Ok(Ok(Ok(value)))) => Ok(value),
            Ok(Ok(Ok(Err(error)))) => Err(ExecuteToolError::ExecutionFailed(error)),
            Ok(Ok(Err(_))) => Err(ExecuteToolError::ChannelClosed),
            Ok(Err(_)) => Err(ExecuteToolError::ChannelClosed),
            Err(_) => Err(ExecuteToolError::Timeout),
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
