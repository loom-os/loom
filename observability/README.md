# Loom Observability Stack

Complete OpenTelemetry-based observability solution for Loom, providing distributed tracing, metrics collection, and visualization through industry-standard tools.

## 📚 Documentation

- **[QUICKSTART.md](./QUICKSTART.md)** - Get started in 5 minutes
- **[METRICS.md](./METRICS.md)** - Complete metrics reference with PromQL examples
- **[FILES.md](./FILES.md)** - Directory structure and file descriptions
- **[Configuration Files](#components)** - YAML configurations for all services

## 🎯 Quick Start (5 Minutes)

### 1. Start the Observability Stack

```bash
cd observability
docker compose -f docker-compose.observability.yaml up -d
```

Wait 10-15 seconds for all services to start.

### 2. Run Loom with Telemetry Enabled

```bash
cd ../core
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=loom-example
cargo run --example mcp_integration
```

### 3. View Your Data

- **Jaeger** (Distributed Tracing): http://localhost:16686
- **Prometheus** (Metrics): http://localhost:9090
- **Grafana** (Dashboards): http://localhost:3000 (admin/admin)

## 📊 Architecture

```
┌─────────────┐
│  Loom Core  │
│   (Rust)    │
└──────┬──────┘
       │ OTLP gRPC (4317)
       │ Traces + Metrics
       ▼
┌──────────────────────┐
│  OTel Collector      │
│  • Receives          │
│  • Processes         │
│  • Exports           │
└───┬──────────────┬───┘
    │ Traces       │ Metrics (Prometheus format)
    │              │
    ▼              ▼
┌─────────┐   ┌─────────────┐
│ Jaeger  │   │ Prometheus  │
│  (UI)   │   │   (TSDB)    │
└─────────┘   └──────┬──────┘
                     │
                     ▼
              ┌──────────────┐
              │   Grafana    │
              │  (Dashboards)│
              └──────────────┘
```

## 🛠 Components

### OpenTelemetry Collector

Central hub for telemetry data collection and export.

- **Receives**: OTLP traces and metrics from Loom Core
- **Processes**: Batching, sampling, resource attribution
- **Exports**: To Jaeger (traces) and Prometheus (metrics)
- **Endpoints**:
  - OTLP gRPC: `localhost:4317`
  - OTLP HTTP: `localhost:4318`
  - Health check: `localhost:13133`
  - Metrics: `localhost:8889`

**Configuration**: [`otel-collector-config.yaml`](./otel-collector-config.yaml)

### Jaeger

Distributed tracing UI for visualizing request flows.

- **Purpose**: View end-to-end traces across Loom components
- **Access**: http://localhost:16686
- **Features**:
  - Trace search by service, operation, tags
  - Timeline visualization
  - Dependency graphs
  - Performance analysis

### Prometheus

Time-series database for metrics storage and querying.

- **Purpose**: Store and query Loom metrics
- **Access**: http://localhost:9090
- **Features**:
  - PromQL query language
  - Built-in graphing
  - Alerting (via Alertmanager)
  - Service discovery

**Configuration**: [`prometheus.yml`](./prometheus.yml)

### Grafana

Unified visualization platform for metrics and traces.

- **Purpose**: Create dashboards and explore telemetry data
- **Access**: http://localhost:3000
- **Credentials**: `admin` / `admin` (change on first login)
- **Features**:
  - Pre-configured Prometheus data source
  - Pre-configured Jaeger data source
  - Custom dashboard creation
  - Alerting

**Configuration**: [`grafana/provisioning/`](./grafana/provisioning/)

## 📈 Available Metrics

Loom Core exports comprehensive metrics for all major components:

### Event Bus

- `loom_event_bus_published_total` - Total events published
- `loom_event_bus_delivered_total` - Total events delivered
- `loom_event_bus_dropped_total` - Total events dropped
- `loom_event_bus_active_subscriptions` - Active subscriptions count
- `loom_event_bus_publish_latency` - Publish latency histogram

### Action Broker

- `loom_action_broker_invocations_total` - Total capability invocations
- `loom_action_broker_registered_capabilities` - Registered capabilities count
- `loom_action_broker_invoke_latency` - Invocation latency histogram
- `loom_action_broker_cache_hits_total` - Cache hit count
- `loom_action_broker_timeouts_total` - Timeout count
- `loom_action_broker_errors_total` - Error count

### Router

- `loom_router_decisions_total` - Routing decisions by type
- `loom_router_confidence` - Confidence score histogram
- `loom_router_estimated_latency` - Estimated latency histogram
- `loom_router_estimated_cost` - Estimated cost histogram
- `loom_router_policy_violations_total` - Policy violations count

### Agent Runtime

- `loom_agent_runtime_agents_active` - Active agents count
- `loom_agent_runtime_agents_created` - Total agents created
- `loom_agent_runtime_subscriptions_total` - Total subscriptions

### MCP Manager

- `loom_mcp_manager_servers_active` - Active MCP server connections
- `loom_mcp_manager_tools_registered` - Total MCP tools registered
- `loom_mcp_manager_reconnections_total` - Reconnection attempts

### Tool Orchestrator

- `loom_tool_orchestrator_runs_total` - Orchestration runs
- `loom_tool_orchestrator_tool_calls_total` - Tool invocations
- `loom_tool_orchestrator_tool_latency` - Tool call latency

> **Note**: Metrics have a `loom_loom_` prefix due to namespace configuration. See [Configuration](#configuration) to customize.

## 🔍 Distributed Tracing

Loom automatically instruments key operations with spans:

- **Event Bus**: `publish`, `subscribe`, `unsubscribe`
- **Action Broker**: `register_provider`, `invoke`
- **Router**: `route`
- **Agent Runtime**: `create_agent`, `delete_agent`, `subscribe_agent`
- **Agent Instance**: `run`, `route_event`, `execute_action`
- **MCP Manager**: `add_server`, `remove_server`, `register_tools`
- **Tool Orchestrator**: `run`

Each span includes:

- Operation identifiers (event_id, call_id, agent_id)
- Timing information
- Success/failure status
- Error details (when applicable)
- Custom attributes

## ⚙️ Configuration

### Environment Variables

Configure OpenTelemetry in Loom:

| Variable                      | Default                 | Description                  |
| ----------------------------- | ----------------------- | ---------------------------- |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OTLP collector endpoint      |
| `OTEL_SERVICE_NAME`           | `loom-core`             | Service name in traces       |
| `OTEL_TRACE_SAMPLER`          | `always_on`             | Trace sampling strategy      |
| `DEPLOYMENT_ENV`              | `development`           | Deployment environment label |

### Sampling Strategies

Control trace volume:

```bash
# Sample 100% (development)
export OTEL_TRACE_SAMPLER=always_on

# Sample 10% (production)
export OTEL_TRACE_SAMPLER=traceidratio=0.1

# Disable tracing
export OTEL_TRACE_SAMPLER=always_off
```

### Customizing the Stack

#### Change Metric Export Interval

Edit `otel-collector-config.yaml`:

```yaml
exporters:
  prometheus:
    endpoint: 0.0.0.0:8889
    namespace: loom
```

Change metric namespace or remove double prefix:

```yaml
exporters:
  prometheus:
    endpoint: 0.0.0.0:8889
    namespace: "" # Remove namespace to avoid loom_loom_ prefix
```

#### Adjust Prometheus Retention

Edit `docker-compose.observability.yaml`:

```yaml
prometheus:
  command:
    - "--storage.tsdb.retention.time=30d" # Keep data for 30 days
```

#### Configure Grafana Dashboards

Add custom dashboards to `grafana/dashboards/`:

```json
{
  "dashboard": {
    "title": "My Custom Dashboard",
    "panels": [...]
  }
}
```

Grafana will automatically load dashboards from this directory.

## 🔬 Example Queries

### Prometheus (PromQL)

```promql
# Event throughput (events/sec)
rate(loom_loom_event_bus_published_total[5m])

# P99 action invocation latency
histogram_quantile(0.99, rate(loom_loom_action_broker_invoke_latency_bucket[5m]))

# Active agents over time
loom_loom_agent_runtime_agents_active

# Tool invocation success rate
sum(rate(loom_loom_action_broker_invocations_total{status="ok"}[5m])) /
sum(rate(loom_loom_action_broker_invocations_total[5m]))

# MCP tools by server
sum by (server) (loom_loom_mcp_manager_tools_registered)
```

### Jaeger

Search traces:

- By service: `service=loom-example`
- By operation: `publish`, `invoke`, `add_server`
- By tag: `agent_id=planner`, `capability=llm.generate`
- By duration: `>100ms`

## 🐛 Troubleshooting

### No Traces in Jaeger

1. **Check Loom logs** for initialization message:

   ```
   INFO telemetry: OpenTelemetry initialized successfully
   ```

2. **Verify OTel Collector is running**:

   ```bash
   curl http://localhost:13133
   docker logs loom-otel-collector
   ```

3. **Check sampling configuration**:
   ```bash
   echo $OTEL_TRACE_SAMPLER  # Should be "always_on" or "traceidratio=X"
   ```

### No Metrics in Prometheus

1. **Check Prometheus targets**:

   - Visit http://localhost:9090/targets
   - Both targets should show "UP"

2. **Verify OTel Collector metrics exporter**:

   ```bash
   curl http://localhost:8889/metrics | grep loom
   ```

3. **Wait for export cycle**:

   - Metrics export every 10 seconds
   - Wait at least 15 seconds after starting Loom

4. **Check for double prefix**:
   - Metrics appear as `loom_loom_*` due to namespace configuration
   - This is expected behavior

### Grafana Shows No Data

1. **Verify Prometheus data source**:

   - Grafana → Configuration → Data Sources → Prometheus
   - URL should be: `http://prometheus:9090`

2. **Check Prometheus has data**:

   ```bash
   curl 'http://localhost:9090/api/v1/query?query=loom_loom_mcp_manager_servers_active'
   ```

3. **Refresh and retry**:
   - Clear browser cache
   - Try a different time range

### Port Conflicts

If ports are already in use:

```bash
# Find and stop conflicting containers
docker ps | grep -E "4317|16686|9090|3000"

# Remove old containers
docker compose -f docker-compose.observability.yaml down
docker rm -f $(docker ps -aq --filter name=loom-)
```

### Container Restart Loops

Check OTel Collector logs:

```bash
docker logs loom-otel-collector

# Common issues:
# - Invalid configuration syntax
# - Deprecated exporter (use 'debug' instead of 'logging')
# - Port binding failures
```

## 🚀 Production Considerations

### 1. Sampling

Reduce overhead with trace sampling:

```bash
export OTEL_TRACE_SAMPLER=traceidratio=0.1  # 10% sampling
```

### 2. Resource Limits

Edit `otel-collector-config.yaml`:

```yaml
processors:
  memory_limiter:
    limit_mib: 1024 # Increase for high-volume
    spike_limit_mib: 256
```

### 3. Retention

Configure Prometheus retention:

```yaml
prometheus:
  command:
    - "--storage.tsdb.retention.time=30d"
    - "--storage.tsdb.retention.size=50GB"
```

### 4. Security

- Enable TLS for OTLP endpoint
- Secure Grafana with proper authentication
- Use Prometheus authentication proxy
- Network isolation with Docker networks

### 5. High Availability

- Run multiple OTel Collector instances
- Use Prometheus federation or Thanos
- Implement Alertmanager for alerting

### 6. Scaling

For high-throughput scenarios:

- Use tail-based sampling
- Implement metric aggregation
- Deploy separate collectors for traces/metrics
- Use remote storage for Prometheus

## 📚 Additional Resources

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)

## 🧹 Cleanup

Stop services:

```bash
docker compose -f docker-compose.observability.yaml down
```

Remove all data (volumes):

```bash
docker compose -f docker-compose.observability.yaml down -v
```

Remove images:

```bash
docker rmi otel/opentelemetry-collector:latest \
           jaegertracing/all-in-one:latest \
           prom/prometheus:latest \
           grafana/grafana:latest
```

## 🤝 Support

For issues or questions:

1. Check the troubleshooting section above
2. Review OTel Collector logs: `docker logs loom-otel-collector`
3. Verify Loom telemetry initialization in application logs
4. Open an issue in the Loom repository

---

**Next Steps:**

- Explore traces in Jaeger to understand request flows
- Create custom Grafana dashboards for your use cases
- Set up alerting rules in Prometheus
- Integrate with your existing observability platform
