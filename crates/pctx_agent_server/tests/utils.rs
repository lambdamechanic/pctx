//! Shared test utilities for integration, REST, and WebSocket tests

use axum::{Router, routing::get};
use axum_test::{TestResponse, TestServer, TestWebSocket};
use pctx_agent_server::{AppState, server::create_router, websocket};
use pctx_code_mode::CodeMode;
use tokio::net::TcpListener;
use uuid::Uuid;

/// Helper to create test app state with a new session
pub(crate) async fn create_test_state() -> (Uuid, AppState) {
    let state = AppState::default();
    let session_id = Uuid::new_v4();
    state
        .code_mode_manager
        .add(session_id, CodeMode::default())
        .await;

    (session_id, state)
}

/// Helper to start a test server with only WebSocket support
/// Returns the WebSocket URL
pub(crate) async fn start_test_server() -> String {
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

/// Helper to start full test server with both REST and WebSocket
/// Returns (session_id, http_url, ws_url)
pub(crate) async fn start_full_test_server() -> (Uuid, String, String) {
    let (session_id, state) = create_test_state().await;
    let router = create_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let http_url = format!("http://127.0.0.1:{}", addr.port());
    let ws_url = format!(
        "ws://127.0.0.1:{}/ws?code_mode_session_id={session_id}",
        addr.port()
    );

    (session_id, http_url, ws_url)
}

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

pub fn insta_filters() -> Vec<(&'static str, &'static str)> {
    vec![(
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "<UUID>",
    )]
}
