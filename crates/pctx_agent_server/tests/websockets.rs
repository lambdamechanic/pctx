//! Unit tests for WebSocket handler

mod utils;

use axum_test::WsMessage;
use serde_json::json;
use similar_asserts::{assert_eq, assert_serde_eq};
use uuid::Uuid;

use crate::utils::{
    connect_websocket, create_session, create_test_server, create_test_server_with_session,
};

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
    let server = create_test_server();
    let session_1 = create_session(&server).await;
    let session_2 = create_session(&server).await;

    let mut ws_1 = connect_websocket(&server, session_1)
        .await
        .into_websocket()
        .await;

    let mut ws_2 = connect_websocket(&server, session_2)
        .await
        .into_websocket()
        .await;

    // confirm they both get the created session message with different ids
    let msg_1: serde_json::Value = ws_1.receive_json().await;
    let msg_2: serde_json::Value = ws_2.receive_json().await;

    assert_serde_eq!(msg_1["method"], "websocket_session_created");
    assert_serde_eq!(msg_2["method"], "websocket_session_created");

    let id_1 = msg_1["params"]["session_id"].clone();
    let id_2 = msg_2["params"]["session_id"].clone();
    assert!(id_1.is_string());
    assert!(id_2.is_string());
    assert_ne!(id_1, id_2);
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
