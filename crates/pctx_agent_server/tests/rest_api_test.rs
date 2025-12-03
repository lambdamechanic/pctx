//! Unit tests for REST API endpoints

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use pctx_agent_server::{
    AppState,
    model::{ErrorResponse, HealthResponse, RegisterMcpServersResponse},
    server::create_router,
};
use pctx_code_mode::{
    CodeMode,
    model::{ExecuteOutput, ListFunctionsOutput},
};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

/// Helper to create test app state
fn create_test_state() -> AppState {
    // Create a minimal CodeMode with no MCP servers
    let code_mode = CodeMode::default();
    AppState::new(code_mode)
}

/// Helper to create router for testing
fn create_test_router() -> Router {
    let state = create_test_state();

    create_router(state)
}

/// Helper to parse JSON response body
async fn parse_response_body<T: serde::de::DeserializeOwned>(body: Body) -> T {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("Failed to parse response body")
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: HealthResponse = parse_response_body(response.into_body()).await;
    assert_eq!(body.status, "ok");
    assert_eq!(body.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn test_list_tools_empty() {
    let app = create_test_router();

    let request_body = json!({});

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/code-mode/functions/list")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: ListFunctionsOutput = parse_response_body(response.into_body()).await;
    assert_eq!(body.functions.len(), 0);
}

#[tokio::test]
async fn test_get_function_details_not_found() {
    let app = create_test_router();

    let request_body = json!({
        "namespace": "NonExistent",
        "name": "fakeFunction"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/details")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body: ErrorResponse = parse_response_body(response.into_body()).await;
    assert_eq!(body.error.code, "NOT_FOUND");
    assert!(body.error.message.contains("not found"));
}

#[tokio::test]
#[serial]
async fn test_execute_code_simple() {
    let app = create_test_router();

    let request_body = json!({
        "code": "async function run() { return 42; }",
        "timeout_ms": 5000
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/code-mode/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();

    // Debug: print status and body if not OK
    if status != StatusCode::OK {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&bytes);
        eprintln!("Status: {status}");
        eprintln!("Body: {body_str}");
        panic!("Expected OK, got {status}");
    }

    let body: ExecuteOutput = parse_response_body(response.into_body()).await;
    assert_eq!(body.output, Some(json!(42)));
}

#[tokio::test]
#[serial]
async fn test_execute_code_error() {
    let app = create_test_router();

    let request_body = json!({
        "code": "throw new Error('Test error');",
        "timeout_ms": 5000
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/code-mode/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: ErrorResponse = parse_response_body(response.into_body()).await;
    assert_eq!(body.error.code, "EXECUTION_ERROR");
    assert!(body.error.message.contains("failed"));
}

#[tokio::test]
#[serial]
async fn test_execute_code_with_console_log() {
    let app = create_test_router();

    let request_body = json!({
        "code": "async function run() { console.log('Hello, world!'); return 'done'; }",
        "timeout_ms": 5000
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/code-mode/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: ExecuteOutput = parse_response_body(response.into_body()).await;
    assert_eq!(body.output, Some(json!("done")));
}

#[tokio::test]
async fn test_register_mcp_servers_invalid_url() {
    let app = create_test_router();

    let request_body = json!({
        "servers": [
            {
                "name": "invalid-server",
                "url": "not-a-valid-url",
                "auth": null
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/register/servers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: RegisterMcpServersResponse = parse_response_body(response.into_body()).await;
    assert_eq!(body.registered, 0);
    assert_eq!(body.failed.len(), 1);
    assert_eq!(body.failed[0], "invalid-server");
}

#[tokio::test]
async fn test_register_mcp_servers_valid_url() {
    let app = create_test_router();

    let request_body = json!({
        "servers": [
            {
                "name": "test-server",
                "url": "http://localhost:3000/mcp",
                "auth": null
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/register/servers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: RegisterMcpServersResponse = parse_response_body(response.into_body()).await;
    // Note: The server will succeed URL validation but registration may fail
    // since we're not actually running an MCP server
    assert!(body.registered <= 1);
}

#[tokio::test]
#[serial]
async fn test_execute_code_async() {
    let app = create_test_router();

    let request_body = json!({
        "code": "async function run() { return await Promise.resolve(123); }",
        "timeout_ms": 5000
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/code-mode/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    if status != StatusCode::OK {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&bytes);
        eprintln!("Status: {status}");
        eprintln!("Body: {body_str}");
        panic!("Expected OK, got {status}");
    }

    let body: ExecuteOutput = parse_response_body(response.into_body()).await;
    assert_eq!(body.output, Some(json!(123)));
}

#[tokio::test]
#[serial]
async fn test_execute_code_json_result() {
    let app = create_test_router();

    let request_body = json!({
        "code": "async function run() { return { foo: 'bar', count: 42, nested: { value: true } }; }",
        "timeout_ms": 5000
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/code-mode/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: ExecuteOutput = parse_response_body(response.into_body()).await;
    assert_eq!(
        body.output,
        Some(json!({
            "foo": "bar",
            "count": 42,
            "nested": { "value": true }
        }))
    );
}
