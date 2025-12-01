use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Test that a client can register a tool successfully
#[tokio::test]
async fn test_client_registers_tool() {
    let server_addr = "127.0.0.1:0";
    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel(1);

    // Spawn server
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();
        server_tx.send(actual_addr).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Handle register_tool request
        while let Some(Ok(Message::Text(text))) = read.next().await {
            let request: serde_json::Value = serde_json::from_str(&text).unwrap();

            if request["method"] == "register_tool" {
                let response = json!({
                    "jsonrpc": "2.0",
                    "result": { "success": true },
                    "id": request["id"]
                });
                write
                    .send(Message::Text(response.to_string().into()))
                    .await
                    .unwrap();
            }
        }
    });

    let addr = server_rx.recv().await.unwrap();
    let url = format!("ws://{}/local-tools", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Send register_tool request
    let register_msg = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "TestTools",
            "name": "getData",
            "description": "Gets data",
            "input_schema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number" }
                }
            },
            "output_schema": {
                "type": "object"
            }
        },
        "id": 1
    });

    write
        .send(Message::Text(register_msg.to_string().into()))
        .await
        .unwrap();

    // Receive response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["success"], true);
    } else {
        panic!("Did not receive registration response");
    }
}

/// Test that multiple clients can register different tools
#[tokio::test]
async fn test_multiple_clients_register_different_tools() {
    let server_addr = "127.0.0.1:0";
    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel(1);

    // Track registered tools
    let registered_tools = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let registered_tools_clone = registered_tools.clone();

    // Spawn server
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();
        server_tx.send(actual_addr).await.unwrap();

        // Accept two connections
        for _ in 0..2 {
            let tools = registered_tools_clone.clone();
            let (stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut write, mut read) = ws_stream.split();

                while let Some(Ok(Message::Text(text))) = read.next().await {
                    let request: serde_json::Value = serde_json::from_str(&text).unwrap();

                    if request["method"] == "register_tool" {
                        let tool_name = format!(
                            "{}.{}",
                            request["params"]["namespace"].as_str().unwrap(),
                            request["params"]["name"].as_str().unwrap()
                        );
                        tools.lock().await.push(tool_name);

                        let response = json!({
                            "jsonrpc": "2.0",
                            "result": { "success": true },
                            "id": request["id"]
                        });
                        write
                            .send(Message::Text(response.to_string().into()))
                            .await
                            .unwrap();
                    }
                }
            });
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    });

    let addr = server_rx.recv().await.unwrap();
    let url = format!("ws://{}/local-tools", addr);

    // Connect first client
    let (ws_stream1, _) = connect_async(&url)
        .await
        .expect("Failed to connect client 1");
    let (mut write1, mut read1) = ws_stream1.split();

    // Connect second client
    let (ws_stream2, _) = connect_async(&url)
        .await
        .expect("Failed to connect client 2");
    let (mut write2, mut read2) = ws_stream2.split();

    // Register tool from client 1
    let register_msg1 = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "Client1",
            "name": "toolA"
        },
        "id": 1
    });
    write1
        .send(Message::Text(register_msg1.to_string().into()))
        .await
        .unwrap();

    // Register tool from client 2
    let register_msg2 = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "Client2",
            "name": "toolB"
        },
        "id": 2
    });
    write2
        .send(Message::Text(register_msg2.to_string().into()))
        .await
        .unwrap();

    // Wait for responses
    let _ = read1.next().await;
    let _ = read2.next().await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify both tools were registered
    let tools = registered_tools.lock().await;
    assert_eq!(tools.len(), 2);
    assert!(tools.contains(&"Client1.toolA".to_string()));
    assert!(tools.contains(&"Client2.toolB".to_string()));
}

/// Test that duplicate tool registration fails
#[tokio::test]
async fn test_duplicate_tool_registration_fails() {
    let server_addr = "127.0.0.1:0";
    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel(1);

    // Track registered tools
    let registered_tools =
        std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::new()));
    let registered_tools_clone = registered_tools.clone();

    // Spawn server
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();
        server_tx.send(actual_addr).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        while let Some(Ok(Message::Text(text))) = read.next().await {
            let request: serde_json::Value = serde_json::from_str(&text).unwrap();

            if request["method"] == "register_tool" {
                let tool_name = format!(
                    "{}.{}",
                    request["params"]["namespace"].as_str().unwrap(),
                    request["params"]["name"].as_str().unwrap()
                );

                let mut tools = registered_tools_clone.lock().await;
                let response = if tools.contains(&tool_name) {
                    json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32000,
                            "message": "Tool already registered"
                        },
                        "id": request["id"]
                    })
                } else {
                    tools.insert(tool_name);
                    json!({
                        "jsonrpc": "2.0",
                        "result": { "success": true },
                        "id": request["id"]
                    })
                };

                write
                    .send(Message::Text(response.to_string().into()))
                    .await
                    .unwrap();
            }
        }
    });

    let addr = server_rx.recv().await.unwrap();
    let url = format!("ws://{}/local-tools", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Register tool first time
    let register_msg = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "TestTools",
            "name": "getData"
        },
        "id": 1
    });
    write
        .send(Message::Text(register_msg.to_string().into()))
        .await
        .unwrap();

    // Receive success response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["result"]["success"], true);
    }

    // Try to register same tool again
    let register_msg2 = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "TestTools",
            "name": "getData"
        },
        "id": 2
    });
    write
        .send(Message::Text(register_msg2.to_string().into()))
        .await
        .unwrap();

    // Receive error response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["message"], "Tool already registered");
    } else {
        panic!("Did not receive error response for duplicate registration");
    }
}
