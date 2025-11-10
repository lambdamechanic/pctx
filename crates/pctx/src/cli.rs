#[cfg(all(
    not(target_env = "msvc"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod commands;
mod mcp;
mod utils;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use log::error;

use crate::commands::{add::AddCmd, list::ListCmd, remove::RemoveCmd, start::StartCmd};
use pctx_config::Config;

#[derive(Parser)]
#[command(name = "pctx")]
#[command(version)]
#[command(about = "PCTX - Code Mode MCP Gateway")]
#[command(
    long_about = "PCTX is a code mode MCP (Model Context Protocol) gateway that aggregates multiple MCP servers \
into a single endpoint and presents them as a TypeScript API for AI agents to call via code execution.\n\n\
Unlike traditional MCP implementations where agents directly call tools, PTCX exposes tools as TypeScript functions. \
This allows agents to write code that calls MCP servers more efficiently, loading only the tools they need and \
processing data in the execution environment before passing results to the model.\n\n\
PTCX supports various authentication methods including OAuth 2.1, making it easy to connect AI assistants to \
protected MCP servers while keeping credentials secure."
)]
#[command(after_help = "EXAMPLES:\n  \
    # Initialize configuration\n  \
    pctx init\n\n  \
    # Add an MCP server with OAuth 2.1 authentication\n  \
    pctx mcp add my-server https://mcp.example.com --auth oauth2\n  \
    pctx mcp auth my-server\n\n  \
    # List servers and check their health\n  \
    pctx mcp list\n\n  \
    # Start the gateway\n  \
    pctx start --port 8080\n\
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path, defaults to ./pctx.json
    #[arg(long, short = 'c', global = true, default_value_t = Config::default_path())]
    config: Utf8PathBuf,

    /// No logging except for errors
    #[arg(long, short = 'q', global = true)]
    quiet: bool,

    /// Verbose logging (-v) or trace logging (-vv)
    #[arg(long, short = 'v', action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

impl Cli {
    pub(crate) async fn handle(&self) -> Result<()> {
        let cfg = Config::load(&self.config)?;

        let _updated_cfg = match &self.command {
            Commands::List(cmd) => cmd.handle(cfg).await?,
            Commands::Add(cmd) => cmd.handle(cfg).await?,
            Commands::Remove(cmd) => cmd.handle(cfg)?,
            Commands::Start(cmd) => cmd.handle(cfg).await?,
            // Legacy
            Commands::Init => todo!(),
        };

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
#[command(styles=utils::styles::get_styles())]
enum Commands {
    /// List MCP servers in the configuration
    #[command(
        long_about = "Lists the MCP servers in the configuration and tests the connection to each."
    )]
    List(ListCmd),

    /// Add a new MCP server to the configuration
    #[command(
        long_about = "Register a new MCP server with PCTX. You will be prompted for auth if it is required."
    )]
    Add(AddCmd),

    /// Remove an MCP server from the configuration
    #[command(long_about = "Removes an MCP server from the configuration.")]
    Remove(RemoveCmd),

    /// Starts PCTX server
    #[command(
        long_about = "Starts the PCTX gateway server that aggregates all configured MCP servers. \
The gateway exposes a single MCP endpoint at /mcp that provides access to tools from all configured servers."
    )]
    Start(StartCmd),

    /// Initialize PCTX configuration directory and files
    #[command(
        long_about = "Creates the ~/.pctx directory and initializes the configuration file. \
This command is safe to run multiple times - it will not overwrite existing configuration."
    )]
    Init,
}

#[tokio::main]
async fn main() {
    // Install default crypto provider for rustls (required for TLS/HTTPS in Deno)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let cli = Cli::parse();
    // Initialize logger
    utils::logger::init_logger(cli.quiet, cli.verbose);

    if let Err(e) = cli.handle().await {
        error!("{e}");
        std::process::exit(1);
    }
}
