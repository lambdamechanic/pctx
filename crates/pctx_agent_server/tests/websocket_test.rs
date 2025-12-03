//! Unit tests for WebSocket handler

mod utils;

use axum_test::WsMessage;
use serde_json::json;
use similar_asserts::assert_eq;
use utils::create_test_server_with_session;
use uuid::Uuid;

use crate::utils::{connect_websocket, create_test_server};

/// Tests opening a websocket connection returns a connected message with a session id
#[tokio::test]
async fn test_websocket_connection() {
    let (session_id, server) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    ws.assert_receive_json_contains(&json!({
        "jsonrpc": "2.0",
        "method": "websocket_session_created",
    }))
    .await;
}

/// Tests opening a websocket connection with a non-existent code mode session
#[tokio::test]
async fn test_websocket_connection_invalid_session() {
    let server = create_test_server();
    let session_id = Uuid::new_v4();
    let res = connect_websocket(&server, session_id).await;

    res.assert_status_bad_request();
    res.assert_text(format!("Code mode session {session_id} not found"));
}

/// Tests opening a two websockets for the same code mode session fails
#[tokio::test]
async fn test_websocket_double_connection() {
    let (session_id, server) = create_test_server_with_session().await;
    let _ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;
    let second = connect_websocket(&server, session_id).await;

    second.assert_status_bad_request();
    second.assert_text(format!(
        "Code mode session {session_id} already has an active WebSocket connection"
    ));
}

#[tokio::test]
async fn test_websocket_different_connections() {
    todo!("waiting for connect endpoint")
}

#[tokio::test]
async fn test_websocket_ping_pong() {
    let (session_id, server) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    // Receive websocket_session_created
    let _session_msg = ws.receive_message().await;

    let gump = vec![1, 2, 3];
    ws.send_message(WsMessage::Ping(gump.clone().into())).await;
    let expect_pong = ws.receive_message().await;
    if let WsMessage::Pong(val) = expect_pong {
        assert_eq!(gump, val);
    } else {
        panic!("didn't receive pong")
    }
}

#[tokio::test]
async fn test_websocket_close() {
    let (session_id, server) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    // Receive session_created
    let _session_msg = ws.receive_message().await;

    // Send close message
    ws.close().await;

    // TODO: check app state
}
