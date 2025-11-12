# Loom Metrics Reference

Complete reference of all metrics exported by Loom Core.

## Metric Naming Convention

Loom follows OpenTelemetry semantic conventions:

- **Namespace**: `loom.` (OpenTelemetry) → `loom_loom_` (Prometheus)*
- **Component**: `<component>.<metric_name>`
- **Type suffix**: `_total` for counters, no suffix for gauges
- **Units**: Durations in seconds, sizes in bytes

\* *The double `loom_loom_` prefix occurs because the OTel Collector Prometheus exporter adds a `loom` namespace prefix to the metric names that already start with `loom`. This can be changed in `otel-collector-config.yaml`.*

## Labels

Common labels used across metrics:

| Label           | Description                      | Example Values                    |
| --------------- | -------------------------------- | --------------------------------- |
| `agent_id`      | Agent identifier                 | `planner`, `researcher`, `writer` |
| `capability`    | Capability/tool name             | `llm.generate`, `filesystem:read` |
| `server`        | MCP server name                  | `filesystem`, `brave-search`      |
| `topic`         | Event topic                      | `agent.task`, `thread.123`        |
| `route`         | Routing decision                 | `Local`, `Cloud`, `Hybrid`        |
| `status`        | Operation status                 | `ok`, `error`, `timeout`          |
| `provider_type` | Provider type                    | `0` (Built-in), `3` (MCP)         |
| `qos`           | Quality of Service level         | `realtime`, `batched`             |

## Event Bus Metrics

### `event_bus.published.total`

**Type**: Counter
**Description**: Total number of events published to the event bus
**Labels**: `topic`, `qos`

```promql
# Events published per second
rate(loom_loom_event_bus_published_total[5m])

# Events by topic
sum by (topic) (loom_loom_event_bus_published_total)
```

### `event_bus.delivered.total`

**Type**: Counter
**Description**: Total number of events successfully delivered to subscribers
**Labels**: `topic`

```promql
# Delivery rate
rate(loom_loom_event_bus_delivered_total[5m])

# Delivery success rate
sum(rate(loom_loom_event_bus_delivered_total[5m])) /
sum(rate(loom_loom_event_bus_published_total[5m]))
```

### `event_bus.dropped.total`

**Type**: Counter
**Description**: Total number of events dropped due to no subscribers or errors
**Labels**: `topic`

```promql
# Drop rate
rate(loom_loom_event_bus_dropped_total[5m])

# Topics with most drops
topk(5, sum by (topic) (loom_loom_event_bus_dropped_total))
```

### `event_bus.active_subscriptions`

**Type**: UpDownCounter (Gauge)
**Description**: Current number of active subscriptions
**Labels**: None

```promql
# Current active subscriptions
loom_loom_event_bus_active_subscriptions

# Subscription changes over time
delta(loom_loom_event_bus_active_subscriptions[1h])
```

### `event_bus.backlog.current`

**Type**: UpDownCounter (Gauge)
**Description**: Current size of the event backlog (pending deliveries)
**Labels**: None

```promql
# Current backlog
loom_loom_event_bus_backlog_current

# Backlog growth rate
rate(loom_loom_event_bus_backlog_current[5m])
```

### `event_bus.publish.latency`

**Type**: Histogram
**Description**: Time taken to publish an event (seconds)
**Labels**: `topic`, `qos`

```promql
# P50 publish latency
histogram_quantile(0.50, rate(loom_loom_event_bus_publish_latency_bucket[5m]))

# P99 publish latency
histogram_quantile(0.99, rate(loom_loom_event_bus_publish_latency_bucket[5m]))

# Average latency by topic
rate(loom_loom_event_bus_publish_latency_sum[5m]) /
rate(loom_loom_event_bus_publish_latency_count[5m])
```

## Action Broker Metrics

### `action_broker.invocations.total`

**Type**: Counter
**Description**: Total number of capability invocations
**Labels**: `capability`, `status`

```promql
# Invocations per second
rate(loom_loom_action_broker_invocations_total[5m])

# Top capabilities by usage
topk(10, sum by (capability) (loom_loom_action_broker_invocations_total))

# Success rate by capability
sum by (capability) (rate(loom_loom_action_broker_invocations_total{status="ok"}[5m])) /
sum by (capability) (rate(loom_loom_action_broker_invocations_total[5m]))
```

### `action_broker.registered_capabilities`

**Type**: UpDownCounter (Gauge)
**Description**: Number of registered capabilities
**Labels**: `provider_type`

```promql
# Total registered capabilities
sum(loom_loom_action_broker_registered_capabilities)

# Capabilities by provider type
sum by (provider_type) (loom_loom_action_broker_registered_capabilities)
```

### `action_broker.invoke.latency`

**Type**: Histogram
**Description**: Time taken to invoke a capability (seconds)
**Labels**: `capability`

```promql
# P95 invocation latency
histogram_quantile(0.95, rate(loom_loom_action_broker_invoke_latency_bucket[5m]))

# Slowest capabilities (avg latency)
topk(5,
  rate(loom_loom_action_broker_invoke_latency_sum[5m]) /
  rate(loom_loom_action_broker_invoke_latency_count[5m])
)
```

### `action_broker.cache_hits.total`

**Type**: Counter
**Description**: Total number of cache hits for capability invocations
**Labels**: `capability`

```promql
# Cache hit rate
sum(rate(loom_loom_action_broker_cache_hits_total[5m])) /
sum(rate(loom_loom_action_broker_invocations_total[5m]))
```

### `action_broker.timeouts.total`

**Type**: Counter
**Description**: Total number of capability invocation timeouts
**Labels**: `capability`

```promql
# Timeout rate
rate(loom_loom_action_broker_timeouts_total[5m])

# Capabilities with most timeouts
topk(5, sum by (capability) (loom_loom_action_broker_timeouts_total))
```

### `action_broker.errors.total`

**Type**: Counter
**Description**: Total number of capability invocation errors
**Labels**: `capability`, `error_code`

```promql
# Error rate
rate(loom_loom_action_broker_errors_total[5m])

# Error types distribution
sum by (error_code) (loom_loom_action_broker_errors_total)
```

## Router Metrics

### `router.decisions.total`

**Type**: Counter
**Description**: Total number of routing decisions made
**Labels**: `route`

```promql
# Decision rate
rate(loom_loom_router_decisions_total[5m])

# Decision distribution
sum by (route) (loom_loom_router_decisions_total)

# Local vs Cloud ratio
sum(loom_loom_router_decisions_total{route="Local"}) /
sum(loom_loom_router_decisions_total)
```

### `router.confidence`

**Type**: Histogram
**Description**: Confidence score for routing decisions (0.0 - 1.0)
**Labels**: `route`

```promql
# Average confidence
rate(loom_loom_router_confidence_sum[5m]) /
rate(loom_loom_router_confidence_count[5m])

# P50 confidence by route type
histogram_quantile(0.50,
  sum by (route, le) (rate(loom_loom_router_confidence_bucket[5m]))
)
```

### `router.estimated_latency`

**Type**: Histogram
**Description**: Estimated latency for routing decisions (milliseconds)
**Labels**: `route`

```promql
# Average estimated latency
rate(loom_loom_router_estimated_latency_sum[5m]) /
rate(loom_loom_router_estimated_latency_count[5m])
```

### `router.estimated_cost`

**Type**: Histogram
**Description**: Estimated cost for routing decisions
**Labels**: `route`

```promql
# Average estimated cost
rate(loom_loom_router_estimated_cost_sum[5m]) /
rate(loom_loom_router_estimated_cost_count[5m])
```

### `router.policy_violations.total`

**Type**: Counter
**Description**: Total number of routing policy violations
**Labels**: `policy_type`

```promql
# Policy violation rate
rate(loom_loom_router_policy_violations_total[5m])

# Violations by type
sum by (policy_type) (loom_loom_router_policy_violations_total)
```

## Agent Runtime Metrics

### `agent_runtime.agents.active`

**Type**: UpDownCounter (Gauge)
**Description**: Number of currently active agents
**Labels**: None

```promql
# Current active agents
loom_loom_agent_runtime_agents_active

# Agent count over time
loom_loom_agent_runtime_agents_active[1h]
```

### `agent_runtime.agents.created`

**Type**: Counter
**Description**: Total number of agents created
**Labels**: None

```promql
# Agent creation rate
rate(loom_loom_agent_runtime_agents_created[5m])
```

### `agent_runtime.agents.deleted`

**Type**: Counter
**Description**: Total number of agents deleted
**Labels**: None

```promql
# Agent deletion rate
rate(loom_loom_agent_runtime_agents_deleted[5m])

# Agent churn rate
(rate(loom_loom_agent_runtime_agents_created[5m]) +
 rate(loom_loom_agent_runtime_agents_deleted[5m])) / 2
```

### `agent_runtime.subscriptions.total`

**Type**: Counter
**Description**: Total number of topic subscriptions created
**Labels**: None

```promql
# Subscription creation rate
rate(loom_loom_agent_runtime_subscriptions_total[5m])
```

### `agent_runtime.unsubscriptions.total`

**Type**: Counter
**Description**: Total number of topic unsubscriptions
**Labels**: None

```promql
# Unsubscription rate
rate(loom_loom_agent_runtime_unsubscriptions_total[5m])
```

## Agent Instance Metrics

### `agent.events.processed`

**Type**: Counter
**Description**: Number of events processed by agent
**Labels**: `agent_id`

```promql
# Event processing rate by agent
rate(loom_loom_agent_events_processed[5m])

# Busiest agents
topk(5, sum by (agent_id) (loom_loom_agent_events_processed))
```

### `agent.actions.executed`

**Type**: Counter
**Description**: Number of actions executed by agent
**Labels**: `agent_id`, `action_type`, `status`

```promql
# Action execution rate
rate(loom_loom_agent_actions_executed[5m])

# Action success rate by agent
sum by (agent_id) (rate(loom_loom_agent_actions_executed{status="ok"}[5m])) /
sum by (agent_id) (rate(loom_loom_agent_actions_executed[5m]))
```

### `agent.event.latency`

**Type**: Histogram
**Description**: Time taken to process an event (seconds)
**Labels**: `agent_id`

```promql
# P99 event processing latency by agent
histogram_quantile(0.99,
  sum by (agent_id, le) (rate(loom_loom_agent_event_latency_bucket[5m]))
)
```

### `agent.routing.decisions`

**Type**: Counter
**Description**: Number of routing decisions made by agent
**Labels**: `agent_id`, `route`

```promql
# Routing decisions by agent
sum by (agent_id, route) (loom_loom_agent_routing_decisions)
```

## MCP Manager Metrics

### `mcp_manager.servers.active`

**Type**: UpDownCounter (Gauge)
**Description**: Number of active MCP server connections
**Labels**: None

```promql
# Current active MCP servers
loom_loom_mcp_manager_servers_active
```

### `mcp_manager.servers.connected`

**Type**: Counter
**Description**: Total number of MCP servers connected
**Labels**: `server`

```promql
# Connection rate
rate(loom_loom_mcp_manager_servers_connected[5m])

# Connections by server
sum by (server) (loom_loom_mcp_manager_servers_connected)
```

### `mcp_manager.servers.disconnected`

**Type**: Counter
**Description**: Total number of MCP servers disconnected
**Labels**: `server`

```promql
# Disconnection rate
rate(loom_loom_mcp_manager_servers_disconnected[5m])
```

### `mcp_manager.tools.registered`

**Type**: Counter
**Description**: Total number of MCP tools registered
**Labels**: `server`

```promql
# Tools registered by server
sum by (server) (loom_loom_mcp_manager_tools_registered)

# Total registered tools
sum(loom_loom_mcp_manager_tools_registered)
```

### `mcp_manager.reconnections.total`

**Type**: Counter
**Description**: Total number of reconnection attempts
**Labels**: `server`

```promql
# Reconnection rate
rate(loom_loom_mcp_manager_reconnections_total[5m])

# Servers with most reconnections
topk(5, sum by (server) (loom_loom_mcp_manager_reconnections_total))
```

## Tool Orchestrator Metrics

### `tool_orchestrator.runs.total`

**Type**: Counter
**Description**: Total number of orchestration runs
**Labels**: None

```promql
# Orchestration rate
rate(loom_loom_tool_orchestrator_runs_total[5m])
```

### `tool_orchestrator.tool_calls.total`

**Type**: Counter
**Description**: Total number of tool calls made
**Labels**: None

```promql
# Tool call rate
rate(loom_loom_tool_orchestrator_tool_calls_total[5m])

# Average tools per orchestration run
rate(loom_loom_tool_orchestrator_tool_calls_total[5m]) /
rate(loom_loom_tool_orchestrator_runs_total[5m])
```

### `tool_orchestrator.tool_errors.total`

**Type**: Counter
**Description**: Total number of tool call errors
**Labels**: None

```promql
# Tool error rate
rate(loom_loom_tool_orchestrator_tool_errors_total[5m])

# Tool success rate
(rate(loom_loom_tool_orchestrator_tool_calls_total[5m]) -
 rate(loom_loom_tool_orchestrator_tool_errors_total[5m])) /
rate(loom_loom_tool_orchestrator_tool_calls_total[5m])
```

### `tool_orchestrator.refine_cycles.total`

**Type**: Counter
**Description**: Total number of refinement cycles executed
**Labels**: None

```promql
# Refine cycle rate
rate(loom_loom_tool_orchestrator_refine_cycles_total[5m])
```

### `tool_orchestrator.discovery.latency`

**Type**: Histogram
**Description**: Time taken to discover tools (seconds)
**Labels**: None

```promql
# P95 discovery latency
histogram_quantile(0.95, rate(loom_loom_tool_orchestrator_discovery_latency_bucket[5m]))
```

### `tool_orchestrator.tool.latency`

**Type**: Histogram
**Description**: Time taken for individual tool calls (seconds)
**Labels**: None

```promql
# P99 tool call latency
histogram_quantile(0.99, rate(loom_loom_tool_orchestrator_tool_latency_bucket[5m]))
```

### `tool_orchestrator.llm.latency`

**Type**: Histogram
**Description**: Time taken for LLM calls (seconds)
**Labels**: None

```promql
# Average LLM latency
rate(loom_loom_tool_orchestrator_llm_latency_sum[5m]) /
rate(loom_loom_tool_orchestrator_llm_latency_count[5m])
```

## Useful Dashboards

### System Overview

```promql
# Active components
loom_loom_agent_runtime_agents_active
loom_loom_mcp_manager_servers_active
loom_loom_action_broker_registered_capabilities

# Throughput
rate(loom_loom_event_bus_published_total[5m])
rate(loom_loom_action_broker_invocations_total[5m])

# Error rates
rate(loom_loom_event_bus_dropped_total[5m])
rate(loom_loom_action_broker_errors_total[5m])
```

### Performance Dashboard

```promql
# Latency percentiles
histogram_quantile(0.50, rate(loom_loom_event_bus_publish_latency_bucket[5m]))
histogram_quantile(0.95, rate(loom_loom_action_broker_invoke_latency_bucket[5m]))
histogram_quantile(0.99, rate(loom_loom_agent_event_latency_bucket[5m]))

# Backpressure indicators
loom_loom_event_bus_backlog_current
rate(loom_loom_action_broker_timeouts_total[5m])
```

### Agent Activity Dashboard

```promql
# Agent metrics
loom_loom_agent_runtime_agents_active
rate(loom_loom_agent_events_processed[5m])
rate(loom_loom_agent_actions_executed[5m])

# Per-agent latency
histogram_quantile(0.99,
  sum by (agent_id, le) (rate(loom_loom_agent_event_latency_bucket[5m]))
)
```

## Alerting Rules

Example Prometheus alerting rules:

```yaml
groups:
  - name: loom_alerts
    rules:
      - alert: HighEventDropRate
        expr: rate(loom_loom_event_bus_dropped_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High event drop rate detected

      - alert: HighErrorRate
        expr: >
          rate(loom_loom_action_broker_errors_total[5m]) /
          rate(loom_loom_action_broker_invocations_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: Action broker error rate above 5%

      - alert: MCPServerDown
        expr: loom_loom_mcp_manager_servers_active == 0
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: No MCP servers connected

      - alert: HighLatency
        expr: >
          histogram_quantile(0.99,
            rate(loom_loom_action_broker_invoke_latency_bucket[5m])
          ) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: P99 invocation latency above 5 seconds
```

---

**See Also:**
- [README.md](./README.md) - Main documentation
- [Prometheus Query Documentation](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [PromQL Examples](https://prometheus.io/docs/prometheus/latest/querying/examples/)
