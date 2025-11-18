# PCTX OpenTelemetry Example

This directory contains a complete setup for testing PCTX's OpenTelemetry (OTLP) functionality with traces and metrics collection.

## Overview

The Docker Compose setup provides:

- **OpenTelemetry Collector**: Receives OTLP data from PCTX and exports to downstream services
- **Tempo**: Distributed tracing backend for storing and querying traces
- **Prometheus**: Metrics storage and querying
- **Grafana**: Visualization dashboard for traces and metrics
- **Memcached**: Cache for Tempo

## Getting Started

### 1. Start the Telemetry Stack

```bash
cd examples/telemetry
docker compose up -d
```

This will start all services:

- OTLP Collector: `http://localhost:4318` (HTTP) and `localhost:4317` (gRPC)
- Grafana: `http://localhost:3000`
- Prometheus: `http://localhost:9090`
- Tempo: `http://localhost:3200`

### 2. Configure PCTX

Create a PCTX configuration file with telemetry enabled. Here's an example configuration:

```json
{
  "name": "pctx-opentelemetry",
  "version": "0.1.0",
  "servers": [
    {
      "name": "stripe",
      "url": "https://mcp.stripe.com/",
      "auth": {
        "type": "custom",
        "headers": {
          "Authorization": "Bearer ${env:STRIPE_MCP_KEY}"
        }
      }
    }
  ],
  "logger": {
    "enabled": true,
    "format": "pretty",
    "level": "info"
  },
  "telemetry": {
    "traces": {
      "enabled": true,
      "exporters": [
        {
          "name": "tempo",
          "url": "http://localhost:4318/v1/traces",
          "protocol": "http"
        }
      ]
    },
    "metrics": {
      "enabled": true,
      "exporters": [
        {
          "name": "prometheus",
          "url": "http://localhost:4318/v1/metrics",
          "protocol": "http"
        }
      ]
    }
  }
}
```

### 3. Run PCTX

Start PCTX with your configuration file:

```bash
pctx start --config path/to/your/pctx.json
```

### 4. View Telemetry Data

#### Grafana Dashboard

1. Open [http://localhost:3000](http://localhost:3000) in your browser
2. Navigate to "Explore" in the left sidebar
3. Select "Tempo" as the data source to view traces
4. Select "Prometheus" to view metrics (when enabled)

#### Prometheus

Direct access to metrics: [http://localhost:9090](http://localhost:9090)

You can also query metrics directly in Prometheus or via Grafana. Here are some example queries:

**Request rate (requests per second):**

```promql
rate(pctx_mcp_requests_total[5m])
```

**Request duration percentiles:**

```promql
histogram_quantile(0.95, rate(pctx_mcp_request_duration_bucket[5m]))
```

**Error rate:**

```promql
rate(pctx_mcp_requests_total{status="error"}[5m])
```

**Average Duration Per Tool:**

```promql
sum(rate(mcp_tool_call_duration_ms_milliseconds_sum[5m])) by (tool_name)
/
sum(rate(mcp_tool_call_duration_ms_milliseconds_count[5m])) by (tool_name)
```

**Active connections by server:**

```promql
pctx_mcp_active_connections{server="stripe"}
```

**Request count by tool:**

```promql
sum by (tool) (pctx_mcp_requests_total)
```

## Configuration Options

See the [Configuration Guide](../../docs/config.md) for details on how to configure OpenTelemetry exporters with PCTX.

## Stopping the Stack

```bash
docker compose down
```

To remove volumes and start fresh:

```bash
docker compose down -v
```

## Persistent Data

The following data is persisted in Docker volumes:

- Tempo traces: `tempo-data`
- Prometheus metrics: `prometheus-data`
- Grafana dashboards and settings: `grafana-data`

## Troubleshooting

### No traces appearing in Grafana

1. Check that PCTX is running with telemetry enabled
2. Verify the OTLP Collector is receiving data: `docker compose logs otel-collector`
3. Check Tempo logs: `docker compose logs tempo`

### Connection refused errors

Ensure all services are running:

```bash
docker compose ps
```

All services should show "Up" status.
