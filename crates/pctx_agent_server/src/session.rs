//! Core types for WebSocket session management for ws callbacks.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{RwLock, mpsc as tokio_mpsc};
use uuid::Uuid;

/// Unique identifier for a WebSocket session
pub type SessionId = String;

/// Information about a registered callback
#[derive(Debug, Clone)]
pub struct CallbackInfo {
    pub name: String,
    pub description: String,
}

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
    /// callbacks registered by this session (name -> info)
    pub registered_callbacks: HashMap<String, CallbackInfo>,
}

impl Session {
    pub fn new(sender: tokio_mpsc::UnboundedSender<OutgoingMessage>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender,
            registered_callbacks: HashMap::new(),
        }
    }

    pub fn with_id(id: String, sender: tokio_mpsc::UnboundedSender<OutgoingMessage>) -> Self {
        Self {
            id,
            sender,
            registered_callbacks: HashMap::new(),
        }
    }

    pub fn register_callback(&mut self, callback_name: String, description: String) {
        self.registered_callbacks.insert(
            callback_name.clone(),
            CallbackInfo {
                name: callback_name,
                description,
            },
        );
    }

    pub fn unregister_callback(&mut self, callback_name: &str) -> bool {
        self.registered_callbacks.remove(callback_name).is_some()
    }

    pub fn has_callback(&self, callback_name: &str) -> bool {
        self.registered_callbacks.contains_key(callback_name)
    }

    pub fn get_callback_info(&self, callback_name: &str) -> Option<&CallbackInfo> {
        self.registered_callbacks.get(callback_name)
    }
}

/// Pending execution request waiting for response from client
pub struct PendingExecution {
    pub callback_name: String,
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

/// Error types for session operations
#[derive(Debug, thiserror::Error)]
pub enum RegistercallbackError {
    #[error("callback already registered")]
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
pub enum ExecutecallbackError {
    #[error("callback not found")]
    CallbackNotFound,
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
    /// callback name → session ID mapping
    callback_sessions: Arc<RwLock<HashMap<String, SessionId>>>,
    /// Request ID → pending execution mapping
    pending_executions: Arc<RwLock<HashMap<serde_json::Value, PendingExecution>>>,
    /// Code execution callback (optional) - uses std::sync::RwLock for synchronous setup
    // code_executor: Arc<StdRwLock<Option<CodeExecutorFn>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            callback_sessions: Arc::new(RwLock::new(HashMap::new())),
            pending_executions: Arc::new(RwLock::new(HashMap::new())),
            // code_executor: Arc::new(StdRwLock::new(None)),
        }
    }

    // pub fn with_code_executor(self, executor: CodeExecutorFn) -> Self {
    //     *self.code_executor.write().unwrap() = Some(executor);
    //     self
    // }

    // /// Set the code executor (for setting after construction)
    // pub fn set_code_executor(&self, executor: CodeExecutorFn) {
    //     *self.code_executor.write().unwrap() = Some(executor);
    // }

    /// Add a new session
    pub async fn add_session(&self, session: Session) -> SessionId {
        let session_id = session.id.clone();
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    /// Remove a session and clean up all its registered callbacks
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            // Clean up callback mappings
            let mut callback_sessions = self.callback_sessions.write().await;
            for (callback_name, _info) in &session.registered_callbacks {
                callback_sessions.remove(callback_name);
            }
            drop(callback_sessions);
        }
    }

    /// Register a callback for a session
    pub async fn register_callback(
        &self,
        session_id: &str,
        callback_name: String,
        description: Option<String>,
    ) -> Result<(), RegistercallbackError> {
        // Check if callback already exists
        let callback_sessions = self.callback_sessions.read().await;
        if callback_sessions.contains_key(&callback_name) {
            return Err(RegistercallbackError::AlreadyRegistered);
        }
        drop(callback_sessions);

        // Add to session
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or(RegistercallbackError::SessionNotFound)?;
        session.register_callback(callback_name.clone(), description.unwrap_or_default());
        drop(sessions);

        // Add to callback mapping
        let mut callback_sessions = self.callback_sessions.write().await;
        callback_sessions.insert(callback_name, session_id.to_string());

        Ok(())
    }

    /// Get the session ID for a callback
    pub async fn get_callback_session(&self, callback_name: &str) -> Option<SessionId> {
        self.callback_sessions
            .read()
            .await
            .get(callback_name)
            .cloned()
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

    /// Get list of all registered callbacks
    pub async fn list_callbacks(&self) -> Vec<String> {
        self.callback_sessions
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// Get list of all registered callbacks with their info (name and description)
    pub async fn list_callbacks_with_info(&self) -> Vec<CallbackInfo> {
        let callback_sessions = self.callback_sessions.read().await;
        let sessions = self.sessions.read().await;

        let mut callbacks = Vec::new();
        for (callback_name, session_id) in callback_sessions.iter() {
            if let Some(session) = sessions.get(session_id) {
                if let Some(info) = session.get_callback_info(callback_name) {
                    callbacks.push(info.clone());
                }
            }
        }
        callbacks
    }

    /// Get description for a specific callback
    pub async fn get_callback_description(&self, callback_name: &str) -> Option<String> {
        let session_id = self.get_callback_session(callback_name).await?;
        let sessions = self.sessions.read().await;
        let session = sessions.get(&session_id)?;
        session
            .get_callback_info(callback_name)
            .map(|info| info.description.clone())
    }

    /// Get number of active sessions
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Get access to pending executions (for WebSocket server use)
    pub fn pending_executions(&self) -> &Arc<RwLock<HashMap<serde_json::Value, PendingExecution>>> {
        &self.pending_executions
    }

    // /// Get access to code executor (for WebSocket server use)
    // pub fn code_executor(&self) -> &Arc<StdRwLock<Option<CodeExecutorFn>>> {
    //     &self.code_executor
    // }

    /// Get access to sessions (for WebSocket server use)
    pub fn sessions(&self) -> &Arc<RwLock<HashMap<SessionId, Session>>> {
        &self.sessions
    }

    /// Get access to callback sessions (for WebSocket server use)
    pub fn callback_sessions(&self) -> &Arc<RwLock<HashMap<String, SessionId>>> {
        &self.callback_sessions
    }

    /// Execute a callback and wait for response
    ///
    /// This is a low-level method that sends a raw response message and waits for the result.
    /// Higher-level wrappers in pctx_agent_server provide better ergonomics.
    pub async fn execute_callback_raw(
        &self,
        callback_name: &str,
        message: OutgoingMessage,
        request_id: serde_json::Value,
    ) -> Result<serde_json::Value, ExecutecallbackError> {
        // Find session for callback
        let session_id = self
            .get_callback_session(callback_name)
            .await
            .ok_or(ExecutecallbackError::CallbackNotFound)?;

        // Create std::sync::mpsc channel for response
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        // Store pending execution
        let pending = PendingExecution {
            callback_name: callback_name.to_string(),
            response_tx,
        };
        self.pending_executions
            .write()
            .await
            .insert(request_id.clone(), pending);

        // Send message to client
        self.send_to_session(&session_id, message)
            .await
            .map_err(|_| ExecutecallbackError::SendFailed)?;

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
            Ok(Ok(Ok(Err(error)))) => Err(ExecutecallbackError::ExecutionFailed(error)),
            Ok(Ok(Err(_))) => Err(ExecutecallbackError::ChannelClosed),
            Ok(Err(_)) => Err(ExecutecallbackError::ChannelClosed),
            Err(_) => Err(ExecutecallbackError::Timeout),
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
