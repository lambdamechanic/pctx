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

#[derive(Parser)]
#[command(name = "ptx")]
#[command(about = "PTX CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
enum Commands {
    #[command(about = "Show version information")]
    Version,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) => {
            println!("ptx {}", deno_executor::version());
        }
        None => {
            println!("ptx {}", deno_executor::version());
        }
    }
}
