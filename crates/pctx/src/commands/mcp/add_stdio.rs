use std::{collections::BTreeMap, str::FromStr};

use anyhow::Result;
use clap::Parser;
use tracing::info;

use crate::{
    commands::USER_CANCELLED,
    utils::{
        spinner::Spinner,
        styles::{fmt_bold, fmt_dimmed, fmt_success},
    },
};
use pctx_config::{
    Config,
    server::{McpConnectionError, ServerConfig},
};

#[derive(Debug, Clone, Parser)]
pub struct AddStdioCmd {
    /// Unique name for this server
    pub name: String,

    /// Command to execute the MCP server
    pub command: String,

    /// Arguments to pass to the command (repeat for multiple)
    #[arg(long = "arg")]
    pub args: Vec<String>,

    /// Environment variables in KEY=VALUE format (repeat for multiple)
    #[arg(long = "env")]
    pub env: Vec<EnvVar>,

    /// Overrides any existing server under the same name &
    /// skips testing connection to the MCP server
    #[arg(long, short)]
    pub force: bool,
}

impl AddStdioCmd {
    pub(crate) async fn handle(&self, mut cfg: Config, save: bool) -> Result<Config> {
        let env = self
            .env
            .iter()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect::<BTreeMap<_, _>>();
        let server = ServerConfig::new_stdio(
            self.name.clone(),
            self.command.clone(),
            self.args.clone(),
            env,
        );

        // check for name clash
        if cfg.servers.iter().any(|s| s.name == server.name) {
            let re_add = self.force
                || inquire::Confirm::new(&format!(
                    "{} already exists, overwrite it?",
                    fmt_bold(&server.name)
                ))
                .with_default(false)
                .prompt()?;

            if !re_add {
                anyhow::bail!(USER_CANCELLED)
            }
        }

        if !self.force {
            let mut sp = Spinner::new("Testing MCP connection...");
            let connected = match server.connect().await {
                Ok(client) => {
                    sp.stop_success("Successfully connected");
                    client.cancel().await?;
                    true
                }
                Err(McpConnectionError::UnsupportedTransport(msg)) => {
                    sp.stop_error(format!("Unsupported transport: {msg}"));
                    false
                }
                Err(McpConnectionError::RequiresAuth) => {
                    sp.stop_error("MCP requires authentication");
                    false
                }
                Err(McpConnectionError::Failed(msg)) => {
                    sp.stop_error(msg);
                    false
                }
            };

            if !connected {
                let add_anyway = inquire::Confirm::new(
                    "Do you still want to add the MCP server with the current settings?",
                )
                .with_default(false)
                .prompt()?;
                if !add_anyway {
                    anyhow::bail!(USER_CANCELLED)
                }
            }
        }

        cfg.add_server(server);

        if save {
            cfg.save()?;
            info!(
                "{}",
                fmt_success(&format!(
                    "{name} upstream MCP added to {path}",
                    name = fmt_bold(&self.name),
                    path = fmt_dimmed(cfg.path().as_str()),
                ))
            );
        }

        Ok(cfg)
    }
}

/// An environment variable in the format "KEY=VALUE"
#[derive(Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

impl FromStr for EnvVar {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("Env var must be in format 'KEY=VALUE'"))?;
        let key = key.trim();
        if key.is_empty() {
            anyhow::bail!("Env var key cannot be empty");
        }

        Ok(EnvVar {
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AddStdioCmd, EnvVar};
    use pctx_config::Config;

    #[tokio::test]
    async fn test_add_stdio_adds_server() {
        let cmd = AddStdioCmd {
            name: "local".to_string(),
            command: "node".to_string(),
            args: vec!["./server.js".to_string()],
            env: vec![EnvVar {
                key: "NODE_ENV".to_string(),
                value: "test".to_string(),
            }],
            force: true,
        };

        let cfg = Config::default();
        let updated = cmd.handle(cfg, false).await.unwrap();
        let server = updated.get_server("local").expect("server added");
        let stdio = server.stdio().expect("stdio config present");
        assert_eq!(stdio.command, "node");
        assert_eq!(stdio.args, vec!["./server.js"]);
        assert_eq!(
            stdio.env.get("NODE_ENV").map(String::as_str),
            Some("test")
        );
    }
}
