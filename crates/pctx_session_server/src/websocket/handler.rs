use std::sync::Arc;

use crate::{
    extractors::CodeModeSession,
    model::{
        ExecuteCodeParams, ExecuteToolParams, PctxJsonRpcRequest, PctxJsonRpcResponse,
        WsJsonRpcMessage,
    },
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
use pctx_code_execution_runtime::{CallbackFn, CallbackRegistry};
use rmcp::{
    ErrorData,
    model::{ErrorCode, JsonRpcMessage, RequestId},
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

    info!("Verified code mode session {code_mode_session} exists, proceeding with WebSocket setup");

    // Split socket into sender and receiver
    let (sender, receiver) = socket.split();

    // Create an in-process channel for outgoing messages - convert OutgoingMessage to WebSocket Message
    let (tx, rx) = mpsc::unbounded_channel::<WsJsonRpcMessage>();

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
    mut rx: mpsc::UnboundedReceiver<WsJsonRpcMessage>,
) {
    while let Some(msg) = rx.recv().await {
        if let Err(e) = sender
            .send(Message::Text(json!(msg).to_string().into()))
            .await
        {
            error!("Error sending WebSocket message: {e}");
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

/// Handle an `execute_code` request from the client
async fn handle_execute_code_request(
    req_id: RequestId,
    params: ExecuteCodeParams,
    ws_session: Uuid,
    state: &AppState,
) -> Result<(), String> {
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

    // Get the relevant CodeMode config for the session
    let Some(code_mode_lock) = state.code_mode_manager.get(code_mode_session_id).await else {
        let err_res = WsJsonRpcMessage::error(
            ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: format!("CodeMode session `{code_mode_session_id}` does not exist").into(),
                data: None,
            },
            req_id,
        );
        let _ = sender.send(err_res);
        return Ok(());
    };

    debug!("Found CodeMode session with ID: {code_mode_session_id}");

    let callback_registry = CallbackRegistry::default();
    let code_mode_lock_clone = code_mode_lock.clone();
    let code_mode_read = code_mode_lock_clone.read().await;

    for callback_cfg in &code_mode_read.callbacks {
        let ws_session_lock_clone = ws_session_lock.clone();
        let cfg = callback_cfg.clone();

        let callback: CallbackFn = Arc::new(move |args: Option<serde_json::Value>| {
            let cfg = cfg.clone();
            let ws_session_lock_clone = ws_session_lock_clone.clone();

            Box::pin(async move {
                let ws_session = ws_session_lock_clone.read().await;

                let callback_res = ws_session
                    .execute_callback(ExecuteToolParams {
                        namespace: cfg.namespace,
                        name: cfg.name,
                        args,
                    })
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(json!(callback_res.output))
            })
        });

        if let Err(add_err) = callback_registry.add(&callback_cfg.id(), callback) {
            let err_res = WsJsonRpcMessage::error(
                ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!(
                        "Failed adding callback `{}` to registry: {add_err}",
                        callback_cfg.id()
                    )
                    .into(),
                    data: None,
                },
                req_id.clone(),
            );
            let _ = sender.send(err_res);
        }
    }

    tokio::spawn(async move {
        let current_span = tracing::Span::current();
        let output = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
            let _guard = current_span.enter();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;

            // create callback registry to execute callback requests over the same ws which
            // initiated the request
            rt.block_on(async {
                code_mode_lock
                    .read()
                    .await
                    .execute(&params.code, Some(callback_registry))
                    .await
                    .map_err(|e| anyhow::anyhow!("Execution error: {e}"))
            })
        })
        .await;

        let msg = match output {
            Ok(Ok(exec_output)) => {
                WsJsonRpcMessage::response(PctxJsonRpcResponse::ExecuteCode(exec_output), req_id)
            }
            Ok(Err(e)) => WsJsonRpcMessage::error(
                ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Execution failed: {e}").into(),
                    data: None,
                },
                req_id,
            ),
            Err(e) => WsJsonRpcMessage::error(
                ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Task join failed: {e}").into(),
                    data: None,
                },
                req_id,
            ),
        };

        if let Err(e) = sender.send(msg) {
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

            let jrpc_msg = serde_json::from_str::<WsJsonRpcMessage>(&text)
                .map_err(|e| format!("Received invalid JsonRpc message from websocket: {e}"))?;

            match jrpc_msg {
                JsonRpcMessage::Request(req) => match req.request {
                    PctxJsonRpcRequest::ExecuteCode { params } => {
                        debug!("Executing code...");
                        handle_execute_code_request(req.id, params, ws_session, state).await
                    }
                    PctxJsonRpcRequest::ExecuteTool { .. } => {
                        // the server is only responsible for servicing execute_code requests, execute_tool
                        // is handled by the client
                        Err(format!("Received unsupported JsonRpc request: {text}"))
                    }
                },
                JsonRpcMessage::Response(res) => match res.result {
                    PctxJsonRpcResponse::ExecuteTool(result) => state
                        .ws_manager
                        .handle_execute_callback_response(res.id, Ok(result))
                        .await
                        .map_err(|()| "Failed to handle execute callback response".to_string()),
                    PctxJsonRpcResponse::ExecuteCode(_) => {
                        // the server is only responsible for handling execute_tool responses, execute_tool
                        // responses should be sent to the client
                        Err(format!("Received unsupported JsonRpc response: {text}"))
                    }
                },
                JsonRpcMessage::Error(err_msg) => state
                    .ws_manager
                    .handle_execute_callback_response(err_msg.id, Err(err_msg.error))
                    .await
                    .map_err(|()| "Failed to handle execute callback response".to_string()),
                JsonRpcMessage::Notification(_) => {
                    info!("Received JsonRpc Notification: {text}");
                    Ok(())
                }
            }
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
