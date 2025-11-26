use std::collections::HashSet;

use pctx_config::server::ServerConfig;
use serde_json::json;
use tracing::debug;

use crate::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct PctxTools {
    tool_sets: Vec<codegen::ToolSet>,

    // configurations
    servers: Vec<ServerConfig>,
    // TODO: callables
}

impl PctxTools {
    pub async fn add_server(&mut self, server: &ServerConfig) -> Result<()> {
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
