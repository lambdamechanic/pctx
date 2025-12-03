use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc as tokio_mpsc};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ExecuteCallbackError {
    #[error("Failed to send execution request")]
    SendFailed,
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Response channel closed")]
    ChannelClosed,
    #[error("Execution timeout")]
    Timeout,
}

pub struct WsManager {
    /// Active sessions by ID
    sessions: Arc<RwLock<HashMap<Uuid, WsSession>>>,
}

impl WsManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new session
    pub async fn add_session(&self, session: WsSession) -> Uuid {
        let session_id = session.id.clone();
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id);
    }

    pub async fn get_for_code_mode_id(&self, code_mode_session_id: Uuid) -> Option<WsSession> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .find(|session| session.code_mode_session_id == code_mode_session_id)
            .cloned()
    }

    /// Handle a response from a client for a pending execution
    /// Finds the session with the matching request_id and delegates to it
    pub async fn handle_execution_response(
        &self,
        request_id: &serde_json::Value,
        result: Result<serde_json::Value, String>,
    ) -> Result<(), ()> {
        let sessions = self.sessions.read().await;

        // Find the session that has this pending execution
        for session in sessions.values() {
            let pending = session.pending_executions.read().await;
            if pending.contains_key(request_id) {
                // Clone the session so we can use it after dropping locks
                let session = session.clone();
                drop(pending);
                drop(sessions);

                // Handle the response on the cloned session
                return session.handle_execution_response(request_id, result).await;
            }
        }

        eprintln!(
            "[WsManager] No session found with pending execution for request_id: {:?}",
            request_id
        );
        Err(())
    }

}

/// Pending execution request waiting for response from client
pub struct PendingExecution {
    pub callback_name: String,
    pub response_tx: std::sync::mpsc::Sender<Result<serde_json::Value, String>>,
}

/// WebSocket session representing a connected client
#[derive(Clone)]
pub struct WsSession {
    pub id: Uuid,
    pub code_mode_session_id: Uuid,
    /// Channel to send messages to the client
    pub sender: tokio_mpsc::UnboundedSender<OutgoingMessage>,
    /// Pending execution requests waiting for responses
    pending_executions: Arc<RwLock<HashMap<serde_json::Value, PendingExecution>>>,
}
impl WsSession {
    pub fn new(
        sender: tokio_mpsc::UnboundedSender<OutgoingMessage>,
        code_mode_session_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            code_mode_session_id,
            pending_executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute a callback on this session, sending a message and waiting for a response
    pub async fn execute_callback_raw(
        &self,
        callback_name: &str,
        message: OutgoingMessage,
        request_id: serde_json::Value,
    ) -> Result<serde_json::Value, ExecuteCallbackError> {
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
        self.sender
            .send(message)
            .map_err(|_| ExecuteCallbackError::SendFailed)?;

        // Wait for response with timeout
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || response_rx.recv()),
        )
        .await;

        // Clean up pending execution
        self.pending_executions
            .write()
            .await
            .remove(&request_id);

        match result {
            Ok(Ok(Ok(Ok(value)))) => Ok(value),
            Ok(Ok(Ok(Err(error)))) => Err(ExecuteCallbackError::ExecutionFailed(error)),
            Ok(Ok(Err(_))) => Err(ExecuteCallbackError::ChannelClosed),
            Ok(Err(_)) => Err(ExecuteCallbackError::ChannelClosed),
            Err(_) => Err(ExecuteCallbackError::Timeout),
        }
    }

    /// Handle a response from a client for a pending execution
    pub async fn handle_execution_response(
        &self,
        request_id: &serde_json::Value,
        result: Result<serde_json::Value, String>,
    ) -> Result<(), ()> {
        eprintln!(
            "[WsSession] Handling execution response for request_id: {:?}",
            request_id
        );
        let mut pending = self.pending_executions.write().await;
        eprintln!(
            "[WsSession] Pending executions count: {}",
            pending.len()
        );
        if let Some(execution) = pending.remove(request_id) {
            eprintln!("[WsSession] Found pending execution, sending result");
            let send_result = execution.response_tx.send(result);
            eprintln!("[WsSession] mpsc send result: {:?}", send_result);
            Ok(())
        } else {
            eprintln!(
                "[WsSession] No pending execution found for request_id: {:?}",
                request_id
            );
            Err(())
        }
    }
}

/// Messages that can be sent to a WebSocket client
#[derive(Debug, Clone)]
pub enum OutgoingMessage {
    /// JSON-RPC response
    Response(serde_json::Value),
    /// JSON-RPC notification
    Notification(serde_json::Value),
}
