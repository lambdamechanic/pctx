use anyhow::Result;
use pctx_config::Config;
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
};
use tabled::{
    Table,
    builder::Builder,
    settings::{
        Alignment, Color, Panel, Style, Width,
        object::{Cell, Columns, Rows},
        peaker::Priority,
        width::MinWidth,
    },
};
use terminal_size::terminal_size;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info;

use crate::{
    mcp::service::PctxMcpService,
    utils::{
        LOGO,
        styles::{fmt_cyan, fmt_dimmed},
    },
};

pub(crate) struct PctxMcpServer {
    host: String,
    port: u16,
    banner: bool,
}

impl PctxMcpServer {
    pub(crate) fn new(host: &str, port: u16, banner: bool) -> Self {
        Self {
            host: host.into(),
            port,
            banner,
        }
    }

    pub(crate) async fn serve(&self, cfg: &Config, code_mode: pctx_core::CodeMode) -> Result<()> {
        let shutdown_signal = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed graceful shutdown");
        };
        self.serve_with_shutdown(cfg, code_mode, shutdown_signal)
            .await
    }

    pub(crate) async fn serve_with_shutdown<F>(
        &self,
        cfg: &Config,
        code_mode: pctx_core::CodeMode,
        shutdown_signal: F,
    ) -> Result<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.banner(cfg, &code_mode);

        let mcp_service = PctxMcpService::new(cfg, code_mode);

        let service = StreamableHttpService::new(
            move || Ok(mcp_service.clone()),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig {
                stateful_mode: false,
                ..Default::default()
            },
        );

        let router = axum::Router::new().nest_service("/mcp", service).layer(
            ServiceBuilder::new()
                // Generate UUID if x-request-id header doesn't exist
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                // Propagate x-request-id to response headers
                .layer(PropagateRequestIdLayer::x_request_id())
                // Add tracing layer that includes request_id in spans
                .layer(TraceLayer::new_for_http().make_span_with(
                    |request: &axum::http::Request<_>| {
                        let request_id = request
                            .extensions()
                            .get::<RequestId>()
                            .map_or("unknown".to_string(), |id| {
                                id.header_value().to_str().unwrap_or("invalid").to_string()
                            });

                        tracing::error_span!(
                            "request",
                            method = %request.method(),
                            uri = %request.uri(),
                            version = ?request.version(),
                            request_id = %request_id,
                        )
                    },
                )),
        );
        let tcp_listener =
            tokio::net::TcpListener::bind(format!("{}:{}", &self.host, self.port)).await?;

        let _ = axum::serve(tcp_listener, router)
            .with_graceful_shutdown(shutdown_signal)
            .await;

        Ok(())
    }

    fn banner(&self, cfg: &pctx_config::Config, code_mode: &pctx_core::CodeMode) {
        let mcp_url = format!("http://{}:{}/mcp", self.host, self.port);
        let logo_max_length = LOGO
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let min_term_width = logo_max_length + 4; // account for padding
        let term_width = terminal_size().map(|(w, _)| w.0).unwrap_or_default() as usize;

        if self.banner && term_width >= min_term_width {
            let mut builder = Builder::default();

            builder.push_record(["Server Name", &cfg.name]);
            builder.push_record(["Server Version", &cfg.version]);
            builder.push_record(["Server URL", &mcp_url]);
            builder.push_record([
                "Tools",
                &["list_functions", "get_function_details", "execute"].join(", "),
            ]);
            builder.push_record(["Docs", &fmt_dimmed("https://github.com/portofcontext/pctx")]);

            if !code_mode.tool_sets.is_empty() {
                builder.push_record(["", ""]);

                let tool_record = |s: &codegen::ToolSet| {
                    format!(
                        "{} - {} tool{}",
                        fmt_cyan(&s.name),
                        s.tools.len(),
                        if s.tools.len() > 1 { "s" } else { "" }
                    )
                };
                builder.push_record([
                    "Upstream MCPs",
                    &code_mode
                        .tool_sets
                        .first()
                        .map(tool_record)
                        .unwrap_or_default(),
                ]);
                for s in &code_mode.tool_sets[1..] {
                    builder.push_record(["", &tool_record(s)]);
                }
            }

            let table_width = (term_width).min(80) as usize;
            let info_table = builder
                .build()
                .with(Style::empty())
                .modify(Columns::first(), Color::BOLD)
                .modify(Cell::new(2, 1), Color::FG_CYAN)
                .modify(Columns::first(), MinWidth::new(20))
                .modify(Columns::new(..2), Width::wrap((term_width - 6) / 2)) // info cols should have equal space
                .to_string();

            let logo_panel = Panel::header(format!("\n{LOGO}\n\n"));
            let logo_row = 0;
            let version_panel = Panel::header(format!(
                "pctx v{}\n\n",
                option_env!("CARGO_PKG_VERSION").unwrap_or_default()
            ));
            let version_row = 1;

            let style = Style::rounded().remove_horizontals().remove_vertical();
            let banner = Table::from_iter([[info_table]])
                .with(style)
                .with(version_panel)
                .with(logo_panel)
                .with(Alignment::center())
                .modify(Rows::single(logo_row), Color::FG_BLUE)
                .modify(Rows::single(version_row), Color::FG_BLUE | Color::BOLD)
                .with((
                    Width::wrap(table_width).priority(Priority::max(true)),
                    Width::increase(table_width).priority(Priority::min(true)),
                ))
                .to_string();

            println!("\n{banner}\n"); // tracing::info doesn't work well with colors / formatting
        }

        info!("PCTX listening at {mcp_url}...");
    }
}
