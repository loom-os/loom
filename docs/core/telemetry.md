## Telemetry

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

Testing and validation

- Unit tests should not rely on specific metrics backends; use in-memory collectors for assertions.
