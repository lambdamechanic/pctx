pub(crate) mod deno_pool;
pub(crate) mod tools;
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
};

use crate::mcp::{
    deno_pool::DenoExecutor,
    tools::{PtxTools, UpstreamMcp},
};

pub(crate) struct PtxMcp;
impl PtxMcp {
    pub(crate) async fn serve(host: &str, port: u16, mcp: UpstreamMcp) {
        let executor = DenoExecutor::new();

        let service = StreamableHttpService::new(
            // || Ok(counter::Counter::new()),
            move || Ok(PtxTools::with_executor(executor.clone()).register_mcp(mcp.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig {
                stateful_mode: false,
                ..Default::default()
            },
        );

        let router = axum::Router::new().nest_service("/mcp", service);
        let tcp_listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
            .await
            .unwrap();
        println!("Listening on {host}:{port}...");
        let _ = axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
            .await;
    }
}
