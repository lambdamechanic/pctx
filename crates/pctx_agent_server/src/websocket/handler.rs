use crate::{
    extractors::CodeModeSession,
    model::{WsExecuteToolResult, WsMessage},
    state::ws_manager::WsSession,
};
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use rmcp::model::{
    JsonRpcMessage, JsonRpcRequest, JsonRpcVersion2_0, NumberOrString,
    Request as JsonRpcRequestData,
};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::AppState;

/// Handle WebSocket upgrade
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    CodeModeSession(code_mode_session): CodeModeSession,
) -> Response {
    // Verify that a code mode session exists with this ID
    if !state.code_mode_manager.exists(code_mode_session).await {
        error!("Rejecting WebSocket connection: code mode session {code_mode_session} not found");
        return (
            StatusCode::BAD_REQUEST,
            format!("Code mode session {code_mode_session} not found"),
        )
            .into_response();
    }

    // Check if there's already a WebSocket session for this code mode ID
    if state
        .ws_manager
        .get_for_code_mode_session(code_mode_session)
        .await
        .is_some()
    {
        error!(
            "Rejecting WebSocket connection: code mode session {code_mode_session} already has an active WebSocket connection"
        );
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "Code mode session {code_mode_session} already has an active WebSocket connection"
            ),
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, code_mode_session))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState, code_mode_session: Uuid) {
    info!("New WebSocket connection with code_mode_session: {code_mode_session}");

    info!(
        "Verified code mode session {} exists, proceeding with WebSocket setup",
        code_mode_session
    );

    // Split socket into sender and receiver
    let (sender, receiver) = socket.split();

    // Create an in-process channel for outgoing messages - convert OutgoingMessage to WebSocket Message
    let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();

    // Create session
    let session = WsSession::new(tx.clone(), code_mode_session);
    let ws_session = session.id;

    info!(
        "Created session {ws_session} connected to code mode session {}",
        session.code_mode_session_id
    );
    state.ws_manager.add(session).await;

    // Spawn task to handle outgoing messages (notifications/execute_tool requests)
    let mut send_task = tokio::spawn(write_messages(sender, rx));

    // Spawn task to handle incoming messages (execute_tool responses)
    let state_clone = state.clone(); // cloning state here is ok because state just has Arc attributes
    let mut recv_task = tokio::spawn(read_messages(receiver, ws_session, state_clone));

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            debug!("Send task completed for session {ws_session}");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("Receive task completed for session {ws_session}");
            send_task.abort();
        }
    }

    state.ws_manager.remove_session(ws_session).await;

    info!("WebSocket connection closed for session {ws_session}");
}

/// Handle outgoing WebSocket messages (`execute_tool` requests from server)
async fn write_messages(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: mpsc::UnboundedReceiver<WsMessage>,
) {
    while let Some(msg) = rx.recv().await {
        // Convert OutgoingMessage to WebSocket Message
        let message_val = match msg {
            WsMessage::ExecuteTool(ws_execute_tool) => {
                let jsonrpc_req = JsonRpcRequest {
                    jsonrpc: JsonRpcVersion2_0,
                    id: NumberOrString::String(ws_execute_tool.id.to_string().into()),
                    request: JsonRpcRequestData {
                        method: "execute_tool",
                        params: json!(ws_execute_tool),
                        ..Default::default()
                    },
                };

                json!(jsonrpc_req)
            }
            WsMessage::ExecuteCodeResponse(response) => {
                if let Some(error) = response.error {
                    json!({
                        "jsonrpc": "2.0",
                        "id": response.id,
                        "error": {
                            "code": error.code,
                            "message": error.message,
                            "data": error.details,
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": response.id,
                        "result": response.result,
                    })
                }
            }
        };

        if let Err(e) = sender
            .send(Message::Text(message_val.to_string().into()))
            .await
        {
            error!("Error sending WebSocket message: {}", e);
            break;
        }
    }
}

/// Handle incoming WebSocket messages (`execute_tool` responses from client)
async fn read_messages(mut receiver: SplitStream<WebSocket>, ws_session: Uuid, state: AppState) {
    while let Some(result) = receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handle_message(msg, ws_session, &state).await {
                    error!("Error handling message for session {ws_session}: {e}");
                }
            }
            Err(e) => {
                error!("WebSocket error for session {ws_session}: {e}");
                break;
            }
        }
    }
}

/// Handle an execute_code request from the client
async fn handle_execute_code_request(
    text: String,
    ws_session: Uuid,
    state: &AppState,
) -> Result<(), String> {
    use crate::model::{ErrorCode, WsExecuteCodeResponse};

    let json_value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    let request_id = json_value["id"].clone();
    let params = json_value["params"]
        .as_object()
        .ok_or("Missing params field")?;
    let code = params["code"]
        .as_str()
        .ok_or("Missing code field")?
        .to_string();

    // Save the WebSocket session for later response
    let ws_session_lock = state
        .ws_manager
        .sessions
        .read()
        .await
        .get(&ws_session)
        .cloned()
        .ok_or_else(|| format!("WebSocket session {ws_session} not found"))?;

    let ws_session_read = ws_session_lock.read().await;
    let code_mode_session_id = ws_session_read.code_mode_session_id;
    let sender = ws_session_read.sender.clone();
    drop(ws_session_read);

    let code_mode_lock = match state.code_mode_manager.get(code_mode_session_id).await {
        Some(lock) => lock,
        None => {
            let error_response = WsExecuteCodeResponse {
                id: request_id,
                result: None,
                error: Some(crate::model::ErrorData {
                    code: ErrorCode::InvalidSession,
                    message: format!("Code mode session {code_mode_session_id} does not exist"),
                    details: None,
                }),
            };
            let _ = sender.send(crate::model::WsMessage::ExecuteCodeResponse(error_response));
            return Ok(());
        }
    };

    tokio::spawn(async move {
        let current_span = tracing::Span::current();
        let output = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
            let _guard = current_span.enter();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;
            rt.block_on(async {
                code_mode_lock
                    .read()
                    .await
                    .execute(&code)
                    .await
                    .map_err(|e| anyhow::anyhow!("Execution error: {e}"))
            })
        })
        .await;

        let response = match output {
            Ok(Ok(exec_output)) => WsExecuteCodeResponse {
                id: request_id,
                result: Some(exec_output),
                error: None,
            },
            Ok(Err(e)) => WsExecuteCodeResponse {
                id: request_id,
                result: None,
                error: Some(crate::model::ErrorData {
                    code: ErrorCode::Execution,
                    message: format!("Execution failed: {e}"),
                    details: None,
                }),
            },
            Err(e) => WsExecuteCodeResponse {
                id: request_id,
                result: None,
                error: Some(crate::model::ErrorData {
                    code: ErrorCode::Internal,
                    message: format!("Task join failed: {e}"),
                    details: None,
                }),
            },
        };

        if let Err(e) = sender.send(crate::model::WsMessage::ExecuteCodeResponse(response)) {
            error!("Failed to send execute_code response: {e}");
        }
    });

    Ok(())
}

/// Handle a single WebSocket message
/// Messages coming from a client, needs to be routed to the correct `WsSession` for handling.
async fn handle_message(msg: Message, ws_session: Uuid, state: &AppState) -> Result<(), String> {
    match msg {
        Message::Text(text) => {
            debug!("Received text message from {ws_session}: {text}");

            let json_value: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {e}"))?;

            // Check if it's a request (has "method" field) or response (has "result" or "error")
            if json_value.get("method").is_some() {
                let method = json_value["method"]
                    .as_str()
                    .ok_or("Missing method field")?;

                match method {
                    "execute_code" => {
                        handle_execute_code_request(text.to_string(), ws_session, state).await?;
                    }
                    _ => {
                        return Err(format!("Unknown method: {method}"));
                    }
                }
            } else {
                let (id, exec_res) = match serde_json::from_str::<
                    JsonRpcMessage<JsonRpcRequestData, WsExecuteToolResult>,
                >(&text)
                {
                    Ok(m) => match m {
                        JsonRpcMessage::Response(res) => {
                            let id: Uuid = serde_json::from_value(res.id.clone().into_json_value())
                                .map_err(|_| {
                                    format!(
                                        "Cannot route execute tool result with invalid uuid: {:?}",
                                        &res.id
                                    )
                                })?;
                            (id, Ok(res.result))
                        }
                        JsonRpcMessage::Error(err) => {
                            let id: Uuid = serde_json::from_value(err.id.clone().into_json_value())
                                .map_err(|_| {
                                    format!("Cannot route error with invalid uuid: {:?}", &err.id)
                                })?;
                            (id, Err(err.error))
                        }
                        JsonRpcMessage::Request(_) | JsonRpcMessage::Notification(_) => {
                            return Err(format!("Received jsonrpc unsupported message: {m:?}"));
                        }
                    },
                    Err(e) => {
                        return Err(format!("Failed deserializing ws message as jsonrpc: {e}"));
                    }
                };

                // Resolve the pending execution with this response
                state
                    .ws_manager
                    .handle_execution_response(id, exec_res)
                    .await
                    .map_err(|()| "Failed to resolve execution".to_string())?;
            }

            Ok(())
        }
        Message::Binary(_) => {
            warn!("Received binary message, ignoring");
            Ok(())
        }
        Message::Close(_) => {
            info!("Received close message for session {ws_session}");
            println!("CLOSING....");
            Ok(())
        }
        Message::Ping(_) | Message::Pong(_) => Ok(()),
    }
}
