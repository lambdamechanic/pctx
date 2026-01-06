use anyhow::Result;
use clap::Parser;
use pctx_code_mode::CodeMode;
use pctx_config::Config;
use tracing::{info, warn};

use pctx_mcp_server::PctxMcpServer;

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

    /// Serve MCP over stdio instead of HTTP
    #[arg(long)]
    pub stdio: bool,
}

impl StartCmd {
    pub(crate) async fn load_code_mode(cfg: &Config) -> Result<CodeMode> {
        // Connect to each MCP server and fetch their tool definitions in parallel
        info!(
            "Creating code mode interface for {} upstream MCP servers (parallel)",
            cfg.servers.len()
        );
        let mut code_mode = CodeMode::default();

        // Use parallel registration with 30 second timeout per server
        let mut results =
            pctx_code_mode::parallel_registration::register_servers_parallel(&cfg.servers, 30)
                .await;

        // Add successful registrations to code_mode
        let registered = results.add_to_code_mode(&mut code_mode);

        // Log failures
        for failure in &results.failed {
            warn!(
                server.name = failure.server_name,
                error = failure.error_message,
                "Failed creating code mode for MCP server"
            );
        }

        info!(
            "Code mode initialized with {}/{} MCP servers",
            registered,
            cfg.servers.len()
        );

        Ok(code_mode)
    }

    pub(crate) async fn handle(&self, cfg: Config) -> Result<Config> {
        if cfg.servers.is_empty() {
            anyhow::bail!(
                "No upstream MCP servers configured. Add servers with 'pctx add <name> <url>'"
            );
        }

        let code_mode = StartCmd::load_code_mode(&cfg).await?;

        let server = PctxMcpServer::new(&self.host, self.port, !self.no_banner);
        if self.stdio {
            server.serve_stdio(&cfg, code_mode).await?;
        } else {
            server.serve(&cfg, code_mode).await?;
        }

        info!("Shutting down...");

        Ok(cfg)
    }
}
