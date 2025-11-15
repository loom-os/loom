# Distributed Tracing Implementation Summary

## 🎯 Goal

Implement end-to-end distributed tracing for the demos (starting with `trace-test`, then `market-analyst`) and close P0 Critical Gap #1.

## ✅ Completed

### 1. Rust Core – Envelope trace context (/core/src/envelope.rs)

**New fields**:

- `trace_id`: OpenTelemetry trace ID (128-bit hex)
- `span_id`: OpenTelemetry span ID (64-bit hex)
- `trace_flags`: Trace flags (8-bit hex, typically "01" for sampled)

**New methods**:

```rust
pub fn inject_trace_context(&mut self)
pub fn extract_trace_context(&self) -> bool
```

**Automatic injection points**:

- `EventBus::publish()` – injects the current span’s trace context into the event metadata before publishing.
- `ActionBroker::invoke()` – injects trace context into ActionCall headers before invoking capabilities.

### 2. Bridge – Trace propagation (/bridge/src/lib.rs)

**event_stream handling**:

- Extracts trace context from inbound events via `Envelope::from_event`.
- Calls `envelope.extract_trace_context()` *after* creating and entering the `bridge.publish` span so the span gets the correct remote parent.
- Emits spans:
    - `bridge.publish` – Python → Bridge → EventBus path
    - `bridge.forward` – EventBus → Bridge → Python agent delivery path
- Span attributes include: `agent_id`, `topic`, `event_id`, `trace_id`, `span_id`.

### 3. Python SDK – OpenTelemetry integration

**Dependencies** (pyproject.toml):

```toml
opentelemetry-api>=1.22.0
opentelemetry-sdk>=1.22.0
opentelemetry-exporter-otlp-proto-grpc>=1.22.0
```

**envelope.py**:

- Adds `trace_id` / `span_id` / `trace_flags` fields.
- `inject_trace_context()` – injects the current span’s IDs and flags into the envelope and metadata.
- `extract_trace_context()` – parses IDs and returns a remote `SpanContext` to be used as parent.

**context.py**:

- `emit()` – calls `env.inject_trace_context()` automatically so every outbound event carries trace context.

**agent.py**:

- `_run_stream()` – before invoking user `on_event`, extracts trace context from the envelope and creates an `agent.on_event` child span.
- Span attributes: `agent.id`, `event.id`, `event.type`, `topic`, `thread_id`, `correlation_id`.
- Agents now auto-initialize telemetry on construction, with defaults:
    - `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317` (if not set)
    - `OTEL_TRACE_SAMPLER=always_on` (if not set)
    - `OTEL_SERVICE_NAME=agent-{agent_id}` unless overridden
    - This can be disabled with `LOOM_TELEMETRY_AUTO=0`.

**tracing.py**:

- `init_telemetry()` – sets up OTLP exporter and `TracerProvider` (still available for manual/custom setups).
- `shutdown_telemetry()` – flushes and shuts down the provider.
- Respects `OTEL_SERVICE_NAME` and `OTEL_EXPORTER_OTLP_ENDPOINT` environment variables.

### 4. Trace Test Demo (/demo/trace-test/)

**Simplified 3‑agent linear workflow**：

```
sensor-agent → sensor.data → processor-agent → processed.data → output-agent
```

**Goals**：

- Validate full Python → Rust → Python trace propagation.
- Validate parent/child span relationships.
- Use a simple topology instead of the complex `market-analyst` fan‑out/fan‑in as a first step.

**Files**：

- `loom.toml` – project config
- `agents/sensor.py` – data producer (every 2 seconds, creates root spans)
- `agents/processor.py` – data transformer (×1.5)
- `agents/output.py` – sink/consumer

## 📋 Next Actions

### Priority 1: Dashboard integration (Roadmap TODO #5)

- Extend `FlowTracker` and `EventFlow` to carry `trace_id`.
- Surface `trace_id` in dashboard APIs and UI.
- Add a Jaeger deep‑link so clicking an event in the dashboard opens the corresponding trace.

### Priority 2: Market‑Analyst validation (Roadmap TODO #6)

- Ensure all agents (data/trend/risk/sentiment/planner) run with telemetry enabled.
- Validate fan‑out/fan‑in trace topology:
    - One root span at request entry.
    - Parallel spans for each analysis agent.
    - A planner span that either parents or links to all upstream spans.
- Confirm LLM spans are visible and correctly attributed.

### Priority 3: E2E tests and docs (Roadmap TODO #7)

- Add end‑to‑end tests that assert trace continuity across Rust Core, Bridge, and Python SDK.
- Update `docs/ROADMAP.md` to mark tracing implementation as done for core/bridge/sdk and move remaining work to dashboard + demos.
- Create a high‑level `docs/observability/TRACING.md` that points to this implementation file and shows “how to use it” for users.

## 🏗️ 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    Distributed Trace Flow                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Python Agent A                                             │
│  ┌──────────────┐                                          │
│  │ agent.emit() │ ←── inject_trace_context()              │
│  │  span_id: A1 │                                          │
│  └──────┬───────┘                                          │
│         │ gRPC ClientEvent                                 │
│         ↓                                                   │
│  ┌─────────────────────┐                                   │
│  │ Bridge              │                                   │
│  │ extract_trace_ctx() │ ←── read from Event.metadata     │
│  │ span_id: B1         │                                   │
│  │ parent: A1          │                                   │
│  └─────────┬───────────┘                                   │
│            │                                                │
│            ↓                                                │
│  ┌─────────────────────┐                                   │
│  │ EventBus.publish()  │                                   │
│  │ inject_trace_ctx()  │ ←── read from current span       │
│  │ span_id: E1         │                                   │
│  │ parent: B1          │                                   │
│  └─────────┬───────────┘                                   │
│            │                                                │
│            ↓                                                │
│  ┌─────────────────────┐                                   │
│  │ Bridge → Python B   │                                   │
│  │ extract_trace_ctx() │ ←── read from Event.metadata     │
│  │ span_id: B2         │                                   │
│  │ parent: E1          │                                   │
│  └─────────┬───────────┘                                   │
│            │                                                │
│            ↓                                                │
│  ┌──────────────────────┐                                  │
│  │ Agent B.on_event()   │                                  │
│  │ span_id: A2          │                                  │
│  │ parent: B2           │                                  │
│  └──────────────────────┘                                  │
│                                                             │
│  Jaeger displays:                                          │
│  trace_id: XXX (same across all spans)                     │
│  ├─ A1 (Python emit root span, e.g. sensor.emit_data)      │
│  │  ├─ B1 (Bridge publish)                                │
│  │  │  ├─ E1 (EventBus publish)                           │
│  │  │  │  ├─ B2 (Bridge forward)                          │
│  │  │  │  │  └─ A2 (Python agent.on_event)                │
└─────────────────────────────────────────────────────────────┘
```

## 🔑 Key code snippets

### Rust: Envelope injection

```rust
// In EventBus::publish()
let mut envelope = crate::Envelope::from_event(&event);
envelope.inject_trace_context();
envelope.attach_to_event(&mut event);
```

### Rust: Bridge extraction

```rust
// In event_stream inbound handler
let envelope = loom_core::Envelope::from_event(&ev);
envelope.extract_trace_context();

let span = tracing::info_span!(
    "bridge_publish",
    trace_id = %envelope.trace_id,
    span_id = %envelope.span_id
);
```

### Python: Agent handling

```python
# In agent._run_stream()
env = Envelope.from_proto(delivery.event)
parent_ctx = env.extract_trace_context()
if parent_ctx:
    ctx = set_span_in_context(trace.NonRecordingSpan(parent_ctx))

with tracer.start_as_current_span("agent.on_event", context=ctx):
    await self._on_event(self._ctx, delivery.topic, env)
```

## 💡 Design decisions

1. **Automatic injection** – EventBus and ActionBroker inject trace context automatically; user code rarely needs to call inject manually.
2. **Backwards compatible** – trace fields are optional and skipped when empty; existing payloads and agents continue to work.
3. **Standard format** – uses W3C Trace Context format (128‑bit `trace_id`, 64‑bit `span_id`).
4. **Envelope as carrier** – the envelope is the single place where cross‑process trace context lives, avoiding ad‑hoc headers.
5. **Environment‑based configuration** – `OTEL_SERVICE_NAME`, `OTEL_EXPORTER_OTLP_ENDPOINT`, and `OTEL_TRACE_SAMPLER` control behavior for both Rust and Python.

## 🐛 Known gaps

1. **Dashboard trace integration** – FlowTracker and the dashboard UI now have access to `trace_id`, but the UI still needs explicit trace timelines + Jaeger deep links.
2. **Market‑Analyst demo** – the demo code must be updated to rely on the new auto‑telemetry behavior and validated end‑to‑end.
3. **Docs & tests** – a user‑facing “Tracing Quickstart” and regression tests for trace propagation are still to be added.

## 📚 References

- [OpenTelemetry Python](https://opentelemetry-python.readthedocs.io/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [Jaeger UI Guide](https://www.jaegertracing.io/docs/latest/frontend-ui/)
- [ROADMAP.md](../../docs/ROADMAP.md) - P0 Critical Gap #1
---

**Status**: ✅ Core implementation (Rust + Bridge + Python SDK) is complete and validated with the `trace-test` demo.

**Next**: Integrate traces into the Dashboard UX and roll tracing out to the `market-analyst` demo and other examples.
