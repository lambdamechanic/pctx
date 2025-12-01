use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
/// WebSocket connection handler
///
/// Manages an individual WebSocket connection, handles incoming messages,
/// and sends outgoing messages.
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{
    protocol::*,
    session::{OutgoingMessage, Session, SessionManager, SessionManagerExt},
};

pub struct WebSocketHandler {
    socket: WebSocket,
    session_manager: Arc<SessionManager>,
}

impl WebSocketHandler {
    pub fn new(socket: WebSocket, session_manager: Arc<SessionManager>) -> Self {
        Self {
            socket,
            session_manager,
        }
    }

    /// Run the WebSocket handler
    pub async fn run(self) -> Result<(), HandlerError> {
        let session_manager = self.session_manager;
        let (mut ws_sender, mut ws_receiver) = self.socket.split();

        // Create channel for outgoing messages
        let (tx, mut rx) = mpsc::unbounded_channel::<OutgoingMessage>();

        // Create session
        let session = Session::new(tx);
        let session_id = session.id.clone();
        let session_id_clone = session_id.clone();

        // Add session to manager
        session_manager.add_session(session).await;
        info!("New session created: {}", session_id);

        // Send session created notification
        let session_notification = JsonRpcNotification::new(
            "session_created",
            Some(json!({
                "session_id": session_id
            })),
        );
        if let Ok(msg_text) = serde_json::to_string(&session_notification) {
            let _ = ws_sender.send(Message::Text(msg_text.into())).await;
        }

        // Spawn task to handle outgoing messages
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let text = match msg {
                    OutgoingMessage::Response(value) | OutgoingMessage::Notification(value) => {
                        match serde_json::to_string(&value) {
                            Ok(text) => text,
                            Err(e) => {
                                error!("Failed to serialize message: {}", e);
                                continue;
                            }
                        }
                    }
                };

                if let Err(e) = ws_sender.send(Message::Text(text.into())).await {
                    error!("Failed to send message: {}", e);
                    break;
                }
            }
        });

        // Handle incoming messages
        let session_manager_clone = session_manager.clone();
        let session_id_for_recv = session_id.clone();
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received message: {}", text);
                    if let Err(e) = Self::handle_text_message_static(
                        &text,
                        &session_id_for_recv,
                        &session_manager_clone,
                    )
                    .await
                    {
                        error!("Error handling message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client closed connection: {}", session_id);
                    break;
                }
                Ok(Message::Ping(_data)) => {
                    // Respond to ping with pong
                    if let Err(e) = session_manager
                        .send_to_session(
                            &session_id,
                            OutgoingMessage::Response(json!({ "pong": true })),
                        )
                        .await
                    {
                        error!("Failed to send pong: {}", e);
                    }
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Clean up
        send_task.abort();
        session_manager.remove_session(&session_id_clone).await;
        info!("Session removed: {}", session_id_clone);

        Ok(())
    }

    /// Handle a text message from the client (static version)
    async fn handle_text_message_static(
        text: &str,
        session_id: &str,
        session_manager: &Arc<SessionManager>,
    ) -> Result<(), HandlerError> {
        // Try to parse as JSON value
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| HandlerError::ParseError(e.to_string()))?;

        // Determine message type:
        // - Response: has "result" or "error" field (response to server's request)
        // - Request: has "method" field
        // - Notification: has "method" but no "id"

        if value.get("result").is_some() || value.get("error").is_some() {
            // This is a response from the client (to an execute_tool request)
            Self::handle_client_response(value, session_manager).await
        } else if value.get("method").is_some() {
            // This is a request from the client
            if value.get("id").is_some() {
                Self::handle_request(value, session_id, session_manager).await
            } else {
                Self::handle_notification(value, session_id).await
            }
        } else {
            Err(HandlerError::ParseError(
                "Invalid message: missing method, result, or error".to_string(),
            ))
        }
    }

    /// Handle a response from the client (to our execute_tool request)
    async fn handle_client_response(
        value: serde_json::Value,
        session_manager: &Arc<SessionManager>,
    ) -> Result<(), HandlerError> {
        debug!("Handling client response: {:?}", value);
        let id = value
            .get("id")
            .ok_or_else(|| HandlerError::ParseError("Response missing id".to_string()))?
            .clone();

        if let Some(result) = value.get("result") {
            debug!("Client response has result: {:?}", result);
            Self::handle_execution_response(&id, Ok(result.clone()), session_manager).await;
        } else if let Some(error) = value.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            debug!("Client response has error: {}", error_msg);
            Self::handle_execution_response(&id, Err(error_msg), session_manager).await;
        }

        Ok(())
    }

    /// Handle a JSON-RPC request (expects response)
    async fn handle_request(
        value: serde_json::Value,
        session_id: &str,
        session_manager: &Arc<SessionManager>,
    ) -> Result<(), HandlerError> {
        let request: JsonRpcRequest = serde_json::from_value(value.clone())
            .map_err(|e| HandlerError::ParseError(e.to_string()))?;

        let response_value = match request.method {
            crate::protocol::Method::RegisterTool => {
                Self::handle_register_tool(&request, session_id, session_manager).await
            }
            crate::protocol::Method::RegisterMcp => {
                Self::handle_register_mcp(&request, session_id, session_manager).await
            }
            crate::protocol::Method::Execute => {
                Self::handle_execute_code(&request, session_id, session_manager).await
            }
            crate::protocol::Method::ExecuteTool => {
                // ExecuteTool is a server-to-client method, not client-to-server
                // If client sends this, it's an error
                Self::error_response(
                    error_codes::METHOD_NOT_FOUND,
                    "execute_tool is a server-to-client method",
                    request.id.clone(),
                )
            }
            crate::protocol::Method::Unknown => {
                // Create error response for unknown method
                Self::error_response(
                    error_codes::METHOD_NOT_FOUND,
                    format!("Method not found: {}", request.method),
                    request.id.clone(),
                )
            }
        };

        // Send response
        session_manager
            .send_to_session(session_id, OutgoingMessage::Response(response_value))
            .await
            .map_err(|e| HandlerError::SendError(e.to_string()))?;

        Ok(())
    }

    /// Handle a JSON-RPC notification (no response expected)
    async fn handle_notification(
        value: serde_json::Value,
        _session_id: &str,
    ) -> Result<(), HandlerError> {
        let _notification: JsonRpcNotification =
            serde_json::from_value(value).map_err(|e| HandlerError::ParseError(e.to_string()))?;

        // Currently no notifications from client to server
        // This could be used for events like tool unregistration

        Ok(())
    }

    /// Handle register_mcp request
    async fn handle_register_mcp(
        request: &JsonRpcRequest,
        session_id: &str,
        session_manager: &Arc<SessionManager>,
    ) -> serde_json::Value {
        let params: RegisterMcpParams = match request.params.as_ref() {
            Some(params) => match serde_json::from_value(params.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return Self::error_response(
                        error_codes::INVALID_PARAMS,
                        format!("Invalid params: {}", e),
                        request.id.clone(),
                    );
                }
            },
            None => {
                return Self::error_response(
                    error_codes::INVALID_PARAMS,
                    "Missing params",
                    request.id.clone(),
                );
            }
        };

        match session_manager
            .register_mcp_server(session_id, params.name.clone(), params.url, params.auth)
            .await
        {
            Ok(_) => {
                info!("MCP server registered: {} (session: {})", params.name, session_id);
                Self::success_response(json!({ "success": true }), request.id.clone())
            }
            Err(e) => {
                warn!("Failed to register MCP server {}: {}", params.name, e);
                Self::error_response(
                    error_codes::TOOL_ALREADY_REGISTERED, // Reuse this code for MCP server conflicts
                    e.to_string(),
                    request.id.clone(),
                )
            }
        }
    }

    /// Handle register_tool request
    async fn handle_register_tool(
        request: &JsonRpcRequest,
        session_id: &str,
        session_manager: &Arc<SessionManager>,
    ) -> serde_json::Value {
        let params: RegisterToolParams = match request.params.as_ref() {
            Some(params) => match serde_json::from_value(params.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return Self::error_response(
                        error_codes::INVALID_PARAMS,
                        format!("Invalid params: {}", e),
                        request.id.clone(),
                    );
                }
            },
            None => {
                return Self::error_response(
                    error_codes::INVALID_PARAMS,
                    "Missing params",
                    request.id.clone(),
                );
            }
        };

        let tool_name = format!("{}.{}", params.namespace, params.name);

        match session_manager
            .register_tool(session_id, tool_name.clone(), params.description)
            .await
        {
            Ok(_) => {
                info!("Tool registered: {} (session: {})", tool_name, session_id);
                Self::success_response(json!({ "success": true }), request.id.clone())
            }
            Err(e) => {
                warn!("Failed to register tool {}: {}", tool_name, e);
                Self::error_response(
                    error_codes::TOOL_ALREADY_REGISTERED,
                    e.to_string(),
                    request.id.clone(),
                )
            }
        }
    }

    /// Handle execute code request
    async fn handle_execute_code(
        request: &JsonRpcRequest,
        session_id: &str,
        session_manager: &Arc<SessionManager>,
    ) -> serde_json::Value {
        let params: ExecuteCodeParams = match request.params.as_ref() {
            Some(params) => match serde_json::from_value(params.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return Self::error_response(
                        error_codes::INVALID_PARAMS,
                        format!("Invalid params: {}", e),
                        request.id.clone(),
                    );
                }
            },
            None => {
                return Self::error_response(
                    error_codes::INVALID_PARAMS,
                    "Missing params",
                    request.id.clone(),
                );
            }
        };

        info!("Executing code for session: {}", session_id);
        debug!("Code to execute: {}", params.code);

        // Execute code using SessionManager
        match session_manager.execute_code(&params.code).await {
            Ok(result) => {
                info!("Code execution completed (success: {})", result.success);

                if result.success {
                    Self::success_response(
                        json!({
                            "value": result.value,
                            "stdout": result.stdout,
                            "stderr": result.stderr
                        }),
                        request.id.clone(),
                    )
                } else {
                    Self::error_response(
                        error_codes::EXECUTION_FAILED,
                        result.stderr.clone(),
                        request.id.clone(),
                    )
                }
            }
            Err(e) => {
                warn!("Code execution failed: {}", e);
                Self::error_response(
                    error_codes::EXECUTION_FAILED,
                    e.to_string(),
                    request.id.clone(),
                )
            }
        }
    }

    /// Handle execution response from client
    async fn handle_execution_response(
        request_id: &serde_json::Value,
        result: Result<serde_json::Value, String>,
        session_manager: &Arc<SessionManager>,
    ) {
        if let Err(()) = session_manager
            .handle_execution_response(request_id, result)
            .await
        {
            warn!("Received response for unknown request: {:?}", request_id);
        }
    }

    /// Helper to create a success response, with proper error handling
    fn success_response(result: serde_json::Value, id: serde_json::Value) -> serde_json::Value {
        serde_json::to_value(JsonRpcResponse::success(result, id)).unwrap_or_else(|e| {
            error!("Critical: Failed to serialize success response: {}", e);
            // Return a minimal error response as fallback
            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": error_codes::INTERNAL_ERROR,
                    "message": "Internal serialization error"
                },
                "id": serde_json::Value::Null
            })
        })
    }

    /// Helper to create an error response, with proper error handling
    fn error_response(
        code: i32,
        message: impl Into<String>,
        id: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::to_value(JsonRpcErrorResponse::error(code, message, id.clone())).unwrap_or_else(
            |e| {
                error!("Critical: Failed to serialize error response: {}", e);
                // Return a minimal error response as fallback
                json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": error_codes::INTERNAL_ERROR,
                        "message": "Internal serialization error"
                    },
                    "id": id
                })
            },
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
