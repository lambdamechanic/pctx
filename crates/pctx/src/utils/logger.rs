use pctx_config::logger::{LogLevel, LoggerConfig, LoggerFormat};
use std::io::Write;
use std::path::PathBuf;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

use super::jsonl_logger::{JsonlLayer, JsonlWriter};

#[derive(Debug, Clone)]
pub enum LoggerMode {
    EnvLogger { verbose: u8, quiet: bool },
    Tracing,
    Dev { log_file: PathBuf },
}

const WHITELISTED_CRATES: &[&str] = &[
    "pctx",
    "pctx_config",
    "deno_executor",
    "codegen",
    "tower_http",
    "axum",
];

pub(crate) fn init_logger(cfg: &LoggerConfig, mode: LoggerMode) {
    // Build filter string: "crate1=level,crate2=level,..."
    let level_str = match &mode {
        LoggerMode::EnvLogger { verbose, quiet } => {
            if *quiet {
                "warn"
            } else if *verbose == 0 {
                "info"
            } else if *verbose == 1 {
                "debug"
            } else {
                "trace"
            }
        }
        LoggerMode::Tracing | LoggerMode::Dev { .. } => match cfg.level {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        },
    };

    let env_filter = WHITELISTED_CRATES
        .iter()
        .map(|crate_name| format!("{crate_name}={level_str}"))
        .collect::<Vec<_>>()
        .join(",");

    match mode {
        LoggerMode::Tracing => {
            if cfg.enabled {
                let env_filter = EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(env_filter));

                if let Err(e) = tracing_subscriber::registry()
                    .with(init_tracing_layer(std::io::stdout, &cfg.format, cfg.colors))
                    .with(env_filter)
                    .try_init()
                {
                    eprintln!("pctx: Failed initializing tracing_subscriber: {e:?}");
                }
            }
        }
        LoggerMode::Dev { log_file } => {
            // For dev mode, always enable logging to JSONL file
            // Set filter to debug level to capture all relevant logs
            let env_filter = WHITELISTED_CRATES
                .iter()
                .map(|crate_name| format!("{crate_name}=debug"))
                .collect::<Vec<_>>()
                .join(",");

            let env_filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

            match JsonlWriter::new(&log_file) {
                Ok(writer) => {
                    let jsonl_layer = JsonlLayer::new(writer);

                    if let Err(e) = tracing_subscriber::registry()
                        .with(jsonl_layer)
                        .with(env_filter)
                        .try_init()
                    {
                        eprintln!(
                            "pctx: Failed initializing tracing_subscriber for dev mode: {e:?}"
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "pctx: Failed to create JSONL log file at {}: {e:?}",
                        log_file.display()
                    );
                }
            }
        }
        LoggerMode::EnvLogger { .. } => {
            let mut builder = env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or(env_filter),
            );

            // For INFO and below, only include/colorize WARN and ERROR levels
            if ["info", "warn", "error"].contains(&level_str) {
                builder.format(|buf, record| {
                    if record.level() == tracing::log::Level::Info {
                        writeln!(buf, "{}", record.args())
                    } else {
                        let log_style = buf.default_level_style(record.level());
                        writeln!(
                            buf,
                            "{log_style}[{}]{log_style:#} {}",
                            record.level(),
                            record.args()
                        )
                    }
                });
            }

            if let Err(e) = builder.try_init() {
                eprintln!("pctx: Failed initializing env_logger: {e:?}");
            }
        }
    }
}

fn init_tracing_layer<W>(
    make_writer: W,
    format: &LoggerFormat,
    colors: bool,
) -> Box<dyn Layer<Registry> + Sync + Send>
where
    W: for<'writer> fmt::MakeWriter<'writer> + Sync + Send + 'static,
{
    match format {
        LoggerFormat::Compact => fmt::Layer::default()
            .with_writer(make_writer)
            .with_ansi(colors)
            .compact()
            .boxed(),
        LoggerFormat::Pretty => fmt::Layer::default()
            .with_writer(make_writer)
            .with_ansi(colors)
            .pretty()
            .boxed(),
        LoggerFormat::Json => fmt::Layer::default()
            .with_writer(make_writer)
            .with_ansi(colors)
            .json()
            .boxed(),
    }
}
