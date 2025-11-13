# Dashboard Metrics Integration (Tracking)

This document tracks the work to replace placeholder metrics with live data in the Dashboard.

## Goals

- Provide real-time metrics for:
  - events_per_sec (publish and/or deliver rate)
  - active_agents (from AgentDirectory)
  - active_subscriptions (from EventBus)
  - tool_invocations_per_sec (from Tool orchestration / EventBus topics)
- Expose metrics via `/api/metrics` and optionally export via OpenTelemetry/Prometheus.

## Approach

1. Source-of-truth and instrumentation

- Use existing OpenTelemetry meters in EventBus (published/delivered counters, backlog gauge, latency histogram).
- Add meters where missing (e.g., tool invocations counter) and derive rates.
- For `active_agents`, query AgentDirectory; for `active_subscriptions`, query EventBus.

2. Aggregation for API

- Implement a lightweight aggregator that:
  - Computes rates over a sliding window (e.g., last 5–10 seconds) or uses OTel SDK if available.
  - Returns a snapshot `{ events_per_sec, active_agents, active_subscriptions, tool_invocations_per_sec }`.

3. Export (optional)

- If Prometheus is enabled, expose /metrics (Prom exporter) and ensure names/labels follow conventions.
- If OTLP is enabled, ensure meters are exported to collector.

## Tasks

- [ ] Wire EventBus counters into an aggregator for events_per_sec
- [ ] Add tool invocation counter (if not already present) and aggregate per-sec
- [ ] Implement getters for active_agents (AgentDirectory) and active_subscriptions (EventBus)
- [ ] Add metrics aggregator module and integrate into `/api/metrics`
- [ ] Add configuration flags and environment variables as needed
- [ ] Document the metrics (names, semantics, labels)
- [ ] Add tests (unit or integration) for aggregator behavior
- [ ] Update docs and README to reflect live metrics

## Non-goals (for now)

- Long-term historical storage and dashboards (Grafana recommended via Prometheus)
- High-cardinality per-topic time series in the API response

## References

- EventBus metrics already initialized in `core/src/event.rs`
- Dashboard README: `core/src/dashboard/README.md`
- Observability setup (Prometheus, OTEL): `observability/`
