pub mod commands;
pub mod mcp;
pub mod utils;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use crate::{
    commands::{add::AddCmd, init::InitCmd, list::ListCmd, remove::RemoveCmd, start::StartCmd},
    utils::logger::{LoggerMode, init_logger},
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
    pctx init \n  \
    pctx add my-server https://mcp.example.com\n  \
    pctx list \n  \
    pctx start --port 8080\n\
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
    pub fn logger_mode(&self) -> LoggerMode {
        match self.command {
            Commands::Start(_) => LoggerMode::Tracing,
            Commands::List(_) | Commands::Add(_) | Commands::Remove(_) | Commands::Init(_) => {
                LoggerMode::EnvLogger {
                    verbose: self.verbose,
                    quiet: self.quiet,
                }
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn handle(&self) -> anyhow::Result<()> {
        let cfg = Config::load(&self.config);

        init_logger(
            &cfg.as_ref().map(|c| c.logger.clone()).unwrap_or_default(),
            self.logger_mode(),
        );

        let _updated_cfg = match &self.command {
            Commands::Init(cmd) => cmd.handle(&self.config).await?,
            Commands::List(cmd) => cmd.handle(cfg?).await?,
            Commands::Add(cmd) => cmd.handle(cfg?, true).await?,
            Commands::Remove(cmd) => cmd.handle(cfg?)?,
            Commands::Start(cmd) => cmd.handle(cfg?).await?,
        };

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
#[command(styles=utils::styles::get_styles())]
pub enum Commands {
    /// List MCP servers and test connections
    #[command(long_about = "Lists configured MCP servers and tests the connection to each.")]
    List(ListCmd),

    /// Add an MCP server to configuration
    #[command(long_about = "Add a new MCP server to the configuration.")]
    Add(AddCmd),

    /// Remove an MCP server from configuration
    #[command(long_about = "Remove an MCP server from the configuration.")]
    Remove(RemoveCmd),

    /// Start the PCTX server
    #[command(long_about = "Start the PCTX server (exposes /mcp endpoint).")]
    Start(StartCmd),

    /// Initialize configuration file
    #[command(long_about = "Initialize pctx.json configuration file.")]
    Init(InitCmd),
}
