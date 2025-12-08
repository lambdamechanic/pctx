use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc as tokio_mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::model::{WsExecuteTool, WsExecuteToolResult, WsMessage};

#[derive(Debug, thiserror::Error)]
pub enum ExecuteCallbackError {
    #[error("Failed to send execution request")]
    SendFailed,
    #[error("Execution failed: {0}")]
    ExecutionFailed(rmcp::model::ErrorData),
    #[error("Response channel closed")]
    ChannelClosed,
    #[error("Execution timeout")]
    Timeout,
}

#[derive(Default)]
pub struct WsManager {
    /// Active sessions by ID
    pub(crate) sessions: Arc<RwLock<HashMap<Uuid, Arc<RwLock<WsSession>>>>>,
}

impl WsManager {
    /// Lists current sessions
    pub async fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.read().await.keys().copied().collect()
    }

    /// Add a new session
    pub async fn add(&self, session: WsSession) -> Uuid {
        let session_id = session.id;
        let session_lock = Arc::new(RwLock::new(session));
        self.sessions.write().await.insert(session_id, session_lock);
        session_id
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id);
    }

    pub async fn get_for_code_mode_session(
        &self,
        code_mode_session_id: Uuid,
    ) -> Option<Arc<RwLock<WsSession>>> {
        let sessions = self.sessions.read().await;

        // Search through sessions to find one with matching code_mode_session_id
        for session_lock in sessions.values() {
            let session = session_lock.read().await;
            if session.code_mode_session_id == code_mode_session_id {
                // Clone the Arc before dropping locks
                return Some(session_lock.clone());
            }
        }

        None
    }

    /// Handle a response from a client for a pending execution
    /// Finds the session with the matching `request_id` and delegates to it
    pub async fn handle_execution_response(
        &self,
        request_id: Uuid,
        result: Result<WsExecuteToolResult, rmcp::model::ErrorData>,
    ) -> Result<(), ()> {
        let sessions = self.sessions.read().await;

        // Find the session that has this pending execution
        for session_lock in sessions.values() {
            let session_read = session_lock.read().await;

            if session_read
                .pending_executions
                .read()
                .await
                .contains_key(&request_id)
            {
                // Handle the response on the cloned Arc
                session_read
                    .handle_execution_response(request_id, result.clone())
                    .await?;
                return Ok(());
            }
        }

        warn!("No session found with pending execution for request_id: {request_id}");
        Err(())
    }
}

type PendingExecutionsMap = Arc<
    RwLock<
        HashMap<Uuid, std::sync::mpsc::Sender<Result<WsExecuteToolResult, rmcp::model::ErrorData>>>,
    >,
>;

/// WebSocket session representing a connected client
#[derive(Clone)]
pub struct WsSession {
    pub id: Uuid,
    pub code_mode_session_id: Uuid,
    /// Channel to send messages to the client
    pub sender: tokio_mpsc::UnboundedSender<WsMessage>,
    /// Pending execution requests waiting for responses
    pending_executions: PendingExecutionsMap,
}
impl WsSession {
    pub fn new(sender: tokio_mpsc::UnboundedSender<WsMessage>, code_mode_session_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            code_mode_session_id,
            pending_executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute a callback on this session, sending a message and waiting for a response
    pub async fn execute_callback(
        &self,
        message: WsExecuteTool,
    ) -> Result<WsExecuteToolResult, ExecuteCallbackError> {
        let req_id = message.id;
        // Create std::sync::mpsc channel for response
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        // Store pending execution
        self.pending_executions
            .write()
            .await
            .insert(req_id, response_tx);

        // Send message to client
        self.sender
            .send(WsMessage::ExecuteTool(message))
            .map_err(|_| ExecuteCallbackError::SendFailed)?;

        // Wait for response with timeout
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || response_rx.recv()),
        )
        .await;

        // Clean up pending execution
        self.pending_executions.write().await.remove(&req_id);

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
        request_id: Uuid,
        result: Result<WsExecuteToolResult, rmcp::model::ErrorData>,
    ) -> Result<(), ()> {
        let pending_read = self.pending_executions.read().await;
        info!(
            pending_count = pending_read.len(),
            "Handling execution response for request_id: {request_id:?}",
        );
        if let Some(response_tx) = pending_read.get(&request_id) {
            debug!("Found pending execution, sending result");
            let send_result = response_tx.send(result);
            debug!("mpsc send result: {send_result:?}");
            Ok(())
        } else {
            warn!("No pending execution found for request_id: {request_id:?}");
            Err(())
        }
    }
}
