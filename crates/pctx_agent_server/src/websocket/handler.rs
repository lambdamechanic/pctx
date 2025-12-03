use crate::session::{OutgoingMessage, Session};
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::protocol::{JsonRpcNotification, SessionCreatedParams};
use crate::AppState;

/// Handle WebSocket upgrade
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    info!("New WebSocket connection");

    // Split socket into sender and receiver
    let (sender, receiver) = socket.split();

    // Create channel for outgoing messages - convert OutgoingMessage to WebSocket Message
    let (tx, rx) = mpsc::unbounded_channel::<OutgoingMessage>();

    // Create session
    let session = Session::new(tx.clone());
    let session_id = session.id.clone();

    info!("Created session: {}", session_id);
    state.session_manager.add_session(session).await;

    // Send session_created notification
    let session_created = JsonRpcNotification::new(
        "session_created",
        Some(json!(SessionCreatedParams {
            session_id: session_id.clone()
        })),
    );

    if let Ok(msg_json) = serde_json::to_value(&session_created) {
        let _ = tx.send(OutgoingMessage::Notification(msg_json));
    }

    // Spawn task to handle outgoing messages (execute_tool requests)
    let mut send_task = tokio::spawn(write_messages(sender, rx));

    // Spawn task to handle incoming messages (execute_tool responses)
    let session_id_clone = session_id.clone();
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(read_messages(receiver, session_id_clone, state_clone));

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            debug!("Send task completed for session {}", session_id);
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("Receive task completed for session {}", session_id);
            send_task.abort();
        }
    }

    // Clean up registered tool callbacks from CallbackRegistry
    let sessions_guard = state.session_manager.sessions().read().await;
    if let Some(session) = sessions_guard.get(&session_id) {
        for tool_name in session.registered_callbacks.keys() {
            state.callback_registry.remove(tool_name);
            debug!("Removed callback for tool: {}", tool_name);
        }
    }

    state.session_manager.remove_session(&session_id).await;

    info!("WebSocket connection closed for session {}", session_id);
}

/// Handle outgoing WebSocket messages (`execute_tool` requests from server)
async fn write_messages(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: mpsc::UnboundedReceiver<OutgoingMessage>,
) {
    while let Some(msg) = rx.recv().await {
        // Convert OutgoingMessage to WebSocket Message
        let ws_msg = match msg {
            OutgoingMessage::Response(json_val) | OutgoingMessage::Notification(json_val) => {
                match serde_json::to_string(&json_val) {
                    Ok(text) => Message::Text(text.into()),
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                }
            }
        };

        if let Err(e) = sender.send(ws_msg).await {
            error!("Error sending WebSocket message: {}", e);
            break;
        }
    }
}

/// Handle incoming WebSocket messages (`execute_tool` responses from client)
async fn read_messages(mut receiver: SplitStream<WebSocket>, session_id: String, state: AppState) {
    while let Some(result) = receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handle_message(msg, &session_id, &state).await {
                    error!("Error handling message for session {}: {}", session_id, e);
                }
            }
            Err(e) => {
                error!("WebSocket error for session {}: {}", session_id, e);
                break;
            }
        }
    }
}

/// Handle a single WebSocket message
async fn handle_message(msg: Message, session_id: &str, state: &AppState) -> Result<(), String> {
    match msg {
        Message::Text(text) => {
            debug!("Received text message from {}: {}", session_id, text);

            // Parse as JSON-RPC response (client responding to execute_tool)
            let response: Value =
                serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {e}"))?;

            // Extract the id from the response
            let id = response
                .get("id")
                .ok_or_else(|| "Response missing id field".to_string())?
                .clone();

            // Determine if this is a success or error response
            let result = if let Some(error) = response.get("error") {
                Err(error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error")
                    .to_string())
            } else if let Some(result_val) = response.get("result") {
                Ok(result_val.clone())
            } else {
                Err("Response has neither result nor error field".to_string())
            };

            // Resolve the pending execution with this response
            state
                .session_manager
                .handle_execution_response(&id, result)
                .await
                .map_err(|()| "Failed to resolve execution".to_string())?;

            Ok(())
        }
        Message::Binary(_) => {
            warn!("Received binary message, ignoring");
            Ok(())
        }
        Message::Close(_) => {
            info!("Received close message for session {}", session_id);
            Ok(())
        }
        Message::Ping(_) | Message::Pong(_) => Ok(()),
    }
}
