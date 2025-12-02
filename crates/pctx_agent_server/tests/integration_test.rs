//! Integration tests for full agent server workflows
//!
//! These tests exercise complete end-to-end scenarios including:
//! - WebSocket connection + tool registration + REST execution
//! - MCP server registration + tool availability + execution
//! - Local tool callbacks via WebSocket

use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use futures::StreamExt;
use pctx_agent_server::{AppState, rest, types::*, websocket};
use pctx_code_mode::CodeMode;
use pctx_code_mode::model::GetFunctionDetailsOutput;
use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to create test app state
fn create_test_state() -> AppState {
    let code_mode = CodeMode::default();
    AppState::new(code_mode)
}

/// Helper to start full test server with both REST and WebSocket
async fn start_full_test_server() -> (String, String) {
    let state = create_test_state();

    let app = Router::new()
        .route("/health", get(rest::health))
        .route("/tools/list", post(rest::list_tools))
        .route("/tools/details", post(rest::get_function_details))
        .route("/tools/execute", post(rest::execute_code))
        .route("/tools/local/register", post(rest::register_local_tools))
        .route("/tools/mcp/register", post(rest::register_mcp_servers))
        .route("/ws", get(websocket::ws_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let http_url = format!("http://127.0.0.1:{}", addr.port());
    let ws_url = format!("ws://127.0.0.1:{}/ws", addr.port());

    (http_url, ws_url)
}

#[tokio::test]
#[serial]
async fn test_full_workflow_websocket_registration_and_list() {
    let (http_url, ws_url) = start_full_test_server().await;

    // 1. Connect via WebSocket
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (_write, mut read) = ws_stream.split();

    // 2. Receive session_created notification
    let session_id = if let Some(Ok(Message::Text(text))) = read.next().await {
        let notification: serde_json::Value = serde_json::from_str(&text).unwrap();
        notification["params"]["session_id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        panic!("Expected session_created notification");
    };

    // 3. Register tools via REST API
    let client = reqwest::Client::new();
    let register_response = client
        .post(format!("{http_url}/tools/local/register"))
        .json(&json!({
            "session_id": session_id,
            "tools": [
                {
                    "namespace": "TestTools",
                    "name": "myFunction",
                    "description": "A test function",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "input": { "type": "string" }
                        }
                    }
                }
            ]
        }))
        .send()
        .await
        .expect("Failed to register tools");

    assert_eq!(register_response.status(), StatusCode::OK);
    let register_body: RegisterLocalToolsResponse = register_response.json().await.unwrap();
    assert_eq!(register_body.registered, 1);

    // 4. List tools via REST API
    let list_response = client
        .post(format!("{http_url}/tools/list"))
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to list tools");

    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body: ListToolsResponse = list_response.json().await.unwrap();

    // Should contain our registered tool
    let found_tool = list_body
        .tools
        .iter()
        .find(|t| t.namespace == "TestTools" && t.name == "myFunction");
    assert!(found_tool.is_some());
}

#[tokio::test]
#[serial]
async fn test_rest_only_code_execution() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    // Execute simple code without any registered tools
    let execute_response = client
        .post(format!("{http_url}/tools/execute"))
        .json(&json!({
            "code": "async function run() { return 1 + 1; }",
            "timeout_ms": 5000
        }))
        .send()
        .await
        .expect("Failed to execute code");

    assert_eq!(execute_response.status(), StatusCode::OK);
    let execute_body: ExecuteCodeResponse = execute_response.json().await.unwrap();
    assert_eq!(execute_body.output, Some(json!(2)));
    assert!(execute_body.execution_time_ms > 0);
}

#[tokio::test]
#[serial]
async fn test_health_check_always_available() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    let health_response = client
        .get(format!("{http_url}/health"))
        .send()
        .await
        .expect("Failed to get health");

    assert_eq!(health_response.status(), StatusCode::OK);
    let health_body: HealthResponse = health_response.json().await.unwrap();
    assert_eq!(health_body.status, "ok");
}

#[tokio::test]
#[serial]
async fn test_mcp_server_registration() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    // Register an MCP server (will fail to connect but should validate URL)
    let register_response = client
        .post(format!("{http_url}/tools/mcp/register"))
        .json(&json!({
            "servers": [
                {
                    "name": "test-mcp",
                    "url": "http://localhost:9999/mcp",
                    "auth": null
                }
            ]
        }))
        .send()
        .await
        .expect("Failed to register MCP server");

    assert_eq!(register_response.status(), StatusCode::OK);
    let register_body: RegisterMcpServersResponse = register_response.json().await.unwrap();

    // URL is valid, so it should at least pass validation
    // (actual connection will fail since no server is running)
    // registered is a usize, so it's always >= 0, just check it exists
    let _ = register_body.registered;
}

#[tokio::test]
#[serial]
async fn test_execute_code_with_async_operations() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    let execute_response = client
        .post(format!("{http_url}/tools/execute"))
        .json(&json!({
            "code": r"
                async function run() {
                    await Promise.resolve();
                    return { completed: true };
                }
            ",
            "timeout_ms": 5000
        }))
        .send()
        .await
        .expect("Failed to execute code");

    assert_eq!(execute_response.status(), StatusCode::OK);
    let execute_body: ExecuteCodeResponse = execute_response.json().await.unwrap();
    assert_eq!(execute_body.output, Some(json!({ "completed": true })));
}

#[tokio::test]
#[serial]
async fn test_execute_code_with_console_output() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    let execute_response = client
        .post(format!("{http_url}/tools/execute"))
        .json(&json!({
            "code": r#"
                async function run() {
                    console.log("Test log");
                    console.error("Test error");
                    return "done";
                }
            "#,
            "timeout_ms": 5000
        }))
        .send()
        .await
        .expect("Failed to execute code");

    assert_eq!(execute_response.status(), StatusCode::OK);
    let execute_body: ExecuteCodeResponse = execute_response.json().await.unwrap();
    assert_eq!(execute_body.output, Some(json!("done")));
}

#[tokio::test]
#[serial]
async fn test_multiple_websocket_sessions_isolated() {
    let (http_url, ws_url) = start_full_test_server().await;

    // Connect two WebSocket clients
    let (ws_stream1, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect client 1");
    let (mut _write1, mut read1) = ws_stream1.split();

    let (ws_stream2, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect client 2");
    let (mut _write2, mut read2) = ws_stream2.split();

    // Get session IDs
    let session_id1 = if let Some(Ok(Message::Text(text))) = read1.next().await {
        let notification: serde_json::Value = serde_json::from_str(&text).unwrap();
        notification["params"]["session_id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        panic!("Expected session_created notification");
    };

    let session_id2 = if let Some(Ok(Message::Text(text))) = read2.next().await {
        let notification: serde_json::Value = serde_json::from_str(&text).unwrap();
        notification["params"]["session_id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        panic!("Expected session_created notification");
    };

    assert_ne!(session_id1, session_id2);

    // Register tool for session 1
    let client = reqwest::Client::new();
    let register_response = client
        .post(format!("{http_url}/tools/local/register"))
        .json(&json!({
            "session_id": session_id1,
            "tools": [
                {
                    "namespace": "Session1Tools",
                    "name": "tool1",
                    "description": "Tool from session 1",
                    "parameters": {}
                }
            ]
        }))
        .send()
        .await
        .expect("Failed to register tools");

    assert_eq!(register_response.status(), StatusCode::OK);

    // Register different tool for session 2
    let register_response2 = client
        .post(format!("{http_url}/tools/local/register"))
        .json(&json!({
            "session_id": session_id2,
            "tools": [
                {
                    "namespace": "Session2Tools",
                    "name": "tool2",
                    "description": "Tool from session 2",
                    "parameters": {}
                }
            ]
        }))
        .send()
        .await
        .expect("Failed to register tools");

    assert_eq!(register_response2.status(), StatusCode::OK);

    // List all tools - should see both
    let list_response = client
        .post(format!("{http_url}/tools/list"))
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to list tools");

    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body: ListToolsResponse = list_response.json().await.unwrap();

    assert!(
        list_body
            .tools
            .iter()
            .any(|t| t.namespace == "Session1Tools" && t.name == "tool1")
    );
    assert!(
        list_body
            .tools
            .iter()
            .any(|t| t.namespace == "Session2Tools" && t.name == "tool2")
    );
}

#[tokio::test]
#[serial]
async fn test_error_handling_invalid_session_id() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    // Try to register tools with invalid session ID
    let register_response = client
        .post(format!("{http_url}/tools/local/register"))
        .json(&json!({
            "session_id": "non-existent-session-id",
            "tools": [
                {
                    "namespace": "TestTools",
                    "name": "myFunction",
                    "description": "A test function",
                    "parameters": {}
                }
            ]
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        register_response.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let error_body: ErrorResponse = register_response.json().await.unwrap();
    assert!(
        error_body.error.message.contains("Session not found")
            || error_body.error.message.contains("Failed to register")
    );
}

#[tokio::test]
#[serial]
async fn test_execute_code_syntax_error() {
    let (http_url, _ws_url) = start_full_test_server().await;

    let client = reqwest::Client::new();

    let execute_response = client
        .post(format!("{http_url}/tools/execute"))
        .json(&json!({
            "code": "this is not valid javascript syntax !!!",
            "timeout_ms": 5000
        }))
        .send()
        .await
        .expect("Failed to execute code");

    assert_eq!(execute_response.status(), StatusCode::BAD_REQUEST);
    let error_body: ErrorResponse = execute_response.json().await.unwrap();
    assert_eq!(error_body.error.code, "EXECUTION_ERROR");
}

#[tokio::test]
#[serial]
async fn test_get_function_details_returns_code_field() {
    let (http_url, ws_url) = start_full_test_server().await;
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (_write, mut read) = ws_stream.split();

    let session_id = if let Some(Ok(Message::Text(text))) = read.next().await {
        let notification: serde_json::Value = serde_json::from_str(&text).unwrap();
        notification["params"]["session_id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        panic!("Expected session_created notification");
    };

    // Register a local tool with JSON schema
    let client = reqwest::Client::new();
    let register_response = client
        .post(format!("{http_url}/tools/local/register"))
        .json(&json!({
            "session_id": session_id,
            "tools": [{
                "namespace": "TestTools",
                "name": "myFunction",
                "description": "A test function",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                }
            }]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(register_response.status(), StatusCode::OK);
    let details_response = client
        .post(format!("{http_url}/tools/details"))
        .json(&json!({
            "namespace": "TestTools",
            "name": "myFunction"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(details_response.status(), StatusCode::OK);
    let details: GetFunctionDetailsOutput = details_response.json().await.unwrap();
    assert!(!details.code.is_empty(), "Code field should not be empty");
    assert!(
        !details.functions.is_empty(),
        "Should have function details"
    );
    assert_eq!(details.functions[0].listed.name, "myFunction");
}
