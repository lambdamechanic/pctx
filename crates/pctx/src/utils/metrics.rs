// Metrics infrastructure for future observability
#![allow(dead_code)]

use opentelemetry::metrics::{Counter, Histogram, Meter};
use std::sync::OnceLock;

static METER: OnceLock<Meter> = OnceLock::new();

pub(crate) fn init_meter() {
    METER.get_or_init(|| opentelemetry::global::meter("pctx"));
}

pub(crate) fn meter() -> &'static Meter {
    METER.get().expect("Meter not initialized")
}

// MCP Tool Operation Metrics
pub(crate) struct McpToolMetrics {
    pub list_duration: Histogram<f64>,
    pub call_duration: Histogram<f64>,
    pub calls_total: Counter<u64>,
    pub errors_total: Counter<u64>,
}

impl McpToolMetrics {
    pub(crate) fn new(meter: &Meter) -> Self {
        Self {
            list_duration: meter
                .f64_histogram("mcp.tool.list_duration_ms")
                .with_description("Duration of tools/list operations in milliseconds")
                .with_unit("ms")
                .build(),
            call_duration: meter
                .f64_histogram("mcp.tool.call_duration_ms")
                .with_description("Duration of tools/call operations in milliseconds")
                .with_unit("ms")
                .build(),
            calls_total: meter
                .u64_counter("mcp.tool.calls_total")
                .with_description("Total number of MCP tool calls")
                .build(),
            errors_total: meter
                .u64_counter("mcp.tool.errors_total")
                .with_description("Total number of MCP tool errors")
                .build(),
        }
    }
}

static MCP_TOOL_METRICS: OnceLock<McpToolMetrics> = OnceLock::new();

pub(crate) fn init_mcp_tool_metrics() {
    MCP_TOOL_METRICS.get_or_init(|| McpToolMetrics::new(meter()));
}

pub(crate) fn mcp_tool_metrics() -> Option<&'static McpToolMetrics> {
    MCP_TOOL_METRICS.get()
}
