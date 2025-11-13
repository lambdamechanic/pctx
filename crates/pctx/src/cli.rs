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

use clap::Parser;
use pctx::{Cli, utils};
use tracing::error;

#[tokio::main]
async fn main() {
    // Install default crypto provider for rustls (required for TLS/HTTPS in Deno)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let cli = Cli::parse();

    // Initialize telemetry/logging
    utils::telemetry::init_telemetry(cli.tracing_level(), cli.telemetry_mode());

    if let Err(e) = cli.handle().await {
        error!("{e}");
        std::process::exit(1);
    }
}
