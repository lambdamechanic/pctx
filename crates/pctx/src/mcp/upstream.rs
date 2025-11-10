use anyhow::Result;
use codegen::case::Case;
use indexmap::IndexMap;
use log::debug;
use pctx_config::server::ServerConfig;

use super::tools::UpstreamMcp;
use crate::mcp::{client::init_mcp_client, tools::UpstreamTool};

/// Fetch tools from an upstream MCP server
pub(crate) async fn fetch_upstream_tools(server: &ServerConfig) -> Result<UpstreamMcp> {
    debug!("Fetching tools from '{}'({})...", &server.name, &server.url);

    let mcp_client = init_mcp_client(&server.url, server.auth.as_ref()).await?;

    debug!(
        "Successfully connected to '{}', inspecting tools...",
        server.name
    );

    let listed_tools = mcp_client.list_all_tools().await?;
    debug!("Found {} tools", listed_tools.len());

    let mut tools = IndexMap::new();
    for t in listed_tools {
        let tool = UpstreamTool::from_tool(t)?;
        tools.insert(tool.fn_name.clone(), tool);
    }

    let description = mcp_client
        .peer_info()
        .and_then(|p| p.server_info.title.clone())
        .unwrap_or(format!("MCP server at {}", server.url));

    mcp_client.cancel().await?;

    Ok(UpstreamMcp {
        name: server.name.clone(),
        namespace: Case::Pascal.sanitize(&server.name),
        description,
        url: server.url.clone(),
        tools,
    })
}
