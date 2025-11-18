use anyhow::{Context, Result};
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;

use camino::Utf8PathBuf;
use opentelemetry_sdk::Resource;
use pctx_config::{Config, logger::LoggerFormat};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};
use tracing_subscriber::{Layer, Registry, util::SubscriberInitExt};

use crate::utils::jsonl_logger::{JsonlLayer, JsonlWriter};
use crate::utils::{logger, metrics};

pub(crate) async fn init_telemetry(cfg: &Config, json_l: Option<Utf8PathBuf>) -> Result<()> {
    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", cfg.name.clone()),
            KeyValue::new("service.version", cfg.version.clone()),
        ])
        .build();

    // build tracing provider with all configured exporters
    if cfg.telemetry.traces.enabled {
        let builder = cfg
            .telemetry
            .traces
            .tracer_provider_builder()
            .await?
            .with_resource(resource.clone());

        let tracer = builder.build().tracer("pctx");

        layers.push(tracing_opentelemetry::layer().with_tracer(tracer).boxed());
    }

    // build meter provider with all configured exporters
    if cfg.telemetry.metrics.enabled {
        let meter_provider = cfg
            .telemetry
            .metrics
            .meter_provider_builder()
            .await?
            .with_resource(resource)
            .build();

        opentelemetry::global::set_meter_provider(meter_provider);

        // Initialize metrics instruments after meter provider is set
        metrics::init_meter();
        metrics::init_mcp_tool_metrics();
    }

    if let Some(log_file) = json_l {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or(EnvFilter::new(logger::default_env_filter("debug")));
        let writer = JsonlWriter::new(&log_file)
            .context(format!("Failed to create JSONL log file at {log_file}"))?;
        layers.push(JsonlLayer::new(writer).with_filter(env_filter).boxed());
    } else if cfg.logger.enabled {
        let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(
            logger::default_env_filter(cfg.logger.level.as_str()),
        ));
        layers.push(
            init_tracing_layer(std::io::stdout, &cfg.logger.format, cfg.logger.colors)
                .with_filter(env_filter)
                .boxed(),
        );
    }

    tracing_subscriber::registry().with(layers).try_init()?;

    Ok(())
}

fn init_tracing_layer<W>(
    make_writer: W,
    format: &LoggerFormat,
    colors: bool,
) -> Box<dyn Layer<Registry> + Sync + Send>
where
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Sync + Send + 'static,
{
    match format {
        LoggerFormat::Compact => tracing_subscriber::fmt::Layer::default()
            .with_writer(make_writer)
            .with_ansi(colors)
            .compact()
            .boxed(),
        LoggerFormat::Pretty => tracing_subscriber::fmt::Layer::default()
            .with_writer(make_writer)
            .with_ansi(colors)
            .pretty()
            .boxed(),
        LoggerFormat::Json => tracing_subscriber::fmt::Layer::default()
            .with_writer(make_writer)
            .with_ansi(colors)
            .json()
            .boxed(),
    }
}
