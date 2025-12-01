use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use serial_test::serial;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use pctx_websocket_server::LocalToolsServer;

/// Create a code executor for testing (without session manager)
fn create_code_executor() -> pctx_websocket_server::CodeExecutorFn {
    let code_mode = pctx_code_mode::CodeMode::default();
    code_mode.as_code_executor()
}

/// Test basic code execution via WebSocket
#[tokio::test]
#[serial]
async fn test_basic_code_execution() {
    let server = LocalToolsServer::with_code_executor(create_code_executor());
    let _session_manager = server.session_manager();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn server
    let app = server.router();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let url = format!("ws://{}/local-tools", addr);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Skip session_created notification
    let _ = read.next().await;

    // Send code execution request
    let execute_msg = json!({
        "jsonrpc": "2.0",
        "method": "execute",
        "params": {
            "code": "async function run() { console.log('Hello from test'); return 1 + 1; }"
        },
        "id": 1
    });

    write
        .send(Message::Text(execute_msg.to_string().into()))
        .await
        .unwrap();

    // Receive execution result
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_object());

        // Should have result value
        assert_eq!(response["result"]["value"], 2);

        // Should have stdout
        assert!(response["result"]["stdout"].is_string());
        let stdout = response["result"]["stdout"].as_str().unwrap();
        assert!(stdout.contains("Hello from test"));
    } else {
        panic!("Did not receive execution result");
    }
}

/// Test code execution with syntax error
#[tokio::test]
#[serial]
async fn test_code_execution_syntax_error() {
    let server = LocalToolsServer::with_code_executor(create_code_executor());

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

    // Send invalid code
    let execute_msg = json!({
        "jsonrpc": "2.0",
        "method": "execute",
        "params": {
            "code": "async function run() { this is not valid javascript { }"
        },
        "id": 2
    });

    write
        .send(Message::Text(execute_msg.to_string().into()))
        .await
        .unwrap();

    // Receive error response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["id"], 2);
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["code"], -32002); // EXECUTION_FAILED

        let error_msg = response["error"]["message"].as_str().unwrap();
        // Type check errors contain "Expected" in the message
        assert!(
            error_msg.contains("Expected")
                || error_msg.contains("Syntax")
                || error_msg.contains("Parse")
        );
    } else {
        panic!("Did not receive error response");
    }
}

/// Test code execution with runtime error
#[tokio::test]
#[serial]
async fn test_code_execution_runtime_error() {
    let server = LocalToolsServer::with_code_executor(create_code_executor());

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

    // Send code that throws runtime error
    let execute_msg = json!({
        "jsonrpc": "2.0",
        "method": "execute",
        "params": {
            "code": "async function run(): Promise<void> { throw new Error('Test error'); }"
        },
        "id": 3
    });

    write
        .send(Message::Text(execute_msg.to_string().into()))
        .await
        .unwrap();

    // Receive error response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["id"], 3);
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["code"], -32002); // EXECUTION_FAILED

        let error_msg = response["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("Test error"));
    } else {
        panic!("Did not receive error response");
    }
}

/// Test code execution that calls local tools
// #[tokio::test]
// #[serial]
// async fn test_code_execution_with_local_tools() {
//     // Wire up session manager with code executor that can call WebSocket tools
//     let session_manager = std::sync::Arc::new(pctx_websocket_server::SessionManager::new());
//     let code_executor =
//         create_code_mode().as_code_executor_with_session_manager(Some(session_manager.clone()));
//     session_manager.set_code_executor(code_executor);
//     let server = LocalToolsServer::with_session_manager(session_manager);

//     let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
//     let addr = listener.local_addr().unwrap();

//     let app = server.router();
//     tokio::spawn(async move {
//         axum::serve(listener, app).await.unwrap();
//     });

//     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

//     let url = format!("ws://{}/local-tools", addr);
//     let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
//     let (mut write, mut read) = ws_stream.split();

//     // Skip session_created
//     let _ = read.next().await;

//     // Register a tool
//     let register_msg = json!({
//         "jsonrpc": "2.0",
//         "method": "register_tool",
//         "params": {
//             "namespace": "MyMath",
//             "name": "square",
//             "description": "Squares a number"
//         },
//         "id": 1
//     });
//     write
//         .send(Message::Text(register_msg.to_string().into()))
//         .await
//         .unwrap();

//     // Read registration response
//     let _ = read.next().await;

//     // Execute code that calls the local tool
//     let execute_msg = json!({
//         "jsonrpc": "2.0",
//         "method": "execute",
//         "params": {
//             "code": r#"
//                 async function run() {
//                     const result = await MyMath.square({ value: 5 });
//                     return result.squared;
//                 }
//             "#
//         },
//         "id": 2
//     });

//     write
//         .send(Message::Text(execute_msg.to_string().into()))
//         .await
//         .unwrap();

//     // Server will request tool execution from client
//     if let Some(Ok(Message::Text(text))) = read.next().await {
//         eprintln!("Received execute_tool request: {}", text);
//         let request: serde_json::Value = serde_json::from_str(&text).unwrap();

//         // Should be execute_tool request
//         assert_eq!(request["method"], "execute_tool");
//         assert_eq!(request["params"]["name"], "MyMath.square");
//         assert_eq!(request["params"]["arguments"]["value"], 5);

//         // Client executes tool and returns result
//         let tool_response = json!({
//             "jsonrpc": "2.0",
//             "result": { "squared": 25 },
//             "id": request["id"]
//         });

//         eprintln!("Sending tool response: {}", tool_response.to_string());
//         write
//             .send(Message::Text(tool_response.to_string().into()))
//             .await
//             .unwrap();
//     } else {
//         panic!("Did not receive execute_tool request");
//     }

//     // Receive final code execution result
//     if let Some(Ok(Message::Text(text))) = read.next().await {
//         eprintln!("Received final execution result: {}", text);
//         let response: serde_json::Value = serde_json::from_str(&text).unwrap();
//         eprintln!("Parsed response: {:?}", response);
//         assert_eq!(response["id"], 2);
//         eprintln!("response[\"result\"]: {:?}", response["result"]);
//         eprintln!(
//             "response[\"result\"][\"value\"]: {:?}",
//             response["result"]["value"]
//         );
//         assert_eq!(response["result"]["value"], 25);
//     } else {
//         panic!("Did not receive execution result");
//     }
// }

/// Test code execution with console output capture
#[tokio::test]
#[serial]
async fn test_code_execution_console_capture() {
    let server = LocalToolsServer::with_code_executor(create_code_executor());

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

    // Execute code with console output
    let execute_msg = json!({
        "jsonrpc": "2.0",
        "method": "execute",
        "params": {
            "code": r#"
                async function run() {
                    console.log("Line 1");
                    console.log("Line 2");
                    console.error("Error line");
                    return "done";
                }
            "#
        },
        "id": 5
    });

    write
        .send(Message::Text(execute_msg.to_string().into()))
        .await
        .unwrap();

    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["id"], 5);
        assert_eq!(response["result"]["value"], "done");

        let stdout = response["result"]["stdout"].as_str().unwrap();
        assert!(stdout.contains("Line 1"));
        assert!(stdout.contains("Line 2"));

        let stderr = response["result"]["stderr"].as_str().unwrap();
        assert!(stderr.contains("Error line"));
    } else {
        panic!("Did not receive execution result");
    }
}

/// Test concurrent code execution requests
#[tokio::test]
#[serial]
async fn test_concurrent_code_execution() {
    let server = LocalToolsServer::with_code_executor(create_code_executor());

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

    // Send two execution requests
    let execute_msg1 = json!({
        "jsonrpc": "2.0",
        "method": "execute",
        "params": {
            "code": "async function run() { return 10 + 5; }"
        },
        "id": 10
    });

    let execute_msg2 = json!({
        "jsonrpc": "2.0",
        "method": "execute",
        "params": {
            "code": "async function run() { return 20 * 2; }"
        },
        "id": 11
    });

    write
        .send(Message::Text(execute_msg1.to_string().into()))
        .await
        .unwrap();

    write
        .send(Message::Text(execute_msg2.to_string().into()))
        .await
        .unwrap();

    // Collect both responses
    let mut responses = vec![];
    for _ in 0..2 {
        if let Some(Ok(Message::Text(text))) = read.next().await {
            let response: serde_json::Value = serde_json::from_str(&text).unwrap();
            responses.push(response);
        }
    }

    // Verify both completed (order may vary)
    assert_eq!(responses.len(), 2);

    let response1 = responses.iter().find(|r| r["id"] == 10).unwrap();
    assert_eq!(response1["result"]["value"], 15);

    let response2 = responses.iter().find(|r| r["id"] == 11).unwrap();
    assert_eq!(response2["result"]["value"], 40);
}

// #[tokio::test]
// #[serial]
// async fn test_async_code_execution() {
// Test code execution with async operations: TODO THIS IS NOT WORKING IT HALTS A PROCESS IN ASYNC MODE
//     let server = LocalToolsServer::with_code_executor(create_code_executor());

//     let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
//     let addr = listener.local_addr().unwrap();

//     let app = server.router();
//     tokio::spawn(async move {
//         axum::serve(listener, app).await.unwrap();
//     });

//     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

//     let url = format!("ws://{}/local-tools", addr);
//     let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
//     let (mut write, mut read) = ws_stream.split();

//     // Skip session_created
//     let _ = read.next().await;

//     // Execute async code
//     let execute_msg = json!({
//         "jsonrpc": "2.0",
//         "method": "execute",
//         "params": {
//             "code": r#"
//                 async function run() {
//                     // Test async/await without timers
//                     const asyncOp = async () => "async done";
//                     return await asyncOp();
//                 }
//             "#
//         },
//         "id": 12
//     });

//     write
//         .send(Message::Text(execute_msg.to_string().into()))
//         .await
//         .unwrap();

//     if let Some(Ok(Message::Text(text))) = read.next().await {
//         let response: serde_json::Value = serde_json::from_str(&text).unwrap();
//         assert_eq!(response["id"], 12);
//         assert_eq!(response["result"]["value"], "async done");
//     } else {
//         panic!("Did not receive async execution result");
//     }
// }
