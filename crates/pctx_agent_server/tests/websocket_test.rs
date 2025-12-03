//! Unit tests for WebSocket handler

use futures::{SinkExt, StreamExt};
use pctx_agent_server::AppState;
use pctx_code_mode::CodeMode;
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Helper to create test app state
async fn create_test_state() -> (Uuid, AppState) {
    let state = AppState::new();
    let session_id = Uuid::new_v4();
    state
        .code_mode_manager
        .set(session_id, CodeMode::default())
        .await;

    (session_id, state)
}

/// Helper to start test server and return the URL
async fn start_test_server() -> String {
    use axum::{Router, routing::get};
    use pctx_agent_server::websocket;

    let (session_id, state) = create_test_state().await;

    let app = Router::new()
        .route("/ws", get(websocket::ws_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    format!(
        "ws://127.0.0.1:{}/ws?code_mode_session_id={session_id}",
        addr.port()
    )
}

#[tokio::test]
async fn test_websocket_connection() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut _write, mut read) = ws_stream.split();

    // Should receive session_created notification
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let notification: serde_json::Value =
            serde_json::from_str(&text).expect("Failed to parse notification");

        assert_eq!(notification["method"], "session_created");
        assert!(notification["params"].is_object());
        assert!(notification["params"]["session_id"].is_string());
    } else {
        panic!("Expected session_created notification");
    }
}

#[tokio::test]
async fn test_websocket_session_id_format() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut _write, mut read) = ws_stream.split();

    // Should receive session_created notification
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let notification: serde_json::Value =
            serde_json::from_str(&text).expect("Failed to parse notification");

        let session_id = notification["params"]["session_id"]
            .as_str()
            .expect("session_id should be a string");

        // Session ID should be a valid UUID
        assert!(uuid::Uuid::parse_str(session_id).is_ok());
    } else {
        panic!("Expected session_created notification");
    }
}

#[tokio::test]
async fn test_websocket_multiple_connections() {
    let url = start_test_server().await;

    // Connect first client
    let (ws_stream1, _) = connect_async(&url)
        .await
        .expect("Failed to connect client 1");
    let (mut _write1, mut read1) = ws_stream1.split();

    // Connect second client should fail as there should only be one callback ws per session
    let ws_2 = connect_async(&url).await;

    // First should receive session_created with an ID
    let msg1 = read1.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg1 {
        let notification: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(notification["params"]["session_id"].as_str().is_some())
    } else {
        panic!("Expected text message");
    };

    // second should be an error
    assert!(ws_2.is_err());
}

#[tokio::test]
async fn test_websocket_tool_execution_response() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Receive session_created
    let _session_msg = read.next().await.unwrap().unwrap();

    // Send a tool execution response
    let response = json!({
        "jsonrpc": "2.0",
        "id": "test-123",
        "result": { "data": "test data" }
    });

    write
        .send(Message::Text(
            serde_json::to_string(&response).unwrap().into(),
        ))
        .await
        .expect("Failed to send message");

    // The response should be processed without error
    // (in a real scenario, this would resolve a pending execution)
}

#[tokio::test]
async fn test_websocket_error_response() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Receive session_created
    let _session_msg = read.next().await.unwrap().unwrap();

    // Send an error response
    let response = json!({
        "jsonrpc": "2.0",
        "id": "test-456",
        "error": {
            "code": -32000,
            "message": "Tool execution failed",
            "data": { "details": "Something went wrong" }
        }
    });

    write
        .send(Message::Text(
            serde_json::to_string(&response).unwrap().into(),
        ))
        .await
        .expect("Failed to send message");

    // The error response should be processed
}

#[tokio::test]
async fn test_websocket_ping_pong() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Receive session_created
    let _session_msg = read.next().await.unwrap().unwrap();

    // Send a ping
    write
        .send(Message::Ping(vec![1, 2, 3].into()))
        .await
        .expect("Failed to send ping");

    // Server should respond with pong (though we might not receive it explicitly in this test)
    // Just verify the connection is still alive
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_websocket_close() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Receive session_created
    let _session_msg = read.next().await.unwrap().unwrap();

    // Send close message
    write
        .send(Message::Close(None))
        .await
        .expect("Failed to send close");

    // Connection should close
    let next_msg = read.next().await;
    assert!(next_msg.is_none() || matches!(next_msg, Some(Ok(Message::Close(_)))));
}

#[tokio::test]
async fn test_websocket_invalid_json() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut write, mut _read) = ws_stream.split();

    // Send invalid JSON
    write
        .send(Message::Text("not valid json".to_string().into()))
        .await
        .expect("Failed to send message");

    // Server should handle the error gracefully without crashing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_websocket_response_missing_id() {
    let url = start_test_server().await;

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Receive session_created
    let _session_msg = read.next().await.unwrap().unwrap();

    // Send a response without an id field
    let response = json!({
        "jsonrpc": "2.0",
        "result": { "data": "test" }
    });

    write
        .send(Message::Text(
            serde_json::to_string(&response).unwrap().into(),
        ))
        .await
        .expect("Failed to send message");

    // Server should handle this gracefully
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
}
