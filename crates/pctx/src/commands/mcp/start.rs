use std::time::Instant;

use anyhow::Result;
use clap::Parser;
use pctx_code_mode::CodeMode;
use pctx_config::Config;
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

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
        // Connect to each MCP server and fetch their tool definitions
        info!(
            "Creating code mode interface for {} upstream MCP servers",
            cfg.servers.len()
        );
        let mut code_mode = CodeMode::default();

        let mut builds = JoinSet::new();
        for server in &cfg.servers {
            debug!("Creating code mode interface for {}", &server.name);
            let server = server.clone();
            builds.spawn(async move {
                let start = Instant::now();
                let result = CodeMode::build_server(&server).await;
                (server, start.elapsed(), result)
            });
        }

        while let Some(joined) = builds.join_next().await {
            match joined {
                Ok((server, duration, result)) => {
                    let duration_ms = duration.as_millis();
                    match result {
                        Ok((tool_set, built_server)) => {
                            if let Err(err) = code_mode.insert_built_server(tool_set, built_server)
                            {
                                warn!(
                                    error = %err,
                                    error_debug = ?err,
                                    server.name = %server.name,
                                    server.target = %server.display_target(),
                                    duration_ms,
                                    "Failed inserting MCP server build"
                                );
                                continue;
                            }
                            info!(
                                server.name = %server.name,
                                server.target = %server.display_target(),
                                duration_ms,
                                "Initialized MCP server"
                            );
                        }
                        Err(err) => warn!(
                            error = %err,
                            error_debug = ?err,
                            server.name = %server.name,
                            server.target = %server.display_target(),
                            duration_ms,
                            "Failed initializing MCP server"
                        ),
                    }
                }
                Err(err) => warn!(error = %err, error_debug = ?err, "MCP build task failed"),
            }
        }

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
