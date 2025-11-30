use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use pctx_websocket_server::LocalToolsServer;

/// Test that server can execute a tool on the client
#[tokio::test]
async fn test_server_executes_tool_on_client() {
    // Start actual server
    let server = LocalToolsServer::new();
    let session_manager = server.session_manager();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn server
    let app = server.router();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let url = format!("ws://{}/local-tools", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Skip session_created notification
    let _ = read.next().await;

    // Register a tool
    let register_msg = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "TestTools",
            "name": "multiply",
            "description": "Multiplies two numbers"
        },
        "id": 1
    });
    write
        .send(Message::Text(register_msg.to_string().into()))
        .await
        .unwrap();

    // Read registration response
    let _ = read.next().await;

    // Server executes the tool
    let execution_result = tokio::spawn(async move {
        session_manager
            .execute_tool(
                "TestTools.multiply",
                Some(json!({ "a": 5, "b": 3 })),
                json!(100),
            )
            .await
    });

    // Client receives execution request
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let request: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify it's an execute_tool request
        assert_eq!(request["method"], "execute_tool");
        assert_eq!(request["params"]["name"], "TestTools.multiply");
        assert_eq!(request["params"]["arguments"]["a"], 5);
        assert_eq!(request["params"]["arguments"]["b"], 3);

        // Client executes the tool and returns result
        let result = json!({ "result": 15 });
        let response = json!({
            "jsonrpc": "2.0",
            "result": result,
            "id": request["id"]
        });
        write
            .send(Message::Text(response.to_string().into()))
            .await
            .unwrap();
    } else {
        panic!("Did not receive execution request");
    }

    // Verify server received the result
    let result = execution_result.await.unwrap().unwrap();
    assert_eq!(result["result"], 15);
}

/// Test multiple concurrent tool executions
#[tokio::test]
async fn test_concurrent_tool_executions() {
    let server = LocalToolsServer::new();
    let session_manager = server.session_manager();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = server.router();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/local-tools", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Skip session_created
    let _ = read.next().await;

    // Register two tools
    for tool_name in ["add", "subtract"] {
        let register_msg = json!({
            "jsonrpc": "2.0",
            "method": "register_tool",
            "params": {
                "namespace": "Math",
                "name": tool_name
            },
            "id": 1
        });
        write
            .send(Message::Text(register_msg.to_string().into()))
            .await
            .unwrap();
        let _ = read.next().await;
    }

    // Spawn multiple executions
    let sm1 = session_manager.clone();
    let sm2 = session_manager.clone();

    let exec1 = tokio::spawn(async move {
        sm1.execute_tool("Math.add", Some(json!({ "a": 10, "b": 5 })), json!(200))
            .await
    });

    let exec2 = tokio::spawn(async move {
        sm2.execute_tool(
            "Math.subtract",
            Some(json!({ "a": 10, "b": 3 })),
            json!(201),
        )
        .await
    });

    // Handle both execution requests
    for _ in 0..2 {
        if let Some(Ok(Message::Text(text))) = read.next().await {
            let request: serde_json::Value = serde_json::from_str(&text).unwrap();
            let tool_name = request["params"]["name"].as_str().unwrap();
            let args = &request["params"]["arguments"];

            let result_value = if tool_name == "Math.add" {
                args["a"].as_i64().unwrap() + args["b"].as_i64().unwrap()
            } else {
                args["a"].as_i64().unwrap() - args["b"].as_i64().unwrap()
            };

            let response = json!({
                "jsonrpc": "2.0",
                "result": { "value": result_value },
                "id": request["id"]
            });

            write
                .send(Message::Text(response.to_string().into()))
                .await
                .unwrap();
        }
    }

    // Verify both executions completed
    let result1 = exec1.await.unwrap().unwrap();
    let result2 = exec2.await.unwrap().unwrap();

    assert_eq!(result1["value"], 15);
    assert_eq!(result2["value"], 7);
}

/// Test execution timeout
#[tokio::test]
async fn test_tool_execution_timeout() {
    let server = LocalToolsServer::new();
    let session_manager = server.session_manager();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = server.router();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/local-tools", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Skip session_created
    let _ = read.next().await;

    // Register tool
    let register_msg = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "SlowTools",
            "name": "slowOp"
        },
        "id": 1
    });
    write
        .send(Message::Text(register_msg.to_string().into()))
        .await
        .unwrap();
    let _ = read.next().await;

    // Execute tool but don't respond
    let execution = tokio::spawn(async move {
        session_manager
            .execute_tool("SlowTools.slowOp", None, json!(300))
            .await
    });

    // Receive request but don't respond (simulating slow client)
    let _ = read.next().await;

    // Wait for timeout
    let result = execution.await.unwrap();

    // Should timeout
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), "Execution timeout");
}

/// Test execution with error response from client
#[tokio::test]
async fn test_tool_execution_error_from_client() {
    let server = LocalToolsServer::new();
    let session_manager = server.session_manager();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = server.router();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/local-tools", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Skip session_created
    let _ = read.next().await;

    // Register tool
    let register_msg = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "ErrorTools",
            "name": "failOp"
        },
        "id": 1
    });
    write
        .send(Message::Text(register_msg.to_string().into()))
        .await
        .unwrap();
    let _ = read.next().await;

    // Execute tool
    let execution = tokio::spawn(async move {
        session_manager
            .execute_tool("ErrorTools.failOp", None, json!(400))
            .await
    });

    // Client receives request and returns error
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let request: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Return error response
        let error_response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32000,
                "message": "Tool execution failed: division by zero"
            },
            "id": request["id"]
        });

        write
            .send(Message::Text(error_response.to_string().into()))
            .await
            .unwrap();
    }

    // Verify server received the error
    let result = execution.await.unwrap();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Tool execution failed"));
}

/// Test async tool execution (client with async callback)
#[tokio::test]
async fn test_async_tool_execution() {
    let server = LocalToolsServer::new();
    let session_manager = server.session_manager();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = server.router();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/local-tools", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Skip session_created
    let _ = read.next().await;

    // Register async tool
    let register_msg = json!({
        "jsonrpc": "2.0",
        "method": "register_tool",
        "params": {
            "namespace": "AsyncTools",
            "name": "fetchData",
            "description": "Fetches data asynchronously"
        },
        "id": 1
    });
    write
        .send(Message::Text(register_msg.to_string().into()))
        .await
        .unwrap();
    let _ = read.next().await;

    // Execute async tool
    let execution = tokio::spawn(async move {
        session_manager
            .execute_tool(
                "AsyncTools.fetchData",
                Some(json!({ "url": "https://api.example.com/data" })),
                json!(500),
            )
            .await
    });

    // Client simulates async operation
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let request: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Simulate async delay
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Return async result
        let response = json!({
            "jsonrpc": "2.0",
            "result": {
                "data": "fetched data",
                "status": "success"
            },
            "id": request["id"]
        });

        write
            .send(Message::Text(response.to_string().into()))
            .await
            .unwrap();
    }

    // Verify async execution completed
    let result = execution.await.unwrap().unwrap();
    assert_eq!(result["data"], "fetched data");
    assert_eq!(result["status"], "success");
}

/// Test tool execution with tool not found
#[tokio::test]
async fn test_execute_nonexistent_tool() {
    let server = LocalToolsServer::new();
    let session_manager = server.session_manager();

    // Try to execute tool that doesn't exist
    let result = session_manager
        .execute_tool("NonExistent.tool", None, json!(600))
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), "Tool not found");
}
