use opentelemetry::KeyValue;
use pctx_core::{
    PctxTools,
    model::{GetFunctionDetailsInput, GetFunctionDetailsOutput, ListFunctionsOutput},
};
use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext, wrapper::Parameters},
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    tool, tool_router,
};
use serde_json::json;
use tracing::{info, instrument};

use crate::utils::metrics::mcp_tool_metrics;

type McpResult<T> = Result<T, rmcp::ErrorData>;

#[derive(Clone)]
pub(crate) struct PctxMcpService {
    name: String,
    version: String,
    description: Option<String>,
    tools: PctxTools,
    tool_router: ToolRouter<PctxMcpService>,
}

#[tool_router]
impl PctxMcpService {
    pub(crate) fn new(cfg: &pctx_config::Config, tools: PctxTools) -> Self {
        Self {
            name: cfg.name.clone(),
            version: cfg.version.clone(),
            description: cfg.description.clone(),
            tools,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        title = "List Functions",
        description = "ALWAYS USE THIS TOOL FIRST to list all available functions organized by namespace.

        WORKFLOW:
        1. Start here - Call this tool to see what functions are available
        2. Then call get_function_details() for specific functions you need to understand
        3. Finally call execute() to run your TypeScript code

        This returns function signatures without full details.",
        output_schema = rmcp::handler::server::tool::cached_schema_for_type::<ListFunctionsOutput>()
    )]
    async fn list_functions(&self) -> McpResult<CallToolResult> {
        let listed = self.tools.list_functions();
        let mut res = CallToolResult::success(vec![Content::text(&listed.code)]);
        res.structured_content = Some(json!(listed));

        Ok(res)
    }

    #[tool(
        title = "Get Function Details",
        description = "Get detailed information about specific functions you want to use.

        WHEN TO USE: After calling list_functions(), use this to learn about parameter types, return values, and usage for specific functions.

        REQUIRED FORMAT: Functions must be specified as 'namespace.functionName' (e.g., 'Namespace.apiPostSearch')

        This tool is lightweight and only returns details for the functions you request, avoiding unnecessary token usage.
        Only request details for functions you actually plan to use in your code.

        NOTE ON RETURN TYPES:
        - If a function returns Promise<any>, the MCP server didn't provide an output schema
        - The actual value is a parsed object (not a string) - access properties directly
        - Don't use JSON.parse() on the results - they're already JavaScript objects",
        output_schema = rmcp::handler::server::tool::cached_schema_for_type::<GetFunctionDetailsOutput>()
    )]
    async fn get_function_details(
        &self,
        Parameters(input): Parameters<GetFunctionDetailsInput>,
    ) -> McpResult<CallToolResult> {
        let details = self.tools.get_function_details(input);
        let mut res = CallToolResult::success(vec![Content::text(&details.code)]);
        res.structured_content = Some(json!(details));

        Ok(res)
    }
}

impl ServerHandler for PctxMcpService {
    fn get_info(&self) -> ServerInfo {
        let default_description = format!(
            "This server provides tools to explore SDK functions and execute SDK scripts for the following services: {}",
            self.tools
                .tool_sets
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: self.name.clone(),
                title: Some(self.name.clone()),
                version: self.version.clone(),
                ..Default::default()
            },
            instructions: Some(self.description.clone().unwrap_or(default_description)),
        }
    }

    #[instrument(skip_all, fields(mcp.method = "tools/list", mcp.id = %ctx.id))]
    async fn list_tools(
        &self,
        _req: Option<PaginatedRequestParam>,
        ctx: RequestContext<RoleServer>,
    ) -> McpResult<ListToolsResult> {
        let start = std::time::Instant::now();
        let res = ListToolsResult::with_all_items(self.tool_router.list_all());
        let latency = start.elapsed();
        info!(
            tools.length = res.tools.len(),
            tools.next_cursor = res.next_cursor.is_some(),
            latency_ms = latency.as_millis(),
            "tools/list"
        );

        // Record metrics
        if let Some(metrics) = mcp_tool_metrics() {
            metrics
                .list_duration
                .record(latency.as_secs_f64() * 1000.0, &[]);
        }

        Ok(res)
    }

    #[instrument(skip_all, fields(mcp.method = "tools/call", mcp.id = %ctx.id, mcp.tool.name = %req.name))]
    async fn call_tool(
        &self,
        req: CallToolRequestParam,
        ctx: RequestContext<RoleServer>,
    ) -> McpResult<CallToolResult> {
        let start = std::time::Instant::now();
        let tool_name = req.name.clone();

        let tcc = ToolCallContext::new(self, req, ctx);
        let res = self.tool_router.call(tcc).await;

        let latency = start.elapsed();
        let is_error = res
            .as_ref()
            .map(|r| r.is_error.unwrap_or_default())
            .unwrap_or(true);

        // Record metrics
        if let Some(metrics) = mcp_tool_metrics() {
            let attrs = vec![
                KeyValue::new("tool_name", tool_name.clone()),
                KeyValue::new("status", if is_error { "error" } else { "success" }),
            ];

            metrics
                .call_duration
                .record(latency.as_secs_f64() * 1000.0, &attrs);
            metrics.calls_total.add(1, &attrs);

            if is_error {
                metrics.errors_total.add(
                    1,
                    &[
                        KeyValue::new("tool_name", tool_name.clone()),
                        KeyValue::new("error_type", "tool_error"),
                    ],
                );
            }
        }

        let res = res?;

        info!(
            tool.result.is_error = res.is_error.unwrap_or_default(),
            tool.result.has_structured_content = res.structured_content.is_some(),
            latency_ms = latency.as_millis(),
            "tools/call - {tool_name}"
        );

        Ok(res)
    }
}
