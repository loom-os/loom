# Quick Start Guide

Get Loom observability up and running in 5 minutes.

## Prerequisites

- Docker and Docker Compose installed
- Loom Core source code

## Step 1: Start Services (30 seconds)

```bash
cd observability
docker compose -f docker-compose.observability.yaml up -d
```

Wait 10-15 seconds for all services to initialize.

## Step 2: Verify Health (10 seconds)

```bash
# Check all containers are running
docker compose ps

# Verify OTel Collector
curl http://localhost:13133
# Should return: {"status":"Server available",...}
```

## Step 3: Run Loom with Telemetry (2 minutes)

```bash
cd ../core

# Enable OpenTelemetry
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=loom-example

# Run example (let it complete the full cycle ~30 seconds)
cargo run --example mcp_integration
```

**Important**: Metrics export every 10 seconds. Wait for the program to finish.

## Step 4: View Data (2 minutes)

### Traces (Jaeger)

```bash
open http://localhost:16686
```

1. Select Service: `loom-example`
2. Click **Find Traces**
3. Click any trace to see details

**You should see:**

- MCP server connection spans
- Tool registration spans (14 tools)
- Event publish spans
- Detailed timing and attributes

### Metrics (Prometheus)

```bash
open http://localhost:9090
```

1. Query: `loom_loom_mcp_manager_servers_active`
2. Click **Execute**
3. Switch to **Graph** tab

**You should see:** Value = 1 (one active MCP server)

Try these queries:

```promql
loom_loom_action_broker_registered_capabilities
loom_loom_mcp_manager_tools_registered
loom_loom_event_bus_published_total
```

### Dashboards (Grafana)

```bash
open http://localhost:3000
```

1. Login: `admin` / `admin`
2. Skip or change password
3. Navigate: **Dashboards** → **Loom** folder
4. Open: **Loom Overview**

**You should see:**

- Active agents, MCP servers
- Event throughput graphs
- Action broker latency
- Routing decisions

## One-Line Verification

```bash
# Quick check: metrics are being exported
sleep 15 && curl -s http://localhost:8889/metrics | grep -c "^loom_"
# Should return a number > 0
```

## Troubleshooting

### No traces in Jaeger

```bash
# Check Loom logs for:
# "INFO telemetry: OpenTelemetry initialized successfully"

# Verify collector
docker logs loom-otel-collector | tail -20
```

### No metrics in Prometheus

```bash
# Check Prometheus targets (should be "UP")
open http://localhost:9090/targets

# Verify metrics endpoint
curl http://localhost:8889/metrics | grep loom | head -5
```

### Services won't start

```bash
# Clean up and retry
docker compose -f docker-compose.observability.yaml down
docker rm -f $(docker ps -aq --filter name=loom-) 2>/dev/null
docker compose -f docker-compose.observability.yaml up -d
```

## Cleanup

```bash
# Stop services
docker compose -f docker-compose.observability.yaml down

# Remove data volumes
docker compose -f docker-compose.observability.yaml down -v
```

## Next Steps

- **[README.md](./README.md)** - Full documentation and architecture
- **[METRICS.md](./METRICS.md)** - Complete metrics reference with PromQL examples
- **[Configuration](./README.md#configuration)** - Customize sampling, retention, etc.

## Common Queries

```promql
# Event throughput
rate(loom_loom_event_bus_published_total[5m])

# P99 latency
histogram_quantile(0.99, rate(loom_loom_action_broker_invoke_latency_bucket[5m]))

# Error rate
rate(loom_loom_action_broker_errors_total[5m]) /
rate(loom_loom_action_broker_invocations_total[5m])
```

---

**Total Time**: ~5 minutes

For detailed information, see [README.md](./README.md).
