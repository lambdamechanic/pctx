use std::collections::{HashMap, HashSet};

use pctx_config::server::ServerConfig;
use serde_json::json;
use tracing::{debug, warn};

use crate::{
    Error, Result,
    model::{
        ExecuteInput, ExecuteOutput, FunctionDetails, GetFunctionDetailsInput,
        GetFunctionDetailsOutput, ListFunctionsOutput, ListedFunction,
    },
};

#[derive(Debug, Clone, Default)]
pub struct PctxTools {
    pub tool_sets: Vec<codegen::ToolSet>,

    // configurations
    pub servers: Vec<ServerConfig>,
    pub local_registry: Option<pctx_code_execution_runtime::LocalToolRegistry>,
}

impl PctxTools {
    /// Convert local tool callbacks into callable ToolSets
    fn local_tools_as_toolsets(&self) -> Vec<codegen::ToolSet> {
        let mut toolsets = Vec::new();

        // Convert local tool callbacks - group by namespace
        if let Some(ref local_registry) = self.local_registry {
            let local_metadata = local_registry.list();
            if !local_metadata.is_empty() {
                let mut tools_by_namespace: HashMap<String, Vec<codegen::Tool>> = HashMap::new();

                for metadata in local_metadata {
                    match Self::local_tool_metadata_to_codegen_tool(&metadata) {
                        Ok(tool) => {
                            tools_by_namespace
                                .entry(metadata.namespace.clone())
                                .or_default()
                                .push(tool);
                        }
                        Err(e) => warn!("Failed to convert local tool '{}': {}", metadata.name, e),
                    }
                }

                for (namespace, tools) in tools_by_namespace {
                    toolsets.push(codegen::ToolSet::new(
                        &namespace,
                        &format!("Local tools in namespace '{}'", namespace),
                        tools,
                    ));
                }
            }
        }

        toolsets
    }

    /// Convert local tool metadata to a codegen Tool
    fn local_tool_metadata_to_codegen_tool(
        metadata: &pctx_code_execution_runtime::LocalToolMetadata,
    ) -> Result<codegen::Tool> {
        let schema_value = if let Some(schema) = &metadata.input_schema {
            schema.clone()
        } else {
            // Default to accepting any object
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": true
            })
        };

        let input_schema: codegen::RootSchema =
            serde_json::from_value(schema_value).map_err(|e| {
                Error::Message(format!(
                    "Failed to parse input schema for '{}': {}",
                    metadata.name, e
                ))
            })?;

        codegen::Tool::new_python(
            &metadata.name,
            metadata.description.as_ref(),
            input_schema,
            None, // Python tools don't have output schemas yet
        )
        .map_err(Error::from)
    }

    /// Get all tool sets including MCP servers and local tools
    fn all_tool_sets(&self) -> Vec<codegen::ToolSet> {
        let mut all = self.tool_sets.clone();
        all.extend(self.local_tools_as_toolsets());
        all
    }

    /// Returns internal tool sets as minimal code interfaces
    pub fn list_functions(&self) -> ListFunctionsOutput {
        let mut namespaces = vec![];
        let mut functions = vec![];

        for tool_set in &self.all_tool_sets() {
            if tool_set.tools.is_empty() {
                // skip sets with no tools
                continue;
            }

            namespaces.push(tool_set.namespace_interface(false));

            functions.extend(tool_set.tools.iter().map(|t| ListedFunction {
                namespace: tool_set.mod_name.clone(),
                name: t.fn_name.clone(),
                description: t.description.clone(),
            }));
        }

        ListFunctionsOutput {
            code: codegen::format::format_d_ts(&namespaces.join("\n\n")),
            functions,
        }
    }

    /// Gets the full typed interface for the requested functions
    pub fn get_function_details(&self, input: GetFunctionDetailsInput) -> GetFunctionDetailsOutput {
        // sort by mod
        let mut by_mod: HashMap<String, HashSet<String>> = HashMap::default();
        for fn_id in &input.functions {
            by_mod
                .entry(fn_id.mod_name.clone())
                .or_default()
                .insert(fn_id.fn_name.clone());
        }

        let mut namespaces = vec![];
        let mut functions = vec![];

        for tool_set in &self.all_tool_sets() {
            if let Some(fn_names) = by_mod.get(&tool_set.mod_name) {
                // filter tools based on requested fn names
                let tools: Vec<&codegen::Tool> = tool_set
                    .tools
                    .iter()
                    .filter(|t| fn_names.contains(&t.fn_name))
                    .collect();

                if !tools.is_empty() {
                    // code definition
                    let fn_details: Vec<String> =
                        tools.iter().map(|t| t.fn_signature(true)).collect();
                    namespaces.push(tool_set.wrap_with_namespace(&fn_details.join("\n\n")));

                    // struct output
                    functions.extend(tools.iter().map(|t| FunctionDetails {
                        listed: ListedFunction {
                            namespace: tool_set.mod_name.clone(),
                            name: t.fn_name.clone(),
                            description: t.description.clone(),
                        },
                        input_type: t.input_signature.clone(),
                        output_type: t.output_signature.clone(),
                        types: t.types.clone(),
                    }));
                }
            }
        }

        let code = if namespaces.is_empty() {
            "// No namespaces/functions match the request".to_string()
        } else {
            codegen::format::format_d_ts(&namespaces.join("\n\n"))
        };

        GetFunctionDetailsOutput { code, functions }
    }

    pub async fn execute(&self, input: ExecuteInput) -> Result<ExecuteOutput> {
        debug!(
            code_from_llm = %input.code,
            code_length = input.code.len(),
            "Received code to execute"
        );

        // generate the full script to be executed
        let namespaces: Vec<String> = self
            .all_tool_sets()
            .iter()
            .filter_map(|s| {
                if s.tools.is_empty() {
                    None
                } else {
                    Some(s.namespace())
                }
            })
            .collect();
        let to_execute = codegen::format::format_ts(&format!(
            "{namespaces}\n\n{code}\n\nexport default await run();\n",
            namespaces = namespaces.join("\n\n"),
            code = &input.code
        ));

        debug!("Executing code in sandbox");

        // Use the unified LocalToolRegistry
        let unified_registry = self
            .local_registry
            .clone()
            .unwrap_or_default();

        let options = deno_executor::ExecuteOptions::new()
            .with_allowed_hosts(self.allowed_hosts().into_iter().collect())
            .with_mcp_configs(self.servers.clone())
            .with_unified_local_registry(unified_registry);

        let execution_res = deno_executor::execute(&to_execute, options).await?;

        if execution_res.success {
            debug!("Sandbox execution completed successfully");
        } else {
            warn!("Sandbox execution failed: {:?}", execution_res.stderr);
        }

        Ok(ExecuteOutput {
            success: execution_res.success,
            stdout: execution_res.stdout,
            stderr: execution_res.stderr,
            output: execution_res.output,
        })
    }

    pub async fn add_server(&mut self, server: &ServerConfig) -> Result<()> {
        if self.tool_sets.iter().any(|t| t.name == server.name) {
            return Err(Error::Message(format!(
                "ToolSet with name `{}` already exists, MCP servers must have unique names",
                &server.name
            )));
        }

        // initialize and list tools
        debug!(
            "Fetching tools from MCP '{}'({})...",
            &server.name, &server.url
        );
        let mcp_client = server.connect().await?;
        debug!(
            "Successfully connected to '{}', inspecting tools...",
            server.name
        );
        let listed_tools = mcp_client.list_all_tools().await?;
        debug!("Found {} tools", listed_tools.len());

        // convert tools into codegen tools
        let mut codegen_tools = vec![];
        for mcp_tool in listed_tools {
            let input_schema: codegen::RootSchema =
                serde_json::from_value(json!(mcp_tool.input_schema)).map_err(|e| {
                    Error::Message(format!(
                        "Failed parsing inputSchema as json schema for tool `{}`: {e}",
                        &mcp_tool.name
                    ))
                })?;

            let output_schema = if let Some(o) = mcp_tool.output_schema {
                Some(
                    serde_json::from_value::<codegen::RootSchema>(json!(o)).map_err(|e| {
                        Error::Message(format!(
                            "Failed parsing outputSchema as json schema for tool `{}`: {e}",
                            &mcp_tool.name
                        ))
                    })?,
                )
            } else {
                None
            };

            codegen_tools.push(codegen::Tool::new_mcp(
                &mcp_tool.name,
                mcp_tool.description.map(String::from).as_ref(),
                input_schema,
                output_schema,
            )?);
        }

        let description = mcp_client
            .peer_info()
            .and_then(|p| p.server_info.title.clone())
            .unwrap_or(format!("MCP server at {}", server.url));

        // add toolset & it's server configuration
        self.tool_sets.push(codegen::ToolSet::new(
            &server.name,
            &description,
            codegen_tools,
        ));
        self.servers.push(server.clone());

        Ok(())
    }

    pub fn allowed_hosts(&self) -> HashSet<String> {
        self.servers
            .iter()
            .filter_map(|s| {
                let host = s.url.host()?;
                let allowed = if let Some(port) = s.url.port() {
                    format!("{host}:{port}")
                } else {
                    let default_port = if s.url.scheme() == "https" { 443 } else { 80 };
                    format!("{host}:{default_port}")
                };
                Some(allowed)
            })
            .collect()
    }
}
