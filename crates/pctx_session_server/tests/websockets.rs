//! Unit tests for WebSocket handler

mod utils;

use axum_test::WsMessage;
use similar_asserts::assert_eq;
use uuid::Uuid;

use crate::utils::{
    connect_websocket, create_session, create_test_server, create_test_server_with_session,
};

/// Tests opening a websocket connection returns a connected message with a session id
#[tokio::test]
async fn test_websocket_connection() {
    let (session_id, server, state) = create_test_server_with_session().await;
    let _ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    assert_eq!(state.ws_manager.list_sessions().await.len(), 1);
}

/// Tests opening a websocket connection with a non-existent code mode session
#[tokio::test]
async fn test_websocket_connection_invalid_session() {
    let (server, state) = create_test_server();
    let session_id = Uuid::new_v4();
    let res = connect_websocket(&server, session_id).await;

    res.assert_status_not_found();
    res.assert_text(format!("Code mode session {session_id} not found"));
    assert!(state.ws_manager.list_sessions().await.is_empty());
}

/// Tests opening a two websockets for the same code mode session fails
#[tokio::test]
async fn test_websocket_double_connection() {
    let (session_id, server, state) = create_test_server_with_session().await;
    let _ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;
    let second = connect_websocket(&server, session_id).await;

    second.assert_status_bad_request();
    second.assert_text(format!(
        "Code mode session {session_id} already has an active WebSocket connection"
    ));
    assert_eq!(state.ws_manager.list_sessions().await.len(), 1);
}

#[tokio::test]
async fn test_websocket_different_connections() {
    let (server, state) = create_test_server();
    let session_1 = create_session(&server).await;
    let session_2 = create_session(&server).await;

    let _ws_1 = connect_websocket(&server, session_1)
        .await
        .into_websocket()
        .await;

    let _ws_2 = connect_websocket(&server, session_2)
        .await
        .into_websocket()
        .await;

    // confirm they both get the created session message with different ids
    assert_eq!(state.ws_manager.list_sessions().await.len(), 2);
}

#[tokio::test]
async fn test_websocket_ping_pong() {
    let (session_id, server, _state) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

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
    let (session_id, server, state) = create_test_server_with_session().await;
    let ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    assert_eq!(state.ws_manager.list_sessions().await.len(), 1);

    // Send close message
    ws.close().await;

    // wait for close
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    assert!(state.ws_manager.list_sessions().await.is_empty());
}
