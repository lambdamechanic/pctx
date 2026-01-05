use std::time::Duration;

use axum::Router;
use pctx_code_mode::CodeMode;
use pctx_config::server::ServerConfig;
use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext, wrapper::Parameters},
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    tool, tool_router,
    transport::{
        StreamableHttpServerConfig,
        streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Clone)]
struct TestMcpService {
    tool_router: ToolRouter<TestMcpService>,
}

type McpResult<T> = Result<T, rmcp::ErrorData>;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct EchoInput {
    message: String,
}

#[tool_router]
impl TestMcpService {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(title = "Echo", description = "Echo input back")]
    async fn echo(&self, Parameters(input): Parameters<EchoInput>) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![Content::text(&input.message)]))
    }
}

impl ServerHandler for TestMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "test-mcp".to_string(),
                title: Some("test-mcp".to_string()),
                version: "0.1.0".to_string(),
                ..Default::default()
            },
            instructions: None,
        }
    }

    async fn list_tools(
        &self,
        _req: Option<PaginatedRequestParam>,
        _ctx: RequestContext<RoleServer>,
    ) -> McpResult<ListToolsResult> {
        Ok(ListToolsResult::with_all_items(self.tool_router.list_all()))
    }

    async fn call_tool(
        &self,
        req: CallToolRequestParam,
        ctx: RequestContext<RoleServer>,
    ) -> McpResult<CallToolResult> {
        let tcc = ToolCallContext::new(self, req, ctx);
        self.tool_router.call(tcc).await
    }
}

async fn spawn_test_server() -> (url::Url, tokio::task::JoinHandle<()>) {
    let service = TestMcpService::new();
    let service = StreamableHttpService::new(
        move || Ok(service.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            stateful_mode: false,
            ..Default::default()
        },
    );
    let router = Router::new().nest_service("/mcp", service);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let addr = listener.local_addr().expect("local addr");
    let url = url::Url::parse(&format!("http://{addr}/mcp")).expect("parse url");
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    (url, handle)
}

#[tokio::test]
async fn build_server_report_builds_toolset() {
    let (url, handle) = spawn_test_server().await;
    let server = ServerConfig::new("test-server".to_string(), url);

    let report = CodeMode::build_server_report(&server).await;
    handle.abort();

    let built = report.result.expect("expected build to succeed");
    assert_eq!(built.tool_set.name, "test-server");
    assert!(!built.tool_set.tools.is_empty());
    assert!(report.duration >= Duration::ZERO);
}

#[tokio::test]
async fn build_server_report_failure_captures_duration() {
    let url = url::Url::parse("http://127.0.0.1:1/mcp").expect("parse url");
    let server = ServerConfig::new("missing-server".to_string(), url);

    let report = CodeMode::build_server_report(&server).await;

    assert!(report.result.is_err());
    assert!(report.duration >= Duration::ZERO);
}
