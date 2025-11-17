use std::time::Duration;

use anyhow::Result;
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;

use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use pctx_config::{
    Config,
    logger::LoggerFormat,
    telemetry::{ExporterConfig, Protocol},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};
use tracing_subscriber::{Layer, Registry, util::SubscriberInitExt};

use crate::utils::logger;

pub(crate) fn init_telemetry(cfg: &Config) -> Result<()> {
    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", cfg.name.clone()),
            KeyValue::new("service.version", cfg.version.clone()),
        ])
        .build();

    // build tracing provider with all configured exporters
    if cfg.telemetry.traces.enabled {
        let mut tracing_provider = SdkTracerProvider::builder().with_resource(resource);
        for exporter_cfg in &cfg.telemetry.traces.exporters {
            tracing_provider =
                tracing_provider.with_batch_exporter(create_span_exporter(exporter_cfg)?);
        }

        let provider = tracing_provider.build();
        let tracer = provider.tracer("pctx");

        layers.push(tracing_opentelemetry::layer().with_tracer(tracer).boxed());
    }

    if cfg.logger.enabled {
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

fn create_span_exporter(config: &ExporterConfig) -> Result<SpanExporter> {
    let timeout = Duration::from_secs(10);
    let endpoint = config.url.to_string();

    let exporter = match config.protocol {
        Protocol::Http => opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .with_timeout(timeout)
            .build()?,
        Protocol::Grpc => opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .with_timeout(timeout)
            .build()?,
    };

    Ok(exporter)
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
