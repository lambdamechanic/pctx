//! Shared test utilities for integration, REST, and WebSocket tests

use std::sync::Arc;

use axum::{Router, routing::get};
use axum_test::{TestResponse, TestServer};
use pctx_agent_server::{AppState, model::CreateSessionResponse, server::create_router, websocket};
use pctx_code_execution_runtime::CallbackFn;
use pctx_code_mode::{CodeMode, model::CallbackConfig};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpListener;
use uuid::Uuid;

pub fn create_test_server() -> TestServer {
    TestServer::builder()
        .http_transport()
        .build(create_router(AppState::default()))
        .expect("Failed starting test server")
}

pub async fn create_test_server_with_session() -> (Uuid, TestServer) {
    let state = AppState::default();
    let session_id = Uuid::new_v4();
    state
        .code_mode_manager
        .add(session_id, CodeMode::default())
        .await;
    (
        session_id,
        TestServer::builder()
            .http_transport()
            .build(create_router(state))
            .expect("Failed starting test server"),
    )
}

pub async fn connect_websocket(server: &TestServer, session_id: Uuid) -> TestResponse {
    server
        .get_websocket("/ws")
        .add_header("x-code-mode-session", session_id.to_string())
        .await
}

pub async fn create_session(server: &TestServer) -> Uuid {
    let res: CreateSessionResponse = server.post("/code-mode/session/create").await.json();
    res.session_id
}

pub fn callback_tools() -> Vec<(CallbackConfig, CallbackFn)> {
    #[derive(Deserialize)]
    struct MathArgs {
        a: isize,
        b: isize,
    }
    let input_schema = json!({
        "type": "object",
        "properties": {
            "a": {
                "type": "number",
                "description": "First number"
            },
            "b": {
                "type": "number",
                "description": "Second number"
            }
        },
        "required": ["a", "b"]
    });
    let output_schema = json!({
        "type": "number",
        "description": "Result of operation on a and b"
    });
    vec![
        (
            CallbackConfig {
                name: "add".into(),
                namespace: "test_math".into(),
                description: Some("Add two numbers & return result".into()),
                input_schema: Some(input_schema.clone()),
                output_schema: Some(output_schema.clone()),
            },
            Arc::new(move |args: Option<serde_json::Value>| {
                Box::pin(async move {
                    let add_args: MathArgs = serde_json::from_value(json!(args))
                        .map_err(|e| format!("Invalid test_math.add args: {e}"))?;

                    let result = add_args.a + add_args.b;
                    Ok(json!(result))
                })
            }),
        ),
        (
            CallbackConfig {
                name: "subtract".into(),
                namespace: "test_math".into(),
                description: Some("Subtract two numbers & return result".into()),
                input_schema: Some(input_schema.clone()),
                output_schema: Some(output_schema.clone()),
            },
            Arc::new(move |args: Option<serde_json::Value>| {
                Box::pin(async move {
                    let subtract_args: MathArgs = serde_json::from_value(json!(args))
                        .map_err(|e| format!("Invalid test_math.subtract args: {e}"))?;

                    let result = subtract_args.a - subtract_args.b;
                    Ok(json!(result))
                })
            }),
        ),
        (
            CallbackConfig {
                name: "multiply".into(),
                namespace: "test_math".into(),
                description: Some("Multiply two numbers & return result".into()),
                input_schema: Some(input_schema.clone()),
                output_schema: Some(output_schema.clone()),
            },
            Arc::new(move |args: Option<serde_json::Value>| {
                Box::pin(async move {
                    let multiply_args: MathArgs = serde_json::from_value(json!(args))
                        .map_err(|e| format!("Invalid test_math.multiply args: {e}"))?;

                    let result = multiply_args.a * multiply_args.b;
                    Ok(json!(result))
                })
            }),
        ),
        (
            CallbackConfig {
                name: "divide".into(),
                namespace: "test_math".into(),
                description: Some("Divide two numbers & return result".into()),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "number",
                            "description": "Numerator"
                        },
                        "b": {
                            "type": "number",
                            "description": "Denominator"
                        }
                    },
                    "required": ["a", "b"]
                })),
                output_schema: Some(output_schema.clone()),
            },
            Arc::new(move |args: Option<serde_json::Value>| {
                Box::pin(async move {
                    let divide_args: MathArgs = serde_json::from_value(json!(args))
                        .map_err(|e| format!("Invalid test_math.divide args: {e}"))?;

                    if divide_args.b == 0 {
                        return Err("Division by zero".to_string());
                    }

                    let result = divide_args.a / divide_args.b;
                    Ok(json!(result))
                })
            }),
        ),
    ]
}
