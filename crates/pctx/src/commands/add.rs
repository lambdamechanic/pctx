use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    mcp::client::{InitMCPClientError, init_mcp_client},
    utils::{
        prompts,
        spinner::Spinner,
        styles::{fmt_bold, fmt_dimmed, fmt_success},
    },
};
use pctx_config::{Config, server::ServerConfig};

#[derive(Debug, Clone, Parser)]
pub(crate) struct AddCmd {
    /// Unique name for this server
    pub(crate) name: String,

    /// HTTP(S) URL of the MCP server endpoint
    pub(crate) url: url::Url,

    /// Overrides any existing server under the same name &
    /// skips testing connection to the MCP server
    #[arg(long, short)]
    pub(crate) force: bool,
}

impl AddCmd {
    pub(crate) async fn handle(&self, mut cfg: Config) -> Result<Config> {
        let mut server_cfg = ServerConfig::new(self.name.clone(), self.url.clone());

        if !self.force {
            let mut sp = Spinner::new("Testing MCP connection...");

            match init_mcp_client(&self.url).await {
                Ok(client) => {
                    sp.stop_success("Successfully connected to MCP without authentication");
                    client.cancel().await?;
                }
                Err(InitMCPClientError::RequiresAuth | InitMCPClientError::RequiresOAuth) => {
                    sp.stop_and_persist("🔒", "MCP requires auth");
                    let add_auth = inquire::Confirm::new("Do you want to add auth interactively?")
                        .with_default(true)
                        .with_help_message(&format!(
                            "you can also manually update the auth configuration later in {}",
                            fmt_dimmed(cfg.path().as_str())
                        ))
                        .prompt()?;
                    if add_auth {
                        // TODO: retry connection
                        server_cfg.auth = Some(prompts::prompt_auth(&self.name)?);
                    }
                }
                Err(InitMCPClientError::Failed(msg)) => {
                    sp.stop_error(msg);
                    let add_anyway =
                        inquire::Confirm::new("Do you still want to add the MCP server?")
                            .with_default(false)
                            .prompt()?;
                    if !add_anyway {
                        anyhow::bail!("User cancelled")
                    }
                }
            }
        }

        cfg.add_server(server_cfg, self.force)?;
        cfg.save()?;
        info!(
            "{}",
            fmt_success(&format!(
                "{name} MCP Server added to {path}",
                name = fmt_bold(&self.name),
                path = fmt_dimmed(cfg.path().as_str()),
            ))
        );

        Ok(cfg)
    }
}
