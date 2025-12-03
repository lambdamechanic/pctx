use crate::state::ws_manager::{OutgoingMessage, WsSession};
use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::protocol::{JsonRpcNotification, SessionCreatedParams};
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct WsQuery {
    pub code_mode_session_id: Uuid,
}

/// Handle WebSocket upgrade
pub async fn ws_handler(
    Query(WsQuery {
        code_mode_session_id,
    }): Query<WsQuery>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    // Verify that a code mode session exists with this ID
    if !state.code_mode_manager.exists(code_mode_session_id).await {
        error!(
            "Rejecting WebSocket connection: code mode session {code_mode_session_id} not found"
        );
        return (
            StatusCode::BAD_REQUEST,
            format!("Code mode session {code_mode_session_id} not found"),
        )
            .into_response();
    }

    // Check if there's already a WebSocket session for this code mode ID
    if state
        .ws_manager
        .get_for_code_mode_session(code_mode_session_id)
        .await
        .is_some()
    {
        error!(
            "Rejecting WebSocket connection: code mode session {code_mode_session_id} already has an active WebSocket connection"
        );
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "Code mode session {code_mode_session_id} already has an active WebSocket connection"
            ),
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, code_mode_session_id))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState, code_mode_session_id: Uuid) {
    info!("New WebSocket connection with code_mode_session_id: {code_mode_session_id}");

    info!(
        "Verified code mode session {} exists, proceeding with WebSocket setup",
        code_mode_session_id
    );

    // Split socket into sender and receiver
    let (sender, receiver) = socket.split();

    // Create channel for outgoing messages - convert OutgoingMessage to WebSocket Message
    let (tx, rx) = mpsc::unbounded_channel::<OutgoingMessage>();

    // Create session
    let session = WsSession::new(tx.clone(), code_mode_session_id);
    let session_id = session.id;

    info!(
        "Created session {session_id} connected to code mode session {}",
        session.code_mode_session_id
    );
    state.ws_manager.add(session).await;

    // Send session_created notification
    let session_created = JsonRpcNotification::new(
        "session_created",
        Some(json!(SessionCreatedParams {
            session_id: session_id.to_string()
        })),
    );

    if let Ok(msg_json) = serde_json::to_value(&session_created) {
        let _ = tx.send(OutgoingMessage::Notification(msg_json));
    }

    // Spawn task to handle outgoing messages (execute_tool requests)
    let mut send_task = tokio::spawn(write_messages(sender, rx));

    // Spawn task to handle incoming messages (execute_tool responses)
    let state_clone = state.clone();
    // TODO: is state clone an issue?
    let mut recv_task = tokio::spawn(read_messages(receiver, session_id, state_clone));

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            debug!("Send task completed for session {session_id}");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("Receive task completed for session {session_id}");
            send_task.abort();
        }
    }

    state.ws_manager.remove_session(session_id).await;

    info!("WebSocket connection closed for session {session_id}");
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
async fn read_messages(mut receiver: SplitStream<WebSocket>, session_id: Uuid, state: AppState) {
    while let Some(result) = receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handle_message(msg, session_id, &state).await {
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
async fn handle_message(msg: Message, session_id: Uuid, state: &AppState) -> Result<(), String> {
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
                .ws_manager
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
