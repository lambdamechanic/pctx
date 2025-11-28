use anyhow::Result;
use clap::Parser;
use pctx_config::Config;
use pctx_core::PctxTools;
use tracing::{debug, info, warn};

use crate::mcp::PctxMcpServer;

#[derive(Debug, Clone, Parser)]
pub struct StartCmd {
    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Host address to bind to (use 0.0.0.0 for external access)
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Don't show the server banner
    #[arg(long)]
    pub no_banner: bool,
}

impl StartCmd {
    pub(crate) async fn load_tools(cfg: &Config) -> Result<PctxTools> {
        // Connect to each MCP server and fetch their tool definitions
        info!(
            "Creating code mode interface for {} upstream MCP servers",
            cfg.servers.len()
        );
        let mut tools = PctxTools::default();

        for server in &cfg.servers {
            debug!("Creating code mode interface for {}", &server.name);
            if let Err(e) = tools.add_server(server).await {
                warn!(
                    err =? e,
                    server.name =? &server.name,
                    server.url =? server.url.to_string(),
                    "Failed creating creating code mode for `{}` MCP server",
                    &server.name
                );
            }
        }

        Ok(tools)
    }

    pub(crate) async fn handle(&self, cfg: Config) -> Result<Config> {
        if cfg.servers.is_empty() {
            anyhow::bail!(
                "No upstream MCP servers configured. Add servers with 'pctx add <name> <url>'"
            );
        }

        let tools = StartCmd::load_tools(&cfg).await?;

        PctxMcpServer::new(&self.host, self.port, !self.no_banner)
            .serve(&cfg, tools)
            .await?;

        info!("Shutting down...");

        Ok(cfg)
    }
}
