use std::io::Write;
use tracing::Level;

#[derive(Debug, Clone, Copy)]
pub enum TelemetryMode {
    OpenTelemetry,
    Local,
}

pub fn init_telemetry(level: Level, mode: TelemetryMode) {
    match mode {
        TelemetryMode::OpenTelemetry => {
            // TODO: Implement OpenTelemetry integration
            todo!("OpenTelemetry integration not yet implemented");
        }
        TelemetryMode::Local => {
            // Use env_logger for local development
            // This properly handles ANSI colors in terminal output
            let filter = match level {
                Level::TRACE => "trace", // Allow trace from all crates
                Level::DEBUG => "pctx=debug,pctx_config=debug", // Allow debug from pctx crates only
                Level::INFO => "pctx=info",
                Level::WARN => "pctx=warn",
                Level::ERROR => "pctx=error",
            };

            let mut builder =
                env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(filter));

            // For INFO and below, only colorize WARN and ERROR levels
            if matches!(level, Level::INFO | Level::WARN | Level::ERROR) {
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

            builder.init();
        }
    }
}
