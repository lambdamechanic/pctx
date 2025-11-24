use anyhow::Result;
use codegen::generate_docstring;
use indexmap::{IndexMap, IndexSet};
use opentelemetry::KeyValue;
use pctx_config::Config;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        tool::{IntoCallToolResult, ToolCallContext},
        wrapper::Parameters,
    },
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars,
    service::RequestContext,
    tool, tool_router,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, instrument, warn};

use crate::mcp::upstream::UpstreamMcp;
use crate::utils::metrics::mcp_tool_metrics;

type McpResult<T> = Result<T, McpError>;

// ----------- TOOL INPUTS/OUTPUTS -------------

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListFunctionsOutput {
    /// Available functions
    functions: Vec<ListedFunction>,

    #[serde(skip)]
    code: String,
}
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListedFunction {
    /// Namespace the function belongs in
    namespace: String,
    /// Function name
    name: String,
    /// Function description
    description: Option<String>,
}

impl ListFunctionsOutput {
    fn from_upstream(upstream: &[UpstreamMcp]) -> Self {
        // structured content
        let functions: Vec<ListedFunction> = upstream
            .iter()
            .flat_map(|m| {
                m.tools
                    .iter()
                    .map(|(_, t)| ListedFunction {
                        namespace: m.namespace.clone(),
                        name: t.fn_name.clone(),
                        description: t.description.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Text content = code
        let namespaces: Vec<String> = upstream
            .iter()
            .map(|m| {
                let fns: Vec<String> = m.tools.iter().map(|(_, t)| t.fn_signature(false)).collect();

                format!(
                    "{docstring}
namespace {namespace} {{
  {fns}
}}",
                    docstring = generate_docstring(&m.description),
                    namespace = &m.namespace,
                    fns = fns.join("\n\n")
                )
            })
            .collect();
        let code = codegen::format::format_d_ts(&namespaces.join("\n\n"));

        Self { functions, code }
    }
}
impl IntoCallToolResult for ListFunctionsOutput {
    fn into_call_tool_result(self) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let mut res = CallToolResult::success(vec![Content::text(&self.code)]);
        res.structured_content = Some(json!(self));
        Ok(res)
    }
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetFunctionDetailsInput {
    /// List of functions to get details of. Functions should be in the form "<namespace>.<function name>".
    /// e.g. If there is a function `getData` within the `DataApi` namespace the value provided in this field is "DataApi.getData"
    pub functions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetFunctionDetailsOutput {
    functions: Vec<FunctionDetails>,

    #[serde(skip)]
    code: String,
}
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct FunctionDetails {
    #[serde(flatten)]
    listed: ListedFunction,

    /// typescript input type for the function
    input_type: String,
    /// typescript output type for the function
    output_type: String,
    /// full typescript type definitions for input/output types
    types: String,
}
impl IntoCallToolResult for GetFunctionDetailsOutput {
    fn into_call_tool_result(self) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let mut res = CallToolResult::success(vec![Content::text(&self.code)]);
        res.structured_content = Some(json!(self));
        Ok(res)
    }
}

#[allow(clippy::doc_markdown)]
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct ExecuteInput {
    /// Typescript code to execute.
    ///
    /// REQUIRED FORMAT:
    /// async function ``run()`` {
    ///   // YOUR CODE GOES HERE e.g. const result = await ``Namespace.method();``
    ///   // ALWAYS RETURN THE RESULT e.g. return result;
    /// }
    ///
    /// IMPORTANT: Your code should ONLY contain the function definition.
    /// The sandbox automatically calls run() and exports the result.
    ///
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct ExecuteOutput {
    /// Success of executed code
    success: bool,
    /// Standard output of executed code
    stdout: String,
    /// Standard error of executed code
    stderr: String,
    /// Value returned by executed function
    output: Option<serde_json::Value>,
}

impl IntoCallToolResult for ExecuteOutput {
    fn into_call_tool_result(self) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let text_content = format!(
            "Code Executed Successfully: {success}

# Return Value
```json
{return_val}
```

# STDOUT
{stdout}

# STDERR
{stderr}
",
            success = self.success,
            return_val = serde_json::to_string_pretty(&self.output)
                .unwrap_or(json!(&self.output).to_string()),
            stdout = &self.stdout,
            stderr = &self.stderr,
        );

        let mut res = if self.success {
            CallToolResult::success(vec![Content::text(text_content)])
        } else {
            CallToolResult::error(vec![Content::text(text_content)])
        };
        res.structured_content = Some(json!(self));

        Ok(res)
    }
}

// ----------- TOOL HANDLERS -----------

#[derive(Clone)]
pub(crate) struct PtcxTools {
    config: Config,
    allowed_hosts: Vec<String>,
    upstream: Vec<UpstreamMcp>,
    tool_router: ToolRouter<PtcxTools>,
}
#[tool_router]
impl PtcxTools {
    pub(crate) fn new(config: Config, allowed_hosts: Vec<String>) -> Self {
        Self {
            config,
            allowed_hosts,
            upstream: vec![],
            tool_router: Self::tool_router(),
        }
    }

    pub(crate) fn with_upstream_mcps(mut self, upstream: Vec<UpstreamMcp>) -> Self {
        self.upstream = upstream;
        self
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
    async fn list_functions(&self) -> McpResult<ListFunctionsOutput> {
        Ok(ListFunctionsOutput::from_upstream(&self.upstream))
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
        Parameters(GetFunctionDetailsInput { functions }): Parameters<GetFunctionDetailsInput>,
    ) -> McpResult<GetFunctionDetailsOutput> {
        // organize tool input by namespace and handle any deduping
        let mut by_namespace: IndexMap<String, IndexSet<String>> = IndexMap::new();
        for func in functions {
            let parts: Vec<&str> = func.split('.').collect();
            if parts.len() != 2 {
                // incorrect format
                continue;
            }
            by_namespace
                .entry(parts[0].to_string())
                .or_default()
                .insert(parts[1].to_string());
        }

        let mut namespace_code = vec![];
        let mut function_details = vec![];

        for (namespace, functions) in by_namespace {
            if let Some(mcp) = self.upstream.iter().find(|m| m.namespace == namespace) {
                let mut fn_details = vec![];
                for fn_name in functions {
                    if let Some(tool) = mcp.tools.get(&fn_name) {
                        fn_details.push(tool.fn_signature(true));

                        function_details.push(FunctionDetails {
                            listed: ListedFunction {
                                namespace: namespace.clone(),
                                name: tool.fn_name.clone(),
                                description: tool.description.clone(),
                            },
                            input_type: tool.input_type.clone(),
                            output_type: tool.output_type.clone(),
                            types: tool.types.clone(),
                        });
                    }
                }

                if !fn_details.is_empty() {
                    namespace_code.push(format!(
                        "{docstring}
namespace {namespace} {{
  {fns}
}}",
                        docstring = generate_docstring(&mcp.description),
                        namespace = &mcp.namespace,
                        fns = fn_details.join("\n\n")
                    ));
                }
            }
        }

        let code = if namespace_code.is_empty() {
            "// No namespaces/functions match the request".to_string()
        } else {
            codegen::format::format_d_ts(&namespace_code.join("\n\n"))
        };

        Ok(GetFunctionDetailsOutput {
            functions: function_details,
            code,
        })
    }

    #[tool(
        title = "Execute Code",
        description = "Execute TypeScript code that calls namespaced functions. USE THIS LAST after list_functions() and get_function_details().

        TOKEN USAGE WARNING: This tool could return LARGE responses if your code returns big objects.
        To minimize tokens:
        - Filter/map/reduce data IN YOUR CODE before returning
        - Only return specific fields you need (e.g., return {id: result.id, count: items.length})
        - Use console.log() for intermediate results instead of returning everything
        - Avoid returning full API responses - extract just what you need

        REQUIRED CODE STRUCTURE:
        async function run() {
            // Your code here
            // Call namespace.functionName() - MUST include namespace prefix
            // Process data here to minimize return size
            return onlyWhatYouNeed; // Keep this small!
        }

        IMPORTANT RULES:
        - Functions MUST be called as 'Namespace.functionName' (e.g., 'Notion.apiPostSearch')
        - Only functions from list_functions() are available - no fetch(), fs, or other Node/Deno APIs
        - Variables don't persist between execute() calls - return or log anything you need later
        - Add console.log() statements between API calls to track progress if errors occur
        - Code runs in an isolated Deno sandbox with restricted network access

        RETURN TYPE NOTE:
        - Functions without output schemas show Promise<any> as return type
        - The actual runtime value is already a parsed JavaScript object, NOT a JSON string
        - Do NOT call JSON.parse() on results - they're already objects
        - Access properties directly (e.g., result.data) or inspect with console.log() first
        - If you see 'Promise<any>', the structure is unknown - log it to see what's returned
        ",
        output_schema = rmcp::handler::server::tool::cached_schema_for_type::<ExecuteOutput>()
    )]
    async fn execute(
        &self,
        Parameters(ExecuteInput { code }): Parameters<ExecuteInput>,
    ) -> McpResult<ExecuteOutput> {
        tracing::debug!(
            code_from_llm = %code,
            code_length = code.len(),
            "Received code to execute"
        );

        let registrations = self
            .upstream
            .iter()
            .map(|m| format!("registerMCP({});", &m.registration))
            .collect::<Vec<String>>()
            .join("\n\n");
        let namespaces = self
            .upstream
            .iter()
            .map(|m| {
                let fns: Vec<String> = m.tools.iter().map(|(_, t)| t.fn_impl(&m.name)).collect();

                format!(
                    "{docstring}
namespace {namespace} {{
  {fns}
}}",
                    docstring = generate_docstring(&m.description),
                    namespace = &m.namespace,
                    fns = fns.join("\n\n")
                )
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        let to_execute = format!(
            "
{registrations}

{namespaces}

{code}

export default await run();"
        );

        debug!("Executing code in sandbox");

        let allowed_hosts = self.allowed_hosts.clone();
        let code_to_execute = to_execute.clone();

        // Capture current tracing context to propagate to spawned thread
        let current_span = tracing::Span::current();

        let result = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
            // Enter the captured span context in the new thread
            let _guard = current_span.enter();

            // Create a new current-thread runtime for Deno ops that use deno_unsync
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;

            rt.block_on(async {
                deno_executor::execute(&code_to_execute, Some(allowed_hosts))
                    .await
                    .map_err(|e| anyhow::anyhow!("Execution error: {e}"))
            })
        })
        .await
        .map_err(|e| {
            error!("Task join failed: {e}");
            McpError::internal_error(format!("Task join failed: {e}"), None)
        })?
        .map_err(|e| {
            error!("Sandbox execution error: {e}");
            McpError::internal_error(format!("Execution failed: {e}"), None)
        })?;

        if result.success {
            debug!("Sandbox execution completed successfully");
        } else {
            warn!("Sandbox execution failed: {:?}", result.stderr);
        }
        Ok(ExecuteOutput {
            success: result.success,
            stdout: result.stdout,
            stderr: result.stderr,
            output: result.output,
        })
    }
}

impl ServerHandler for PtcxTools {
    fn get_info(&self) -> ServerInfo {
        let default_description = format!(
            "This server provides tools to explore SDK functions and execute SDK scripts for the following services: {}",
            self.upstream
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        );

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: self.config.name.clone(),
                title: Some(self.config.name.clone()),
                version: self.config.version.clone(),
                ..Default::default()
            },
            instructions: Some(
                self.config
                    .description
                    .clone()
                    .unwrap_or(default_description),
            ),
        }
    }

    #[instrument(skip_all, fields(mcp.method = "tools/list", mcp.id = %ctx.id))]
    async fn list_tools(
        &self,
        _req: Option<PaginatedRequestParam>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
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
    ) -> Result<CallToolResult, McpError> {
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
