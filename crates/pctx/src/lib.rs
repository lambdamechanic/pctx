pub mod commands;
pub mod utils;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use crate::utils::{
    logger::init_cli_logger,
    telemetry::{init_telemetry, init_telemetry_minimal},
};
use pctx_config::Config;

#[derive(Parser)]
#[command(name = "pctx")]
#[command(version)]
#[command(about = "PCTX - Code Mode MCP")]
#[command(
    long_about = "PCTX aggregates multiple MCP servers into a single endpoint, exposing them as a TypeScript API \
for AI agents to call via code execution."
)]
#[command(after_help = "EXAMPLES:\n  \
    # MCP mode (with pctx.json configuration)\n  \
    pctx mcp init \n  \
    pctx mcp add my-server https://mcp.example.com\n  \
    pctx mcp dev\n\n  \
    # Agent mode (REST API + WebSocket, no config)\n  \
    pctx agent start\n\
")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Config file path, defaults to ./pctx.json
    #[arg(long, short = 'c', global = true, default_value_t = Config::default_path())]
    pub config: Utf8PathBuf,

    /// No logging except for errors
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    /// Verbose logging (-v) or trace logging (-vv)
    #[arg(long, short = 'v', action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
}

impl Cli {
    fn cli_logger(&self) -> bool {
        !matches!(
            &self.command,
            Commands::Mcp(McpCommands::Start(_) | McpCommands::Dev(_))
        )
    }

    fn json_l(&self) -> Option<Utf8PathBuf> {
        if let Commands::Mcp(McpCommands::Dev(dev)) = &self.command {
            Some(dev.log_file.clone())
        } else {
            None
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn handle(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Mcp(mcp_cmd) => self.handle_mcp(mcp_cmd).await,
            Commands::Agent(agent_cmd) => self.handle_agent(agent_cmd).await,
        }
    }

    async fn handle_mcp(&self, cmd: &McpCommands) -> anyhow::Result<()> {
        let cfg = Config::load(&self.config);

        if self.cli_logger() {
            init_cli_logger(self.verbose, self.quiet);
        } else if let Ok(c) = &cfg {
            init_telemetry(c, self.json_l()).await?;
        }

        let _updated_cfg = match cmd {
            McpCommands::Init(cmd) => cmd.handle(&self.config).await?,
            McpCommands::List(cmd) => cmd.handle(cfg?).await?,
            McpCommands::Add(cmd) => cmd.handle(cfg?, true).await?,
            McpCommands::Remove(cmd) => cmd.handle(cfg?)?,
            McpCommands::Start(cmd) => cmd.handle(cfg?).await?,
            McpCommands::Dev(cmd) => cmd.handle(cfg?).await?,
        };

        Ok(())
    }

    async fn handle_agent(&self, cmd: &AgentCommands) -> anyhow::Result<()> {
        // Agent mode doesn't need config file
        match cmd {
            AgentCommands::Start(start_cmd) => {
                // Init minimal telemetry with optional JSONL logging
                init_telemetry_minimal(self.json_l()).await?;
                start_cmd.handle().await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
#[command(styles=utils::styles::get_styles())]
pub enum Commands {
    /// MCP server commands (with pctx.json configuration)
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Agent mode commands (REST API + WebSocket, no config file)
    #[command(subcommand)]
    Agent(AgentCommands),
}

#[derive(Debug, Subcommand)]
pub enum McpCommands {
    /// Initialize pctx.json configuration file
    #[command(long_about = "Initialize pctx.json configuration file.")]
    Init(commands::mcp::InitCmd),

    /// List MCP servers and test connections
    #[command(long_about = "Lists configured MCP servers and tests the connection to each.")]
    List(commands::mcp::ListCmd),

    /// Add an MCP server to configuration
    #[command(long_about = "Add a new MCP server to the configuration.")]
    Add(commands::mcp::AddCmd),

    /// Remove an MCP server from configuration
    #[command(long_about = "Remove an MCP server from the configuration.")]
    Remove(commands::mcp::RemoveCmd),

    /// Start the PCTX MCP server
    #[command(long_about = "Start the PCTX MCP server (exposes /mcp endpoint).")]
    Start(commands::mcp::StartCmd),

    /// Start the PCTX MCP server with terminal UI
    #[command(
        long_about = "Start the PCTX MCP server in development mode with an interactive terminal UI with data and logging."
    )]
    Dev(commands::mcp::DevCmd),
}

#[derive(Debug, Subcommand)]
pub enum AgentCommands {
    /// Start agent mode (REST API + WebSocket)
    #[command(
        long_about = "Start agent mode with REST API and WebSocket server. No tools preloaded - use REST API to register tools and MCP servers dynamically."
    )]
    Start(commands::agent::StartCmd),
}
