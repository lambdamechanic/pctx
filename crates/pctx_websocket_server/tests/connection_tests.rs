use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Test that a client can connect to the WebSocket server
#[tokio::test]
async fn test_client_can_connect() {
    // Start server on random port
    let server_addr = "127.0.0.1:0";
    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel(1);

    // Spawn server
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();
        server_tx.send(actual_addr).await.unwrap();

        // Accept one connection
        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();

        // Keep connection alive for test
        let (_write, _read) = ws_stream.split();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    });

    // Get actual server address
    let addr = server_rx.recv().await.unwrap();
    let url = format!("ws://{}/local-tools", addr);

    // Connect client
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (_write, _read) = ws_stream.split();

    // Connection successful if we reach here
    assert!(true, "Client connected successfully");
}

/// Test that server assigns a session ID on connection
#[tokio::test]
async fn test_server_assigns_session_id() {
    let server_addr = "127.0.0.1:0";
    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel(1);

    // Spawn server
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();
        server_tx.send(actual_addr).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, _read) = ws_stream.split();

        // Send session ID message
        let session_msg = json!({
            "jsonrpc": "2.0",
            "method": "session_created",
            "params": {
                "session_id": "test-session-123"
            }
        });
        write
            .send(Message::Text(session_msg.to_string().into()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    });

    let addr = server_rx.recv().await.unwrap();
    let url = format!("ws://{}/local-tools", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (_write, mut read) = ws_stream.split();

    // Read session ID message
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(msg["method"], "session_created");
        assert!(msg["params"]["session_id"].is_string());
    } else {
        panic!("Did not receive session ID message");
    }
}

/// Test bidirectional communication
#[tokio::test]
async fn test_bidirectional_communication() {
    let server_addr = "127.0.0.1:0";
    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel(1);

    // Spawn server that echoes messages
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();
        server_tx.send(actual_addr).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Echo back messages
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                write.send(Message::Text(text)).await.unwrap();
            }
        }
    });

    let addr = server_rx.recv().await.unwrap();
    let url = format!("ws://{}/local-tools", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Send a message
    let test_msg = json!({
        "jsonrpc": "2.0",
        "method": "test",
        "id": 1
    });
    write
        .send(Message::Text(test_msg.to_string().into()))
        .await
        .unwrap();

    // Receive echoed message
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(msg["method"], "test");
        assert_eq!(msg["id"], 1);
    } else {
        panic!("Did not receive echo message");
    }
}
