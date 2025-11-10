## Telemetry and Observability

Responsibility

- Expose metrics, traces, and structured logs to observe core behavior and diagnose issues.

Key files

- `core/src/telemetry.rs` — telemetry helpers and common metrics/tags.

Recommended metrics and spans

- EventBus: published_events_total, delivered_events_total, event_dispatch_latency_seconds (histogram).
- AgentRuntime: agent_uptime_seconds, agent_mailbox_size, agent_dispatch_latency_seconds.
- ActionBroker: action_invocations_total, action_errors_total, action_latency_seconds.
- Router: routing_decisions_total, routing_no_match_total.

Tracing

- Instrument routing and capability invocation with spans that include request id, agent id, and routing decision.

## Tool Use (LLM Orchestrator) observability

Tracing targets

- `tool_orch` — logs and spans from the tool orchestrator
  - discovery: number of tools exposed and discovery latency
  - invoke: tool name, status, per-call latency (ms)
  - refine: refinement turn latency
- `action_broker` — provider registration and invoke outcomes

Suggested RUST_LOG

```
RUST_LOG=info,tool_orch=debug,action_broker=info
```

Runtime counters

- `ToolOrchestratorStats` (in-memory):
  - `total_invocations`
  - `total_tool_calls`
  - `total_tool_errors`
  - `avg_tool_latency_ms`

Exporting metrics

- Short term: log snapshots at INFO on interval or when finishing a request.
- Mid term: wire these counters into a central `MetricsCollector` or expose via an HTTP metrics endpoint (e.g., Prometheus) if needed.

Testing and validation

- Unit tests should not rely on specific metrics backends; use in-memory collectors for assertions.

---
